use std::{sync::{mpsc::{Receiver, Sender}, OnceLock}, time::{Duration, Instant}};

use eframe::App;
use egui::{Layout, Rect};
use egui_extras::{Column, TableBuilder};
use mumble_link::MumbleLink;
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::HWND;
use winit::{raw_window_handle::{HasDisplayHandle, HasWindowHandle}, window::Window};

use crate::{get_window_handle, set_window_visible, tcp_server::FromTM, WINDOW_HANDLE};

const MUMBLE_SCALE_INV: f32 = 32.0;

#[derive(Debug, Clone)]
pub enum ToGUI {
    TaskBarIconMsg(String),
    IsConnected(bool),
    MumbleError(String),
    ListeningOn(String, u16),
    ProtocolError(String),
    FromTM(FromTM)
}

impl From<FromTM> for ToGUI {
    fn from(from_tm: FromTM) -> Self {
        ToGUI::FromTM(from_tm)
    }
}

pub enum FromGuiToServer {
    TryConnectMumble(),
}

#[derive(Serialize, Deserialize)]
pub struct MumbleBridgeApp {
    connected: bool,
    in_server: bool,
    client_connected: bool,
    player_name: String,
    player_login: String,
    server_login: String,
    server_team: String,
    #[serde(skip)]
    e_state: MumbleBridgeEphemeralState,
    #[serde(skip)]
    rx_gui: OnceLock<Receiver<ToGUI>>,
    #[serde(skip)]
    tx_gui: OnceLock<Sender<FromGuiToServer>>,
}

impl App for MumbleBridgeApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self._update();
        self.render_main_top(ctx, frame);
        self.render_main_body(ctx, frame);
        self.render_main_footer(ctx, frame);
        ctx.request_repaint_after(Duration::from_millis(50));

        // frame.set_window_size(Vec2::new(400.0, 240.0));
        // let x = frame.display_handle().unwrap();
        // let x = frame.window_handle().unwrap();
        // let window: Window = unsafe { Window::d(x) };
        // let rect = window.inner_size();
        // if rect.width != 400 || rect.height != 240 {
        //     window.set_min_inner_size(Some(winit::dpi::PhysicalSize::new(400, 240)));
        // }
    }
}

impl Default for MumbleBridgeApp {
    fn default() -> Self {
        MumbleBridgeApp {
            connected: false,
            in_server: false,
            client_connected: false,
            player_name: String::new(),
            player_login: String::new(),
            server_login: String::new(),
            server_team: String::new(),
            e_state: Default::default(),
            rx_gui: OnceLock::new(),
            tx_gui: OnceLock::new(),
        }
    }
}

impl MumbleBridgeApp {
    pub fn new(rx_gui: Receiver<ToGUI>, tx_gui: Sender<FromGuiToServer>) -> Self {
        let app = MumbleBridgeApp::default();
        app.rx_gui.set(rx_gui).expect("Failed to set rx_gui");
        app.tx_gui.set(tx_gui).expect("Failed to set tx_gui");
        // match app.try_connect() {
        //     Ok(_) => {}
        //     Err(e) => {
        //         app.e_state.last_error_msg = format!("Error connecting to Mumble: {}", e);
        //     }
        // }
        app
    }

    fn _update(&mut self) {
        if let Some(rx) = self.rx_gui.get_mut() {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ToGUI::TaskBarIconMsg(msg) => {
                        self.e_state.last_task_bar_msg = msg;
                    },
                    ToGUI::IsConnected(is_connected) => {
                        self.connected = is_connected;
                    },
                    ToGUI::MumbleError(e) => {
                        self.e_state.last_error_msg = e;
                    },
                    ToGUI::ListeningOn(ip, port) => {
                        self.e_state.listening = Some((ip, port));
                    },
                    ToGUI::ProtocolError(e) => {
                        self.e_state.last_error_msg = e;
                    },
                    ToGUI::FromTM(from_tm) => {
                        match from_tm {
                            FromTM::Positions { p, c } => {
                                self.e_state.last_player_pos = vec_flip_z(vecm(p.pos, MUMBLE_SCALE_INV));
                                self.e_state.last_camera_pos = vec_flip_z(vecm(c.pos, MUMBLE_SCALE_INV));
                                self.e_state.last_update = Instant::now();
                            },
                            FromTM::LeftServer() => {
                                self.in_server = false;
                            },
                            FromTM::PlayerDetails(name, login) => {
                                self.player_name = name;
                                self.player_login = login;
                            },
                            FromTM::ServerDetails(server, team) => {
                                self.server_login = server;
                                self.server_team = team;
                            }
                            FromTM::Ping() => {
                                self.e_state.last_ping = Instant::now();
                            }
                            FromTM::NetAccepted(_) => {
                                self.client_connected = true;
                            },
                            FromTM::NetDisconnected(_) => {
                                self.client_connected = false;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn render_main_top(&self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("TM to Mumble: Proximity Chat");
                self.ui_status(ui);
                self.ui_version(ui);
            });
        });
    }

    fn render_main_body(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Minimize to tray icon").clicked() {
                set_window_visible(get_window_handle(), false);
            }
            if self.connected {
                self.ui_listening_on(ui);
                self.ui_tm_game_status(ui);
                self.ui_last_positions(ui);
                self.ui_curr_details(ui);
            } else {
                if ui.button("Connect to Mumble").clicked() {
                    self.tx_gui.get().expect("tx_gui not set").send(FromGuiToServer::TryConnectMumble()).expect("to send to server");
                    self.e_state.last_error_msg = String::new();
                }
            }
            self.ui_dbg_task_bar_msg(ui);
            self.ui_opt_last_error_msg(ui);
        });
    }

    fn render_main_footer(&self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // ui.with_layout(
            //     egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        self.ui_status(ui);
                        ui.separator();
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.horizontal_centered(|ui| {
                            self.ui_version(ui);
                            ui.separator();
                            self.last_update(ui);
                            ui.separator();
                            self.last_ping(ui);
                            ui.separator();
                        });
                    });
                },
            );
        });
    }

    fn ui_curr_details(&self, ui: &mut egui::Ui) {
        ui.label(format!("Player: {} | {}", self.player_name, self.player_login));
        ui.label(format!("Server: {}", self.server_login));
        ui.label(format!("Team: {}", self.server_team));
    }

    fn ui_last_positions(&self, ui: &mut egui::Ui) {
        let p1 = fmt_vec3(self.e_state.last_player_pos);
        let p2 = fmt_vec3(self.e_state.last_camera_pos);
        ui.label("Last positions:");
        ui.indent("poss", |ui| {
            TableBuilder::new(ui)
            .column(Column::auto().resizable(true))
            .column(Column::remainder())
            .auto_shrink([true, true])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)

            .body(|mut body| {
                body.rows(16.0, 2, |mut row| {
                    let (label, val) = match row.index() {
                        0 => ("Player", p1.clone()),
                        1 => ("Camera", p2.clone()),
                        _ => unreachable!(),
                    };
                    row.col(|ui| {
                        ui.label(label);
                    });
                    row.col(|ui| {
                        ui.label(val);
                    });
                });
            });
        });
    }

    fn ui_listening_on(&self, ui: &mut egui::Ui) {
        if let Some((ip, port)) = &self.e_state.listening {
            ui.label(format!("Listening on {}:{}", ip, port));
        }
    }

    fn ui_status(&self, ui: &mut egui::Ui) {
        ui.label(if self.connected {
            "✅"
        } else {
            "❌"
        });
    }

    fn ui_tm_game_status(&self, ui: &mut egui::Ui) {
        ui.label(if self.client_connected {
            "TM Plugin: ✅"
        } else {
            "TM Plugin: ❌"
        });
    }

    fn ui_version(&self, ui: &mut egui::Ui) {
        ui.label(format!("v: {}", env!("CARGO_PKG_VERSION")));
    }

    fn last_update(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Pos: {:.1} s ago",
            self.since_last_update().as_secs_f32()
        ));
    }

    fn last_ping(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Ping: {:.1} s ago",
            self.since_last_ping().as_secs_f32()
        ));
    }

    fn since_last_update(&self) -> Duration {
        Instant::now().duration_since(self.e_state.last_update)
    }

    fn since_last_ping(&self) -> Duration {
        Instant::now().duration_since(self.e_state.last_ping)
    }

    // fn try_connect(&mut self) -> std::io::Result<()> {
    //     Ok(())
    // }

    fn ui_opt_last_error_msg(&self, ui: &mut egui::Ui) {
        if !self.e_state.last_error_msg.is_empty() {
            ui.label(&self.e_state.last_error_msg);
        }
    }

    fn ui_dbg_task_bar_msg(&self, ui: &mut egui::Ui) {
        #[cfg(debug_assertions)]
        ui.label(&self.e_state.last_task_bar_msg);
    }
}

struct MumbleBridgeEphemeralState {
    last_update: Instant,
    last_ping: Instant,
    last_error_msg: String,
    last_task_bar_msg: String,
    last_player_pos: [f32; 3],
    last_camera_pos: [f32; 3],
    listening: Option<(String, u16)>,
}

impl Default for MumbleBridgeEphemeralState {
    fn default() -> Self {
        MumbleBridgeEphemeralState {
            last_update: Instant::now(),
            last_ping: Instant::now(),
            last_error_msg: String::new(),
            last_task_bar_msg: String::new(),
            last_player_pos: [-1.0, -1.0, -1.0],
            last_camera_pos: [-1.0, -1.0, -1.0],
            listening: None,
        }
    }
}

pub fn fmt_vec3(v: [f32; 3]) -> String {
    format!("<{:.2}, {:.2}, {:.2}>", v[0], v[1], v[2])
}

pub fn vecm(v: [f32; 3], m: f32) -> [f32; 3] {
    [v[0] * m, v[1] * m, v[2] * m]
}

pub fn vec_flip_z(v: [f32; 3]) -> [f32; 3] {
    [v[0], v[1], -v[2]]
}
