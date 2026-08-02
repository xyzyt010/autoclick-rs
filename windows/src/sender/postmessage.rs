#![allow(unsafe_code)]

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CHAR, WM_KEYDOWN, WM_KEYUP};

/// Send a key via PostMessage (WM_KEYDOWN + WM_KEYUP, or WM_CHAR for unicode).
/// Returns true if the message was accepted by the target window.
pub unsafe fn send(hwnd: HWND, vk: u16, unicode: u16) -> bool {
    // For printable characters, send WM_CHAR directly.
    if unicode != 0 && unicode >= 0x20 && unicode != 0x7F {
        let ok = PostMessageW(hwnd, WM_CHAR, WPARAM(unicode as usize), LPARAM(0));
        return ok.is_ok();
    }

    // For non-printable keys, send WM_KEYDOWN + WM_KEYUP.
    down(hwnd, vk) && up(hwnd, vk)
}

/// Post only WM_KEYDOWN (used to hold modifier keys during a combo).
pub unsafe fn down(hwnd: HWND, vk: u16) -> bool {
    let lparam_down = LPARAM(0x0000_0001); // repeat=1, scancode=0
    PostMessageW(hwnd, WM_KEYDOWN, WPARAM(vk as usize), lparam_down).is_ok()
}

/// Post only WM_KEYUP (used to release modifier keys after a combo).
pub unsafe fn up(hwnd: HWND, vk: u16) -> bool {
    let lparam_up = LPARAM(0xC000_0001u32 as isize); // repeat=1, transition=1
    PostMessageW(hwnd, WM_KEYUP, WPARAM(vk as usize), lparam_up).is_ok()
}
