//! Target enumeration for Linux.
//! - X11: enumerate visible windows with titles + PIDs via x11rb (no XTest needed).
//! - Wayland: list all processes (no window targeting possible).

use crate::detect::DisplayServer;
use crate::injector::x11::X11Connection;

#[derive(Clone, Debug)]
pub struct Target {
    pub pid: u32,
    /// X11 window ID (0 if not applicable / Wayland).
    pub window_id: u32,
    pub name: String,
    pub title: String,
}

impl Target {
    pub fn label(&self) -> String {
        if self.window_id != 0 {
            format!("{}  (PID {})  \"{}\"", self.name, self.pid, self.title)
        } else {
            format!("{}  (PID {})", self.name, self.pid)
        }
    }
}

/// Enumerate ALL targets (terminals + apps) for the unified dropdown.
pub fn enumerate_all(ds: DisplayServer, exclude_pid: u32) -> Vec<Target> {
    match ds {
        DisplayServer::X11 => enumerate_x11_windows(exclude_pid),
        DisplayServer::Wayland => enumerate_processes_all(exclude_pid),
    }
}

/// X11: list windows. Uses lightweight X11Connection (no XTest required).
fn enumerate_x11_windows(exclude_pid: u32) -> Vec<Target> {
    let conn = match X11Connection::connect() {
        Ok(c) => c,
        Err(_) => return enumerate_processes_all(exclude_pid),
    };

    let windows = match conn.list_windows() {
        Ok(w) => w,
        Err(_) => return enumerate_processes_all(exclude_pid),
    };

    let mut results: Vec<Target> = windows
        .into_iter()
        .filter(|(_, pid, title)| *pid != exclude_pid && !title.is_empty())
        .map(|(wid, pid, title)| {
            let name = process_name_by_pid(pid).unwrap_or_else(|| "unknown".into());
            Target {
                pid,
                window_id: wid,
                name,
                title,
            }
        })
        .collect();

    results.truncate(100);
    results
}

/// Wayland fallback: scan ALL processes (no window IDs available).
fn enumerate_processes_all(exclude_pid: u32) -> Vec<Target> {
    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut results = Vec::new();
    for (pid, proc) in sys.processes() {
        let pid_u32 = pid.as_u32();
        if pid_u32 == exclude_pid || pid_u32 < 100 {
            continue;
        }
        let name = proc.name().to_string_lossy().to_lowercase();
        if name.is_empty() {
            continue;
        }
        results.push(Target {
            pid: pid_u32,
            window_id: 0,
            name: proc.name().to_string_lossy().to_string(),
            title: String::new(),
        });
    }

    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    results.truncate(80);
    results
}

/// Get process name from /proc/<pid>/comm.
fn process_name_by_pid(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
}
