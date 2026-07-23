pub mod gui;
pub mod terminal;

#[derive(Clone)]
pub struct Target {
    pub pid: u32,
    /// HWND as u32/u64; stored as i64 for Slint compatibility.
    pub hwnd: i64,
    pub name: String,
    pub title: String,
    #[allow(dead_code)]
    pub mode: TargetMode,
    #[allow(dead_code)]
    pub accessible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TargetMode {
    Terminal,
    App,
}

impl Target {
    pub fn label(&self) -> String {
        format!("{}  (PID {})  \"{}\"", self.name, self.pid, self.title)
    }
}

/// Enumerate ALL targets (terminals + apps) for the unified dropdown.
pub fn enumerate_all(exclude_pid: u32) -> Vec<Target> {
    let mut all = gui::list_candidate_apps(exclude_pid);
    let shells = terminal::list_candidate_shells();
    all.extend(shells);
    all.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    all
}
