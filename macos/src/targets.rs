//! macOS window enumeration via CGWindowListCopyWindowInfo (raw FFI).
//! Lists all visible application windows with their titles and owning PIDs.

use std::ffi::c_void;

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

// CoreFoundation FFI
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(arr: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(arr: *const c_void, idx: isize) -> *const c_void;
    fn CFDictionaryGetValue(dict: *const c_void, key: *const c_void) -> *const c_void;
    fn CFStringGetCStringPtr(s: *const c_void, encoding: u32) -> *const i8;
    fn CFStringGetLength(s: *const c_void) -> isize;
    fn CFStringGetCString(s: *const c_void, buf: *mut i8, size: isize, encoding: u32) -> u8;
    fn CFNumberGetValue(num: *const c_void, the_type: isize, value: *mut c_void) -> u8;
    fn CFRelease(cf: *const c_void);
    static kCFStringEncodingUTF8: u32;
}

// CoreGraphics FFI
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> *const c_void;
}

// kCGWindowListOptionOnScreenOnly = 1
const KCG_WINDOW_LIST_ON_SCREEN_ONLY: u32 = 1;
// kCGNullWindowID = 0
const KCG_NULL_WINDOW_ID: u32 = 0;
// kCFNumberSInt64Type = 4
const KCF_NUMBER_SINT64_TYPE: isize = 4;

/// CFString key constants (lazily created).
fn cfstr(s: &str) -> *const c_void {
    // Use __CFStringMakeConstantString via a simple approach:
    // We create CFString from UTF8 buffer.
    extern "C" {
        fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const i8,
            encoding: u32,
        ) -> *const c_void;
    }
    let c_str = std::ffi::CString::new(s).unwrap();
    unsafe {
        CFStringCreateWithCString(std::ptr::null(), c_str.as_ptr(), kCFStringEncodingUTF8)
    }
}

/// Extract a string value from a CFDictionary by key name.
unsafe fn dict_get_string(dict: *const c_void, key_name: &str) -> String {
    let key = cfstr(key_name);
    if key.is_null() {
        return String::new();
    }
    let val = CFDictionaryGetValue(dict, key);
    CFRelease(key);
    if val.is_null() {
        return String::new();
    }

    // Try fast path first.
    let ptr = CFStringGetCStringPtr(val, kCFStringEncodingUTF8);
    if !ptr.is_null() {
        return std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
    }

    // Slow path: allocate buffer.
    let len = CFStringGetLength(val);
    if len <= 0 {
        return String::new();
    }
    let buf_size = len * 4 + 1; // UTF-8 max 4 bytes per char + null.
    let mut buf: Vec<i8> = vec![0; buf_size as usize];
    let ok = CFStringGetCString(val, buf.as_mut_ptr(), buf_size, kCFStringEncodingUTF8);
    if ok != 0 {
        std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
    } else {
        String::new()
    }
}

/// Extract a numeric value from a CFDictionary by key name.
unsafe fn dict_get_number(dict: *const c_void, key_name: &str) -> Option<i64> {
    let key = cfstr(key_name);
    if key.is_null() {
        return None;
    }
    let val = CFDictionaryGetValue(dict, key);
    CFRelease(key);
    if val.is_null() {
        return None;
    }
    let mut result: i64 = 0;
    let ok = CFNumberGetValue(val, KCF_NUMBER_SINT64_TYPE, &mut result as *mut i64 as *mut c_void);
    if ok != 0 {
        Some(result)
    } else {
        None
    }
}

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
            KCG_WINDOW_LIST_ON_SCREEN_ONLY,
            KCG_NULL_WINDOW_ID,
        );
        if window_list.is_null() {
            return results;
        }

        let count = CFArrayGetCount(window_list);
        for i in 0..count {
            let dict = CFArrayGetValueAtIndex(window_list, i);
            if dict.is_null() {
                continue;
            }

            let name = dict_get_string(dict, "kCGWindowOwnerName");
            let title = dict_get_string(dict, "kCGWindowName");
            let pid = dict_get_number(dict, "kCGWindowOwnerPID").unwrap_or(0) as u32;
            let wid = dict_get_number(dict, "kCGWindowNumber").unwrap_or(0) as u32;
            let layer = dict_get_number(dict, "kCGWindowLayer").unwrap_or(-1);

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

        CFRelease(window_list);
    }

    // Deduplicate by (pid, title).
    results.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    results.dedup_by(|a, b| a.pid == b.pid && a.title == b.title);
    results.truncate(50);
    results
}
