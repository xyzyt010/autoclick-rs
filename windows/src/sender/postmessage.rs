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
    let lparam_down = LPARAM(0x0000_0001); // repeat=1, scancode=0
    let lparam_up = LPARAM(0xC000_0001u32 as isize); // repeat=1, transition=1

    let down = PostMessageW(hwnd, WM_KEYDOWN, WPARAM(vk as usize), lparam_down);
    let up = PostMessageW(hwnd, WM_KEYUP, WPARAM(vk as usize), lparam_up);
    down.is_ok() && up.is_ok()
}
