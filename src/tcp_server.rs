use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, SendError, Sender, TryRecvError};
use std::sync::{Arc, Mutex, RwLock};

use byteorder::{LittleEndian, ReadBytesExt};
use lazy_static::lazy_static;
use message_io::network::{Endpoint, NetEvent, Transport};
use message_io::node::{self};
use mumble_link::{MumbleLink, Position};
use serde::{Deserialize, Serialize};

use crate::app::{FromGuiToServer, ToGUI};
use crate::maniaplanet_telemetry::run_mp_telemetry_loop;
use crate::VISIBLE;

const DEFAULT_PORT: u16 = 46323;

#[derive(Debug, Deserialize, Serialize, Copy, Clone)]
pub struct MPos {
    /// The character's position in space.
    pub pos: [f32; 3],
    /// A unit vector pointing out of the character's eyes.
    pub dir: [f32; 3],
    /// A unit vector pointing out of the top of the character's head.
    pub up: [f32; 3],
}

impl MPos {
    pub fn new(position: [f32; 3], front: [f32; 3], top: [f32; 3]) -> Self {
        MPos {
            pos: position,
            dir: front,
            up: top,
        }
    }

    pub fn example(x: f32) -> Self {
        MPos {
            pos: [x, x, x],
            dir: [x, x, x],
            up: [x, x, x],
        }
    }
}

impl From<Position> for MPos {
    fn from(pos: Position) -> Self {
        MPos {
            pos: pos.position,
            dir: pos.front,
            up: pos.top,
        }
    }
}

impl From<MPos> for Position {
    fn from(pos: MPos) -> Self {
        Position {
            position: pos.pos,
            front: pos.dir,
            top: pos.up,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum FromTM {
    NetConnected(SocketAddr, bool),
    NetDisconnected(SocketAddr),
    NetAccepted(SocketAddr),
    Positions { p: MPos, c: MPos }, // serializes as `{"Positions": {"p": {"pos": [1.0, 1.0, 1.0], "dir": [1.0, 1.0, 1.0], "up": [1.0, 1.0, 1.0]}, "c": {"pos": [2.0, 2.0, 2.0], "dir": [2.0, 2.0, 2.0], "up": [2.0, 2.0, 2.0]}}`
    PlayerDetails(String, String),
    ServerDetails(String, String),
    LeftServer(),
    Ping(),
}

impl FromTM {
    pub fn get_pos_p(&self) -> Option<&MPos> {
        match self {
            FromTM::Positions { p, c: _ } => Some(p),
            _ => None,
        }
    }

    pub fn get_pos_c(&self) -> Option<&MPos> {
        match self {
            FromTM::Positions { p: _, c } => Some(c),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ToTM {
    ConnectedStatus(bool),
    Ping(),
    LinkAppInfo {
        version: String,
        options: Vec<(String, String)>,
    },
    ShutdownNow {},
}

type LE = LittleEndian;

lazy_static! {
    pub static ref LAST_CONTEXT: Mutex<String> = Mutex::new(String::new());
    pub static ref TCP_HANDLER: Arc<RwLock<Option<node::NodeHandler<()>>>> = Arc::new(RwLock::new(None));
    pub static ref LAST_ENDPOINT: Mutex<Option<Endpoint>> = Mutex::new(None);
}

#[allow(unused)]
pub fn shutdown_tcp_server() {
    if let Some(handler) = TCP_HANDLER.write().unwrap().take() {
        if let Some(endpoint) = LAST_ENDPOINT.lock().unwrap().take() {
            handler.network().send(endpoint, serde_json::to_string(&ToTM::ShutdownNow {}).unwrap().as_bytes());
        }
        handler.stop();
    }
}

pub fn server_main(
    ip_addr: &str,
    port: u16,
    to_gui: Sender<ToGUI>,
    from_gui: Receiver<FromGuiToServer>,
) {
    let ip_addr = if ip_addr.is_empty() {
        "127.0.0.1"
    } else {
        ip_addr
    };
    let port = if port == 0 { DEFAULT_PORT } else { port };

    let mumble: Arc<RwLock<std::io::Result<MumbleLink>>> = Arc::new(RwLock::new(Err(
        std::io::Error::new(std::io::ErrorKind::Other, "Mumble not connected"),
    )));
    try_connect_mumble(&mumble, &to_gui);

    while mumble.read().unwrap().as_ref().is_err() {
        std::thread::sleep(std::time::Duration::from_millis(10));
        match from_gui.try_recv() {
            Ok(FromGuiToServer::TryConnectMumble()) => {
                try_connect_mumble(&mumble, &to_gui);
            }
            Ok(_) => {}
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                log::warn!("GUI channel disconnected");
                return;
            }
        }
    }
    log::info!("Mumble connected");

    loop {
        match from_gui.try_recv() {
            Ok(FromGuiToServer::UseManiaPlanetTelemetry()) => {
                let _ = run_mp_telemetry_loop(&mumble, &to_gui);
                return;
            }
            Ok(FromGuiToServer::UseSocketServer()) => {
                break;
            }
            Ok(FromGuiToServer::Shutdown()) => {
                shutdown_tcp_server();
                return;
            }
            Ok(_) => {}
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                log::warn!("GUI channel disconnected");
                return;
            }
        }
    }



    let (handler, listener) = node::split::<()>();

    *TCP_HANDLER.write().unwrap() = Some(handler.clone());

    handler
        .network()
        .listen(Transport::FramedTcp, &format!("{}:{}", ip_addr, port))
        .unwrap();
    log::info!("Listening on {}:{}", ip_addr, port);
    to_gui
        .send(ToGUI::ListeningOn(ip_addr.to_string(), port))
        .unwrap();



    let player_login = Arc::new(RwLock::new(String::new()));
    let player_name = Arc::new(RwLock::new(String::new()));
    let server_login = Arc::new(RwLock::new(String::new()));
    let server_team = Arc::new(RwLock::new(String::new()));
    let update_nonce = Arc::new(RwLock::new(0));

    let update_context = |mumble: &mut MumbleLink| {
        let ctx = format!(
            "TM|{}|{}",
            &server_login.read().unwrap(),
            &server_team.read().unwrap()
        );
        let un = *update_nonce.read().unwrap();
        mumble.set_identity(
            format!(
                "{}|{}|{}",
                &player_name.read().unwrap(),
                &player_login.read().unwrap(),
                &un
            )
            .as_str(),
        );
        mumble.set_context(ctx.as_bytes());
        *update_nonce.write().unwrap() += 1;
        *LAST_CONTEXT.lock().unwrap() = ctx;
    };

    let pl: Arc<RwLock<String>> = player_login.clone();
    let pn = player_name.clone();
    let sl = server_login.clone();
    let st = server_team.clone();

    listener.for_each(move |event| {
        match event.network() {
            NetEvent::Message(_endpoint, data) => {
                LAST_ENDPOINT.lock().unwrap().replace(_endpoint);
                // position
                if data.len() > 0 && data[0] == 1 {
                    match read_pos_msg(&data) {
                        Ok(from_tm) => {
                            let mut mumble_w = mumble.write().unwrap();
                            let mumble = mumble_w.as_mut().unwrap();
                            mumble.update(
                                from_tm.get_pos_p().unwrap().clone().into(),
                                from_tm.get_pos_c().unwrap().clone().into(),
                            );
                            if to_gui.send(ToGUI::FromTM(from_tm)).is_err() {
                                log::warn!("GUI channel disconnected");
                                shutdown_tcp_server();
                            }
                        }
                        Err(e) => {
                            log::warn!("Error parsing position message: {}", e);
                            to_gui
                                .send(ToGUI::ProtocolError(format!(
                                    "Error parsing position message: {}",
                                    e
                                )))
                                .unwrap();
                        }
                    }
                    return;
                }
                let json_raw = String::from_utf8_lossy(&data);
                if !json_raw.starts_with("{\"Positions\":") {
                    log::info!("Received: {:?}", &json_raw);
                    log::info!("Received len: {}", data.len());
                }
                match serde_json::from_str::<FromTM>(&json_raw) {
                    Ok(from_tm) => {
                        let mut mumble_w = mumble.write().unwrap();
                        let mumble = mumble_w.as_mut().unwrap();
                        match from_tm {
                            m @ FromTM::Positions { p, c } => {
                                mumble.update(p.into(), c.into());
                                if *VISIBLE.lock().unwrap() {
                                    to_gui.send(ToGUI::FromTM(m)).unwrap();
                                }
                            }
                            ref m @ FromTM::PlayerDetails(ref name, ref login) => {
                                *pn.write().unwrap() = name.clone();
                                *pl.write().unwrap() = login.clone();
                                if *VISIBLE.lock().unwrap() {
                                    let _ = to_gui.send(ToGUI::FromTM(m.clone()));
                                }
                                // update_context(mumble);
                            }
                            ref m @ FromTM::ServerDetails(ref name, ref team) => {
                                *sl.write().unwrap() = name.clone();
                                *st.write().unwrap() = team.clone();
                                update_context(mumble);
                                if *VISIBLE.lock().unwrap() {
                                    let _ = to_gui.send(ToGUI::FromTM(m.clone()));
                                }
                            }
                            m @ FromTM::LeftServer() => {
                                *sl.write().unwrap() = String::new();
                                *st.write().unwrap() = "All".to_string();
                                update_context(mumble);
                                if *VISIBLE.lock().unwrap() {
                                    let _ = to_gui.send(ToGUI::FromTM(m.clone()));
                                }
                            }
                            m @ FromTM::Ping() => {
                                handler.network().send(
                                    _endpoint,
                                    serde_json::to_string(&ToTM::Ping()).unwrap().as_bytes(),
                                );
                                update_context(mumble);
                                if *VISIBLE.lock().unwrap() {
                                    let _ = to_gui.send(ToGUI::FromTM(m.clone()));
                                }
                            }
                            FromTM::NetConnected(_, _)
                            | FromTM::NetDisconnected(_)
                            | FromTM::NetAccepted(_) => {
                                log::warn!("Unexpected message: {:?}", from_tm);
                                to_gui
                                    .send(ToGUI::ProtocolError(format!(
                                        "Unexpected message: {:?}",
                                        from_tm
                                    )))
                                    .unwrap();
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Error parsing message: {}", e);
                        to_gui
                            .send(ToGUI::ProtocolError(format!(
                                "Error parsing message: {}",
                                e
                            )))
                            .unwrap();
                    }
                }
            }
            NetEvent::Connected(_endpoint, _connection_success) => {
                unreachable!();
                // log::info!("Client connected");
                // to_gui
                //     .send(FromTM::NetConnected(_endpoint.addr(), connection_success).into())
                //     .unwrap();
                // handler.network().send(_endpoint, serde_json::to_string(&ToTM::ConnectedStatus(mumble.read().unwrap().as_ref().is_ok())).unwrap().as_bytes());
            }
            NetEvent::Disconnected(_endpoint) => {
                let mut mumble_w = mumble.write().unwrap();
                let mumble = mumble_w.as_mut().unwrap();
                log::info!("Client disconnected");
                *sl.write().unwrap() = String::new();
                *st.write().unwrap() = "All".to_string();
                update_context(mumble);
                let r: Result<_, SendError<_>> = (|| {
                    to_gui.send(FromTM::LeftServer().into())?;
                    to_gui.send(FromTM::NetDisconnected(_endpoint.addr()).into())?;
                    Ok(())
                })();
                match r {
                    Ok(_) => {}
                    Err(_) => {
                        log::warn!("GUI channel disconnected");
                        shutdown_tcp_server();
                    }
                }
            }
            NetEvent::Accepted(_endpoint, _listener) => {
                log::info!("Client accepted");
                to_gui
                    .send(FromTM::NetAccepted(_endpoint.addr()).into())
                    .unwrap();
                handler.network().send(
                    _endpoint,
                    serde_json::to_string(&ToTM::LinkAppInfo {
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        options: vec![],
                    })
                    .unwrap()
                    .as_bytes(),
                );
                handler.network().send(
                    _endpoint,
                    serde_json::to_string(&ToTM::ConnectedStatus(
                        mumble.read().unwrap().as_ref().is_ok(),
                    ))
                    .unwrap()
                    .as_bytes(),
                );
            }
        }
    });
}

fn try_connect_mumble(mumble: &Arc<RwLock<std::io::Result<MumbleLink>>>, to_gui: &Sender<ToGUI>) {
    let mut mumble_w = mumble.write().unwrap();
    match mumble_w.as_ref() {
        Ok(_) => {
            log::warn!("Mumble already connected");
            to_gui
                .send(ToGUI::MumbleError("Mumble already connected".to_string()))
                .unwrap();
        }
        Err(_) => {}
    }
    *mumble_w = MumbleLink::new(
        "TM-Proximity-Chat",
        "Bridge to TM2020 plugin for proximity chat",
    );
    drop(mumble_w);

    let mumble_r = mumble.read().unwrap();
    to_gui.send(ToGUI::IsConnected(mumble_r.is_ok())).unwrap();
    if let Err(e) = mumble_r.as_ref() {
        to_gui.send(ToGUI::MumbleError(e.to_string())).unwrap();
    }
    drop(mumble_r);
}

fn read_vec3(r: &mut std::io::Cursor<&[u8]>) -> Result<[f32; 3], std::io::Error> {
    Ok([
        r.read_f32::<LE>()?,
        r.read_f32::<LE>()?,
        r.read_f32::<LE>()?,
    ])
}

fn read_pos_msg(data: &[u8]) -> Result<FromTM, std::io::Error> {
    let mut r = std::io::Cursor::new(data);
    r.read_u8()?; // skip the message type
    let pos = read_vec3(&mut r)?;
    let dir = read_vec3(&mut r)?;
    let up = read_vec3(&mut r)?;
    let pos = MPos::new(pos, dir, up);
    let c_pos = read_vec3(&mut r)?;
    let c_dir = read_vec3(&mut r)?;
    let c_up = read_vec3(&mut r)?;
    let c_pos = MPos::new(c_pos, c_dir, c_up);
    Ok(FromTM::Positions { p: pos, c: c_pos })
}
