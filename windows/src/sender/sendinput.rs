#![allow(unsafe_code)]

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY, KEYBD_EVENT_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};

/// Send a key via SendInput after bringing the target window to foreground.
pub unsafe fn send_with_focus(hwnd: HWND, vk: u16, unicode: u16) {
    let _ = ShowWindow(hwnd, SW_RESTORE);
    let _ = SetForegroundWindow(hwnd);
    std::thread::sleep(std::time::Duration::from_millis(30));

    if unicode != 0 && unicode >= 0x20 {
        send_unicode(unicode);
    } else {
        send_vk(vk);
    }
}

unsafe fn send_vk(vk: u16) {
    let mut inputs = [
        make_vk_input(vk, false),
        make_vk_input(vk, true),
    ];
    SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32);
}

unsafe fn send_unicode(ch: u16) {
    let mut inputs = [
        make_unicode_input(ch, false),
        make_unicode_input(ch, true),
    ];
    SendInput(&mut inputs, std::mem::size_of::<INPUT>() as i32);
}

unsafe fn make_vk_input(vk: u16, up: bool) -> INPUT {
    let mut input: INPUT = std::mem::zeroed();
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki.wVk = VIRTUAL_KEY(vk);
    input.Anonymous.ki.wScan = 0;
    input.Anonymous.ki.dwFlags = if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) };
    input.Anonymous.ki.time = 0;
    input.Anonymous.ki.dwExtraInfo = 0;
    input
}

unsafe fn make_unicode_input(ch: u16, up: bool) -> INPUT {
    let mut input: INPUT = std::mem::zeroed();
    input.r#type = INPUT_KEYBOARD;
    input.Anonymous.ki.wVk = VIRTUAL_KEY(0);
    input.Anonymous.ki.wScan = ch;
    input.Anonymous.ki.dwFlags = if up {
        KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
    } else {
        KEYEVENTF_UNICODE
    };
    input.Anonymous.ki.time = 0;
    input.Anonymous.ki.dwExtraInfo = 0;
    input
}
