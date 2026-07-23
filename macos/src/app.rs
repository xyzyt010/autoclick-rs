#![allow(unsafe_code)]

use crossbeam_channel::{bounded, Sender};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::engine::{Event, KeySender};
use crate::keyboard::all_keys;
use crate::targets::{self, Target};

slint::include_modules!();

fn key_desc(idx: usize) -> String {
    let keys = all_keys();
    if idx < keys.len() {
        let info = keys[idx].1;
        format!("keycode=0x{:02X}", info.keycode)
    } else {
        String::new()
    }
}

struct PanelState {
    id: i32,
    key_index: usize,
    interval_sec: SharedString,
    interval_min: SharedString,
    duration: SharedString,
    target_index: i32,
    targets: Vec<Target>,
    all_targets: Vec<Target>,
    target_labels: Rc<VecModel<SharedString>>,
    search_text: SharedString,
    status: SharedString,
    running: bool,
    scanning: bool,
    sender: Option<KeySender>,
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
        ui.on_key_changed(move |id, idx| {
            if let Some(ui) = ui_weak5.upgrade() {
                key_changed(&inner6, &ui, id, idx);
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

        let inner15 = inner.clone();
        let ui_weak14 = ui.as_weak();
        ui.on_filter_targets(move |id, txt| {
            if let Some(ui) = ui_weak14.upgrade() {
                filter_targets(&inner15, &ui, id, &txt);
            }
        });

        // Poll events timer.
        let inner14 = inner.clone();
        let ui_weak13 = ui.as_weak();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(200), move || {
            if let Some(ui) = ui_weak13.upgrade() {
                poll_events(&inner14, &ui, &event_rx);
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
        key_index: 0,
        interval_sec: SharedString::from("1"),
        interval_min: SharedString::from("0"),
        duration: SharedString::from("0"),
        target_index: -1,
        targets: Vec::new(),
        all_targets: Vec::new(),
        target_labels: target_labels.clone(),
        search_text: SharedString::from(""),
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

    let panel_data = PanelData {
        id: p.id,
        key_index: p.key_index as i32,
        keys: ModelRc::new(VecModel::from(key_names)),
        key_desc: SharedString::from(key_desc(p.key_index)),
        interval_sec: p.interval_sec.clone(),
        interval_min: p.interval_min.clone(),
        duration: p.duration.clone(),
        target_index: p.target_index,
        target_labels: ModelRc::from(p.target_labels.clone()),
        search_text: p.search_text.clone(),
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

fn key_changed(inner: &Rc<Inner>, ui: &AppWindow, id: i32, idx: i32) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.key_index = idx.max(0) as usize;
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn refresh_targets(inner: &Rc<Inner>, ui: &AppWindow, id: i32) {
    let my_pid = std::process::id();
    let targets = targets::enumerate_all(my_pid);

    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.all_targets = targets.clone();
        p.search_text = SharedString::from("");
        let labels: Vec<SharedString> = targets.iter().map(|t| SharedString::from(t.label())).collect();
        p.targets = targets;
        p.target_labels.set_vec(labels);
        p.target_index = if p.targets.is_empty() { -1 } else { 0 };
        p.status = SharedString::from(format!("{} windows found", p.targets.len()));
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn filter_targets(inner: &Rc<Inner>, ui: &AppWindow, id: i32, txt: &SharedString) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.search_text = txt.clone();
        let query = txt.as_str().to_lowercase();
        if query.is_empty() {
            let labels: Vec<SharedString> = p.all_targets.iter().map(|t| SharedString::from(t.label())).collect();
            p.targets = p.all_targets.clone();
            p.target_labels.set_vec(labels);
        } else {
            let starts: Vec<&Target> = p.all_targets.iter()
                .filter(|t| t.label().to_lowercase().starts_with(&query))
                .collect();
            let contains: Vec<&Target> = p.all_targets.iter()
                .filter(|t| !t.label().to_lowercase().starts_with(&query) && t.label().to_lowercase().contains(&query))
                .collect();
            let filtered: Vec<Target> = starts.into_iter().chain(contains.into_iter()).cloned().collect();
            let labels: Vec<SharedString> = filtered.iter().map(|t| SharedString::from(t.label())).collect();
            p.targets = filtered;
            p.target_labels.set_vec(labels);
        }
        p.target_index = if p.targets.is_empty() { -1 } else { 0 };
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn start_sender(inner: &Rc<Inner>, ui: &AppWindow, id: i32) {
    let (key_index, sec_s, min_s, duration_s, tgt_idx, tgt_len, target_pid) = {
        let panels = inner.panels.borrow();
        match panels.iter().find(|p| p.id == id) {
            Some(p) => {
                let pid = if p.target_index >= 0 && (p.target_index as usize) < p.targets.len() {
                    p.targets[p.target_index as usize].pid
                } else {
                    0
                };
                (
                    p.key_index,
                    p.interval_sec.clone(),
                    p.interval_min.clone(),
                    p.duration.clone(),
                    p.target_index,
                    p.targets.len(),
                    pid,
                )
            }
            None => return,
        }
    };

    if tgt_idx < 0 || tgt_len == 0 {
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

    let keys = all_keys();
    if key_index >= keys.len() {
        return;
    }
    let key = keys[key_index].1;

    let sender = KeySender::start(key, interval, duration, target_pid);

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
