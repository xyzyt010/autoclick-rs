#![allow(unsafe_code)]

use crate::keyboard::KeyInfo;
use crate::sender::{console, postmessage, screen, sendinput, Method};
use windows::Win32::Foundation::HWND;

#[derive(Debug)]
pub struct KeySendError(pub String);

impl std::fmt::Display for KeySendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for KeySendError {}

/// Injection mode — controls which backend is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Window mode: PostMessage → WriteConsoleInput → SendInput fallback chain.
    Window,
    /// Screen Automation: direct OS-level SendInput (no focus steal attempt).
    Screen,
}

/// Window mode — method chosen by target type (no fallback chain):
///   GUI app   (hwnd present)  → PostMessage — no focus steal
///   Terminal  (hwnd absent)   → WriteConsoleInput
///
/// `force_vk` is true when the combo contains modifier keys: printable keys
/// must then be injected as virtual-key presses so the app sees the shortcut
/// (e.g. Ctrl+C) instead of plain text input.
pub fn send_key(
    hwnd: Option<HWND>,
    pid: Option<u32>,
    key: KeyInfo,
    force_vk: bool,
) -> Result<Method, KeySendError> {
    let uni = if force_vk { 0 } else { key.unicode };

    if let Some(h) = hwnd {
        // GUI target — PostMessage only, no focus steal.
        unsafe {
            if postmessage::send(h, key.vk, uni) {
                return Ok(Method::PostMessage);
            }
        }
        return Err(KeySendError(
            "PostMessage failed — app did not accept posted keys".into(),
        ));
    }

    if let Some(p) = pid {
        // Terminal / console target — WriteConsoleInput.
        match console::send(p, key.vk, uni) {
            Ok(()) => return Ok(Method::WriteConsoleInput),
            Err(e) => return Err(KeySendError(format!("WriteConsoleInput: {e}"))),
        }
    }

    Err(KeySendError(
        "No injection target (hwnd/pid) provided".into(),
    ))
}

/// Press a key down only (modifier hold). Same method-per-target as `send_key`.
pub fn key_down(
    hwnd: Option<HWND>,
    pid: Option<u32>,
    key: KeyInfo,
) -> Result<Method, KeySendError> {
    if let Some(h) = hwnd {
        unsafe {
            if postmessage::down(h, key.vk) {
                return Ok(Method::PostMessage);
            }
        }
        return Err(KeySendError("PostMessage key-down failed".into()));
    }

    if let Some(p) = pid {
        match console::down(p, key.vk, 0) {
            Ok(()) => return Ok(Method::WriteConsoleInput),
            Err(e) => return Err(KeySendError(format!("WriteConsoleInput: {e}"))),
        }
    }

    Err(KeySendError("No injection target (hwnd/pid) for key-down".into()))
}

/// Release a key (modifier release). Same method-per-target as `send_key`.
pub fn key_up(
    hwnd: Option<HWND>,
    pid: Option<u32>,
    key: KeyInfo,
) -> Result<Method, KeySendError> {
    if let Some(h) = hwnd {
        unsafe {
            if postmessage::up(h, key.vk) {
                return Ok(Method::PostMessage);
            }
        }
        return Err(KeySendError("PostMessage key-up failed".into()));
    }

    if let Some(p) = pid {
        match console::up(p, key.vk, 0) {
            Ok(()) => return Ok(Method::WriteConsoleInput),
            Err(e) => return Err(KeySendError(format!("WriteConsoleInput: {e}"))),
        }
    }

    Err(KeySendError("No injection target (hwnd/pid) for key-up".into()))
}

/// Screen Automation mode: inject at OS level via SendInput.
/// Does not attempt PostMessage or WriteConsoleInput.
pub fn send_key_screen(key: KeyInfo, force_vk: bool) -> Result<Method, KeySendError> {
    unsafe {
        let uni = if force_vk { 0 } else { key.unicode };
        Ok(screen::send(key.vk, uni))
    }
}

/// Screen Automation mode: press a key down only (modifier hold).
pub fn key_down_screen(key: KeyInfo) -> Result<Method, KeySendError> {
    unsafe {
        screen::down(key.vk);
        Ok(Method::ScreenInput)
    }
}

/// Screen Automation mode: release a key (modifier release).
pub fn key_up_screen(key: KeyInfo) -> Result<Method, KeySendError> {
    unsafe {
        screen::up(key.vk);
        Ok(Method::ScreenInput)
    }
}
