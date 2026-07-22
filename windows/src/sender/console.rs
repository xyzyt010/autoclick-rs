#![allow(unsafe_code)]

use windows::Win32::System::Console::{
    AttachConsole, FreeConsole, GetStdHandle, WriteConsoleInputW, STD_INPUT_HANDLE,
    INPUT_RECORD, KEY_EVENT,
};

/// Send a key to a console process via WriteConsoleInput.
pub fn send(pid: u32, vk: u16, unicode: u16) -> Result<(), String> {
    unsafe {
        let _ = FreeConsole();
        AttachConsole(pid).map_err(|e| format!("AttachConsole({pid}): {e}"))?;

        let handle = GetStdHandle(STD_INPUT_HANDLE)
            .map_err(|e| format!("GetStdHandle: {e}"))?;

        // Build key-down event.
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

        // Key-up event.
        record.Event.KeyEvent.bKeyDown = false.into();
        let mut written2: u32 = 0;
        let _ = WriteConsoleInputW(handle, &[record], &mut written2);

        let _ = FreeConsole();

        if ok.is_ok() && written > 0 {
            Ok(())
        } else {
            Err("WriteConsoleInput failed".into())
        }
    }
}
