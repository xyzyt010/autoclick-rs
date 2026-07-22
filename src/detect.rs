use std::env;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayServer {
    X11,
    Wayland,
}

impl DisplayServer {
    pub fn name(&self) -> &'static str {
        match self {
            Self::X11 => "X11",
            Self::Wayland => "Wayland",
        }
    }
}

static DS: OnceLock<DisplayServer> = OnceLock::new();

/// Detect the display server once at startup.
pub fn detect() -> DisplayServer {
    *DS.get_or_init(|| {
        if env::var("WAYLAND_DISPLAY").is_ok() {
            if let Ok(t) = env::var("XDG_SESSION_TYPE") {
                if t == "x11" {
                    return DisplayServer::X11;
                }
            }
            return DisplayServer::Wayland;
        }
        if let Ok(t) = env::var("XDG_SESSION_TYPE") {
            if t == "wayland" {
                return DisplayServer::Wayland;
            }
        }
        if env::var("DISPLAY").is_ok() {
            return DisplayServer::X11;
        }
        DisplayServer::Wayland
    })
}
