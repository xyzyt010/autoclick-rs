#![allow(unsafe_code)]

use crossbeam_channel::{bounded, Sender};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::engine::{Event, KeySender};
use crate::keyboard::all_keys;
use crate::targets::{self, Target, TargetMode};

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
    mode: TargetMode,
    key_index: usize,
    interval: SharedString,
    duration: SharedString,
    target_index: i32,
    targets: Vec<Target>,
    target_labels: Rc<VecModel<SharedString>>,
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
        ui.on_mode_changed(move |id, mode_str| {
            if let Some(ui) = ui_weak5.upgrade() {
                mode_changed(&inner6, &ui, id, &mode_str);
            }
        });

        let inner7 = inner.clone();
        let ui_weak6 = ui.as_weak();
        ui.on_key_changed(move |id, idx| {
            if let Some(ui) = ui_weak6.upgrade() {
                key_changed(&inner7, &ui, id, idx);
            }
        });

        let inner8 = inner.clone();
        let ui_weak7 = ui.as_weak();
        ui.on_interval_changed(move |id, val| {
            if let Some(_ui) = ui_weak7.upgrade() {
                interval_changed(&inner8, id, &val);
            }
        });

        let inner9 = inner.clone();
        let ui_weak8 = ui.as_weak();
        ui.on_duration_changed(move |id, val| {
            if let Some(_ui) = ui_weak8.upgrade() {
                duration_changed(&inner9, id, &val);
            }
        });

        let inner10 = inner.clone();
        let ui_weak9 = ui.as_weak();
        ui.on_target_changed(move |id, idx| {
            if let Some(_ui) = ui_weak9.upgrade() {
                target_changed(&inner10, id, idx);
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

        // Poll events timer.
        let inner14 = inner.clone();
        let ui_weak13 = ui.as_weak();
        let timer = Timer::default();
        timer.start(TimerMode::Repeated, Duration::from_millis(100), move || {
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
        mode: TargetMode::Terminal,
        key_index: 0,
        interval: SharedString::from("1.0"),
        duration: SharedString::from("0"),
        target_index: -1,
        targets: Vec::new(),
        target_labels: target_labels.clone(),
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
        mode: SharedString::from(match p.mode {
            TargetMode::Terminal => "terminal",
            TargetMode::App => "app",
        }),
        key_index: p.key_index as i32,
        keys: ModelRc::new(VecModel::from(key_names)),
        key_desc: SharedString::from(key_desc(p.key_index)),
        interval: p.interval.clone(),
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

fn mode_changed(inner: &Rc<Inner>, ui: &AppWindow, id: i32, mode_str: &SharedString) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.mode = match mode_str.as_str() {
            "app" => TargetMode::App,
            _ => TargetMode::Terminal,
        };
        p.target_index = -1;
        p.targets.clear();
        p.target_labels.set_vec(Vec::new());
    }
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

fn interval_changed(inner: &Rc<Inner>, id: i32, val: &SharedString) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.interval = val.clone();
    }
}

fn duration_changed(inner: &Rc<Inner>, id: i32, val: &SharedString) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.duration = val.clone();
    }
}

fn target_changed(inner: &Rc<Inner>, id: i32, idx: i32) {
    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        p.target_index = idx;
    }
}

fn refresh_targets(inner: &Rc<Inner>, ui: &AppWindow, id: i32) {
    let mode = {
        let panels = inner.panels.borrow();
        match panels.iter().find(|p| p.id == id) {
            Some(p) => p.mode,
            None => return,
        }
    };

    let my_pid = std::process::id();
    let targets = targets::enumerate(mode, my_pid);

    let mut panels = inner.panels.borrow_mut();
    if let Some(p) = panels.iter_mut().find(|p| p.id == id) {
        let labels: Vec<SharedString> = targets.iter().map(|t| SharedString::from(t.label())).collect();
        p.targets = targets;
        p.target_labels.set_vec(labels);
        p.target_index = if p.targets.is_empty() { -1 } else { 0 };
        p.status = SharedString::from(format!("{} targets found", p.targets.len()));
    }
    drop(panels);
    sync_panel(inner, ui);
}

fn start_sender(inner: &Rc<Inner>, ui: &AppWindow, id: i32) {
    let (key_index, interval_s, duration_s, tgt_idx, tgt_len, target_pid) = {
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
                    p.interval.clone(),
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

    let interval: f64 = interval_s.as_str().parse().unwrap_or(1.0);
    let interval = Duration::from_secs_f64(interval.max(0.01));

    let duration: Option<Duration> = {
        let mins: f64 = duration_s.as_str().parse().unwrap_or(0.0);
        if mins > 0.0 {
            Some(Duration::from_secs_f64(mins * 60.0))
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
    while let Ok((id, event)) = rx.try_recv() {
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
        sync_panel(inner, ui);
    }
}
