//! macOS key injection via CGEvent (Core Graphics Event API).
//! Uses raw FFI to avoid version-specific API issues with the core-graphics crate.
//! Requires Accessibility permission (System Settings > Privacy > Accessibility).

use std::ffi::c_void;

use crate::keyboard::KeyInfo;

// CoreGraphics FFI
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceCreate(state_id: u32) -> *mut c_void;
    fn CGEventCreateKeyboardEvent(source: *mut c_void, virtual_key: u16, key_down: bool) -> *mut c_void;
    fn CGEventPost(tap: u32, event: *mut c_void);
    fn CFRelease(cf: *const c_void);
}

// kCGEventSourceStateHIDSystemState = 1
const KCG_EVENT_SOURCE_STATE_HID: u32 = 1;
// kCGHIDEventTap = 0
const KCG_HID_EVENT_TAP: u32 = 0;

pub struct MacOsBackend {
    source: *mut c_void,
}

// CGEventSource is thread-safe for posting events.
unsafe impl Send for MacOsBackend {}
unsafe impl Sync for MacOsBackend {}

impl MacOsBackend {
    /// Create the event source. Fails if Accessibility permission is not granted.
    pub fn create() -> Result<Self, String> {
        let source = unsafe { CGEventSourceCreate(KCG_EVENT_SOURCE_STATE_HID) };
        if source.is_null() {
            return Err(
                "CGEventSource creation failed — grant Accessibility permission in System Settings > Privacy & Security > Accessibility".to_string()
            );
        }
        Ok(Self { source })
    }

    /// Send a key press + release to the system (goes to the focused window).
    pub fn send_key(&self, key: KeyInfo) -> Result<(), String> {
        unsafe {
            let press = CGEventCreateKeyboardEvent(self.source, key.keycode, true);
            if press.is_null() {
                return Err("Failed to create key-down event".to_string());
            }
            let release = CGEventCreateKeyboardEvent(self.source, key.keycode, false);
            if release.is_null() {
                CFRelease(press as *const c_void);
                return Err("Failed to create key-up event".to_string());
            }

            CGEventPost(KCG_HID_EVENT_TAP, press);
            CGEventPost(KCG_HID_EVENT_TAP, release);

            CFRelease(press as *const c_void);
            CFRelease(release as *const c_void);
        }
        Ok(())
    }
}

impl Drop for MacOsBackend {
    fn drop(&mut self) {
        if !self.source.is_null() {
            unsafe { CFRelease(self.source as *const c_void) };
        }
    }
}
