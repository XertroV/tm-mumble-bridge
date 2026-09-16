//! Headless terminal client. It deliberately never initializes eframe/winit.
use std::sync::mpsc;
use std::time::Duration;
use tm_mumble_link::app::{FromGuiToServer, ToGUI};
use tm_mumble_link::tcp_server::{self, FromTM};

fn main() {
    let env_ll = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    std::env::set_var("RUST_LOG", env_ll);
    env_logger::init();
    eprintln!("TM to Mumble Link (TUI/headless)");
    eprintln!("Waiting for Mumble shared memory and the Trackmania client; press Ctrl-C to stop.");
    let (to_client, from_server) = mpsc::channel::<ToGUI>();
    let (to_server, from_client) = mpsc::channel::<FromGuiToServer>();
    std::thread::spawn(move || tcp_server::server_main("", 0, to_client, from_client));
    let mut socket_started = false;
    let mut last_retry = std::time::Instant::now() - Duration::from_secs(10);
    loop {
        if !socket_started && last_retry.elapsed() >= Duration::from_secs(2) {
            let _ = to_server.send(FromGuiToServer::TryConnectMumble());
            last_retry = std::time::Instant::now();
        }
        match from_server.recv_timeout(Duration::from_millis(250)) {
            Ok(ToGUI::IsConnected(true)) if !socket_started => {
                eprintln!("Mumble shared memory connected; starting TCP server on 127.0.0.1:46323");
                let _ = to_server.send(FromGuiToServer::UseSocketServer());
                socket_started = true;
            }
            Ok(ToGUI::IsConnected(true)) => {}
            Ok(ToGUI::IsConnected(false)) => eprintln!("Mumble shared memory is unavailable; is Mumble running?"),
            Ok(ToGUI::MumbleError(error)) => eprintln!("Mumble: {error}"),
            Ok(ToGUI::ListeningOn(ip, port)) => eprintln!("Listening on {ip}:{port}"),
            Ok(ToGUI::ProtocolError(error)) => eprintln!("Protocol error: {error}"),
            Ok(ToGUI::FromTM(message)) => print_event(&message),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            #[cfg(windows)]
            Ok(ToGUI::Telemetry(_)) => {}
        }
    }
}

fn print_event(event: &FromTM) {
    match event {
        FromTM::NetAccepted(address) => eprintln!("Trackmania connected: {address}"),
        FromTM::NetDisconnected(address) => eprintln!("Trackmania disconnected: {address}"),
        FromTM::PlayerDetails(name, login) => eprintln!("Player: {name} ({login})"),
        FromTM::ServerDetails(name, team) => eprintln!("Server: {name} ({team})"),
        FromTM::LeftServer() => eprintln!("Left server"),
        FromTM::Ping() => {}
        FromTM::Positions { .. } | FromTM::NetConnected(_, _) => {}
    }
}

