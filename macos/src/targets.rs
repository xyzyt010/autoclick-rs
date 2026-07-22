//! macOS window enumeration via CGWindowListCopyWindowInfo.
//! Lists all visible application windows with their titles and owning PIDs.

use core_foundation::array::CFArray;
use core_foundation::base::TCFType;
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::string::CFString;
use core_graphics::window::{
    kCGNullWindowID, kCGWindowListOptionAll, kCGWindowListOptionOnScreenOnly,
    CGWindowListCopyWindowInfo,
};

#[derive(Clone, Debug)]
pub struct Target {
    pub pid: u32,
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
        if self.title.is_empty() {
            format!("{}  (PID {})", self.name, self.pid)
        } else {
            format!("{}  (PID {})  \"{}\"", self.name, self.pid, self.title)
        }
    }
}

/// Known terminal emulator process names on macOS.
const TERMINAL_NAMES: &[&str] = &[
    "terminal",
    "iterm2",
    "iterm",
    "alacritty",
    "kitty",
    "wezterm",
    "hyper",
    "tabby",
    "warp",
];

/// Enumerate visible windows on macOS.
pub fn enumerate(mode: TargetMode, exclude_pid: u32) -> Vec<Target> {
    let windows = list_windows(exclude_pid);

    match mode {
        TargetMode::Terminal => windows
            .into_iter()
            .filter(|t| {
                let lower = t.name.to_lowercase();
                TERMINAL_NAMES.iter().any(|n| lower.contains(n))
            })
            .collect(),
        TargetMode::App => windows,
    }
}

/// Use CGWindowListCopyWindowInfo to get all on-screen windows.
fn list_windows(exclude_pid: u32) -> Vec<Target> {
    let mut results = Vec::new();

    unsafe {
        let window_list = CGWindowListCopyWindowInfo(
            kCGWindowListOptionOnScreenOnly,
            kCGNullWindowID,
        );

        let windows: CFArray<CFDictionary<CFString, *const std::ffi::c_void>> =
            TCFType::wrap_under_create_rule(window_list as *const _);

        for i in 0..windows.len() {
            let dict = windows.get(i).unwrap();

            // Get window name (kCGWindowOwnerName).
            let name = get_dict_string(&dict, "kCGWindowOwnerName");
            // Get window title (kCGWindowName).
            let title = get_dict_string(&dict, "kCGWindowName");
            // Get owning PID (kCGWindowOwnerPID).
            let pid = get_dict_number(&dict, "kCGWindowOwnerPID").unwrap_or(0) as u32;
            // Get window ID (kCGWindowNumber).
            let wid = get_dict_number(&dict, "kCGWindowNumber").unwrap_or(0) as u32;
            // Get layer (kCGWindowLayer) — skip non-zero layers (menus, overlays).
            let layer = get_dict_number(&dict, "kCGWindowLayer").unwrap_or(-1);

            if pid == exclude_pid || pid == 0 {
                continue;
            }
            // Skip system layers (menu bar, dock, overlays).
            if layer != 0 {
                continue;
            }
            // Skip windows with no name and no title.
            if name.is_empty() && title.is_empty() {
                continue;
            }

            results.push(Target {
                pid,
                window_id: wid,
                name: if name.is_empty() { "unknown".to_string() } else { name },
                title,
                mode: TargetMode::App,
            });
        }
    }

    // Deduplicate by (pid, title).
    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    results.dedup_by(|a, b| a.pid == b.pid && a.title == b.title);
    results.truncate(50);
    results
}

unsafe fn get_dict_string(
    dict: &CFDictionary<CFString, *const std::ffi::c_void>,
    key: &str,
) -> String {
    let cf_key = CFString::new(key);
    match dict.find(cf_key.as_CFTypeRef() as *const _) {
        Some(val) => {
            let s: CFString = TCFType::wrap_under_get_rule(val as *const _);
            s.to_string()
        }
        None => String::new(),
    }
}

unsafe fn get_dict_number(
    dict: &CFDictionary<CFString, *const std::ffi::c_void>,
    key: &str,
) -> Option<i64> {
    let cf_key = CFString::new(key);
    match dict.find(cf_key.as_CFTypeRef() as *const _) {
        Some(val) => {
            let n: CFNumber = TCFType::wrap_under_get_rule(val as *const _);
            n.to_i64()
        }
        None => None,
    }
}
