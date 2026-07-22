use super::{Target, TargetMode};
use std::sync::OnceLock;

use windows::Win32::System::Console::{AttachConsole, FreeConsole, GetConsoleTitleW};

const SHELL_NAMES: [&str; 4] = ["powershell.exe", "pwsh.exe", "cmd.exe", "wsl.exe"];

pub fn list_candidate_shells() -> Vec<Target> {
    let mut out = Vec::with_capacity(8);
    let sys = sysinfo_guard();
    for (pid, process) in sys.processes() {
        let name = lossy_lowercase(process.name());
        if !SHELL_NAMES.contains(&name.as_str()) {
            continue;
        }
        let pid_u32 = pid.as_u32();
        let (title, accessible) = probe_console(pid_u32);
        out.push(Target {
            pid: pid_u32,
            hwnd: 0,
            name: lossy_string(process.name()),
            title,
            mode: TargetMode::Terminal,
            accessible,
        });
    }
    out
}

fn probe_console(pid: u32) -> (String, bool) {
    static LOCK: parking_lot::Mutex<()> = parking_lot::Mutex::new(());
    let _guard = LOCK.lock();

    unsafe {
        let _ = FreeConsole();
        match AttachConsole(pid) {
            Ok(()) => {}
            Err(e) => {
                let code = e.code().0 as u32;
                let reason = if code == 5 {
                    "access denied (elevated target?)"
                } else if code == 6 {
                    "no console attached"
                } else if code == 87 {
                    "process exited"
                } else {
                    return (format!("PID {pid} (error {code})"), false);
                };
                return (format!("PID {pid} ({reason})"), false);
            }
        }

        let mut buf = [0u16; 512];
        let len = GetConsoleTitleW(&mut buf);
        let title = String::from_utf16_lossy(&buf[..len as usize])
            .trim_end_matches('\0')
            .to_string();
        let title = if title.is_empty() { format!("PID {pid}") } else { title };
        let _ = FreeConsole();
        (title, true)
    }
}

fn sysinfo_guard() -> &'static sysinfo::System {
    static SYS: OnceLock<sysinfo::System> = OnceLock::new();
    SYS.get_or_init(|| {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        sys
    })
}

fn lossy_lowercase(name: impl AsRef<std::ffi::OsStr>) -> String {
    name.as_ref().to_string_lossy().to_lowercase()
}

fn lossy_string(name: impl AsRef<std::ffi::OsStr>) -> String {
    name.as_ref().to_string_lossy().into_owned()
}

#[allow(dead_code)]
pub fn refresh_system() {
    let _ = sysinfo_guard();
}
