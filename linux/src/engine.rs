//! Key-sending engine: spawns a worker thread that injects keys at a fixed interval.
//! Dispatches to X11 (XTest or XSendEvent) or uinput based on detected display server.

use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::detect::DisplayServer;
use crate::injector::uinput::UinputBackend;
use crate::injector::x11::X11Injector;
use crate::keyboard::{is_modifier, KeyInfo};

#[derive(Clone, Debug)]
pub enum Event {
    Tick { count: u64, method: String },
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
    /// Start a sender thread.
    /// `window_id`: X11 window to target (0 = focused window / Wayland global).
    pub fn start(
        ds: DisplayServer,
        window_id: u32,
        keys: Vec<KeyInfo>,
        interval: Duration,
        duration: Option<Duration>,
    ) -> Self {
        let (tx, rx) = bounded::<Event>(16);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();

        let handle = thread::spawn(move || {
            worker(ds, window_id, keys, interval, duration, stop_clone, tx);
        });

        Self {
            handle: Mutex::new(Some(handle)),
            stop_flag: stop,
            events_rx: rx,
        }
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    pub fn is_stopped(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }

    pub fn try_recv(&self) -> Option<Event> {
        self.events_rx.try_recv().ok()
    }
}

fn worker(
    ds: DisplayServer,
    window_id: u32,
    keys: Vec<KeyInfo>,
    interval: Duration,
    duration: Option<Duration>,
    stop: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    match ds {
        DisplayServer::X11 => worker_x11(window_id, keys, interval, duration, stop, tx),
        DisplayServer::Wayland => worker_uinput(keys, interval, duration, stop, tx),
    }
}

/// Send one combo: hold modifier keys down, tap the regular keys, then
/// release the modifiers in reverse order. Releases held keys on error so
/// a modifier never stays stuck.
fn send_combo<D, S, U>(
    keys: &[KeyInfo],
    mut down: D,
    mut send: S,
    mut up: U,
) -> Result<(), String>
where
    D: FnMut(&KeyInfo) -> Result<(), String>,
    S: FnMut(&KeyInfo) -> Result<(), String>,
    U: FnMut(&KeyInfo) -> Result<(), String>,
{
    let modifiers: Vec<KeyInfo> = keys.iter().copied().filter(|k| is_modifier(k)).collect();
    let regular: Vec<KeyInfo> = keys.iter().copied().filter(|k| !is_modifier(k)).collect();

    let mut pressed: Vec<KeyInfo> = Vec::new();
    for m in &modifiers {
        match down(m) {
            Ok(()) => pressed.push(*m),
            Err(e) => {
                for p in pressed.iter().rev() {
                    let _ = up(p);
                }
                return Err(e);
            }
        }
    }
    for k in &regular {
        if let Err(e) = send(k) {
            for p in pressed.iter().rev() {
                let _ = up(p);
            }
            return Err(e);
        }
    }
    for p in pressed.iter().rev() {
        up(p)?;
    }
    Ok(())
}

fn worker_x11(
    window_id: u32,
    keys: Vec<KeyInfo>,
    interval: Duration,
    duration: Option<Duration>,
    stop: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    let injector = match X11Injector::connect() {
        Ok(b) => b,
        Err(e) => {
            let _ = tx.send(Event::Error(format!("X11 init failed: {e}")));
            return;
        }
    };

    let method = injector.method_name().to_string();
    let mut count: u64 = 0;
    let start_time = Instant::now();

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if let Some(dur) = duration {
            if start_time.elapsed() >= dur {
                break;
            }
        }

        let result = send_combo(
            &keys,
            |k| injector.key_down(*k, window_id),
            |k| injector.send_key(*k, window_id),
            |k| injector.key_up(*k, window_id),
        );
        if let Err(e) = result {
            let _ = tx.send(Event::Error(e));
            break;
        }

        count += 1;
        if count % 10 == 0 || count == 1 {
            let _ = tx.send(Event::Tick { count, method: method.clone() });
        }

        thread::sleep(interval);
    }

    let _ = tx.send(Event::Done(count));
}

fn worker_uinput(
    keys: Vec<KeyInfo>,
    interval: Duration,
    duration: Option<Duration>,
    stop: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    let backend = match UinputBackend::create() {
        Ok(b) => b,
        Err(e) => {
            let _ = tx.send(Event::Error(format!("uinput init failed: {e}")));
            return;
        }
    };

    let mut count: u64 = 0;
    let start_time = Instant::now();

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if let Some(dur) = duration {
            if start_time.elapsed() >= dur {
                break;
            }
        }

        let result = send_combo(
            &keys,
            |k| backend.key_down(*k),
            |k| backend.send_key(*k),
            |k| backend.key_up(*k),
        );
        if let Err(e) = result {
            let _ = tx.send(Event::Error(e));
            break;
        }

        count += 1;
        if count % 10 == 0 || count == 1 {
            let _ = tx.send(Event::Tick { count, method: "uinput".to_string() });
        }

        thread::sleep(interval);
    }

    let _ = tx.send(Event::Done(count));
}
