//! Target enumeration for Linux.
//! - X11: enumerate visible windows with titles + PIDs via x11rb.
//! - Wayland: list terminal/app processes (no window targeting possible).
//! - Both: scan /proc for shell/terminal processes.

use crate::detect::DisplayServer;
use crate::injector::x11::X11Backend;

#[derive(Clone, Debug)]
pub struct Target {
    pub pid: u32,
    /// X11 window ID (0 if not applicable / Wayland).
    pub window_id: u32,
    pub name: String,
    pub title: String,
    pub mode: TargetMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetMode {
    Terminal,
    App,
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

/// Known terminal emulator process names on Linux.
const TERMINAL_NAMES: &[&str] = &[
    "gnome-terminal",
    "konsole",
    "xfce4-terminal",
    "mate-terminal",
    "tilix",
    "terminator",
    "alacritty",
    "kitty",
    "wezterm",
    "foot",
    "st",
    "xterm",
    "urxvt",
    "lxterminal",
    "sakura",
    "hyper",
    "tabby",
    "warp",
    "bash",
    "zsh",
    "fish",
    "sh",
];

/// Enumerate targets based on display server.
pub fn enumerate(ds: DisplayServer, mode: TargetMode, exclude_pid: u32) -> Vec<Target> {
    match (ds, mode) {
        (DisplayServer::X11, TargetMode::App) => enumerate_x11_windows(exclude_pid),
        (DisplayServer::X11, TargetMode::Terminal) => enumerate_terminals_x11(exclude_pid),
        (DisplayServer::Wayland, _) => enumerate_processes(mode, exclude_pid),
    }
}

/// X11: list visible windows (App mode).
fn enumerate_x11_windows(exclude_pid: u32) -> Vec<Target> {
    let backend = match X11Backend::connect() {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };
    let windows = match backend.list_windows() {
        Ok(w) => w,
        Err(_) => return Vec::new(),
    };

    windows
        .into_iter()
        .filter(|(_, pid, title)| {
            *pid != exclude_pid && !title.is_empty()
        })
        .map(|(wid, pid, title)| {
            let name = process_name_by_pid(pid).unwrap_or_else(|| "unknown".into());
            Target {
                pid,
                window_id: wid,
                name,
                title,
                mode: TargetMode::App,
            }
        })
        .collect()
}

/// X11: find terminal windows specifically.
fn enumerate_terminals_x11(exclude_pid: u32) -> Vec<Target> {
    let all = enumerate_x11_windows(exclude_pid);
    all.into_iter()
        .filter(|t| {
            let lower = t.name.to_lowercase();
            TERMINAL_NAMES.iter().any(|n| lower.contains(n))
        })
        .map(|mut t| {
            t.mode = TargetMode::Terminal;
            t
        })
        .collect()
}

/// Wayland fallback: scan processes (no window IDs available).
fn enumerate_processes(mode: TargetMode, exclude_pid: u32) -> Vec<Target> {
    use sysinfo::System;

    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    let mut results = Vec::new();
    for (pid, proc) in sys.processes() {
        let pid_u32 = pid.as_u32();
        if pid_u32 == exclude_pid {
            continue;
        }
        let name = proc.name().to_string_lossy().to_lowercase();

        let is_terminal = TERMINAL_NAMES.iter().any(|t| name.contains(t));
        match mode {
            TargetMode::Terminal if !is_terminal => continue,
            TargetMode::App if is_terminal => continue,
            _ => {}
        }

        // Skip kernel threads and system daemons.
        if pid_u32 < 100 && mode == TargetMode::App {
            continue;
        }

        results.push(Target {
            pid: pid_u32,
            window_id: 0,
            name: proc.name().to_string_lossy().to_string(),
            title: String::new(),
            mode,
        });
    }

    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    results.truncate(50); // Limit list size.
    results
}

/// Get process name from /proc/<pid>/comm.
fn process_name_by_pid(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
}
