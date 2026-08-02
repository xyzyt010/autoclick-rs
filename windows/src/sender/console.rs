#![allow(unsafe_code)]

use windows::Win32::System::Console::{
    AttachConsole, FreeConsole, GetStdHandle, WriteConsoleInputW, STD_INPUT_HANDLE,
    INPUT_RECORD, KEY_EVENT,
};

/// Send a key to a console process via WriteConsoleInput.
pub fn send(pid: u32, vk: u16, unicode: u16) -> Result<(), String> {
    down(pid, vk, unicode)?;
    up(pid, vk, unicode)
}

/// Write only a key-down event (used to hold modifier keys during a combo).
pub fn down(pid: u32, vk: u16, unicode: u16) -> Result<(), String> {
    unsafe {
        let _ = FreeConsole();
        AttachConsole(pid).map_err(|e| format!("AttachConsole({pid}): {e}"))?;

        let handle = GetStdHandle(STD_INPUT_HANDLE)
            .map_err(|e| format!("GetStdHandle: {e}"))?;

        let mut record: INPUT_RECORD = std::mem::zeroed();
        record.EventType = KEY_EVENT as u16;
        record.Event.KeyEvent.bKeyDown = true.into();
        record.Event.KeyEvent.wRepeatCount = 1;
        record.Event.KeyEvent.wVirtualKeyCode = vk;
        record.Event.KeyEvent.wVirtualScanCode = 0;
        record.Event.KeyEvent.uChar.UnicodeChar = unicode;
        record.Event.KeyEvent.dwControlKeyState = std::mem::zeroed();

        let mut written: u32 = 0;
        let ok = WriteConsoleInputW(handle, &[record], &mut written);

        let _ = FreeConsole();

        if ok.is_ok() && written > 0 {
            Ok(())
        } else {
            Err("WriteConsoleInput failed".into())
        }
    }
}

/// Write only a key-up event (used to release modifier keys after a combo).
pub fn up(pid: u32, vk: u16, unicode: u16) -> Result<(), String> {
    unsafe {
        let _ = FreeConsole();
        AttachConsole(pid).map_err(|e| format!("AttachConsole({pid}): {e}"))?;

        let handle = GetStdHandle(STD_INPUT_HANDLE)
            .map_err(|e| format!("GetStdHandle: {e}"))?;

        let mut record: INPUT_RECORD = std::mem::zeroed();
        record.EventType = KEY_EVENT as u16;
        record.Event.KeyEvent.bKeyDown = false.into();
        record.Event.KeyEvent.wRepeatCount = 1;
        record.Event.KeyEvent.wVirtualKeyCode = vk;
        record.Event.KeyEvent.wVirtualScanCode = 0;
        record.Event.KeyEvent.uChar.UnicodeChar = unicode;
        record.Event.KeyEvent.dwControlKeyState = std::mem::zeroed();

        let mut written: u32 = 0;
        let ok = WriteConsoleInputW(handle, &[record], &mut written);

        let _ = FreeConsole();

        if ok.is_ok() && written > 0 {
            Ok(())
        } else {
            Err("WriteConsoleInput failed".into())
        }
    }
}
