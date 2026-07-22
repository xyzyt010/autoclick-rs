//! Key-sending engine for macOS: spawns a worker thread that injects keys via CGEvent.

use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::injector::MacOsBackend;
use crate::keyboard::KeyInfo;

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
        key: KeyInfo,
        interval: Duration,
        duration: Option<Duration>,
        target_pid: u32,
    ) -> Self {
        let (tx, rx) = bounded::<Event>(16);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();

        let handle = thread::spawn(move || {
            worker(key, interval, duration, target_pid, stop_clone, tx);
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
    key: KeyInfo,
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

        match backend.send_key(key, target_pid) {
            Ok(()) => {
                count += 1;
                if count % 10 == 0 || count == 1 {
                    let _ = tx.send(Event::Tick { count, method: "CGEvent" });
                }
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e));
                break;
            }
        }

        thread::sleep(interval);
    }

    let _ = tx.send(Event::Done(count));
}
