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

impl From<String> for KeySendError {
    fn from(s: String) -> Self { KeySendError(s) }
}

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
/// When `force_vk` is true the combo contains modifier keys. In that case
/// the regular key is routed through SendInput too (matching the modifier
/// hold/release path) so the target sees a coherent keyboard state.
pub fn send_key(
    hwnd: Option<HWND>,
    pid: Option<u32>,
    key: KeyInfo,
    force_vk: bool,
) -> Result<Method, KeySendError> {
    let uni = if force_vk { 0 } else { key.unicode };

    if let Some(h) = hwnd {
        // GUI target — PostMessage for plain keys, SendInput for combos.
        if force_vk {
            unsafe {
                sendinput::send_with_focus(h, key.vk, 0);
            }
            return Ok(Method::SendInput);
        }
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

/// Press a key down only (modifier hold). Uses SendInput for GUI targets
/// because PostMessage doesn't actually hold keys at the OS level — it just
/// posts messages. For modifier combos (Ctrl+C, Alt+Tab, etc.) the modifier
/// must be held via SendInput so the target sees the real keyboard state.
pub fn key_down(
    hwnd: Option<HWND>,
    pid: Option<u32>,
    key: KeyInfo,
) -> Result<Method, KeySendError> {
    // Console target — WriteConsoleInput.
    if let Some(p) = pid {
        match console::down(p, key.vk, 0) {
            Ok(()) => return Ok(Method::WriteConsoleInput),
            Err(_) => {} // fall through to SendInput
        }
    }

    // GUI target or fallback — SendInput (the only method that actually holds
    // keys at the OS level so the target sees the modifier state).
    if let Some(h) = hwnd {
        unsafe {
            sendinput::down_vk(h, key.vk);
            return Ok(Method::SendInput);
        }
    }

    Err(KeySendError("No injection target (hwnd/pid) for key-down".into()))
}

/// Release a key (modifier release). Same logic as key_down — uses SendInput
/// for GUI targets to match the hold method.
pub fn key_up(
    hwnd: Option<HWND>,
    pid: Option<u32>,
    key: KeyInfo,
) -> Result<Method, KeySendError> {
    // Console target — WriteConsoleInput.
    if let Some(p) = pid {
        match console::up(p, key.vk, 0) {
            Ok(()) => return Ok(Method::WriteConsoleInput),
            Err(_) => {} // fall through to SendInput
        }
    }

    // GUI target or fallback — SendInput.
    if let Some(h) = hwnd {
        unsafe {
            sendinput::up_vk(h, key.vk);
            return Ok(Method::SendInput);
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

/// Send an entire modifier combo in one shot for a GUI target.
/// Builds: [mod_down, …, regular_down, regular_up, …, mod_up] and sends
/// them in a single `SendInput` call so the OS sees a coherent keyboard state.
pub fn send_combo_gui(hwnd: HWND, modifiers: &[KeyInfo], regular: &[KeyInfo]) -> Result<Method, KeySendError> {
    use crate::sender::sendinput::ComboEvent;
    let mut events = Vec::with_capacity(modifiers.len() * 2 + regular.len() * 2);
    for &k in modifiers {
        events.push(ComboEvent::Down(k.vk));
    }
    for &k in regular {
        events.push(ComboEvent::Down(k.vk));
        events.push(ComboEvent::Up(k.vk));
    }
    for &k in modifiers.iter().rev() {
        events.push(ComboEvent::Up(k.vk));
    }
    unsafe { sendinput::combo(hwnd, &events); }
    Ok(Method::SendInput)
}

/// Screen Automation mode: send an entire combo in one shot.
pub fn send_combo_screen(modifiers: &[KeyInfo], regular: &[KeyInfo]) -> Result<Method, KeySendError> {
    unsafe {
        for &k in modifiers { screen::down(k.vk); }
        for &k in regular { screen::send(k.vk, 0); }
        for &k in modifiers.iter().rev() { screen::up(k.vk); }
    }
    Ok(Method::ScreenInput)
}

/// Console target: send an entire combo. Modifiers are held/released via
/// WriteConsoleInput. The regular key gets `dwControlKeyState` set so the
/// terminal app sees the modifier state.
pub fn send_combo_console(
    pid: Option<u32>,
    modifiers: &[KeyInfo],
    regular: &[KeyInfo],
) -> Result<Method, KeySendError> {
    let p = pid.ok_or_else(|| KeySendError("No pid for console combo".into()))?;

    // Compute the Windows control-key-state flags from the modifier list.
    // Raw constants from Win32 Console API (windows 0.58).
    const SHIFT_PRESSED: u32 = 16;
    const LEFT_CTRL_PRESSED: u32 = 8;
    const LEFT_ALT_PRESSED: u32 = 2;
    const NUMLOCK_ON: u32 = 32;
    const SCROLLLOCK_ON: u32 = 64;
    const ENHANCED_KEY: u32 = 256;
    let mut ctrl_state: u32 = 0;
    for &k in modifiers {
        match k.vk {
            0x10 => ctrl_state |= SHIFT_PRESSED,          // VK_SHIFT
            0x11 => ctrl_state |= LEFT_CTRL_PRESSED,       // VK_CONTROL
            0x12 => ctrl_state |= LEFT_ALT_PRESSED,        // VK_MENU (Alt)
            0x14 => ctrl_state |= ENHANCED_KEY,            // VK_CAPITAL (CapsLock)
            0x90 => ctrl_state |= NUMLOCK_ON,              // VK_NUMLOCK
            0x91 => ctrl_state |= SCROLLLOCK_ON,           // VK_SCROLL
            _ => {}
        }
    }
    if ctrl_state != 0 {
        ctrl_state |= ENHANCED_KEY;
    }

    // 1. Hold modifiers.
    for &k in modifiers {
        console::down(p, k.vk, 0)?;
    }

    // 2. Tap each regular key with the modifier state.
    for &k in regular {
        console::send_with_ctrl(p, k.vk, k.unicode, ctrl_state)?;
    }

    // 3. Release modifiers in reverse.
    for &k in modifiers.iter().rev() {
        console::up(p, k.vk, 0)?;
    }

    Ok(Method::WriteConsoleInput)
}
