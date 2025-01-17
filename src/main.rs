#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{borrow::Borrow, sync::{Mutex, OnceLock}};

use crate::app::MumbleBridgeApp;
use app::{FromGuiToServer, ToGUI};
// use crate::error::AppError;
use eframe::Renderer;
use egui::{vec2, Visuals};
use tcp_server::{FromTM, MPos, ToTM};
// use egui::mutex::RwLock;
// use lazy_static::lazy_static;
// use shmem_bind::{self as shmem, ShmemBox, ShmemError};
// use sysinfo::{ProcessRefreshKind, System};
use tray_icon::{menu::{self, accelerator::{self, Accelerator}, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem}, Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(windows)]
use windows::Win32::Foundation::HWND;
use winit::raw_window_handle::{HasWindowHandle, Win32WindowHandle, WaylandWindowHandle, XlibWindowHandle};

mod app;
mod tcp_server;
mod error;

pub static VISIBLE: Mutex<bool> = Mutex::new(true);
#[cfg(windows)]
pub static WINDOW_HANDLE: OnceLock<Win32WindowHandle> = OnceLock::new();
// #[cfg(not(windows))]
// pub static WAYLAND_HANDLE: OnceLock<WaylandWindowHandle> = OnceLock::new();
// #[cfg(not(windows))]
// pub static XLIB_HANDLE: OnceLock<XlibWindowHandle> = OnceLock::new();


// lazy_static! {
//     pub static ref MUMBLE_LINK: Arc<RwLock<Option<mumble_link::MumbleLink>>> =
//         RwLock::new(None).into();
// }

fn main() {
    let mut nat_opts = eframe::NativeOptions::default();
    nat_opts.centered = true;
    nat_opts.viewport = nat_opts.viewport.with_inner_size(vec2(400.0, 240.0))
        .with_min_inner_size(vec2(400.0, 240.0))
        .with_resizable(false)
        .with_minimize_button(false)
        .with_maximize_button(false);
    nat_opts.renderer = Renderer::Wgpu;

    // let (from_tm_tx, from_tm_rx) = std::sync::mpsc::channel::<FromTM>();
    // let (to_tm_tx, to_tm_rx) = std::sync::mpsc::channel::<ToTM>();

    let pos = FromTM::Positions { p: MPos::example(1.0), c: MPos::example(2.0) };
    let pos_str = serde_json::to_string(&pos).unwrap();
    eprintln!("Position: {}", pos_str);
    let pd = FromTM::PlayerDetails("name".to_string(), "login".to_string());
    let pd_str = serde_json::to_string(&pd).unwrap();
    eprintln!("PlayerDetails: {}", pd_str);
    let ping = FromTM::Ping();
    let ping_str = serde_json::to_string(&ping).unwrap();
    eprintln!("Ping: {}", ping_str);

    let (to_gui_tx, mut to_gui_rx) = std::sync::mpsc::channel::<ToGUI>();
    let (from_gui_tx, from_gui_rx) = std::sync::mpsc::channel::<FromGuiToServer>();

    let mut icon_data: Vec<u8> = Vec::with_capacity(16 * 16 * 4);
    for _ in 0..256 {
        // all red
        icon_data.extend_from_slice(&[255, 0, 0, 255]);
    }
    let icon = Icon::from_rgba(icon_data, 16, 16).expect("to create icon");
    // let menu_entries = generate_menu_entries();
    // let tray_menu = Menu::with_items(&menu_entries.iter().map(|mi| mi.borrow()).collect::<Vec<_>>()).expect("to create menu");
    let _tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("TM to Mumble Link")
        // .with_menu(Box::new(tray_menu))
        .build().expect("to build tray icon");

    let to_gui_tx2 = to_gui_tx.clone();
    std::thread::spawn(|| {
        tcp_server::server_main("", 0, to_gui_tx2, from_gui_rx);
    });

    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel::<()>();

    while (shutdown_rx.try_recv().is_err()) {
        let borrowed_to_gui_rx = &mut to_gui_rx;
        let cloned_to_gui_tx = to_gui_tx.clone();
        let cloned_shutdown_tx  = shutdown_tx.clone();
        eframe::run_native(
            "TM to Mumble Link",
            nat_opts.clone(),
            // tray icon stuff via: https://github.com/emilk/egui/discussions/737#discussioncomment-8830140
            Box::new(|cc| {
                // if windows

                // #[cfg(windows)]
                // {
                //     let winit::raw_window_handle::RawWindowHandle::Win32(handle) =
                //         cc.window_handle().unwrap().as_raw()
                //     else {
                //         panic!("Expected a Windows window handle");
                //     };

                //     let context = cc.egui_ctx.clone();

                //     WINDOW_HANDLE.set(handle.clone()).expect("to set window handle");
                // }
                // #[cfg(not(windows))]
                {
                    let win_handle: winit::raw_window_handle::WindowHandle<'_> = cc.window_handle().unwrap();
                    let handle = win_handle.as_raw();
                    let context = cc.egui_ctx.clone();
                    match handle {
                        winit::raw_window_handle::RawWindowHandle::Win32(handle) => {
                            // WINDOW_HANDLE.set(handle.clone()).expect("to set window handle");
                        },
                        winit::raw_window_handle::RawWindowHandle::Wayland(handle) => {
                            // WAYLAND_HANDLE.set(handle.clone()).expect("to set wayland handle");
                        },
                        winit::raw_window_handle::RawWindowHandle::Xlib(handle) => {
                            // XLIB_HANDLE.set(handle.clone()).expect("to set xlib handle");
                        },
                        _ => {
                            eprintln!("Expected a Windows, Wayland or Xlib window handle");
                        }
                    }
                }

                // MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                //     // println!("MenuEvent: {:?}", event);
                //     // cloned_to_gui_tx.send(ToGUI::TaskBarIconMsg(format!("MenuEvent: {:?}", event))).expect("to send to gui");
                //     let MenuEvent { id } = event;
                //     match id.0.as_str() {
                //         MID_SHOW => show_window(get_window_handle()),
                //         MID_HIDE => hide_window(get_window_handle()),
                //         MID_EXIT => {
                //             std::process::exit(0);
                //         },
                //         _ => {
                //             eprintln!("Unknown menu id: {}", id.0);
                //         }
                //     }

                // }));


                // tray-icon crate
                // https://docs.rs/tray-icon/0.12.0/tray_icon/struct.TrayIconEvent.html#method.set_event_handler
                TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                    // println!("TrayIconEvent: {:?}", event);
                    let _ = cloned_to_gui_tx.send(ToGUI::TaskBarIconMsg(format!("TrayIconEvent: {:?}", event))).expect("to send to gui");
                    let (id, pos, rect, btn, btn_state) = match event {
                        TrayIconEvent::Click {
                            id,
                            position,
                            rect,
                            button,
                            button_state,
                        } => (id, position, rect, button, button_state),
                        _ => {
                            return;
                        }
                    };

                    if btn_state == MouseButtonState::Down {
                        match btn {
                            MouseButton::Left => set_window_visible(get_window_handle(), !is_window_visible()),
                            MouseButton::Right => {
                                // let _ = tray_icon.hide();
                                // let _ = tray_icon.show();
                            },
                            _ => {}
                        }
                    }
                }));
                Ok(Box::new(MumbleBridgeApp::new(borrowed_to_gui_rx, from_gui_tx.clone(), cloned_shutdown_tx)))
            }),
        )
        .expect("to run the app");
        while !*VISIBLE.lock().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

pub fn is_window_visible() -> bool {
    *VISIBLE.lock().unwrap()
}

// #[cfg(windows)]
// pub fn hide_window(handle: HWND) {
//     set_window_visible(handle, false);
// }

// #[cfg(windows)]
// pub fn show_window(handle: HWND) {
//     set_window_visible(handle, true);
// }

// #[cfg(windows)]
// pub fn set_window_visible(handle: HWND, visible: bool) {
//     let show = windows::Win32::UI::WindowsAndMessaging::SW_SHOWDEFAULT;
//     let hide = windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
//     let cmd = if visible { show } else { hide };
//     unsafe {
//         let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
//             handle,
//             cmd,
//         );
//     }
//     println!("Setting window visible: {}", visible);
//     *VISIBLE.lock().unwrap() = visible;
//     println!("Window visible: {}", *VISIBLE.lock().unwrap());
// }

// #[cfg(windows)]
// pub fn get_window_handle() -> HWND {
//     HWND(WINDOW_HANDLE.get().unwrap().hwnd.into())
// }

// #[cfg(not(windows))]
pub fn get_window_handle() -> () {
    ()
}

// #[cfg(not(windows))]
pub fn hide_window(_h: ()) {
    set_window_visible((), false);
}

// #[cfg(not(windows))]
pub fn show_window(_h: ()) {
    set_window_visible((), true);
}

// #[cfg(not(windows))]
pub fn set_window_visible(_h: (), visible: bool) {
    // if let Some(wh) = WAYLAND_HANDLE.get() {

    // }
    println!("Setting window visible: {}", visible);
    *VISIBLE.lock().unwrap() = visible;
    println!("Window visible: {}", *VISIBLE.lock().unwrap());
}

const MID_SHOW: &str = "1";
const MID_HIDE: &str = "2";
const MID_EXIT: &str = "3";

pub fn generate_menu_entries<'a>() -> Vec<Box<dyn IsMenuItem>> {
    vec![
        Box::new(MenuItem::with_id(MID_SHOW, "Show", true, None)),
        Box::new(MenuItem::with_id(MID_HIDE, "Hide", true, None)),
        Box::new(MenuItem::with_id(MID_EXIT, "Exit", true, None)),
    ]
}
