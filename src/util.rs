use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_MENU};


pub fn is_alt_held() -> bool {
    unsafe {
        // GetAsyncKeyState returns a SHORT, and the high bit (0x8000)
        // indicates whether the key is currently down.
        (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000u16) != 0
    }
}
