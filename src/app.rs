use std::{
    sync::{
        mpsc::{Receiver, Sender},
        OnceLock,
    },
    time::{Duration, Instant},
};

use crate::{
    mp_telemetry_data::STelemetry,
    tcp_server::{FromTM, LAST_CONTEXT}, ALT_HELD_AT_STARTUP,
};
use eframe::App;
use egui::vec2;
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};

pub const MUMBLE_SCALE_INV: f32 = 32.0;
pub const MUMBLE_SCALE: f32 = 1.0 / 32.0;

#[derive(Debug, Clone)]
pub enum ToGUI {
    // TaskBarIconMsg(String),
    IsConnected(bool),
    MumbleError(String),
    ListeningOn(String, u16),
    ProtocolError(String),
    FromTM(FromTM),
    Telemetry(STelemetry),
    // HideMainWindow()
}

impl From<FromTM> for ToGUI {
    fn from(from_tm: FromTM) -> Self {
        ToGUI::FromTM(from_tm)
    }
}

pub enum FromGuiToServer {
    TryConnectMumble(),
    UseManiaPlanetTelemetry(),
    UseSocketServer(),
    #[allow(unused)]
    Shutdown(),
}

#[derive(Serialize, Deserialize)]
pub struct MumbleBridgeApp<'a> {
    connected: bool,
    in_server: bool,
    client_connected: bool,
    has_chosen_method: bool,
    offer_manual_choice: bool,
    player_name: String,
    player_login: String,
    server_login: String,
    server_team: String,
    #[serde(skip)]
    telemetry: Option<STelemetry>,
    #[serde(skip)]
    e_state: MumbleBridgeEphemeralState,
    #[serde(skip)]
    rx_gui: OnceLock<&'a mut Receiver<ToGUI>>,
    #[serde(skip)]
    tx_gui: OnceLock<Sender<FromGuiToServer>>,
    #[serde(skip)]
    shutdown_tx: OnceLock<Sender<()>>,
}

impl App for MumbleBridgeApp<'_> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self._update(ctx, frame);
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

impl Default for MumbleBridgeApp<'_> {
    fn default() -> Self {
        MumbleBridgeApp {
            connected: false,
            in_server: false,
            client_connected: false,
            has_chosen_method: false,
            offer_manual_choice: *ALT_HELD_AT_STARTUP.lock().unwrap(),
            player_name: String::new(),
            player_login: String::new(),
            server_login: String::new(),
            server_team: String::new(),
            telemetry: None,
            e_state: Default::default(),
            rx_gui: OnceLock::new(),
            tx_gui: OnceLock::new(),
            shutdown_tx: OnceLock::new(),
        }
    }
}

impl MumbleBridgeApp<'_> {
    pub fn new<'a>(
        rx_gui: &'a mut Receiver<ToGUI>,
        tx_gui: Sender<FromGuiToServer>,
        shutdown_tx: Sender<()>,
    ) -> MumbleBridgeApp<'a> {
        let app = MumbleBridgeApp::default();
        app.rx_gui.set(rx_gui).expect("Failed to set rx_gui");
        app.tx_gui.set(tx_gui).expect("Failed to set tx_gui");
        app.shutdown_tx
            .set(shutdown_tx)
            .expect("Failed to set shutdown_tx");
        app
    }

    fn _update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut reset_error_msg = false;
        if let Some(rx) = self.rx_gui.get_mut() {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    // ToGUI::TaskBarIconMsg(msg) => {
                    //     self.e_state.last_task_bar_msg = msg;
                    // },
                    ToGUI::Telemetry(telemetry) => {
                        self.telemetry.replace(telemetry);
                        // log::info!("Got telemetry: {:?}", telemetry.object);
                    }
                    ToGUI::IsConnected(is_connected) => {
                        self.connected = is_connected;
                    }
                    ToGUI::MumbleError(e) => {
                        self.e_state.last_error_msg = e;
                        self.e_state.last_error_msg_time = Instant::now();
                    }
                    ToGUI::ListeningOn(ip, port) => {
                        self.e_state.listening = Some((ip, port));
                    }
                    ToGUI::ProtocolError(e) => {
                        self.e_state.last_error_msg = e;
                        self.e_state.last_error_msg_time = Instant::now();
                    }
                    // ToGUI::HideMainWindow() => {
                    //     hide_window = true;
                    // },
                    ToGUI::FromTM(from_tm) => match from_tm {
                        FromTM::Positions { p, c } => {
                            self.e_state.last_player_pos =
                                vec_flip_z(vecm(p.pos, MUMBLE_SCALE_INV));
                            self.e_state.last_camera_pos =
                                vec_flip_z(vecm(c.pos, MUMBLE_SCALE_INV));
                            self.e_state.last_update = Instant::now();
                        }
                        FromTM::LeftServer() => {
                            self.in_server = false;
                            self.server_login = String::new();
                            self.server_team = "All".into();
                        }
                        FromTM::PlayerDetails(name, login) => {
                            self.player_name = name;
                            self.player_login = login;
                        }
                        FromTM::ServerDetails(server, team) => {
                            self.server_login = server;
                            self.server_team = team;
                        }
                        FromTM::Ping() => {
                            self.e_state.last_ping = Instant::now();
                        }
                        FromTM::NetAccepted(_) => {
                            self.client_connected = true;
                            reset_error_msg = true;
                        }
                        FromTM::NetDisconnected(_) => {
                            self.client_connected = false;
                        }
                        _ => {}
                    },
                }
            }
        }
        if reset_error_msg {
            self.reset_err_msg();
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            log::info!("Close requested");
        }
    }

    fn reset_err_msg(&mut self) {
        self.e_state.last_error_msg = String::new();
    }

    fn render_main_top(&self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("TM to Mumble: Proximity Chat");
                self.ui_mumble_status_small(ui);
                self.ui_version(ui);
            });
        });
    }

    fn render_main_body(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::new([false, true]).auto_shrink([false, true]).show(ui, |ui| {
                // if ui.button("Minimize to tray icon").clicked() {
                //     set_window_visible(ctx, false);
                //     // self.hide_main_window(ctx, frame);
                // }
                if self.connected && self.has_chosen_method {
                    self.ui_listening_on(ui);
                    ui.horizontal(|ui| {
                        self.ui_mumble_status(ui);
                        self.ui_tm_game_status(ui);
                    });
                    self.ui_last_positions(ui);
                    self.ui_curr_details(ui);
                } else if !self.connected {
                    if ui.button("Connect to Mumble").clicked() {
                        self.tx_gui
                            .get()
                            .expect("tx_gui not set")
                            .send(FromGuiToServer::TryConnectMumble())
                            .expect("to send to server");
                        self.e_state.last_error_msg = String::new();
                    }
                } else if !self.has_chosen_method && !self.offer_manual_choice {
                    self.tx_gui
                        .get()
                        .expect("tx_gui not set")
                        .send(FromGuiToServer::UseSocketServer())
                        .expect("to send to server");
                    self.has_chosen_method = true;
                } else if !self.has_chosen_method && self.offer_manual_choice {
                    if ui.button("Use the Plugin (Recommended)").clicked() {
                        self.tx_gui
                            .get()
                            .expect("tx_gui not set")
                            .send(FromGuiToServer::UseSocketServer())
                            .expect("to send to server");
                        self.e_state.last_error_msg = String::new();
                        self.has_chosen_method = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(vec2(400.0, 240.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(vec2(400.0, 240.0)));
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(vec2(400.0, 240.0)));
                    }
                    if ui.button("Use TM Telemetry").clicked() {
                        self.tx_gui
                            .get()
                            .expect("tx_gui not set")
                            .send(FromGuiToServer::UseManiaPlanetTelemetry())
                            .expect("to send to server");
                        self.e_state.last_error_msg = String::new();
                        self.has_chosen_method = true;
                    }
                } else {
                    ui.label("UNKNOWN STATE");
                }

                if let Some(t) = self.telemetry.as_ref() {
                    self.render_telemetry(ui, t);
                }

                self.ui_dbg_task_bar_msg(ui);
                self.ui_opt_last_error_msg(ui);
            });
        });
    }

    fn render_main_footer(&self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            // ui.with_layout(
            //     egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.horizontal_centered(|ui| {
                        self.ui_mumble_status_small(ui);
                        ui.separator();
                        self.ui_tm_game_status_small(ui);
                        ui.separator();
                    });
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
            });
        });
    }

    fn ui_curr_details(&self, ui: &mut egui::Ui) {
        ui.label(format!(
            "Player: {} | {}",
            self.player_name, self.player_login
        ));
        ui.horizontal(|ui| {
            ui.label(format!(
                "Server: {}  |  Team: {}",
                self.server_login, self.server_team
            ));
        });
        ui.label(format!("Mumble Ctx: {}", &LAST_CONTEXT.lock().unwrap()));
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
                .body(|body| {
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

    fn ui_mumble_status_small(&self, ui: &mut egui::Ui) {
        ui.label(if self.connected { "M: ✅" } else { "M: ❌" });
    }

    fn ui_mumble_status(&self, ui: &mut egui::Ui) {
        ui.label(if self.connected {
            "Mumble: ✅"
        } else {
            "Mumble: ❌"
        });
    }

    fn ui_tm_game_status_small(&self, ui: &mut egui::Ui) {
        ui.label(if self.client_connected {
            "TM: ✅"
        } else {
            "TM: ❌"
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

    fn ui_dbg_task_bar_msg(&self, _ui: &mut egui::Ui) {
        #[cfg(debug_assertions)]
        _ui.label(&self.e_state.last_task_bar_msg);
    }

    fn render_telemetry(&self, ui: &mut egui::Ui, telemetry: &STelemetry) {
        // egui::CentralPanel::default().show(ui, |ui| {
        ui.label("Telemetry");
        // header
        ui.label(format!("Header: {:#?}", telemetry.header));
        ui.label(format!("UpdateNb: {:#?}", telemetry.update_number));
        ui.label(format!("Game: {:#?}", &telemetry.game));
        ui.label(format!("Race: {:#?}", &telemetry.race));
        ui.label(format!("Object: {:#?}", &telemetry.object));
        ui.label(format!("Vehicle: {:#?}", &telemetry.vehicle));
        ui.label(format!("Device: {:#?}", &telemetry.device));
        ui.label(format!("Player: {:#?}", &telemetry.player));
        // });
    }
}

struct MumbleBridgeEphemeralState {
    last_update: Instant,
    last_ping: Instant,
    last_error_msg: String,
    last_error_msg_time: Instant,
    #[allow(unused)]
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
            last_error_msg_time: Instant::now(),
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
