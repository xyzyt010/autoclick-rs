use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyInfo {
    pub keysym: u32,
    pub keycode: u16,
    pub display: &'static str,
}

macro_rules! keys {
    ($($name:literal => ($keysym:expr, $keycode:expr, $disp:literal)),* $(,)?) => {
        fn common_keys() -> &'static [(&'static str, KeyInfo)] {
            static KEYS: OnceLock<Vec<(&'static str, KeyInfo)>> = OnceLock::new();
            KEYS.get_or_init(|| vec![$(($name, KeyInfo { keysym: $keysym, keycode: $keycode, display: $disp })),*])
        }
    };
}

const KEY_ESC: u16 = 1;
const KEY_1: u16 = 2;
const KEY_0: u16 = 11;
const KEY_MINUS: u16 = 12;
const KEY_EQUAL: u16 = 13;
const KEY_BACKSPACE: u16 = 14;
const KEY_TAB: u16 = 15;
const KEY_LEFTBRACE: u16 = 26;
const KEY_RIGHTBRACE: u16 = 27;
const KEY_ENTER: u16 = 28;
const KEY_LEFTCTRL: u16 = 29;
const KEY_SEMICOLON: u16 = 39;
const KEY_APOSTROPHE: u16 = 40;
const KEY_GRAVE: u16 = 41;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_BACKSLASH: u16 = 43;
const KEY_COMMA: u16 = 51;
const KEY_DOT: u16 = 52;
const KEY_SLASH: u16 = 53;
const KEY_RIGHTSHIFT: u16 = 54;
const KEY_LEFTALT: u16 = 56;
const KEY_SPACE: u16 = 57;
const KEY_CAPSLOCK: u16 = 58;
const KEY_F1: u16 = 59;
const KEY_F10: u16 = 68;
const KEY_NUMLOCK: u16 = 69;
const KEY_F11: u16 = 87;
const KEY_F12: u16 = 88;
const KEY_RIGHTCTRL: u16 = 97;
const KEY_RIGHTALT: u16 = 100;
const KEY_HOME: u16 = 102;
const KEY_UP: u16 = 103;
const KEY_PAGEUP: u16 = 104;
const KEY_LEFT: u16 = 105;
const KEY_RIGHT: u16 = 106;
const KEY_END: u16 = 107;
const KEY_DOWN: u16 = 108;
const KEY_PAGEDOWN: u16 = 109;
const KEY_INSERT: u16 = 110;
const KEY_DELETE: u16 = 111;

keys! {
    "Enter"      => (0xFF0D, KEY_ENTER, "Enter"),
    "Space"      => (0x0020, KEY_SPACE, "Space"),
    "Tab"        => (0xFF09, KEY_TAB, "Tab"),
    "Escape"     => (0xFF1B, KEY_ESC, "Esc"),
    "Backspace"  => (0xFF08, KEY_BACKSPACE, "Bksp"),
    "Delete"     => (0xFFFF, KEY_DELETE, "Del"),
    "Insert"     => (0xFF63, KEY_INSERT, "Ins"),
    "Home"       => (0xFF50, KEY_HOME, "Home"),
    "End"        => (0xFF57, KEY_END, "End"),
    "Page Up"    => (0xFF55, KEY_PAGEUP, "PgUp"),
    "Page Down"  => (0xFF56, KEY_PAGEDOWN, "PgDn"),
    "Up Arrow"   => (0xFF52, KEY_UP, "Up"),
    "Down Arrow" => (0xFF54, KEY_DOWN, "Down"),
    "Left Arrow" => (0xFF51, KEY_LEFT, "Left"),
    "Right Arrow"=> (0xFF53, KEY_RIGHT, "Right"),
    "F1"  => (0xFFBE, KEY_F1, "F1"),
    "F2"  => (0xFFBF, KEY_F1 + 1, "F2"),
    "F3"  => (0xFFC0, KEY_F1 + 2, "F3"),
    "F4"  => (0xFFC1, KEY_F1 + 3, "F4"),
    "F5"  => (0xFFC2, KEY_F1 + 4, "F5"),
    "F6"  => (0xFFC3, KEY_F1 + 5, "F6"),
    "F7"  => (0xFFC4, KEY_F1 + 6, "F7"),
    "F8"  => (0xFFC5, KEY_F1 + 7, "F8"),
    "F9"  => (0xFFC6, KEY_F1 + 8, "F9"),
    "F10" => (0xFFC7, KEY_F10, "F10"),
    "F11" => (0xFFC8, KEY_F11, "F11"),
    "F12" => (0xFFC9, KEY_F12, "F12"),
    "Left Ctrl"  => (0xFFE3, KEY_LEFTCTRL, "LCtrl"),
    "Right Ctrl" => (0xFFE4, KEY_RIGHTCTRL, "RCtrl"),
    "Left Alt"   => (0xFFE9, KEY_LEFTALT, "LAlt"),
    "Right Alt"  => (0xFFEA, KEY_RIGHTALT, "RAlt"),
    "Left Shift" => (0xFFE1, KEY_LEFTSHIFT, "LShift"),
    "Right Shift"=> (0xFFE2, KEY_RIGHTSHIFT, "RShift"),
    "Caps Lock"  => (0xFFE5, KEY_CAPSLOCK, "Caps"),
    "Num Lock"   => (0xFF7F, KEY_NUMLOCK, "NumLk"),
}

fn alpha_keys() -> Vec<(&'static str, KeyInfo)> {
    let mut v = Vec::with_capacity(62);
    for ch in 'a'..='z' {
        let upper = ch.to_ascii_uppercase();
        let name: &'static str = Box::leak(upper.to_string().into_boxed_str());
        let kc = evdev_letter(ch);
        v.push((name, KeyInfo { keysym: ch as u32, keycode: kc, display: name }));
    }
    for (i, ch) in ('1'..='9').enumerate() {
        let name: &'static str = Box::leak(ch.to_string().into_boxed_str());
        v.push((name, KeyInfo { keysym: ch as u32, keycode: KEY_1 + i as u16, display: name }));
    }
    v.push(("0", KeyInfo { keysym: 0x0030, keycode: KEY_0, display: "0" }));
    v.push(("-", KeyInfo { keysym: 0x002D, keycode: KEY_MINUS, display: "-" }));
    v.push(("=", KeyInfo { keysym: 0x003D, keycode: KEY_EQUAL, display: "=" }));
    v.push(("[", KeyInfo { keysym: 0x005B, keycode: KEY_LEFTBRACE, display: "[" }));
    v.push(("]", KeyInfo { keysym: 0x005D, keycode: KEY_RIGHTBRACE, display: "]" }));
    v.push((";", KeyInfo { keysym: 0x003B, keycode: KEY_SEMICOLON, display: ";" }));
    v.push(("'", KeyInfo { keysym: 0x0027, keycode: KEY_APOSTROPHE, display: "'" }));
    v.push(("`", KeyInfo { keysym: 0x0060, keycode: KEY_GRAVE, display: "`" }));
    v.push(("\\", KeyInfo { keysym: 0x005C, keycode: KEY_BACKSLASH, display: "\\" }));
    v.push((",", KeyInfo { keysym: 0x002C, keycode: KEY_COMMA, display: "," }));
    v.push((".", KeyInfo { keysym: 0x002E, keycode: KEY_DOT, display: "." }));
    v.push(("/", KeyInfo { keysym: 0x002F, keycode: KEY_SLASH, display: "/" }));
    v
}

fn evdev_letter(ch: char) -> u16 {
    match ch {
        'a' => 30, 'b' => 48, 'c' => 46, 'd' => 32, 'e' => 18,
        'f' => 33, 'g' => 34, 'h' => 35, 'i' => 23, 'j' => 36,
        'k' => 37, 'l' => 38, 'm' => 50, 'n' => 49, 'o' => 24,
        'p' => 25, 'q' => 16, 'r' => 19, 's' => 31, 't' => 20,
        'u' => 22, 'v' => 47, 'w' => 17, 'x' => 45, 'y' => 21,
        'z' => 44, _ => 30,
    }
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
