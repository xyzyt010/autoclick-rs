use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyInfo {
    pub vk: u16,
    /// Unicode code point for printable characters, 0 for non-printable keys.
    pub unicode: u16,
    pub display: &'static str,
}

macro_rules! keys {
    ($($name:literal => ($vk:expr, $uni:expr, $disp:literal)),* $(,)?) => {
        pub fn common_keys() -> &'static [(&'static str, KeyInfo)] {
            static KEYS: OnceLock<Vec<(&'static str, KeyInfo)>> = OnceLock::new();
            KEYS.get_or_init(|| vec![$(($name, KeyInfo { vk: $vk, unicode: $uni, display: $disp })),*])
        }
    };
}

keys! {
    "Enter (Return)" => (0x0D, 0x000D, "Enter"),
    "Space"        => (0x20, 0x0020, "Space"),
    "Tab"          => (0x09, 0x0009, "Tab"),
    "Escape (Esc)" => (0x1B, 0x001B, "Escape"),
    "Backspace"    => (0x08, 0x0008, "Backspace"),
    "Delete"       => (0x2E, 0x007F, "Delete"),
    "Up Arrow"     => (0x26, 0x0000, "Up"),
    "Down Arrow"   => (0x28, 0x0000, "Down"),
    "Left Arrow"   => (0x25, 0x0000, "Left"),
    "Right Arrow"  => (0x27, 0x0000, "Right"),
    "F1"  => (0x70, 0x0000, "F1"),
    "F2"  => (0x71, 0x0000, "F2"),
    "F3"  => (0x72, 0x0000, "F3"),
    "F4"  => (0x73, 0x0000, "F4"),
    "F5"  => (0x74, 0x0000, "F5"),
    "F6"  => (0x75, 0x0000, "F6"),
    "F7"  => (0x76, 0x0000, "F7"),
    "F8"  => (0x77, 0x0000, "F8"),
    "F9"  => (0x78, 0x0000, "F9"),
    "F10" => (0x79, 0x0000, "F10"),
    "F11" => (0x7A, 0x0000, "F11"),
    "F12" => (0x7B, 0x0000, "F12"),
    "Home"      => (0x24, 0x0000, "Home"),
    "End"       => (0x23, 0x0000, "End"),
    "Page Up"   => (0x21, 0x0000, "PageUp"),
    "Page Down" => (0x22, 0x0000, "PageDown"),
    "Insert"    => (0x2D, 0x0000, "Insert"),
}

// Alphanumerics generated at runtime to keep the static table small.
fn alpha_keys() -> impl Iterator<Item = (&'static str, KeyInfo)> {
    let mut v = Vec::with_capacity(36);
    for ch in 'A'..='Z' {
        let s: &'static str = Box::leak(ch.to_string().into_boxed_str());
        v.push((s, KeyInfo {
            vk: 0x41 + (ch as u16 - b'A' as u16),
            unicode: ch.to_ascii_lowercase() as u16,
            display: s,
        }));
    }
    for ch in '0'..='9' {
        let s: &'static str = Box::leak(ch.to_string().into_boxed_str());
        v.push((s, KeyInfo {
            vk: 0x30 + (ch as u16 - b'0' as u16),
            unicode: ch as u16,
            display: s,
        }));
    }
    v.into_iter()
}

pub fn all_keys() -> &'static [(&'static str, KeyInfo)] {
    static ALL: OnceLock<Vec<(&'static str, KeyInfo)>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v = common_keys().to_vec();
        v.extend(alpha_keys());
        v
    })
}

#[allow(dead_code)]
pub fn find_key(name: &str) -> Option<KeyInfo> {
    all_keys().iter().find(|(k, _)| *k == name).map(|(_, i)| *i)
}

pub fn key_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| all_keys().iter().map(|(k, _)| *k).collect::<Vec<_>>())
}
