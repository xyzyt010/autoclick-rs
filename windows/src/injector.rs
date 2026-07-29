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

/// Window mode: composite key sender with fallback chain:
/// 1. PostMessage (if hwnd) — no focus steal
/// 2. WriteConsoleInput (if pid — console apps)
/// 3. SendInput (bring to foreground first)
pub fn send_key(hwnd: Option<HWND>, pid: Option<u32>, key: KeyInfo) -> Result<Method, KeySendError> {
    let mut errors: Vec<String> = Vec::with_capacity(3);

    if let Some(h) = hwnd {
        unsafe {
            if postmessage::send(h, key.vk, key.unicode) {
                return Ok(Method::PostMessage);
            } else {
                errors.push("PostMessage: app did not accept posted keys".into());
            }
        }
    }

    if let Some(p) = pid {
        match console::send(p, key.vk, key.unicode) {
            Ok(()) => return Ok(Method::WriteConsoleInput),
            Err(e) => errors.push(format!("WriteConsoleInput: {e}")),
        }
    }

    if let Some(h) = hwnd {
        unsafe {
            sendinput::send_with_focus(h, key.vk, key.unicode);
            return Ok(Method::SendInput);
        }
    }

    Err(KeySendError(format!(
        "All injection methods failed: {}",
        errors.join("; ")
    )))
}

/// Screen Automation mode: inject at OS level via SendInput.
/// Does not attempt PostMessage or WriteConsoleInput.
pub fn send_key_screen(key: KeyInfo) -> Result<Method, KeySendError> {
    unsafe {
        Ok(screen::send(key.vk, key.unicode))
    }
}
