//! Shared bridge core used by the graphical and terminal clients.

use std::sync::Mutex;

pub mod app;
pub mod mp_telemetry_data;
pub mod tcp_server;
pub mod util;
#[cfg(windows)]
pub mod maniaplanet_telemetry;

pub static VISIBLE: Mutex<bool> = Mutex::new(true);
pub static ALT_HELD_AT_STARTUP: Mutex<bool> = Mutex::new(false);

