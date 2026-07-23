#![allow(unsafe_code)]

use crossbeam_channel::{bounded, Sender};
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use crate::engine::{Event, KeySender};
use crate::keyboard::all_keys;
use crate::targets::{gui as tgui, terminal as tterminal, Target, TargetMode};

slint::include_modules!();

fn key_desc(idx: usize) -> String {
    let keys = all_keys();
    if idx < keys.len() {
        let info = keys[idx].1;
        if info.unicode != 0 {
            format!("VK=0x{:02X}  Char=0x{:04X}", info.vk, info.unicode)
        } else {
            format!("VK=0x{:02X}", info.vk)
        }
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
    all_targets: Vec<Target>,
    target_labels: Rc<VecModel<SharedString>>,
    search_text: SharedString,
    status: SharedString,
    running: bool,
    scanning: bool,
    sender: Option<KeySender>,
}

impl PanelState {
    fn new(id: i32) -> Self {
        Self {
            id,
            mode: TargetMode::Terminal,
            key_index: 0,
            interval: SharedString::from("5"),
            duration: SharedString::from("0"),
            target_index: -1,
            targets: Vec::new(),
            all_targets: Vec::new(),
            target_labels: Rc::new(VecModel::default()),
            search_text: SharedString::from(""),
            status: SharedString::from("Idle. Configure settings and click Start."),
            running: false,
            scanning: false,
            sender: None,
        }
    }

    fn to_panel_data(&self, keys: &ModelRc<SharedString>) -> PanelData {
        PanelData {
            id: self.id,
            mode: SharedString::from(match self.mode {
                TargetMode::Terminal => "terminal",
                TargetMode::App => "app",
            }),
            key_index: self.key_index as i32,
            keys: keys.clone(),
            key_desc: SharedString::from(key_desc(self.key_index)),
            interval: self.interval.clone(),
            duration: self.duration.clone(),
            target_index: self.target_index,
            target_labels: ModelRc::from(self.target_labels.clone()),
            search_text: self.search_text.clone(),
            status: self.status.clone(),
            running: self.running,
            scanning: self.scanning,
        }
    }
}

struct AppInner {
    panels: RefCell<Vec<PanelState>>,
    tabs_model: Rc<VecModel<TabData>>,
    panels_model: Rc<VecModel<PanelData>>,
    keys_model: Rc<VecModel<SharedString>>,
    scan_tx: Sender<(i32, Vec<Target>)>,
}

#[derive(Clone)]
struct Handle {
    inner: Rc<AppInner>,
    app: slint::Weak<AppWindow>,
}

impl Handle {
    fn find_panel(&self, id: i32) -> Option<usize> {
        self.inner.panels.borrow().iter().position(|p| p.id == id)
    }

    fn refresh_panel(&self, pos: usize) {
        let data = {
            let panels = self.inner.panels.borrow();
            panels[pos].to_panel_data(&ModelRc::from(self.inner.keys_model.clone()))
        };
        self.inner.panels_model.set_row_data(pos, data);
    }

    fn spawn_scan(&self, id: i32, mode: TargetMode) {
        let my_pid = std::process::id();
        let tx = self.inner.scan_tx.clone();
        std::thread::spawn(move || {
            let targets = match mode {
                TargetMode::Terminal => tterminal::list_candidate_shells(),
                TargetMode::App => tgui::list_candidate_apps(my_pid),
            };
            let _ = tx.send((id, targets));
        });
    }

    fn set_status(&self, id: i32, msg: &str) {
        if let Some(pos) = self.find_panel(id) {
            self.inner.panels.borrow_mut()[pos].status = SharedString::from(msg);
            self.refresh_panel(pos);
        }
    }

    fn add_tab(&self) {
        let next_id = {
            let panels = self.inner.panels.borrow();
            panels.iter().map(|p| p.id).max().unwrap_or(0) + 1
        };
        let mut state = PanelState::new(next_id);
        state.scanning = true;
        state.status = SharedString::from("Scanning for shells...");
        let panel_data = state.to_panel_data(&ModelRc::from(self.inner.keys_model.clone()));
        self.inner.panels.borrow_mut().push(state);
        self.inner.tabs_model.push(TabData {
            id: next_id,
            title: format!("Instance {next_id}").into(),
        });
        self.inner.panels_model.push(panel_data);
        let idx = self.inner.panels.borrow().len() - 1;
        if let Some(app) = self.app.upgrade() {
            app.set_current_idx(idx as i32);
        }
        self.spawn_scan(next_id, TargetMode::Terminal);
    }

    fn close_all(&self) {
        {
            let mut panels = self.inner.panels.borrow_mut();
            for p in panels.iter_mut() {
                if let Some(s) = p.sender.take() {
                    s.stop();
                }
            }
            panels.clear();
        }
        while self.inner.tabs_model.row_count() > 0 {
            self.inner.tabs_model.remove(0);
        }
        while self.inner.panels_model.row_count() > 0 {
            self.inner.panels_model.remove(0);
        }
        self.add_tab();
    }

    fn close_tab(&self, idx: usize) {
        {
            let len = self.inner.panels.borrow().len();
            if idx >= len {
                return;
            }
        }
        {
            let mut panels = self.inner.panels.borrow_mut();
            if let Some(s) = panels[idx].sender.take() {
                s.stop();
            }
            panels.remove(idx);
        }
        self.inner.tabs_model.remove(idx);
        self.inner.panels_model.remove(idx);
        let len = self.inner.panels.borrow().len();
        if len == 0 {
            self.add_tab();
        } else {
            let cur = self.app.upgrade().map(|a| a.get_current_idx() as usize).unwrap_or(0);
            if cur >= len {
                if let Some(app) = self.app.upgrade() {
                    app.set_current_idx((len - 1) as i32);
                }
            }
        }
    }

    fn select_tab(&self, idx: usize) {
        if idx < self.inner.panels.borrow().len() {
            if let Some(app) = self.app.upgrade() {
                app.set_current_idx(idx as i32);
            }
        }
    }

    fn set_mode(&self, id: i32, mode_str: &str) {
        let pos = match self.find_panel(id) {
            Some(p) => p,
            None => return,
        };
        let mode = if mode_str == "app" {
            TargetMode::App
        } else {
            TargetMode::Terminal
        };
        {
            let mut panels = self.inner.panels.borrow_mut();
            let p = &mut panels[pos];
            p.mode = mode;
            p.target_index = -1;
            p.targets.clear();
            p.target_labels.clear();
            p.scanning = true;
            p.status = SharedString::from("Scanning for targets...");
        }
        self.refresh_panel(pos);
        self.spawn_scan(id, mode);
    }

    fn set_key(&self, id: i32, key_index: i32) {
        if let Some(pos) = self.find_panel(id) {
            self.inner.panels.borrow_mut()[pos].key_index = key_index.max(0) as usize;
            self.refresh_panel(pos);
        }
    }

    fn set_interval(&self, id: i32, val: SharedString) {
        if let Some(pos) = self.find_panel(id) {
            self.inner.panels.borrow_mut()[pos].interval = val;
        }
    }

    fn set_duration(&self, id: i32, val: SharedString) {
        if let Some(pos) = self.find_panel(id) {
            self.inner.panels.borrow_mut()[pos].duration = val;
        }
    }

    fn set_target(&self, id: i32, idx: i32) {
        if let Some(pos) = self.find_panel(id) {
            self.inner.panels.borrow_mut()[pos].target_index = idx;
        }
    }

    fn start_sender(&self, id: i32) {
        let pos = match self.find_panel(id) {
            Some(p) => p,
            None => return,
        };
        {
            let mut panels = self.inner.panels.borrow_mut();
            if let Some(s) = panels[pos].sender.take() {
                s.stop();
            }
        }

        let (key_index, interval_s, duration_s, mode, tgt_idx, tgt_len) = {
            let panels = self.inner.panels.borrow();
            let p = &panels[pos];
            (
                p.key_index,
                p.interval.clone(),
                p.duration.clone(),
                p.mode,
                p.target_index,
                p.targets.len(),
            )
        };

        let keys = all_keys();
        if key_index >= keys.len() {
            self.set_status(id, "Select a valid key.");
            return;
        }
        let key_info = keys[key_index].1;

        let interval_secs: f64 = match interval_s.as_str().parse() {
            Ok(v) if v > 0.0 => v,
            _ => {
                self.set_status(id, "Interval must be > 0.");
                return;
            }
        };
        let duration_secs: Option<f64> = match duration_s.as_str().parse::<f64>() {
            Ok(v) if v > 0.0 => Some(v * 60.0),
            _ => None,
        };
        if tgt_idx < 0 || (tgt_idx as usize) >= tgt_len {
            self.set_status(id, "Select an accessible target first.");
            return;
        }

        let (pid, hwnd_raw) = {
            let panels = self.inner.panels.borrow();
            let t = &panels[pos].targets[tgt_idx as usize];
            let h = if mode == TargetMode::App { t.hwnd } else { 0 };
            (Some(t.pid), h)
        };

        let interval = Duration::from_secs_f64(interval_secs);
        let duration = duration_secs.map(Duration::from_secs_f64);
        let sender = KeySender::start(hwnd_raw, pid, key_info, interval, duration);

        let dur_text = match duration {
            Some(d) => format!("for {:.1} min", d.as_secs_f64() / 60.0),
            None => "until stopped".to_string(),
        };
        let key_name = keys[key_index].0;
        let status = format!("Running: '{key_name}' every {interval_secs:.1}s {dur_text}");

        {
            let mut panels = self.inner.panels.borrow_mut();
            let p = &mut panels[pos];
            p.sender = Some(sender);
            p.running = true;
            p.status = SharedString::from(status);
        }
        self.refresh_panel(pos);
    }

    fn stop_sender(&self, id: i32) {
        if let Some(pos) = self.find_panel(id) {
            let mut panels = self.inner.panels.borrow_mut();
            let p = &mut panels[pos];
            if let Some(s) = p.sender.as_ref() {
                s.stop();
            }
            p.status = SharedString::from("Stopping...");
            drop(panels);
            self.refresh_panel(pos);
        }
    }

    fn drain_sender_events(&self) {
        let snapshot: Vec<(usize, Vec<Event>)> = {
            let panels = self.inner.panels.borrow();
            panels
                .iter()
                .enumerate()
                .filter_map(|(pos, p)| {
                    let sender = p.sender.as_ref()?;
                    let events = sender.drain();
                    if events.is_empty() {
                        None
                    } else {
                        Some((pos, events))
                    }
                })
                .collect()
        };

        for (pos, events) in snapshot {
            let key_name = {
                let panels = self.inner.panels.borrow();
                let p = &panels[pos];
                let keys = all_keys();
                if p.key_index < keys.len() {
                    keys[p.key_index].1.display.to_string()
                } else {
                    "key".to_string()
                }
            };
            let mut status: Option<SharedString> = None;
            let mut keep_running = true;
            for e in events {
                match e {
                    Event::Tick { count, method } => {
                        status = Some(SharedString::from(format!(
                            "Pressed {key_name} {count} time(s) [{method}]"
                        )));
                    }
                    Event::Error(msg) => {
                        status = Some(SharedString::from(format!("Stopped \u{2014} {msg}")));
                        keep_running = false;
                    }
                    Event::Done(count) => {
                        status = Some(SharedString::from(format!(
                            "Finished. Pressed {key_name} {count} time(s)."
                        )));
                        keep_running = false;
                    }
                }
            }
            {
                let mut panels = self.inner.panels.borrow_mut();
                let p = &mut panels[pos];
                if let Some(s) = status {
                    p.status = s;
                }
                if !keep_running {
                    p.running = false;
                    p.sender = None;
                }
            }
            self.refresh_panel(pos);
        }
    }

    fn apply_scan_result(&self, id: i32, targets: Vec<Target>) {
        if let Some(pos) = self.find_panel(id) {
            {
                let mut panels = self.inner.panels.borrow_mut();
                let p = &mut panels[pos];
                p.all_targets = targets.clone();
                p.search_text = SharedString::from("");
                p.targets = targets;
                p.target_index = if p.targets.is_empty() { -1 } else { 0 };
                p.scanning = false;
                p.target_labels.clear();
                for t in &p.targets {
                    p.target_labels.push(SharedString::from(t.label()));
                }
                let n = p.targets.len();
                p.status = SharedString::from(if n == 0 {
                    "No targets found. Click Refresh to retry.".to_string()
                } else {
                    format!("Found {n} target(s). Select one and click Start.")
                });
            }
            self.refresh_panel(pos);
        }
    }

    fn filter_targets(&self, id: i32, txt: &SharedString) {
        if let Some(pos) = self.find_panel(id) {
            {
                let mut panels = self.inner.panels.borrow_mut();
                let p = &mut panels[pos];
                p.search_text = txt.clone();
                let query = txt.as_str().to_lowercase();
                if query.is_empty() {
                    p.targets = p.all_targets.clone();
                } else {
                    let starts: Vec<Target> = p.all_targets.iter()
                        .filter(|t| t.label().to_lowercase().starts_with(&query))
                        .cloned().collect();
                    let contains: Vec<Target> = p.all_targets.iter()
                        .filter(|t| !t.label().to_lowercase().starts_with(&query) && t.label().to_lowercase().contains(&query))
                        .cloned().collect();
                    p.targets = starts.into_iter().chain(contains.into_iter()).collect();
                }
                p.target_index = if p.targets.is_empty() { -1 } else { 0 };
                p.target_labels.clear();
                for t in &p.targets {
                    p.target_labels.push(SharedString::from(t.label()));
                }
            }
            self.refresh_panel(pos);
        }
    }
}

pub struct App {
    _timer: Timer,
}

impl App {
    pub fn run() {
        let app = AppWindow::new().expect("failed to create window");

        let tabs_model = Rc::new(VecModel::<TabData>::default());
        let panels_model = Rc::new(VecModel::<PanelData>::default());
        let keys: Vec<SharedString> = crate::keyboard::key_names()
            .iter()
            .map(|s| SharedString::from(*s))
            .collect();
        let keys_model = Rc::new(VecModel::from(keys));
        let (scan_tx, scan_rx) = bounded::<(i32, Vec<Target>)>(8);

        let inner = Rc::new(AppInner {
            panels: RefCell::new(Vec::new()),
            tabs_model: tabs_model.clone(),
            panels_model: panels_model.clone(),
            keys_model: keys_model.clone(),
            scan_tx,
        });

        app.set_tabs(ModelRc::from(tabs_model));
        app.set_panels(ModelRc::from(panels_model));

        let handle = Handle {
            inner: inner.clone(),
            app: app.as_weak(),
        };

        {
            let h = handle.clone();
            app.on_add_tab(move || h.add_tab());
        }
        {
            let h = handle.clone();
            app.on_close_all(move || h.close_all());
        }
        {
            let h = handle.clone();
            app.on_select_tab(move |i| h.select_tab(i as usize));
        }
        {
            let h = handle.clone();
            app.on_close_tab(move |i| h.close_tab(i as usize));
        }
        {
            let h = handle.clone();
            app.on_mode_changed(move |id, m| h.set_mode(id, m.as_str()));
        }
        {
            let h = handle.clone();
            app.on_key_changed(move |id, k| h.set_key(id, k));
        }
        {
            let h = handle.clone();
            app.on_interval_changed(move |id, v| h.set_interval(id, v));
        }
        {
            let h = handle.clone();
            app.on_duration_changed(move |id, v| h.set_duration(id, v));
        }
        {
            let h = handle.clone();
            app.on_target_changed(move |id, i| h.set_target(id, i));
        }
        {
            let h = handle.clone();
            app.on_refresh_targets(move |id| {
                let mode = {
                    let panels = h.inner.panels.borrow();
                    match panels.iter().find(|p| p.id == id) {
                        Some(p) => p.mode,
                        None => return,
                    }
                };
                {
                    if let Some(pos) = h.find_panel(id) {
                        h.inner.panels.borrow_mut()[pos].scanning = true;
                        h.refresh_panel(pos);
                    }
                }
                h.spawn_scan(id, mode);
            });
        }
        {
            let h = handle.clone();
            app.on_start_sender(move |id| h.start_sender(id));
        }
        {
            let h = handle.clone();
            app.on_stop_sender(move |id| h.stop_sender(id));
        }
        {
            let h = handle.clone();
            app.on_filter_targets(move |id, txt| h.filter_targets(id, &txt));
        }

        handle.add_tab();

        let timer = Timer::default();
        {
            let h = handle.clone();
            let rx = scan_rx;
            timer.start(TimerMode::Repeated, Duration::from_millis(150), move || {
                while let Ok((id, targets)) = rx.try_recv() {
                    h.apply_scan_result(id, targets);
                }
                h.drain_sender_events();
            });
        }

        let me = Self { _timer: timer };
        app.run().expect("event loop failed");
        drop(me);
    }
}
