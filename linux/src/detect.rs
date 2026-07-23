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
/// Priority: XDG_SESSION_TYPE > DISPLAY env > WAYLAND_DISPLAY env.
/// On most VMs and desktops, DISPLAY is set for X11 sessions.
pub fn detect() -> DisplayServer {
    *DS.get_or_init(|| {
        // 1. Explicit session type takes highest priority.
        if let Ok(t) = env::var("XDG_SESSION_TYPE") {
            match t.as_str() {
                "x11" => return DisplayServer::X11,
                "wayland" => return DisplayServer::Wayland,
                _ => {}
            }
        }

        // 2. If DISPLAY is set, it's almost certainly X11 (or XWayland).
        //    Even under GNOME Wayland, DISPLAY is set for XWayland compat.
        //    We prefer X11 here because XTest/XSendEvent work through XWayland.
        if env::var("DISPLAY").is_ok() {
            return DisplayServer::X11;
        }

        // 3. Only Wayland (no XWayland).
        if env::var("WAYLAND_DISPLAY").is_ok() {
            return DisplayServer::Wayland;
        }

        // 4. Default: try X11 (most common on VMs and servers).
        DisplayServer::X11
    })
}
