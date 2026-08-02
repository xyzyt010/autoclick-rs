//! Key-sending engine for macOS: spawns a worker thread that injects keys via CGEvent.

use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::injector::MacOsBackend;
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
    pub fn start(
        keys: Vec<KeyInfo>,
        interval: Duration,
        duration: Option<Duration>,
        target_pid: u32,
    ) -> Self {
        let (tx, rx) = bounded::<Event>(16);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();

        let handle = thread::spawn(move || {
            worker(keys, interval, duration, target_pid, stop_clone, tx);
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
    keys: Vec<KeyInfo>,
    interval: Duration,
    duration: Option<Duration>,
    target_pid: u32,
    stop: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    let backend = match MacOsBackend::create() {
        Ok(b) => b,
        Err(e) => {
            let _ = tx.send(Event::Error(e));
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
            |k| backend.key_down(*k, target_pid),
            |k| backend.send_key(*k, target_pid),
            |k| backend.key_up(*k, target_pid),
        );
        if let Err(e) = result {
            let _ = tx.send(Event::Error(e));
            break;
        }

        count += 1;
        if count % 10 == 0 || count == 1 {
            let _ = tx.send(Event::Tick { count, method: "CGEvent" });
        }

        thread::sleep(interval);
    }

    let _ = tx.send(Event::Done(count));
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
