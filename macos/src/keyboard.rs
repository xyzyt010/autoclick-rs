//! macOS virtual keycodes (Carbon/CGEvent keycode layout).
//! These are the hardware-independent virtual keycodes used by CGEventCreateKeyboardEvent.

use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyInfo {
    /// macOS virtual keycode (CGKeyCode).
    pub keycode: u16,
    /// Display name.
    pub display: &'static str,
}

// macOS virtual keycodes (ANSI US layout).
const VK_RETURN: u16 = 0x24;
const VK_TAB: u16 = 0x30;
const VK_SPACE: u16 = 0x31;
const VK_DELETE: u16 = 0x33; // Backspace
const VK_ESCAPE: u16 = 0x35;
const VK_FORWARD_DELETE: u16 = 0x75;
const VK_HOME: u16 = 0x73;
const VK_END: u16 = 0x77;
const VK_PAGE_UP: u16 = 0x74;
const VK_PAGE_DOWN: u16 = 0x79;
const VK_LEFT: u16 = 0x7B;
const VK_RIGHT: u16 = 0x7C;
const VK_DOWN: u16 = 0x7D;
const VK_UP: u16 = 0x7E;
const VK_F1: u16 = 0x7A;
const VK_F2: u16 = 0x78;
const VK_F3: u16 = 0x63;
const VK_F4: u16 = 0x76;
const VK_F5: u16 = 0x60;
const VK_F6: u16 = 0x61;
const VK_F7: u16 = 0x62;
const VK_F8: u16 = 0x64;
const VK_F9: u16 = 0x65;
const VK_F10: u16 = 0x6D;
const VK_F11: u16 = 0x67;
const VK_F12: u16 = 0x6F;
const VK_SHIFT: u16 = 0x38;
const VK_CONTROL: u16 = 0x3B;
const VK_OPTION: u16 = 0x3A; // Alt
const VK_COMMAND: u16 = 0x37;
const VK_CAPS_LOCK: u16 = 0x39;
const VK_RIGHT_SHIFT: u16 = 0x3C;
const VK_RIGHT_CONTROL: u16 = 0x3E;
const VK_RIGHT_OPTION: u16 = 0x3D;

macro_rules! keys {
    ($($name:literal => ($kc:expr, $disp:literal)),* $(,)?) => {
        fn common_keys() -> &'static [(&'static str, KeyInfo)] {
            static KEYS: OnceLock<Vec<(&'static str, KeyInfo)>> = OnceLock::new();
            KEYS.get_or_init(|| vec![$(($name, KeyInfo { keycode: $kc, display: $disp })),*])
        }
    };
}

keys! {
    "Enter"       => (VK_RETURN, "Enter"),
    "Space"       => (VK_SPACE, "Space"),
    "Tab"         => (VK_TAB, "Tab"),
    "Escape"      => (VK_ESCAPE, "Esc"),
    "Backspace"   => (VK_DELETE, "Bksp"),
    "Delete"      => (VK_FORWARD_DELETE, "Del"),
    "Home"        => (VK_HOME, "Home"),
    "End"         => (VK_END, "End"),
    "Page Up"     => (VK_PAGE_UP, "PgUp"),
    "Page Down"   => (VK_PAGE_DOWN, "PgDn"),
    "Up Arrow"    => (VK_UP, "Up"),
    "Down Arrow"  => (VK_DOWN, "Down"),
    "Left Arrow"  => (VK_LEFT, "Left"),
    "Right Arrow" => (VK_RIGHT, "Right"),
    "F1"  => (VK_F1, "F1"),
    "F2"  => (VK_F2, "F2"),
    "F3"  => (VK_F3, "F3"),
    "F4"  => (VK_F4, "F4"),
    "F5"  => (VK_F5, "F5"),
    "F6"  => (VK_F6, "F6"),
    "F7"  => (VK_F7, "F7"),
    "F8"  => (VK_F8, "F8"),
    "F9"  => (VK_F9, "F9"),
    "F10" => (VK_F10, "F10"),
    "F11" => (VK_F11, "F11"),
    "F12" => (VK_F12, "F12"),
    "Left Ctrl"   => (VK_CONTROL, "LCtrl"),
    "Right Ctrl"  => (VK_RIGHT_CONTROL, "RCtrl"),
    "Left Alt"    => (VK_OPTION, "LOpt"),
    "Right Alt"   => (VK_RIGHT_OPTION, "ROpt"),
    "Left Shift"  => (VK_SHIFT, "LShift"),
    "Right Shift" => (VK_RIGHT_SHIFT, "RShift"),
    "Left Cmd"    => (VK_COMMAND, "LCmd"),
    "Caps Lock"   => (VK_CAPS_LOCK, "Caps"),
}

fn alpha_keys() -> Vec<(&'static str, KeyInfo)> {
    let mut v = Vec::with_capacity(62);
    // macOS ANSI keycodes for letters.
    let letter_codes: [(char, u16); 26] = [
        ('a', 0x00), ('b', 0x0B), ('c', 0x08), ('d', 0x02), ('e', 0x0E),
        ('f', 0x03), ('g', 0x05), ('h', 0x04), ('i', 0x22), ('j', 0x26),
        ('k', 0x28), ('l', 0x25), ('m', 0x2E), ('n', 0x2D), ('o', 0x1F),
        ('p', 0x23), ('q', 0x0C), ('r', 0x0F), ('s', 0x01), ('t', 0x11),
        ('u', 0x20), ('v', 0x09), ('w', 0x0D), ('x', 0x07), ('y', 0x10),
        ('z', 0x06),
    ];
    for (ch, kc) in letter_codes {
        let upper = ch.to_ascii_uppercase();
        let name: &'static str = Box::leak(upper.to_string().into_boxed_str());
        v.push((name, KeyInfo { keycode: kc, display: name }));
    }
    // Number row.
    let num_codes: [(char, u16); 10] = [
        ('1', 0x12), ('2', 0x13), ('3', 0x14), ('4', 0x15), ('5', 0x17),
        ('6', 0x16), ('7', 0x1A), ('8', 0x1C), ('9', 0x19), ('0', 0x1D),
    ];
    for (ch, kc) in num_codes {
        let name: &'static str = Box::leak(ch.to_string().into_boxed_str());
        v.push((name, KeyInfo { keycode: kc, display: name }));
    }
    // Punctuation.
    let punct: [(&str, u16, &str); 9] = [
        ("-", 0x1B, "-"), ("=", 0x18, "="), ("[", 0x21, "["),
        ("]", 0x1E, "]"), ("\\", 0x2A, "\\"), (";", 0x29, ";"),
        ("'", 0x27, "'"), (",", 0x2B, ","), (".", 0x2F, "."),
    ];
    for (name, kc, disp) in punct {
        v.push((name, KeyInfo { keycode: kc, display: disp }));
    }
    v.push(("/", KeyInfo { keycode: 0x2C, display: "/" }));
    v.push(("`", KeyInfo { keycode: 0x32, display: "`" }));
    v
}

pub fn all_keys() -> &'static [(&'static str, KeyInfo)] {
    static ALL: OnceLock<Vec<(&'static str, KeyInfo)>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut v = common_keys().to_vec();
        v.extend(alpha_keys());
        v
    })
}

pub fn key_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| all_keys().iter().map(|(k, _)| *k).collect::<Vec<_>>())
}
