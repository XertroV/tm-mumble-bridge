#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{borrow::Borrow, sync::Mutex};

use crate::app::MumbleBridgeApp;
use app::{FromGuiToServer, ToGUI};
// use crate::error::AppError;
use eframe::Renderer;
use egui::{vec2, Context};
use tcp_server::{FromTM, MPos};
// use egui::mutex::RwLock;
use lazy_static::lazy_static;
// use shmem_bind::{self as shmem, ShmemBox, ShmemError};
// use sysinfo::{ProcessRefreshKind, System};
use tray_icon::{
    menu::{IsMenuItem, Menu, MenuEvent, MenuItem},
    Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
};
// #[cfg(windows)]
// use windows::Win32::Foundation::HWND;
// use winit::raw_window_handle::{HasWindowHandle, Win32WindowHandle, WaylandWindowHandle, XlibWindowHandle};

mod app;
mod tcp_server;

pub static VISIBLE: Mutex<bool> = Mutex::new(true);
// #[cfg(windows)]
// pub static WINDOW_HANDLE: RwLock<Option<Win32WindowHandle>> = RwLock::new();
// #[cfg(not(windows))]
// pub static WAYLAND_HANDLE: RwLock<Option<WaylandWindowHandle>> = RwLock::new();
// #[cfg(not(windows))]
// pub static XLIB_HANDLE: RwLock<Option<XlibWindowHandle>> = RwLock::new();

// lazy_static! {
//     pub static ref MUMBLE_LINK: Arc<RwLock<Option<mumble_link::MumbleLink>>> =
//         RwLock::new(None).into();
// }

fn main() {
    let env_ll = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    std::env::set_var("RUST_LOG", env_ll);
    env_logger::init();
    log::set_max_level(log::LevelFilter::Info);

    log::error!("Starting TM to Mumble Link");

    let mut nat_opts = eframe::NativeOptions::default();
    nat_opts.centered = true;
    nat_opts.viewport = nat_opts
        .viewport
        .with_inner_size(vec2(400.0, 240.0))
        .with_min_inner_size(vec2(400.0, 240.0))
        .with_icon(load_icon())
        .with_resizable(false)
        .with_minimize_button(true)
        .with_maximize_button(false);
    nat_opts.renderer = Renderer::Wgpu;

    // let (from_tm_tx, from_tm_rx) = std::sync::mpsc::channel::<FromTM>();
    // let (to_tm_tx, to_tm_rx) = std::sync::mpsc::channel::<ToTM>();

    let pos = FromTM::Positions {
        p: MPos::example(1.0),
        c: MPos::example(2.0),
    };
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

    let icon =
        Icon::from_rgba(ICON_DATA.0.clone(), ICON_DATA.1, ICON_DATA.2).expect("to create icon");
    let menu_entries = generate_menu_entries();
    let tray_menu = Menu::with_items(
        &menu_entries
            .iter()
            .map(|mi| mi.borrow())
            .collect::<Vec<_>>(),
    )
    .expect("to create menu");
    let _tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("TM to Mumble Link")
        .with_menu(Box::new(tray_menu))
        .build()
        .expect("to build tray icon");

    let to_gui_tx2 = to_gui_tx.clone();
    std::thread::spawn(|| {
        tcp_server::server_main("", 0, to_gui_tx2, from_gui_rx);
    });

    let (shutdown_tx, _shutdown_rx) = std::sync::mpsc::channel::<()>();

    // 'outer: while shutdown_rx.try_recv().is_err() {
    // shutdown_tx.send(()).expect("to send shutdown signal");
    let borrowed_to_gui_rx = &mut to_gui_rx;
    // let cloned_to_gui_tx = to_gui_tx.clone();
    let cloned_shutdown_tx = shutdown_tx.clone();
    eframe::run_native(
        "TM to Mumble Link",
        nat_opts.clone(),
        // tray icon stuff via: https://github.com/emilk/egui/discussions/737#discussioncomment-8830140
        Box::new(|_cc| {
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
            // let context = cc.egui_ctx.clone();
            // let context2 = cc.egui_ctx.clone();
            {
                // let win_handle: winit::raw_window_handle::WindowHandle<'_> = cc.window_handle().unwrap();
                // let handle = win_handle.as_raw();
                // match handle {
                //     winit::raw_window_handle::RawWindowHandle::Win32(handle) => {
                //         // WINDOW_HANDLE.set(handle.clone()).expect("to set window handle");
                //     },
                //     winit::raw_window_handle::RawWindowHandle::Wayland(handle) => {
                //         // WAYLAND_HANDLE.set(handle.clone()).expect("to set wayland handle");
                //     },
                //     winit::raw_window_handle::RawWindowHandle::Xlib(handle) => {
                //         // XLIB_HANDLE.set(handle.clone()).expect("to set xlib handle");
                //     },
                //     _ => {
                //         eprintln!("Expected a Windows, Wayland or Xlib window handle");
                //     }
                // }
            }

            MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
                // println!("MenuEvent: {:?}", event);
                // cloned_to_gui_tx.send(ToGUI::TaskBarIconMsg(format!("MenuEvent: {:?}", event))).expect("to send to gui");
                let MenuEvent { id } = event;
                match id.0.as_str() {
                    MID_SHOW => {
                        // show_window(&context);
                    }
                    MID_HIDE => {
                        // hide_window(&context);
                        // cloned_to_gui_tx.send(ToGUI::HideMainWindow()).expect("to send to gui");
                    }
                    MID_EXIT => {
                        std::process::exit(0);
                    }
                    _ => {
                        eprintln!("Unknown menu id: {}", id.0);
                    }
                }
            }));

            // tray-icon crate
            // https://docs.rs/tray-icon/0.12.0/tray_icon/struct.TrayIconEvent.html#method.set_event_handler
            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                // println!("TrayIconEvent: {:?}", event);
                // let _ = cloned_to_gui_tx.send(ToGUI::TaskBarIconMsg(format!("TrayIconEvent: {:?}", event))).expect("to send to gui");
                let (_id, _pos, _rect, btn, btn_state) = match event {
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
                        MouseButton::Left => {}
                        MouseButton::Right => {}
                        _ => {}
                    }
                }
            }));
            Ok(Box::new(MumbleBridgeApp::new(
                borrowed_to_gui_rx,
                from_gui_tx.clone(),
                cloned_shutdown_tx,
            )))
        }),
    )
    .expect("to run the app");
    // let null_tray_handler = move |_: TrayIconEvent| {};
    // TrayIconEvent::set_event_handler(Some(null_tray_handler));
    // let null_menu_handler = move |_: MenuEvent| {};
    // MenuEvent::set_event_handler(Some(null_menu_handler));

    println!("App closed");
    // set_window_visible((), false);
    // return;
    // while !is_window_visible() {
    //     std::thread::yield_now();
    //     std::thread::sleep(std::time::Duration::from_millis(1000));
    //     if shutdown_rx.try_recv().is_ok() {
    //         break 'outer;
    //     }
    //     println!("Waiting for window to be visible");
    // }
    // }
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
pub fn hide_window(ctx: &Context) {
    set_window_visible(ctx, false);
}

// #[cfg(not(windows))]
pub fn show_window(ctx: &Context) {
    set_window_visible(ctx, true);
}

// #[cfg(not(windows))]
pub fn set_window_visible(ctx: &Context, visible: bool) {
    // if let Some(wh) = WAYLAND_HANDLE.get() {

    // }
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(visible));
    println!("Setting window visible: {}", visible);
    *VISIBLE.lock().unwrap() = visible;
    println!("Window visible: {}", *VISIBLE.lock().unwrap());
}

const MID_SHOW: &str = "1";
const MID_HIDE: &str = "2";
const MID_EXIT: &str = "3";

pub fn generate_menu_entries<'a>() -> Vec<Box<dyn IsMenuItem>> {
    vec![
        // Box::new(MenuItem::with_id(MID_SHOW, "Show", true, None)),
        // Box::new(MenuItem::with_id(MID_HIDE, "Hide", true, None)),
        Box::new(MenuItem::with_id(MID_EXIT, "Exit", true, None)),
    ]
}

lazy_static! {
    static ref ICON_DATA: (Vec<u8>, u32, u32) = {
        let icon = include_bytes!("../assets/icon.ico");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let pixels = width * height;
        let mut rgba = image.into_raw();
        for _i in 0..pixels {
            let i = _i as usize * 4;
            // swap blue and red
            let tmp = rgba[i];
            rgba[i] = (rgba[i+2] as u16 * 2).min(255) as u8;
            rgba[i+2] = tmp;
            // let tmp_red = [rgba[i], rgba[i+1], rgba[i+2], rgba[i+3]];
            // let tmp_blue = [rgba[i+8], rgba[i+9], rgba[i+10], rgba[i+11]];
            // rgba[i..i+4].copy_from_slice(&tmp_blue);
            // rgba[i+8..i+12].copy_from_slice(&tmp_red);
        }
        (rgba, width, height)
    };
}

pub(crate) fn load_icon() -> egui::IconData {
    egui::IconData {
        rgba: ICON_DATA.0.clone(),
        width: ICON_DATA.1,
        height: ICON_DATA.2,
    }
}
