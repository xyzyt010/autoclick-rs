pub mod gui;
pub mod terminal;

#[derive(Clone)]
pub struct Target {
    pub pid: u32,
    /// HWND as u32/u64; stored as i64 for Slint compatibility.
    pub hwnd: i64,
    pub name: String,
    pub title: String,
    /// true = detected as a terminal/console host (cmd, powershell, pwsh, wsl, …)
    #[allow(dead_code)]
    pub is_terminal: bool,
    #[allow(dead_code)]
    pub accessible: bool,
}

impl Target {
    pub fn label(&self) -> String {
        format!("{}  (PID {})  \u{201C}{}\u{201D}", self.name, self.pid, self.title)
    }
}

/// Enumerate ALL targets for the unified dropdown.
///
/// Returns a single ordered list where **terminals come first, then GUI
/// apps**, each group sorted A–Z (case-insensitive) by `name`. The number
/// of leading terminal entries is returned as `terminal_count` so the UI
/// can render a section header at that boundary.
pub fn enumerate_all(exclude_pid: u32) -> (Vec<Target>, usize) {
    let mut terminals = terminal::list_candidate_shells();
    let mut apps = gui::list_candidate_apps(exclude_pid);

    // a–z within each group (case-insensitive, then by pid as tie-breaker)
    terminals.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.pid.cmp(&b.pid))
    });
    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.pid.cmp(&b.pid))
    });

    let terminal_count = terminals.len();
    let mut all = terminals;
    all.extend(apps);
    (all, terminal_count)
}
