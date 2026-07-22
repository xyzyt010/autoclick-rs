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
