use std::ffi::CString;
use std::ops::Deref;
use std::time::{Duration, Instant};
use std::{io, thread};
use std::sync::mpsc::{self, SendError, Sender};
use std::sync::{Arc, Mutex, RwLock};

use cgmath::{Vector3};
use cgmath::Quaternion;
use lazy_static::lazy_static;
use md5::{Md5, Digest};
use mumble_link::{MumbleLink, Position};
use windows::core::PCSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Memory::{
    MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS,
};

use crate::app::{ToGUI, MUMBLE_SCALE};
use crate::mp_telemetry_data::{AsCStrSlice, STelemetry};
use crate::tcp_server::{FromTM, LAST_CONTEXT};


pub fn read_telemetry() -> windows_core::Result<STelemetry> {
    let t_data;

    unsafe {
        let shared_mem_name = CString::new("ManiaPlanet_Telemetry").unwrap();
        // Attempt to open an existing file mapping with read-only access.
        let file_mapping: HANDLE = OpenFileMappingA(
            FILE_MAP_READ.0,
            false,
            PCSTR(shared_mem_name.as_ptr().cast()),
        )?;

        if file_mapping.is_invalid() {
            eprintln!(
                "Failed to open file mapping: GetLastError() = {}",
                windows::core::Error::from_win32()
            );
            return Err(windows::core::Error::from_win32());
        }

        // Map a view of the file into our address space.
        let view = MapViewOfFile(file_mapping, FILE_MAP_READ, 0, 0, 0);

        t_data = read_mp_telemetry(view)?;

        // Clean up.
        if let Err(e) = UnmapViewOfFile(view) {
            eprintln!("Failed to unmap view of file: {}", e);
        }
        if let Err(e) = CloseHandle(file_mapping) {
            eprintln!("Failed to close file mapping: {}", e);
        }
    }

    Ok(t_data)
}

unsafe fn read_mp_telemetry(view: MEMORY_MAPPED_VIEW_ADDRESS) -> windows_core::Result<STelemetry> {
    let telemetry_ptr = view.Value as *const STelemetry;
    let telemetry_val: STelemetry = *telemetry_ptr;
    Ok(telemetry_val)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MPData {
    pub curr_map: String,
    pub is_local_player: bool,
    pub player_name: String,
    pub race_state: u32,
}

impl Default for MPData {
    fn default() -> Self {
        MPData {
            curr_map: String::new(),
            is_local_player: false,
            player_name: String::new(),
            race_state: 0,
        }
    }
}

pub static MP_DATA_UDPATE: Mutex<usize> = Mutex::new(0);
lazy_static! {
    pub static ref MP_CTX_LAST: Mutex<MPData> = Mutex::new(Default::default());
}

pub fn run_mp_telemetry_loop(mumble: &Arc<RwLock<io::Result<MumbleLink>>>, to_gui: &Sender<ToGUI>) -> Result<(), SendError<ToGUI>> {
    let mut last_ctx_update = Instant::now();
    let mut last_update_nb_change = Instant::now();
    let mut last_update_nb = 0;
    let mut last_obj_ts = 0;
    let mut no_obj_frames = 0;
    let mut no_updates = false;
    let mut obj_updated;
    loop {
        thread::sleep(Duration::from_millis(10));
        let telemetry = match read_telemetry() {
            Ok(t) => t,
            Err(e) => {
                log::error!("Failed to read telemetry: {}", e);
                return Ok(());
            }
        };

        // Update the MumbleLink data
        let mut mumble_w = mumble.write().unwrap();
        let mumble = mumble_w.as_mut().unwrap();

        // Send the telemetry data to the GUI
        to_gui.send(ToGUI::Telemetry(telemetry)).unwrap();

        if last_update_nb != telemetry.update_number {
            last_update_nb = telemetry.update_number;
            last_update_nb_change = Instant::now();
            no_updates = false;
            obj_updated = last_obj_ts != telemetry.object.timestamp;
            last_obj_ts = telemetry.object.timestamp;
            if !obj_updated {
                no_obj_frames += 1;
            } else {
                no_obj_frames = 0;
            }
        } else if (Instant::now() - last_update_nb_change).as_secs_f32() > 2.0 && !no_updates {
            no_updates = true;
            // log::warn!("No telemetry update for 2 seconds. Exiting MP telemetry loop.");
            // break;
        }



        let curr_ctx = MPData {
            // curr_map: String::from_utf8_lossy(&telemetry.game.map_id).to_string(),
            // read cstr
            curr_map: String::from_utf8_lossy(&telemetry.game.map_id.as_cstr_vec()).to_string(),
            is_local_player: telemetry.player.is_local_player > 0,
            player_name: String::from_utf8_lossy(&telemetry.player.user_name.as_cstr_vec()).to_string(),
            race_state: telemetry.race.state,
        };

        // what has changed since last time
        let mut mp_data_update = MP_DATA_UDPATE.lock().unwrap();
        let mut mp_ctx_last = MP_CTX_LAST.lock().unwrap();

        let update_ctx = &curr_ctx != mp_ctx_last.deref()
            || (Instant::now() - last_ctx_update).as_secs_f32() > 5.0;

        if update_ctx {
            *mp_data_update += 1;
            let team_str = "All".to_string();
            let identity: String = format!("{}|{}|{}", &curr_ctx.player_name, &curr_ctx.player_name, *mp_data_update);
            let context: String = format!("TM|{}|{}", &obfs_uid_or_svr_login(&curr_ctx.curr_map), &team_str);
            mumble.set_identity(identity.as_str());
            mumble.set_context(context.as_bytes());
            *LAST_CONTEXT.lock().unwrap() = context;
            if let Err(e) = to_gui.send(FromTM::PlayerDetails(curr_ctx.player_name.clone(), curr_ctx.player_name.clone()).into()) {
                log::error!("Failed to send ctx/id to GUI: {:?}", e);
                break;
            }
            if let Err(e) = to_gui.send(FromTM::ServerDetails(curr_ctx.curr_map.clone(), team_str.clone()).into()) {
                log::error!("Failed to send ctx/id to GUI: {:?}", e);
                break;
            }
            last_ctx_update = Instant::now();
        }

        let unspawned = !curr_ctx.is_local_player
            || no_updates
            || no_obj_frames > 10;

        let mut player: Position = get_player_data_from_telemetry(&telemetry);
        if unspawned || no_updates {
            player = position_near_zero();
        }
        // let camera: Position = get_camera_data_from_telemetry(&telemetry);
        let p = player.into();
        let c = player.into();
        mumble.update(player, player);
        if let Err(e) = to_gui.send(FromTM::Positions {p, c}.into()) {
            log::error!("Failed to send p & c to GUI: {:?}", e);
            break;
        }


        *mp_ctx_last = curr_ctx;
    }
    log::warn!("MP telemetry loop ended.");
    Ok(())
}


pub fn position_near_zero() -> Position {
    Position {
        position: [0.005; 3],
        front: [0.0, 0.0, -1.0],
        top: [0.0, 1.0, 0.0],
    }
}


fn get_player_data_from_telemetry(telemetry: &STelemetry) -> Position {
    let position: Vector3<_> = telemetry.object.translation.into();
    let position = position * MUMBLE_SCALE;
    let rot_q: Quaternion<_> = telemetry.object.rotation.into();
    let dir = rot_q * cgmath::Vector3::unit_z();
    let up = rot_q * cgmath::Vector3::unit_y();
    Position {
        position: position.into(), front: dir.into(), top: up.into()
    }
}



/*
    -- Angelscript code:


string ObfsUidOrSvrLogin(const string &in svrLogin) {
    if (svrLogin.Length == 0) {
        return svrLogin;
    }
    MemoryBuffer@ buf = MemoryBuffer(16);
    buf.WriteFromHex(Crypto::MD5(svrLogin));
    return base63Encode(buf);
}

const string BASE63 = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";
const string EMPTY_25_CHARS = "                         ";
string base63Encode(MemoryBuffer@ buf) {
    buf.Seek(0);
    auto size = buf.GetSize();
    // preallocate 25 chars
    string ret = EMPTY_25_CHARS;
    uint64 val = 0;
    uint ix = 0;
    for (uint i = 0; i < size; i++) {
        val = (val << 8) | uint8(buf.ReadUInt8());
        while (val >= 63 && ix < 25) {
            ret[ix++] = BASE63[val % 63];
            val /= 63;
        }
    }
    if (val > 0 && ix < 25) {
        ret[ix++] = BASE63[val % 63];
    }
    ret = ret.SubStr(0, ix);
    // print("base63Encode: " + ret + " (" + size + " -> " + ret.Length + ")");
    return ret;
}


*/

pub fn obfs_uid_or_svr_login(svr_login: &str) -> String {
    if svr_login.is_empty() {
        return svr_login.to_string();
    }

    let mut hasher = Md5::new();
    hasher.update(svr_login.as_bytes());
    let result = hasher.finalize();
    unsafe { base63_encode(&result) }
}

const BASE63: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";
const EMPTY_25_CHARS: &str = "                         ";
pub unsafe fn base63_encode(buf: &[u8]) -> String {
    let mut ret = EMPTY_25_CHARS.to_string();
    let mut val = 0u64;
    let mut ix = 0;

    for &b in buf {
        val = (val << 8) | b as u64;
        while val >= 63 && ix < 25 {
            ret.as_bytes_mut()[ix] = BASE63.as_bytes()[val as usize % 63];
            val /= 63;
            ix += 1;
        }
    }

    if val > 0 && ix < 25 {
        ret.as_bytes_mut()[ix] = BASE63.as_bytes()[val as usize % 63];
        ix += 1;
    }

    ret.truncate(ix);
    ret
}
