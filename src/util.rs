#[cfg(windows)]
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_MENU};


#[cfg(windows)]
pub fn is_alt_held() -> bool {
    unsafe {
        // GetAsyncKeyState returns a SHORT, and the high bit (0x8000)
        // indicates whether the key is currently down.
        (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000u16) != 0
    }
}


// Native Linux supports the TCP plugin path; Windows shared-memory telemetry
// cannot be opened outside the game's Wine/Proton process environment.
#[cfg(not(windows))]
pub fn is_alt_held() -> bool {
    false
}
