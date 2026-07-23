//! X11 backend: window enumeration + key injection.
//! Enumeration uses basic X11 (no extensions required).
//! Injection uses XTest (preferred) or XSendEvent (fallback).

use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

use crate::keyboard::KeyInfo;

// ─── Lightweight X11 connection (for enumeration only) ───────────────────────

pub struct X11Connection {
    conn: RustConnection,
    screen_num: usize,
}

impl X11Connection {
    /// Connect to X server. Only needs DISPLAY env var — no extensions required.
    pub fn connect() -> Result<Self, String> {
        let (conn, screen_num) =
            RustConnection::connect(None).map_err(|e| format!("X11 connect failed: {e}. Is DISPLAY set?"))?;
        Ok(Self { conn, screen_num })
    }

    fn root(&self) -> u32 {
        self.conn.setup().roots[self.screen_num].root
    }

    /// Enumerate all managed client windows with (window_id, pid, title).
    /// Tries multiple EWMH properties, then falls back to recursive tree walk.
    pub fn list_windows(&self) -> Result<Vec<(u32, u32, String)>, String> {
        let root = self.root();

        // Strategy 1: _NET_CLIENT_LIST (standard EWMH — all modern WMs).
        let mut windows = self.get_property_windows(root, "_NET_CLIENT_LIST");

        // Strategy 2: _NET_CLIENT_LIST_STACKING (some WMs only set this).
        if windows.is_empty() {
            windows = self.get_property_windows(root, "_NET_CLIENT_LIST_STACKING");
        }

        // Strategy 3: Recursive tree walk (works on any WM, even non-EWMH).
        if windows.is_empty() {
            let mut all = Vec::new();
            self.collect_windows_recursive(root, &mut all, 0);
            windows = all;
        }

        let mut results = Vec::new();
        for wid in windows {
            let title = self.get_window_title(wid).unwrap_or_default();
            if title.is_empty() {
                continue;
            }
            let pid = self.get_window_pid(wid).unwrap_or(0);
            // Skip windows that are definitely not user-facing.
            if self.is_override_redirect(wid) {
                continue;
            }
            results.push((wid, pid, title));
        }
        Ok(results)
    }

    /// Read a window-list property from the root window.
    fn get_property_windows(&self, root: u32, prop_name: &str) -> Vec<u32> {
        let atom = match self.intern_atom(prop_name) {
            Some(a) => a,
            None => return Vec::new(),
        };
        let reply = match self
            .conn
            .get_property(false, root, atom, u32::from(AtomEnum::WINDOW), 0, 2048)
            .ok()
            .and_then(|c| c.reply().ok())
        {
            Some(r) => r,
            None => return Vec::new(),
        };
        if reply.format != 32 {
            return Vec::new();
        }
        reply
            .value
            .chunks_exact(4)
            .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
            .filter(|&w| w != 0)
            .collect()
    }

    /// Recursively collect windows that have a title (actual client windows).
    fn collect_windows_recursive(&self, wid: u32, out: &mut Vec<u32>, depth: u8) {
        if depth > 10 {
            return; // Prevent infinite recursion.
        }
        let tree = match self.conn.query_tree(wid).ok().and_then(|c| c.reply().ok()) {
            Some(t) => t,
            None => return,
        };
        for &child in tree.children.iter() {
            if self.get_window_title(child).is_some() {
                out.push(child);
            }
            self.collect_windows_recursive(child, out, depth + 1);
        }
    }

    fn get_window_title(&self, wid: u32) -> Option<String> {
        // Try _NET_WM_NAME (UTF-8).
        if let (Some(atom), Some(utf8)) = (self.intern_atom("_NET_WM_NAME"), self.intern_atom("UTF8_STRING")) {
            if let Some(reply) = self
                .conn
                .get_property(false, wid, atom, utf8, 0, 512)
                .ok()
                .and_then(|c| c.reply().ok())
            {
                if !reply.value.is_empty() {
                    if let Ok(s) = String::from_utf8(reply.value.clone()) {
                        if !s.is_empty() {
                            return Some(s);
                        }
                    }
                }
            }
        }
        // Fallback: WM_NAME (latin1).
        if let Some(reply) = self
            .conn
            .get_property(false, wid, u32::from(AtomEnum::WM_NAME), u32::from(AtomEnum::STRING), 0, 512)
            .ok()
            .and_then(|c| c.reply().ok())
        {
            if !reply.value.is_empty() {
                let s = String::from_utf8_lossy(&reply.value).into_owned();
                if !s.is_empty() {
                    return Some(s);
                }
            }
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

    /// Check if window has override_redirect set (popup menus, tooltips — skip these).
    fn is_override_redirect(&self, wid: u32) -> bool {
        self.conn
            .get_window_attributes(wid)
            .ok()
            .and_then(|c| c.reply().ok())
            .map(|a| a.override_redirect)
            .unwrap_or(false)
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

// ─── X11 Key Injector (XTest + XSendEvent fallback) ─────────────────────────

pub struct X11Injector {
    conn: RustConnection,
    screen_num: usize,
    keymap: Vec<(u32, u8)>,
    has_xtest: bool,
}

impl X11Injector {
    /// Connect and prepare for key injection.
    /// Tries XTest first; if unavailable, falls back to XSendEvent.
    pub fn connect() -> Result<Self, String> {
        let (conn, screen_num) =
            RustConnection::connect(None).map_err(|e| format!("X11 connect: {e}"))?;

        // Check XTest availability.
        let has_xtest = Self::check_xtest(&conn);

        // Build keysym→keycode cache.
        let keymap = Self::build_keymap(&conn)?;

        Ok(Self {
            conn,
            screen_num,
            keymap,
            has_xtest,
        })
    }

    fn check_xtest(conn: &RustConnection) -> bool {
        use x11rb::protocol::xtest;
        xtest::get_version(conn, 2, 2)
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some()
    }

    pub fn method_name(&self) -> &'static str {
        if self.has_xtest { "XTest" } else { "XSendEvent" }
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

    fn keysym_to_keycode(&self, keysym: u32) -> Option<u8> {
        for &(ks, kc) in &self.keymap {
            if ks == keysym {
                return Some(kc);
            }
        }
        // For uppercase, try lowercase (X maps shift+lower).
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

    /// Send a key press+release to the target window.
    pub fn send_key(&self, key: KeyInfo, window_id: u32) -> Result<(), String> {
        let keycode = self
            .keysym_to_keycode(key.keysym)
            .ok_or_else(|| format!("No keycode for keysym 0x{:04X} ('{}')", key.keysym, key.display))?;

        // Focus the target window.
        if window_id != 0 {
            self.focus_window(window_id)?;
        }

        if self.has_xtest {
            self.send_key_xtest(keycode)
        } else {
            self.send_key_xsendevent(keycode, window_id)
        }
    }

    /// XTest injection (preferred — works with all apps).
    fn send_key_xtest(&self, keycode: u8) -> Result<(), String> {
        use x11rb::protocol::xtest;

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

    /// XSendEvent fallback (works without XTest, but some apps may ignore synthetic events).
    fn send_key_xsendevent(&self, keycode: u8, window_id: u32) -> Result<(), String> {
        let target = if window_id != 0 { window_id } else { self.get_focus_window()? };
        let root = self.root();

        // Key press event.
        let press = KeyPressEvent {
            response_type: KEY_PRESS_EVENT,
            detail: keycode,
            sequence: 0,
            time: x11rb::CURRENT_TIME,
            root,
            event: target,
            child: 0,
            root_x: 0,
            root_y: 0,
            event_x: 0,
            event_y: 0,
            state: KeyButMask::from(0u16),
            same_screen: true,
        };
        self.conn
            .send_event(false, target, EventMask::KEY_PRESS, press)
            .map_err(|e| format!("XSendEvent press: {e}"))?;

        // Key release event.
        let release = KeyReleaseEvent {
            response_type: KEY_RELEASE_EVENT,
            detail: keycode,
            sequence: 0,
            time: x11rb::CURRENT_TIME,
            root,
            event: target,
            child: 0,
            root_x: 0,
            root_y: 0,
            event_x: 0,
            event_y: 0,
            state: KeyButMask::from(0u16),
            same_screen: true,
        };
        self.conn
            .send_event(false, target, EventMask::KEY_RELEASE, release)
            .map_err(|e| format!("XSendEvent release: {e}"))?;

        self.conn.flush().map_err(|e| format!("flush: {e}"))?;
        Ok(())
    }

    fn get_focus_window(&self) -> Result<u32, String> {
        let reply = self
            .conn
            .get_input_focus()
            .map_err(|e| format!("get_input_focus: {e}"))?
            .reply()
            .map_err(|e| format!("get_input_focus reply: {e}"))?;
        Ok(reply.focus)
    }

    fn focus_window(&self, wid: u32) -> Result<(), String> {
        self.conn
            .set_input_focus(InputFocus::PARENT, wid, x11rb::CURRENT_TIME)
            .map_err(|e| format!("set_input_focus: {e}"))?;
        self.conn.flush().map_err(|e| format!("flush: {e}"))?;
        std::thread::sleep(std::time::Duration::from_millis(30));
        Ok(())
    }

    fn root(&self) -> u32 {
        self.conn.setup().roots[self.screen_num].root
    }
}
