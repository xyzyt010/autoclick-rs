//! Key-sending engine: spawns a worker thread that injects keys at a fixed interval.
//! Dispatches to X11 (XTest) or uinput based on detected display server.

use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::detect::DisplayServer;
use crate::injector::uinput::UinputBackend;
use crate::injector::x11::X11Backend;
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
    /// Start a sender thread.
    /// `window_id`: X11 window to target (0 = focused window / Wayland global).
    pub fn start(
        ds: DisplayServer,
        window_id: u32,
        key: KeyInfo,
        interval: Duration,
        duration: Option<Duration>,
    ) -> Self {
        let (tx, rx) = bounded::<Event>(16);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();

        let handle = thread::spawn(move || {
            worker(ds, window_id, key, interval, duration, stop_clone, tx);
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
    key: KeyInfo,
    interval: Duration,
    duration: Option<Duration>,
    stop: Arc<AtomicBool>,
    tx: Sender<Event>,
) {
    // Create the appropriate backend.
    let mut count: u64 = 0;
    let start_time = Instant::now();

    match ds {
        DisplayServer::X11 => {
            let backend = match X11Backend::connect() {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(Event::Error(format!("X11 init failed: {e}")));
                    return;
                }
            };

            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                if let Some(dur) = duration {
                    if start_time.elapsed() >= dur {
                        break;
                    }
                }

                match backend.send_key(key, window_id) {
                    Ok(()) => {
                        count += 1;
                        if count % 10 == 0 || count == 1 {
                            let _ = tx.send(Event::Tick { count, method: "XTest" });
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e));
                        break;
                    }
                }

                thread::sleep(interval);
            }
        }
        DisplayServer::Wayland => {
            let backend = match UinputBackend::create() {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx.send(Event::Error(format!("uinput init failed: {e}")));
                    return;
                }
            };

            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                if let Some(dur) = duration {
                    if start_time.elapsed() >= dur {
                        break;
                    }
                }

                match backend.send_key(key) {
                    Ok(()) => {
                        count += 1;
                        if count % 10 == 0 || count == 1 {
                            let _ = tx.send(Event::Tick { count, method: "uinput" });
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Event::Error(e));
                        break;
                    }
                }

                thread::sleep(interval);
            }
        }
    }

    let _ = tx.send(Event::Done(count));
}
