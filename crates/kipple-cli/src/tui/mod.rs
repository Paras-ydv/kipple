//! The terminal UI (SPEC §12, UI.md §4). Talks to the core only through the
//! public `Kipple` API.

pub mod app;
pub mod theme;
pub mod ui;

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use anyhow::Result;
use kipple_core::Kipple;
use kipple_core::config::Units;
use kipple_core::model::*;
use ratatui::crossterm::event::{self, Event, KeyEventKind};

use self::app::{App, Effect, PlanState, Screen};
use self::theme::Theme;
use self::ui::View;

enum Msg {
    Key(event::KeyEvent),
    Resize,
    ScanProgress(u64),
    ScanDone(std::result::Result<(), String>),
    ExecProgress,
    ExecDone(std::result::Result<ExecReport, String>),
}

struct ScanChannel {
    tx: Sender<Msg>,
    cancel: CancelToken,
}

impl ScanSink for ScanChannel {
    fn event(&self, event: ScanEvent) {
        if let ScanEvent::Progress { done, .. } = event {
            let _ = self.tx.send(Msg::ScanProgress(done));
        }
    }

    fn cancel_token(&self) -> Option<&CancelToken> {
        Some(&self.cancel)
    }
}

struct ExecChannel {
    tx: Sender<Msg>,
    cancel: CancelToken,
}

impl ExecSink for ExecChannel {
    fn event(&self, event: ExecEvent) {
        if let ExecEvent::EntryFinished { .. } = event {
            let _ = self.tx.send(Msg::ExecProgress);
        }
    }

    fn cancel_token(&self) -> Option<&CancelToken> {
        Some(&self.cancel)
    }
}

pub struct Options {
    pub theme: Theme,
    pub units: Units,
    pub home: Option<std::path::PathBuf>,
    /// Scan before showing results (otherwise use the latest scan).
    pub scan_first: bool,
    pub request: ScanRequest,
}

struct Runtime {
    k: Arc<Kipple>,
    tx: Sender<Msg>,
    scan_cancel: CancelToken,
    exec_cancel: CancelToken,
    request: ScanRequest,
}

impl Runtime {
    fn start_scan(&mut self, app: &mut App) {
        let k = Arc::clone(&self.k);
        let tx = self.tx.clone();
        self.scan_cancel = CancelToken::new();
        let sink = ScanChannel {
            tx: tx.clone(),
            cancel: self.scan_cancel.clone(),
        };
        let req = self.request.clone();
        app.scanning = Some(0);
        std::thread::spawn(move || {
            let r = k.scan(req, &sink).map(|_| ()).map_err(|e| e.to_string());
            let _ = tx.send(Msg::ScanDone(r));
        });
    }

    fn load_latest(&self, app: &mut App) -> bool {
        match (self.k.all_items(), self.k.projects(Default::default())) {
            (Ok((scan, items)), Ok(projects)) => {
                app.load(scan.scan, items, projects);
                true
            }
            _ => false,
        }
    }

    fn refresh_quarantine(&self, app: &mut App) {
        app.quarantine = self.k.quarantine().unwrap_or_default();
        app.quarantine.reverse();
    }

    fn apply(&mut self, app: &mut App, effect: Effect) {
        match effect {
            Effect::None | Effect::Quit => {}
            Effect::MakePlan(ids) => {
                let req = PlanRequest {
                    filter: ItemFilter {
                        ids: ids.iter().map(|i| i.to_string()).collect(),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                match self.k.plan(req) {
                    Ok(plan) => {
                        app.plan = Some(plan);
                        app.plan_state = PlanState::Review;
                        app.plan_idx = 0;
                        app.go(Screen::Plan);
                    }
                    Err(e) => app.message = Some(e.to_string()),
                }
            }
            Effect::Execute(confirmation) => {
                let Some(plan) = app.plan.clone() else { return };
                let k = Arc::clone(&self.k);
                let tx = self.tx.clone();
                self.exec_cancel = CancelToken::new();
                let sink = ExecChannel {
                    tx: tx.clone(),
                    cancel: self.exec_cancel.clone(),
                };
                std::thread::spawn(move || {
                    let r = k
                        .execute(&plan, confirmation, &sink)
                        .map_err(|e| e.to_string());
                    let _ = tx.send(Msg::ExecDone(r));
                });
            }
            Effect::Undo => {
                match self.k.undo() {
                    Ok(r) if r.restored.is_empty() && r.skipped.is_empty() => {
                        app.message = Some("Quarantine is empty; there's nothing to undo.".into());
                    }
                    Ok(r) => {
                        app.message = Some(format!(
                            "Restored {} items{}.",
                            r.restored.len(),
                            if r.skipped.is_empty() {
                                String::new()
                            } else {
                                format!(", {} couldn't be restored", r.skipped.len())
                            }
                        ));
                    }
                    Err(e) => app.message = Some(e.to_string()),
                }
                self.refresh_quarantine(app);
            }
            Effect::Restore(id) => {
                match self.k.restore(&id, None) {
                    Ok(r) => {
                        app.message = Some(match (r.restored.first(), r.skipped.first()) {
                            (Some(x), _) => format!("Restored {}.", x.title),
                            (None, Some(s)) => s.reason.clone(),
                            _ => String::new(),
                        })
                    }
                    Err(e) => app.message = Some(e.to_string()),
                }
                self.refresh_quarantine(app);
            }
            Effect::Rescan => {
                if app.scanning.is_none() {
                    self.start_scan(app);
                }
            }
        }
    }
}

fn input_thread(tx: Sender<Msg>) {
    std::thread::spawn(move || {
        loop {
            match event::poll(Duration::from_millis(200)) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
                        if tx.send(Msg::Key(k)).is_err() {
                            break;
                        }
                    }
                    Ok(Event::Resize(..)) => {
                        let _ = tx.send(Msg::Resize);
                    }
                    Ok(_) => {}
                    Err(_) => break,
                },
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });
}

/// Runs the TUI until the user quits.
pub fn run(k: Kipple, opts: Options) -> Result<()> {
    let k = Arc::new(k);
    let (tx, rx): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();
    let mut app = App::new(k.config().quarantine.keep_days);
    let mut rt = Runtime {
        k: Arc::clone(&k),
        tx: tx.clone(),
        scan_cancel: CancelToken::new(),
        exec_cancel: CancelToken::new(),
        request: opts.request.clone(),
    };
    if opts.scan_first || !rt.load_latest(&mut app) {
        rt.start_scan(&mut app);
    }
    rt.refresh_quarantine(&mut app);

    let mut terminal = ratatui::init();
    input_thread(tx);
    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|f| {
                View {
                    app: &app,
                    theme: opts.theme,
                    units: opts.units,
                    home: opts.home.clone(),
                }
                .draw(f)
            })?;
            let Ok(msg) = rx.recv() else { break };
            match msg {
                Msg::Key(key) => {
                    let effect = app.on_key(key);
                    if effect == Effect::Quit {
                        rt.scan_cancel.cancel();
                        if matches!(app.plan_state, PlanState::Running { .. }) {
                            rt.exec_cancel.cancel();
                            app.message = Some("Stopping after the current item…".into());
                            continue;
                        }
                        break;
                    }
                    rt.apply(&mut app, effect);
                    if app.screen == Screen::Quarantine {
                        rt.refresh_quarantine(&mut app);
                    }
                }
                Msg::Resize => {}
                Msg::ScanProgress(n) => {
                    if app.scanning.is_some() {
                        app.scanning = Some(n);
                    }
                }
                Msg::ScanDone(r) => {
                    app.scanning = None;
                    match r {
                        Ok(()) => {
                            rt.load_latest(&mut app);
                        }
                        Err(e) => app.message = Some(e),
                    }
                }
                Msg::ExecProgress => {
                    if let PlanState::Running { done, .. } = &mut app.plan_state {
                        *done += 1;
                    }
                }
                Msg::ExecDone(r) => {
                    match r {
                        Ok(report) => {
                            app.selected.clear();
                            app.plan_state = PlanState::Finished(Box::new(report));
                        }
                        Err(e) => {
                            app.plan_state = PlanState::Review;
                            app.message = Some(e);
                        }
                    }
                    rt.refresh_quarantine(&mut app);
                }
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}

#[cfg(test)]
mod tests {
    use super::app::tests::app;
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render(app: &App, w: u16, h: u16, theme: Theme) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|f| {
                View {
                    app,
                    theme,
                    units: Units::Si,
                    home: None,
                }
                .draw(f)
            })
            .unwrap();
        terminal.backend().to_string()
    }

    fn mono() -> Theme {
        Theme {
            color: false,
            ascii: false,
        }
    }

    fn redact(s: String) -> String {
        s.lines()
            .map(|l| l.replace("scanned just now", "scanned <age>"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn overview_80x24() {
        let mut a = app();
        a.on_key(event::KeyEvent::from(event::KeyCode::Char(' ')));
        insta::assert_snapshot!(redact(render(&a, 80, 24, mono())));
    }

    #[test]
    fn overview_120x32_ascii() {
        let a = app();
        insta::assert_snapshot!(redact(render(
            &a,
            120,
            32,
            Theme {
                color: false,
                ascii: true
            }
        )));
    }

    #[test]
    fn help_overlay() {
        let mut a = app();
        a.on_key(event::KeyEvent::from(event::KeyCode::Char('?')));
        insta::assert_snapshot!(redact(render(&a, 80, 24, mono())));
    }

    #[test]
    fn empty_screens_render() {
        let mut a = app();
        for s in Screen::ALL {
            a.go(s);
            let out = render(&a, 80, 24, mono());
            assert!(out.contains(s.label()), "{s:?}");
        }
    }

    #[test]
    fn scanning_state() {
        let mut a = App::new(7);
        a.scanning = Some(123_456);
        let out = render(&a, 80, 24, mono());
        assert!(out.contains("Scanning… 123k files"));
    }
}
