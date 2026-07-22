//! macOS key injection via CGEvent (Core Graphics Event API).
//! Requires Accessibility permission (System Settings > Privacy > Accessibility).

use core_graphics::event::{CGEvent, CGEventTap, CGKeyCode, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

use crate::keyboard::KeyInfo;

pub struct MacOsBackend {
    source: CGEventSource,
}

impl MacOsBackend {
    /// Create the event source. Fails if Accessibility permission is not granted.
    pub fn create() -> Result<Self, String> {
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
            .map_err(|_| "CGEventSource creation failed — grant Accessibility permission in System Settings > Privacy & Security > Accessibility".to_string())?;
        Ok(Self { source })
    }

    /// Send a key press + release to the system (goes to the focused window).
    pub fn send_key(&self, key: KeyInfo) -> Result<(), String> {
        let kc = key.keycode as CGKeyCode;

        let press = CGEvent::new_keyboard_event(self.source.clone(), kc, true)
            .map_err(|_| "Failed to create key-down event".to_string())?;
        let release = CGEvent::new_keyboard_event(self.source.clone(), kc, false)
            .map_err(|_| "Failed to create key-up event".to_string())?;

        press.post(CGEventTap::HID);
        release.post(CGEventTap::HID);

        Ok(())
    }

    /// Check if Accessibility permission is granted by attempting to create an event.
    pub fn check_accessibility() -> bool {
        CGEventSource::new(CGEventSourceStateID::HIDSystemState).is_ok()
    }
}
