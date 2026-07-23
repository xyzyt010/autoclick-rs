//! Target enumeration for Linux.
//! - X11: enumerate visible windows with titles + PIDs via x11rb (no XTest needed).
//! - Wayland: list terminal/app processes (no window targeting possible).

use crate::detect::DisplayServer;
use crate::injector::x11::X11Connection;

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

/// Enumerate ALL targets (terminals + apps) for the unified dropdown.
pub fn enumerate_all(ds: DisplayServer, exclude_pid: u32) -> Vec<Target> {
    match ds {
        DisplayServer::X11 => enumerate_x11_windows(exclude_pid, false),
        DisplayServer::Wayland => enumerate_processes_all(exclude_pid),
    }
}

/// Enumerate targets based on display server.
pub fn enumerate(ds: DisplayServer, mode: TargetMode, exclude_pid: u32) -> Vec<Target> {
    match (ds, mode) {
        (DisplayServer::X11, TargetMode::App) => enumerate_x11_windows(exclude_pid, false),
        (DisplayServer::X11, TargetMode::Terminal) => enumerate_x11_windows(exclude_pid, true),
        (DisplayServer::Wayland, _) => enumerate_processes(mode, exclude_pid),
    }
}

/// X11: list windows. Uses lightweight X11Connection (no XTest required).
fn enumerate_x11_windows(exclude_pid: u32, terminals_only: bool) -> Vec<Target> {
    let conn = match X11Connection::connect() {
        Ok(c) => c,
        Err(_) => {
            // X11 not available — fall back to process listing.
            return enumerate_processes(
                if terminals_only { TargetMode::Terminal } else { TargetMode::App },
                exclude_pid,
            );
        }
    };

    let windows = match conn.list_windows() {
        Ok(w) => w,
        Err(_) => {
            return enumerate_processes(
                if terminals_only { TargetMode::Terminal } else { TargetMode::App },
                exclude_pid,
            );
        }
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
                mode: TargetMode::App,
            }
        })
        .collect();

    // Filter for terminals if requested.
    if terminals_only {
        results.retain(|t| {
            let lower = t.name.to_lowercase();
            let title_lower = t.title.to_lowercase();
            TERMINAL_NAMES.iter().any(|n| lower.contains(n) || title_lower.contains(n))
        });
        for t in results.iter_mut() {
            t.mode = TargetMode::Terminal;
        }
    }

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
        // Skip kernel threads.
        if name.is_empty() {
            continue;
        }
        let is_terminal = TERMINAL_NAMES.iter().any(|t| name.contains(t));
        results.push(Target {
            pid: pid_u32,
            window_id: 0,
            name: proc.name().to_string_lossy().to_string(),
            title: String::new(),
            mode: if is_terminal { TargetMode::Terminal } else { TargetMode::App },
        });
    }

    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    results.truncate(80);
    results
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
    results.truncate(50);
    results
}

/// Get process name from /proc/<pid>/comm.
fn process_name_by_pid(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
}
