//! X11 key injection via XTest extension + window enumeration.
//! Pure Rust through x11rb — no libX11 C dependency, musl-safe.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::protocol::xtest;
use x11rb::rust_connection::RustConnection;

use crate::keyboard::KeyInfo;

pub struct X11Backend {
    conn: RustConnection,
    screen_num: usize,
    /// Cached keysym → keycode mapping.
    keymap: Vec<(u32, u8)>,
}

impl X11Backend {
    /// Connect to the X server. Fails if DISPLAY is unset or unreachable.
    pub fn connect() -> Result<Self, String> {
        let (conn, screen_num) =
            RustConnection::connect(None).map_err(|e| format!("X11 connect: {e}"))?;

        // Verify XTest extension is available via get_version.
        xtest::get_version(&conn, 2, 2)
            .map_err(|e| format!("XTest query: {e}"))?
            .reply()
            .map_err(|e| format!("XTest not available: {e}"))?;

        // Build keysym→keycode cache from the server's keyboard mapping.
        let keymap = Self::build_keymap(&conn)?;

        Ok(Self {
            conn,
            screen_num,
            keymap,
        })
    }

    fn build_keymap(conn: &RustConnection) -> Result<Vec<(u32, u8)>, String> {
        let setup = conn.setup();
        let min_kc = setup.min_keycode;
        let max_kc = setup.max_keycode;
        let count = (max_kc - min_kc + 1) as u32;

        let reply = conn
            .get_keyboard_mapping(min_kc, count as u8)
            .map_err(|e| format!("get_keyboard_mapping: {e}"))?
            .reply()
            .map_err(|e| format!("get_keyboard_mapping reply: {e}"))?;

        let keysyms_per = reply.keysyms_per_keycode as usize;
        let mut map = Vec::with_capacity(count as usize);

        for i in 0..count as usize {
            let keycode = (min_kc as usize + i) as u8;
            let base = i * keysyms_per;
            if base < reply.keysyms.len() {
                let ks = reply.keysyms[base];
                if ks != 0 {
                    map.push((ks, keycode));
                }
            }
        }
        Ok(map)
    }

    /// Resolve a keysym to an X keycode.
    fn keysym_to_keycode(&self, keysym: u32) -> Option<u8> {
        // Direct lookup in cached map.
        for &(ks, kc) in &self.keymap {
            if ks == keysym {
                return Some(kc);
            }
        }
        // For uppercase letters, try lowercase (X maps shift+lower).
        if keysym >= 0x41 && keysym <= 0x5A {
            let lower = keysym + 0x20;
            for &(ks, kc) in &self.keymap {
                if ks == lower {
                    return Some(kc);
                }
            }
        }
        None
    }

    /// Send a key press+release to the focused window (or specified window).
    pub fn send_key(&self, key: KeyInfo, window_id: u32) -> Result<(), String> {
        let keycode = self
            .keysym_to_keycode(key.keysym)
            .ok_or_else(|| format!("No keycode for keysym 0x{:04X}", key.keysym))?;

        // If a specific window is targeted, focus it first.
        if window_id != 0 {
            self.focus_window(window_id)?;
        }

        // XTest fake key press + release.
        xtest::fake_input(
            &self.conn,
            KEY_PRESS_EVENT,
            keycode,
            x11rb::CURRENT_TIME,
            self.root(),
            0,
            0,
            0,
        )
        .map_err(|e| format!("XTest press: {e}"))?;

        xtest::fake_input(
            &self.conn,
            KEY_RELEASE_EVENT,
            keycode,
            x11rb::CURRENT_TIME,
            self.root(),
            0,
            0,
            0,
        )
        .map_err(|e| format!("XTest release: {e}"))?;

        self.conn.flush().map_err(|e| format!("flush: {e}"))?;
        Ok(())
    }

    /// Focus a window by ID (sets input focus).
    fn focus_window(&self, wid: u32) -> Result<(), String> {
        self.conn
            .set_input_focus(
                InputFocus::PARENT,
                wid,
                x11rb::CURRENT_TIME,
            )
            .map_err(|e| format!("set_input_focus: {e}"))?;
        self.conn.flush().map_err(|e| format!("flush: {e}"))?;
        // Small delay for WM to process focus change.
        std::thread::sleep(std::time::Duration::from_millis(20));
        Ok(())
    }

    fn root(&self) -> u32 {
        self.conn.setup().roots[self.screen_num].root
    }

    /// Enumerate visible top-level windows with their titles and PIDs.
    /// Uses _NET_CLIENT_LIST (EWMH) which correctly lists all managed client
    /// windows regardless of WM framing. Falls back to recursive tree walk.
    pub fn list_windows(&self) -> Result<Vec<(u32, u32, String)>, String> {
        let root = self.root();

        // Preferred: _NET_CLIENT_LIST gives all managed windows directly.
        let clients = self.get_client_list(root);
        let windows: Vec<u32> = if !clients.is_empty() {
            clients
        } else {
            // Fallback: recursively walk the window tree.
            let mut all = Vec::new();
            self.collect_windows_recursive(root, &mut all);
            all
        };

        let mut results = Vec::new();
        for wid in windows {
            // Get window title (_NET_WM_NAME or WM_NAME).
            let title = self.get_window_title(wid).unwrap_or_default();
            if title.is_empty() {
                continue;
            }
            // Get PID (_NET_WM_PID).
            let pid = self.get_window_pid(wid).unwrap_or(0);
            // Check visibility.
            if self.is_visible(wid) {
                results.push((wid, pid, title));
            }
        }
        Ok(results)
    }

    /// Read _NET_CLIENT_LIST from the root window (EWMH standard).
    fn get_client_list(&self, root: u32) -> Vec<u32> {
        let atom = match self.intern_atom("_NET_CLIENT_LIST") {
            Some(a) => a,
            None => return Vec::new(),
        };
        let reply = match self
            .conn
            .get_property(false, root, atom, u32::from(AtomEnum::WINDOW), 0, 1024)
            .ok()
            .and_then(|c| c.reply().ok())
        {
            Some(r) => r,
            None => return Vec::new(),
        };
        // Value is a list of 32-bit window IDs.
        reply
            .value
            .chunks_exact(4)
            .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .collect()
    }

    /// Recursively collect windows that have a title (actual client windows).
    fn collect_windows_recursive(&self, wid: u32, out: &mut Vec<u32>) {
        let tree = match self.conn.query_tree(wid).ok().and_then(|c| c.reply().ok()) {
            Some(t) => t,
            None => return,
        };
        for &child in tree.children.iter() {
            // If this child has a title, it's likely a client window.
            if self.get_window_title(child).is_some() {
                out.push(child);
            }
            // Always recurse — WM frames wrap client windows.
            self.collect_windows_recursive(child, out);
        }
    }

    fn get_window_title(&self, wid: u32) -> Option<String> {
        // Try _NET_WM_NAME first (UTF-8).
        let atom = self.intern_atom("_NET_WM_NAME")?;
        let utf8 = self.intern_atom("UTF8_STRING")?;
        let reply = self
            .conn
            .get_property(false, wid, atom, utf8, 0, 256)
            .ok()?
            .reply()
            .ok()?;
        if !reply.value.is_empty() {
            if let Ok(s) = String::from_utf8(reply.value.clone()) {
                return Some(s);
            }
        }
        // Fallback to WM_NAME.
        let reply = self
            .conn
            .get_property(false, wid, u32::from(AtomEnum::WM_NAME), u32::from(AtomEnum::STRING), 0, 256)
            .ok()?
            .reply()
            .ok()?;
        if !reply.value.is_empty() {
            return Some(String::from_utf8_lossy(&reply.value).into_owned());
        }
        None
    }

    fn get_window_pid(&self, wid: u32) -> Option<u32> {
        let atom = self.intern_atom("_NET_WM_PID")?;
        let reply = self
            .conn
            .get_property(false, wid, atom, u32::from(AtomEnum::CARDINAL), 0, 1)
            .ok()?
            .reply()
            .ok()?;
        if reply.value.len() >= 4 {
            Some(u32::from_ne_bytes([
                reply.value[0],
                reply.value[1],
                reply.value[2],
                reply.value[3],
            ]))
        } else {
            None
        }
    }

    fn is_visible(&self, wid: u32) -> bool {
        if let Some(reply) = self.conn.get_window_attributes(wid).ok().and_then(|c| c.reply().ok()) {
            reply.map_state == MapState::VIEWABLE
        } else {
            false
        }
    }

    fn intern_atom(&self, name: &str) -> Option<u32> {
        self.conn
            .intern_atom(false, name.as_bytes())
            .ok()?
            .reply()
            .ok()
            .map(|r| r.atom)
    }
}
