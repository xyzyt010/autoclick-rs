use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::injector::send_key;
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
    /// Start a sender thread. `hwnd_raw` is the raw HWND pointer value (0 = none).
    pub fn start(
        hwnd_raw: i64,
        pid: Option<u32>,
        key: KeyInfo,
        interval: Duration,
        duration: Option<Duration>,
    ) -> Self {
        let (tx, rx) = bounded::<Event>(16);
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_clone = stop_flag.clone();

        let builder = thread::Builder::new().stack_size(256 * 1024);
        let handle = builder
            .spawn(move || {
                run_loop(hwnd_raw, pid, key, interval, duration, tx, stop_clone);
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
    key: KeyInfo,
    interval: Duration,
    duration: Option<Duration>,
    tx: Sender<Event>,
    stop_flag: Arc<AtomicBool>,
) {
    let start = Instant::now();
    let mut count: u64 = 0;

    while !stop_flag.load(Ordering::SeqCst) {
        if let Some(d) = duration {
            if start.elapsed() >= d {
                let _ = tx.send(Event::Done(count));
                return;
            }
        }

        // Reconstruct HWND inside the worker thread (HWND is !Send)
        let hwnd = if hwnd_raw != 0 {
            Some(windows::Win32::Foundation::HWND(hwnd_raw as *mut _))
        } else {
            None
        };

        match send_key(hwnd, pid, key) {
            Ok(method) => {
                count += 1;
                let _ = tx.send(Event::Tick {
                    count,
                    method: method.name(),
                });
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.to_string()));
                return;
            }
        }

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
