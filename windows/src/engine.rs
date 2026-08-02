use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::injector::{key_down, key_down_screen, key_up, key_up_screen, send_key, send_key_screen, Mode};
use crate::keyboard::{is_modifier, KeyInfo};

#[derive(Clone, Debug)]
pub enum Event {
    Tick { count: u64, method: &'static str },
    Error(String),
    Done(u64),
}

pub struct KeySender {
    #[allow(dead_code)]
    handle: Mutex<Option<JoinHandle<()>>>,
    stop_flag: Arc<AtomicBool>,
    events_rx: Receiver<Event>,
}

impl KeySender {
    /// Start a sender thread. `hwnd_raw` is the raw HWND pointer value (0 = none).
    pub fn start(
        hwnd_raw: i64,
        pid: Option<u32>,
        keys: Vec<KeyInfo>,
        interval: Duration,
        duration: Option<Duration>,
        mode: Mode,
    ) -> Self {
        let (tx, rx) = bounded::<Event>(16);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        let builder = thread::Builder::new().stack_size(256 * 1024);
        let handle = builder
            .spawn(move || {
                run_loop(hwnd_raw, pid, keys, interval, duration, tx, stop_clone, mode);
            })
            .ok();

        Self {
            handle: Mutex::new(handle),
            stop_flag,
            events_rx: rx,
        }
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    pub fn drain(&self) -> Vec<Event> {
        let mut out = Vec::new();
        loop {
            match self.events_rx.try_recv() {
                Ok(e) => out.push(e),
                Err(_) => break,
            }
        }
        out
    }
}

fn run_loop(
    hwnd_raw: i64,
    pid: Option<u32>,
    keys: Vec<KeyInfo>,
    interval: Duration,
    duration: Option<Duration>,
    tx: Sender<Event>,
    stop_flag: Arc<AtomicBool>,
    mode: Mode,
) {
    let start = Instant::now();
    let mut count: u64 = 0;

    // Reconstruct HWND inside the worker thread (HWND is !Send).
    let hwnd = if hwnd_raw != 0 {
        Some(windows::Win32::Foundation::HWND(hwnd_raw as *mut _))
    } else {
        None
    };

    // Split combo into modifier keys (held down) and regular keys (tapped).
    let modifiers: Vec<KeyInfo> = keys.iter().copied().filter(|k| is_modifier(k)).collect();
    let regular: Vec<KeyInfo> = keys.iter().copied().filter(|k| !is_modifier(k)).collect();
    let has_mods = !modifiers.is_empty();

    // Helper to send one full key press.
    let press_key = |key: KeyInfo| -> Result<&'static str, String> {
        let result = match mode {
            Mode::Window => send_key(hwnd, pid, key, has_mods),
            Mode::Screen => send_key_screen(key, has_mods),
        };
        result.map(|m| m.name()).map_err(|e| e.to_string())
    };
    // Helper to press a key down (modifiers only).
    let press_down = |key: KeyInfo| -> Result<(), String> {
        let result = match mode {
            Mode::Window => key_down(hwnd, pid, key),
            Mode::Screen => key_down_screen(key),
        };
        result.map(|_| ()).map_err(|e| e.to_string())
    };
    // Helper to release a key (modifiers only).
    let release_up = |key: KeyInfo| -> Result<(), String> {
        let result = match mode {
            Mode::Window => key_up(hwnd, pid, key),
            Mode::Screen => key_up_screen(key),
        };
        result.map(|_| ()).map_err(|e| e.to_string())
    };

    while !stop_flag.load(Ordering::SeqCst) {
        if let Some(d) = duration {
            if start.elapsed() >= d {
                let _ = tx.send(Event::Done(count));
                return;
            }
        }

        let mut last_method: &'static str = "Window";
        let mut had_error = false;
        let mut pressed: Vec<KeyInfo> = Vec::new();

        // 1. Hold modifier keys down.
        for &key in &modifiers {
            match press_down(key) {
                Ok(()) => pressed.push(key),
                Err(e) => {
                    for &pk in pressed.iter().rev() {
                        let _ = release_up(pk);
                    }
                    let _ = tx.send(Event::Error(e));
                    return;
                }
            }
        }

        // 2. Tap the regular keys.
        for &key in &regular {
            match press_key(key) {
                Ok(m) => {
                    last_method = m;
                }
                Err(e) => {
                    for &pk in pressed.iter().rev() {
                        let _ = release_up(pk);
                    }
                    let _ = tx.send(Event::Error(e));
                    return;
                }
            }
        }

        // 3. Release modifiers in reverse order.
        for &key in pressed.iter().rev() {
            if let Err(e) = release_up(key) {
                let _ = tx.send(Event::Error(e));
                had_error = true;
                break;
            }
        }

        if had_error {
            return;
        }

        count += 1;
        let _ = tx.send(Event::Tick {
            count,
            method: last_method,
        });

        let mut remaining = interval;
        while remaining > Duration::ZERO {
            if stop_flag.load(Ordering::SeqCst) {
                let _ = tx.send(Event::Done(count));
                return;
            }
            let slice = remaining.min(Duration::from_millis(100));
            thread::sleep(slice);
            remaining -= slice;
        }
    }

    let _ = tx.send(Event::Done(count));
}
