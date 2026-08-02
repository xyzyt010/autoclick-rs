#![allow(unsafe_code)]

use crossbeam_channel::{bounded, Sender};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::engine::{Event, KeySender};
use crate::injector;
use crate::keyboard::all_keys;
use crate::targets::{self, Target};

slint::include_modules!();

thread_local! {
    static TICK_COUNT: std::cell::RefCell<u32> = std::cell::RefCell::new(0);
}

fn check_permission(ui: &AppWindow) {
    if injector::is_accessibility_trusted() {
        ui.set_perm_warning(SharedString::from(""));
    } else {
        ui.set_perm_warning(SharedString::from(
            "Key automation will NOT work until Accessibility permission is granted.\n\nSteps:\n1. Click \"Open System Settings\" below\n2. Find \"autoclick-rs\" in the list and toggle it ON\n3. If it's not listed, click + and add the autoclick-rs binary\n4. Return here — the warning will disappear automatically"
        ));
    }
}

fn key_desc_multi(indices: &[usize]) -> String {
    let keys = all_keys();
    let names: Vec<&str> = indices
        .iter()
        .filter_map(|&i| keys.get(i).map(|(n, _)| *n))
        .collect();
    names.join(" + ")
}

struct PanelState {
    id: i32,
    key_count: usize,
    key_indices: Vec<usize>, // always 5 elements, only first key_count are active
    mode_index: usize,
    interval_sec: SharedString,
    interval_min: SharedString,
    duration: SharedString,
    target_index: i32,
    targets: Vec<Target>,
    target_labels: Rc<VecModel<SharedString>>,
    terminal_count: usize,
    gui_header_index: i32,
    status: SharedString,
    running: bool,
    scanning: bool,
    sender: Option<KeySender>,
}

impl PanelState {
    fn label_index_to_target(&self, raw: i32) -> Option<usize> {
        if raw < 0 {
            return None;
        }
        let raw = raw as usize;
        if raw == 0 {
            return None;
        }
        let mut pos = raw - 1;
        if self.gui_header_index > 0 && raw > self.gui_header_index as usize {
            pos -= 1;
        } else if self.gui_header_index > 0 && raw == self.gui_header_index as usize {
            return None;
        }
        if pos < self.targets.len() {
            Some(pos)
        } else {
            None
        }
    }
}

struct Inner {
    panels: RefCell<Vec<PanelState>>,
    next_id: RefCell<i32>,
    event_tx: Sender<(i32, Event)>,
}

pub struct App {
    inner: Rc<Inner>,
}

impl App {
    pub fn run() {
        let (event_tx, event_rx) = bounded::<(i32, Event)>(64);

        let inner = Rc::new(Inner {
            panels: RefCell::new(Vec::new()),
            next_id: RefCell::new(1),
            event_tx,
        });

        let app = Self {
            inner: inner.clone(),
        };

        let ui = AppWindow::new().expect("Failed to create UI");

        // Add first tab.
        app.add_panel(&ui);

        // Wire callbacks.
        let inner2 = inner.clone();
        let ui_weak = ui.as_weak();
        ui.on_add_tab(move || {
            if let Some(ui) = ui_weak.upgrade() {
                app_add_panel(&inner2, &ui);
            }
        });

        let inner3 = inner.clone();
        let ui_weak2 = ui.as_weak();
        ui.on_close_all(move || {
            if let Some(ui) = ui_weak2.upgrade() {
                close_all(&inner3, &ui);
            }
        });

        let inner4 = inner.clone();
        let ui_weak3 = ui.as_weak();
        ui.on_select_tab(move |idx| {
            if let Some(ui) = ui_weak3.upgrade() {
                ui.set_current_idx(idx);
                sync_panel(&inner4, &ui);
            }
        });

        let inner5 = inner.clone();
        let ui_weak4 = ui.as_weak();
        ui.on_close_tab(move |idx| {
            if let Some(ui) = ui_weak4.upgrade() {
                close_tab(&inner5, &ui, idx);
            }
        });

        let inner6 = inner.clone();
        let ui_weak5 = ui.as_weak();
        ui.on_key_slot_changed(move |id, slot, idx| {
            if let Some(ui) = ui_weak5.upgrade() {
                set_key_slot(&inner6, &ui, id, slot as usize, idx as usize);
            }
        });

        let inner20 = inner.clone();
        let ui_weak20 = ui.as_weak();
        ui.on_key_count_changed(move |id, count| {
            if let Some(ui) = ui_weak20.upgrade() {
                set_key_count(&inner20, &ui, id, count as usize);
            }
        });

        let inner7 = inner.clone();
        ui.on_interval_sec_changed(move |id, val| {
            let mut panels = inner7.panels.borrow_mut();
            if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
                p.interval_sec = val.clone();
            }
        });

        let inner8 = inner.clone();
        ui.on_interval_min_changed(move |id, val| {
            let mut panels = inner8.panels.borrow_mut();
            if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
                p.interval_min = val.clone();
            }
        });

        let inner9 = inner.clone();
        ui.on_duration_changed(move |id, val| {
            let mut panels = inner9.panels.borrow_mut();
            if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
                p.duration = val.clone();
            }
        });

        let inner10 = inner.clone();
        ui.on_target_changed(move |id, idx| {
            let mut panels = inner10.panels.borrow_mut();
            if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
                p.target_index = idx;
            }
        });

        let inner11 = inner.clone();
        let ui_weak10 = ui.as_weak();
        ui.on_refresh_targets(move |id| {
            if let Some(ui) = ui_weak10.upgrade() {
                refresh_targets(&inner11, &ui, id);
            }
        });

        let inner12 = inner.clone();
        let ui_weak11 = ui.as_weak();
        ui.on_start_sender(move |id| {
            if let Some(ui) = ui_weak11.upgrade() {
                start_sender(&inner12, &ui, id);
            }
        });

        let inner13 = inner.clone();
        let ui_weak12 = ui.as_weak();
        ui.on_stop_sender(move |id| {
            if let Some(ui) = ui_weak12.upgrade() {
                stop_sender(&inner13, &ui, id);
            }
        });

        // Open Accessibility settings.
        ui.on_open_accessibility_settings(move || {
            injector::open_accessibility_settings();
        });

        // Check Accessibility permission on startup.
        check_permission(&ui);

        // Poll events timer.
        let inner14 = inner.clone();
        let ui_weak13 = ui.as_weak();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(200), move || {
            if let Some(ui) = ui_weak13.upgrade() {
                poll_events(&inner14, &ui, &event_rx);
                // Re-check permission every ~2 seconds (10 ticks).
                TICK_COUNT.with(|c| {
                    let mut count = c.borrow_mut();
                    *count += 1;
                    if *count % 10 == 0 {
                        check_permission(&ui);
                    }
                });
            }
        });
        std::mem::forget(timer);

        ui.run().expect("UI run failed");
    }

    fn add_panel(&self, ui: &AppWindow) {
        app_add_panel(&self.inner, ui);
    }
}

fn app_add_panel(inner: &Rc<Inner>, ui: &AppWindow) {
    let id = {
        let mut nid = inner.next_id.borrow_mut();
        let id = *nid;
        *nid += 1;
        id
    };

    let target_labels = Rc::new(VecModel::from(Vec::<SharedString>::new()));

    let panel = PanelState {
        id,
        key_count: 1,
        key_indices: vec![0; 5],
        mode_index: 0,
        interval_sec: SharedString::from("1"),
        interval_min: SharedString::from("0"),
        duration: SharedString::from("0"),
        target_index: -1,
        targets: Vec::new(),
        target_labels: target_labels.clone(),
        terminal_count: 0,
        gui_header_index: -1,
        status: SharedString::from("macOS — CGEvent backend"),
        running: false,
        scanning: false,
        sender: None,
    };

    inner.panels.borrow_mut().push(panel);

    let tabs = ui.get_tabs();
    let tabs_vec: Vec<TabData> = tabs.iter().collect();
    let mut new_tabs: Vec<TabData> = tabs_vec;
    new_tabs.push(TabData {
        id,
        title: SharedString::from(format!("Tab {id}")),
    });
    ui.set_tabs(ModelRc::new(VecModel::from(new_tabs)));

    let count = inner.panels.borrow().len();
    ui.set_current_idx((count - 1) as i32);

    sync_panel(inner, ui);
}

fn sync_panel(inner: &Rc<Inner>, ui: &AppWindow) {
    let idx = ui.get_current_idx();
    let panels = inner.panels.borrow();
    if idx < 0 || (idx as usize) >= panels.len() {
        return;
    }
    let p = &panels[idx as usize];

    let keys = all_keys();
    let key_names: Vec<SharedString> = keys.iter().map(|(n, _)| SharedString::from(*n)).collect();

    let active_indices: Vec<i32> = p.key_indices.iter().take(p.key_count).map(|&i| i as i32).collect();
    let key_indices_model = Rc::new(VecModel::from(active_indices));

    let panel_data = PanelData {
        id: p.id,
        mode_index: p.mode_index as i32,
        keys: ModelRc::new(VecModel::from(key_names)),
        key_count: p.key_count as i32,
        key_indices: ModelRc::from(key_indices_model),
        key_desc: SharedString::from(key_desc_multi(&p.key_indices[..p.key_count])),
        interval_sec: p.interval_sec.clone(),
        interval_min: p.interval_min.clone(),
        duration: p.duration.clone(),
        target_index: p.target_index,
        target_labels: ModelRc::from(p.target_labels.clone()),
        status: p.status.clone(),
        running: p.running,
        scanning: p.scanning,
    };

    let panels_model = ui.get_panels();
    let mut panels_vec: Vec<PanelData> = panels_model.iter().collect();
    if (idx as usize) < panels_vec.len() {
        panels_vec[idx as usize] = panel_data;
    } else {
        panels_vec.push(panel_data);
    }
    ui.set_panels(ModelRc::new(VecModel::from(panels_vec)));
}

fn close_all(inner: &Rc<Inner>, ui: &AppWindow) {
    for p in inner.panels.borrow_mut().iter_mut() {
        if let Some(ref sender) = p.sender {
            sender.stop();
        }
        p.sender = None;
        p.running = false;
    }
    inner.panels.borrow_mut().clear();
    ui.set_tabs(ModelRc::new(VecModel::from(Vec::<TabData>::new())));
    ui.set_panels(ModelRc::new(VecModel::from(Vec::<PanelData>::new())));
    ui.set_current_idx(-1);
}

fn close_tab(inner: &Rc<Inner>, ui: &AppWindow, idx: i32) {
    let mut panels = inner.panels.borrow_mut();
    if idx < 0 || (idx as usize) >= panels.len() {
        return;
    }
    if let Some(ref sender) = panels[idx as usize].sender {
        sender.stop();
    }
    panels.remove(idx as usize);
    drop(panels);

    let panels = inner.panels.borrow();
    let new_tabs: Vec<TabData> = panels
        .iter()
        .map(|p| TabData {
            id: p.id,
            title: SharedString::from(format!("Tab {}", p.id)),
        })
        .collect();
    ui.set_tabs(ModelRc::new(VecModel::from(new_tabs)));

    let new_idx = if idx > 0 { idx - 1 } else { 0 };
    ui.set_current_idx(if panels.is_empty() { -1 } else { new_idx });
    drop(panels);
    sync_panel(inner, ui);
}

fn set_key_slot(inner: &Rc<Inner>, ui: &AppWindow, id: i32, slot: usize, key_index: usize) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        if slot < p.key_indices.len() {
            p.key_indices[slot] = key_index;
        }
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn set_key_count(inner: &Rc<Inner>, ui: &AppWindow, id: i32, count: usize) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.key_count = count.clamp(1, 5);
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn refresh_targets(inner: &Rc<Inner>, ui: &AppWindow, id: i32) {
    let my_pid = std::process::id();
    let (targets, terminal_count) = targets::enumerate_all(my_pid);

    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.targets = targets;
        p.terminal_count = terminal_count;
        p.scanning = false;

        if p.targets.is_empty() {
            p.target_index = -1;
            p.gui_header_index = -1;
            p.target_labels.set_vec(Vec::<SharedString>::new());
            p.status = SharedString::from("No windows found. Click Refresh to retry.");
        } else {
            let mut labels: Vec<SharedString> = vec![SharedString::from("\u{2500}\u{2500} TERMINALS \u{2500}\u{2500}")];
            for i in 0..p.terminal_count {
                labels.push(SharedString::from(p.targets[i].label()));
            }
            if p.terminal_count < p.targets.len() {
                let hdr = labels.len() as i32;
                labels.push(SharedString::from("\u{2500}\u{2500} GUI APPS \u{2500}\u{2500}"));
                for i in p.terminal_count..p.targets.len() {
                    labels.push(SharedString::from(p.targets[i].label()));
                }
                p.gui_header_index = hdr;
            } else {
                p.gui_header_index = -1;
            }
            p.target_labels.set_vec(labels);
            p.target_index = 1; // skip TERMINALS header
            p.status = SharedString::from(format!("{} windows found", p.targets.len()));
        }
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn start_sender(inner: &Rc<Inner>, ui: &AppWindow, id: i32) {
    let (key_indices, key_count, sec_s, min_s, duration_s, tgt_pos, target_pid) = {
        let panels = inner.panels.borrow();
        match panels.iter().find(|p| p.id == id) {
            Some(p) => {
                let pos = p.label_index_to_target(p.target_index);
                let pid = pos
                    .filter(|i| *i < p.targets.len())
                    .map(|i| p.targets[i].pid)
                    .unwrap_or(0);
                (
                    p.key_indices.clone(),
                    p.key_count,
                    p.interval_sec.clone(),
                    p.interval_min.clone(),
                    p.duration.clone(),
                    pos,
                    pid,
                )
            }
            None => return,
        }
    };

    if tgt_pos.is_none() {
        let mut panels = inner.panels.borrow_mut();
        if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
            p.status = SharedString::from("No target selected — click Refresh first");
        }
        drop(panels);
        sync_panel(inner, ui);
        return;
    }

    let secs: f64 = sec_s.as_str().parse().unwrap_or(0.0);
    let mins: f64 = min_s.as_str().parse().unwrap_or(0.0);
    let total_secs = secs + mins * 60.0;
    let interval = Duration::from_secs_f64(total_secs.max(0.01));

    let duration: Option<Duration> = {
        let d_mins: f64 = duration_s.as_str().parse().unwrap_or(0.0);
        if d_mins > 0.0 {
            Some(Duration::from_secs_f64(d_mins * 60.0))
        } else {
            None
        }
    };

    let all_keys = all_keys();
    let mut keys = Vec::new();
    for i in 0..key_count {
        let idx = key_indices[i];
        if idx >= all_keys.len() {
            return;
        }
        keys.push(all_keys[idx].1);
    }

    let sender = KeySender::start(keys, interval, duration, target_pid);

    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.sender = Some(sender);
        p.running = true;
        p.status = SharedString::from("Running...");
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn stop_sender(inner: &Rc<Inner>, ui: &AppWindow, id: i32) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        if let Some(ref sender) = p.sender {
            sender.stop();
        }
        p.sender = None;
        p.running = false;
        p.status = SharedString::from("Stopped");
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn poll_events(inner: &Rc<Inner>, ui: &AppWindow, rx: &crossbeam_channel::Receiver<(i32, Event)>) {
    let mut had_event = false;
    while let Ok((id, event)) = rx.try_recv() {
        had_event = true;
        let mut panels = inner.panels.borrow_mut();
        if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
            match event {
                Event::Tick { count, method } => {
                    p.status = SharedString::from(format!("[{method}] Sent {count} keys"));
                }
                Event::Error(e) => {
                    p.status = SharedString::from(format!("Error: {e}"));
                    p.running = false;
                    p.sender = None;
                }
                Event::Done(count) => {
                    p.status = SharedString::from(format!("Done — {count} keys sent"));
                    p.running = false;
                    p.sender = None;
                }
            }
        }
        drop(panels);
    }
    if had_event {
        sync_panel(inner, ui);
    }
}
