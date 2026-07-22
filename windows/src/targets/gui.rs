use super::{Target, TargetMode};

use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};

pub fn list_candidate_apps(exclude_pid: u32) -> Vec<Target> {
    let mut state = EnumState {
        exclude_pid,
        results: Vec::with_capacity(64),
        seen: std::collections::HashSet::with_capacity(64),
    };
    let lparam = LPARAM(&mut state as *mut EnumState as isize);
    unsafe {
        let _ = EnumWindows(Some(enum_callback), lparam);
    }
    state.results.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
    });
    state.results
}

struct EnumState {
    exclude_pid: u32,
    results: Vec<Target>,
    seen: std::collections::HashSet<(u32, String)>,
}

unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let state = &mut *(lparam.0 as *mut EnumState);

    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }

    let title = match get_window_text(hwnd) {
        Some(t) if !t.trim().is_empty() => t,
        _ => return BOOL(1),
    };

    let mut pid: u32 = 0;
    let _tid = GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 || pid == state.exclude_pid {
        return BOOL(1);
    }

    if skip_class(hwnd) {
        return BOOL(1);
    }

    let name = get_process_name(pid).unwrap_or_else(|| "unknown".into());

    let key = (pid, title.clone());
    if !state.seen.insert(key) {
        return BOOL(1);
    }

    state.results.push(Target {
        pid,
        hwnd: hwnd.0 as isize as i64,
        name,
        title,
        mode: TargetMode::App,
        accessible: true,
    });

    BOOL(1)
}

unsafe fn get_window_text(hwnd: HWND) -> Option<String> {
    let mut buf = [0u16; 512];
    let len = GetWindowTextW(hwnd, &mut buf);
    if len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

const SKIP_CLASSES: &[&str] = &[
    "Shell_TrayWnd",
    "NotifyIconOverflowWindow",
    "tooltips_class32",
    "Shell_SecondaryTrayWnd",
];

unsafe fn skip_class(hwnd: HWND) -> bool {
    let mut buf = [0u16; 256];
    let len = GetClassNameW(hwnd, &mut buf);
    if len == 0 {
        return false;
    }
    let class = String::from_utf16_lossy(&buf[..len as usize]);
    SKIP_CLASSES.contains(&class.as_str())
}

fn get_process_name(pid: u32) -> Option<String> {
    fn as_string(s: impl AsRef<std::ffi::OsStr>) -> String {
        s.as_ref().to_string_lossy().into_owned()
    }

    let mut sys = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::new().with_processes(sysinfo::ProcessRefreshKind::new()),
    );
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    sys.process(sysinfo::Pid::from_u32(pid))
        .map(|p| as_string(p.name()))
}
