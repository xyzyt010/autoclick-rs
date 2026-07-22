//! Wayland/headless key injection via Linux uinput virtual device.
//! Creates a virtual keyboard at /dev/uinput and writes input_event structs.
//! Works on ALL Wayland compositors (GNOME, KDE, wlroots, Hyprland, etc.)
//! Requires: /dev/uinput accessible (group `input` or udev rule).

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::time::Duration;

use crate::keyboard::KeyInfo;

// ioctl constants for uinput (architecture-independent values).
const UI_DEV_CREATE: libc::c_ulong = 0x5501; // _IO('U', 1)
const UI_DEV_DESTROY: libc::c_ulong = 0x5502; // _IO('U', 2)
const UI_SET_EVBIT: libc::c_ulong = 0x40045564; // _IOW('U', 100, int)
const UI_SET_KEYBIT: libc::c_ulong = 0x40045565; // _IOW('U', 101, int)

// Event types
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;

// Sync report
const SYN_REPORT: u16 = 0x00;

// Max keycode we register (covers all standard keys).
const KEY_MAX: u16 = 255;

/// input_event struct layout (16 bytes on 64-bit, 24 with padding — we use libc).
#[repr(C)]
struct InputEvent {
    tv_sec: libc::time_t,
    tv_usec: libc::suseconds_t,
    ev_type: u16,
    code: u16,
    value: i32,
}

/// uinput_user_dev struct for device setup.
#[repr(C)]
struct UinputUserDev {
    name: [u8; 80],
    id_bustype: u16,
    id_vendor: u16,
    id_product: u16,
    id_version: u16,
    ff_effects_max: u32,
    absmax: [i32; 64],
    absmin: [i32; 64],
    absfuzz: [i32; 64],
    absflat: [i32; 64],
}

pub struct UinputBackend {
    file: File,
}

impl UinputBackend {
    /// Open /dev/uinput and create a virtual keyboard device.
    pub fn create() -> Result<Self, String> {
        // Check permissions hint.
        if unsafe { libc::getuid() } != 0 {
            // Non-root: try to open anyway (udev rules may allow it).
        }

        let file = OpenOptions::new()
            .write(true)
            .open("/dev/uinput")
            .map_err(|e| {
                format!(
                    "Cannot open /dev/uinput: {e}. \
                     Run: sudo modprobe uinput && sudo chmod 666 /dev/uinput \
                     (or add user to 'input' group)"
                )
            })?;

        let fd = file.as_raw_fd();

        // Enable EV_KEY event type.
        Self::ioctl(fd, UI_SET_EVBIT, EV_KEY as libc::c_ulong)?;

        // Enable all key codes we might send.
        for kc in 0..=KEY_MAX {
            Self::ioctl(fd, UI_SET_KEYBIT, kc as libc::c_ulong)?;
        }

        // Write device info.
        let mut dev: UinputUserDev = unsafe { std::mem::zeroed() };
        let name = b"autoclick-rs-vkbd";
        dev.name[..name.len()].copy_from_slice(name);
        dev.id_bustype = 0x03; // BUS_USB
        dev.id_vendor = 0x1234;
        dev.id_product = 0x5678;
        dev.id_version = 1;

        let dev_bytes = unsafe {
            std::slice::from_raw_parts(
                &dev as *const UinputUserDev as *const u8,
                std::mem::size_of::<UinputUserDev>(),
            )
        };
        let mut f = &file;
        f.write_all(dev_bytes)
            .map_err(|e| format!("write uinput_user_dev: {e}"))?;

        // Create the device.
        Self::ioctl(fd, UI_DEV_CREATE, 0)?;

        // Wait for device node to appear.
        std::thread::sleep(Duration::from_millis(200));

        Ok(Self { file })
    }

    /// Send a key press + release via the virtual keyboard.
    pub fn send_key(&self, key: KeyInfo) -> Result<(), String> {
        let mut f = &self.file;

        // Key down.
        self.write_event(&mut f, EV_KEY, key.keycode, 1)?;
        self.write_event(&mut f, EV_SYN, SYN_REPORT, 0)?;

        // Key up.
        self.write_event(&mut f, EV_KEY, key.keycode, 0)?;
        self.write_event(&mut f, EV_SYN, SYN_REPORT, 0)?;

        Ok(())
    }

    fn write_event(&self, f: &mut &File, ev_type: u16, code: u16, value: i32) -> Result<(), String> {
        let mut ev: InputEvent = unsafe { std::mem::zeroed() };
        ev.ev_type = ev_type;
        ev.code = code;
        ev.value = value;

        let bytes = unsafe {
            std::slice::from_raw_parts(
                &ev as *const InputEvent as *const u8,
                std::mem::size_of::<InputEvent>(),
            )
        };
        f.write_all(bytes)
            .map_err(|e| format!("write input_event: {e}"))?;
        Ok(())
    }

    fn ioctl(fd: libc::c_int, request: libc::c_ulong, arg: libc::c_ulong) -> Result<(), String> {
        let ret = unsafe { libc::ioctl(fd, request, arg) };
        if ret < 0 {
            Err(format!(
                "ioctl(0x{:X}) failed: {}",
                request,
                std::io::Error::last_os_error()
            ))
        } else {
            Ok(())
        }
    }
}

impl Drop for UinputBackend {
    fn drop(&mut self) {
        let _ = Self::ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY, 0);
    }
}
