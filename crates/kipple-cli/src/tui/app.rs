//! TUI state and key handling (UI.md §4). Pure: side effects are returned
//! as [`Effect`]s for the runtime to perform, so this is testable without a
//! terminal.

use std::collections::BTreeSet;

use kipple_core::model::*;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Overview,
    Projects,
    AiHistory,
    Plan,
    Quarantine,
}

impl Screen {
    pub const ALL: [Screen; 5] = [
        Screen::Overview,
        Screen::Projects,
        Screen::AiHistory,
        Screen::Plan,
        Screen::Quarantine,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Screen::Overview => "Overview",
            Screen::Projects => "Projects",
            Screen::AiHistory => "AI history",
            Screen::Plan => "Plan",
            Screen::Quarantine => "Quarantine",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Categories,
    Items,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Size,
    Age,
    Name,
}

impl Sort {
    pub fn next(self) -> Self {
        match self {
            Sort::Size => Sort::Age,
            Sort::Age => Sort::Name,
            Sort::Name => Sort::Size,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Sort::Size => "size",
            Sort::Age => "age",
            Sort::Name => "name",
        }
    }
}

/// Where the plan screen is.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanState {
    /// Showing the plan; Enter asks for confirmation.
    Review,
    /// Typing `y` or `reclaim`.
    Confirm {
        input: String,
    },
    Running {
        done: u64,
        total: u64,
    },
    Finished(Box<ExecReport>),
}

/// What the runtime should do after a key.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    None,
    Quit,
    /// Build a plan from the selected item ids.
    MakePlan(Vec<ItemId>),
    Execute(Confirmation),
    Undo,
    Restore(QuarantineId),
    Rescan,
}

pub struct App {
    pub screen: Screen,
    pub focus: Focus,
    pub items: Vec<Item>,
    pub projects: Vec<ProjectSummary>,
    pub scan: Option<ScanInfo>,
    pub summary: Option<Summary>,
    /// Entries walked so far while a scan runs.
    pub scanning: Option<u64>,
    pub cat_idx: usize,
    pub item_idx: usize,
    pub project_idx: usize,
    /// Drilled into a project on the Projects screen.
    pub project_open: Option<ProjectId>,
    pub ai_idx: usize,
    pub selected: BTreeSet<ItemId>,
    pub filter: String,
    pub filtering: bool,
    pub sort: Sort,
    pub show_explain: bool,
    pub show_help: bool,
    pub plan: Option<Plan>,
    pub plan_state: PlanState,
    pub plan_idx: usize,
    pub quarantine: Vec<QuarantineEntry>,
    pub q_idx: usize,
    pub message: Option<String>,
    pub keep_days: u32,
}

impl App {
    pub fn new(keep_days: u32) -> Self {
        Self {
            screen: Screen::Overview,
            focus: Focus::Items,
            items: Vec::new(),
            projects: Vec::new(),
            scan: None,
            summary: None,
            scanning: None,
            cat_idx: 0,
            item_idx: 0,
            project_idx: 0,
            project_open: None,
            ai_idx: 0,
            selected: BTreeSet::new(),
            filter: String::new(),
            filtering: false,
            sort: Sort::Size,
            show_explain: true,
            show_help: false,
            plan: None,
            plan_state: PlanState::Review,
            plan_idx: 0,
            quarantine: Vec::new(),
            q_idx: 0,
            message: None,
            keep_days,
        }
    }

    /// Loads a finished scan.
    pub fn load(&mut self, scan: ScanInfo, items: Vec<Item>, projects: Vec<ProjectSummary>) {
        self.summary = Some(kipple_core::summarize(&items));
        self.scan = Some(scan);
        self.items = items;
        self.projects = projects;
        self.scanning = None;
        let ids: BTreeSet<&ItemId> = self.items.iter().map(|i| &i.id).collect();
        self.selected.retain(|id| ids.contains(id));
        self.clamp();
    }

    pub fn categories(&self) -> Vec<CategorySummary> {
        self.summary
            .as_ref()
            .map(|s| s.categories.clone())
            .unwrap_or_default()
    }

    pub fn current_category(&self) -> Option<Category> {
        self.categories().get(self.cat_idx).map(|c| c.category)
    }

    fn matches_filter(&self, i: &Item) -> bool {
        if self.filter.is_empty() {
            return true;
        }
        let f = self.filter.to_lowercase();
        i.title.to_lowercase().contains(&f)
            || i.integration.as_str().contains(&f)
            || i.locator
                .primary_path()
                .is_some_and(|p| p.to_string_lossy().to_lowercase().contains(&f))
    }

    fn sorted<'a>(&self, mut v: Vec<&'a Item>) -> Vec<&'a Item> {
        match self.sort {
            Sort::Size => v.sort_by_key(|i| std::cmp::Reverse(i.size.reclaimable)),
            Sort::Age => v.sort_by_key(|i| i.last_used.map(|l| l.at)),
            Sort::Name => v.sort_by_key(|i| i.title.to_lowercase()),
        }
        v
    }

    /// Items in the list the cursor is on.
    pub fn visible_items(&self) -> Vec<&Item> {
        let v: Vec<&Item> = match self.screen {
            Screen::Overview => {
                let cat = self.current_category();
                self.items
                    .iter()
                    .filter(|i| Some(i.category) == cat && i.safety != Safety::Keep)
                    .filter(|i| self.matches_filter(i))
                    .collect()
            }
            Screen::Projects => match &self.project_open {
                Some(id) => self
                    .items
                    .iter()
                    .filter(|i| i.project.as_ref().is_some_and(|p| &p.id == id))
                    .filter(|i| self.matches_filter(i))
                    .collect(),
                None => Vec::new(),
            },
            Screen::AiHistory => self
                .items
                .iter()
                .filter(|i| i.category == Category::AiHistory)
                .filter(|i| self.matches_filter(i))
                .collect(),
            Screen::Plan | Screen::Quarantine => Vec::new(),
        };
        self.sorted(v)
    }

    pub fn visible_projects(&self) -> Vec<&ProjectSummary> {
        let f = self.filter.to_lowercase();
        let mut v: Vec<&ProjectSummary> = self
            .projects
            .iter()
            .filter(|p| {
                f.is_empty()
                    || p.name.to_lowercase().contains(&f)
                    || p.root.to_string_lossy().to_lowercase().contains(&f)
            })
            .collect();
        match self.sort {
            Sort::Size => v.sort_by_key(|p| std::cmp::Reverse(p.total.reclaimable)),
            Sort::Age => v.sort_by_key(|p| p.last_used.map(|l| l.at)),
            Sort::Name => v.sort_by_key(|p| p.name.to_lowercase()),
        }
        v
    }

    fn cursor(&self) -> usize {
        match self.screen {
            Screen::AiHistory => self.ai_idx,
            _ => self.item_idx,
        }
    }

    /// The highlighted item, for the explain panel.
    pub fn current_item(&self) -> Option<&Item> {
        self.visible_items().get(self.cursor()).copied()
    }

    pub fn selected_bytes(&self) -> u64 {
        self.items
            .iter()
            .filter(|i| self.selected.contains(&i.id))
            .map(|i| i.size.reclaimable)
            .sum()
    }

    fn clamp(&mut self) {
        let cats = self.categories().len();
        self.cat_idx = self.cat_idx.min(cats.saturating_sub(1));
        let n = self.visible_items().len();
        self.item_idx = self.item_idx.min(n.saturating_sub(1));
        self.ai_idx = self.ai_idx.min(n.saturating_sub(1));
        let p = self.visible_projects().len();
        self.project_idx = self.project_idx.min(p.saturating_sub(1));
        self.q_idx = self.q_idx.min(self.quarantine.len().saturating_sub(1));
        let pe = self.plan.as_ref().map(|p| p.entries.len()).unwrap_or(0);
        self.plan_idx = self.plan_idx.min(pe.saturating_sub(1));
    }

    fn move_by(&mut self, delta: isize) {
        let step = |idx: usize, len: usize| -> usize {
            if len == 0 {
                0
            } else {
                (idx as isize + delta).clamp(0, len as isize - 1) as usize
            }
        };
        match self.screen {
            Screen::Overview if self.focus == Focus::Categories => {
                self.cat_idx = step(self.cat_idx, self.categories().len());
                self.item_idx = 0;
            }
            Screen::Overview => self.item_idx = step(self.item_idx, self.visible_items().len()),
            Screen::Projects if self.project_open.is_none() => {
                self.project_idx = step(self.project_idx, self.visible_projects().len());
            }
            Screen::Projects => self.item_idx = step(self.item_idx, self.visible_items().len()),
            Screen::AiHistory => self.ai_idx = step(self.ai_idx, self.visible_items().len()),
            Screen::Plan => {
                let n = self.plan.as_ref().map(|p| p.entries.len()).unwrap_or(0);
                self.plan_idx = step(self.plan_idx, n);
            }
            Screen::Quarantine => self.q_idx = step(self.q_idx, self.quarantine.len()),
        }
    }

    fn toggle_current(&mut self) {
        let Some(item) = self.current_item() else {
            return;
        };
        if item.safety == Safety::Keep || item.actions.is_empty() {
            self.message = Some("kipple won't act on this item.".into());
            return;
        }
        let id = item.id.clone();
        if !self.selected.remove(&id) {
            self.selected.insert(id);
        }
    }

    fn select_all_safe(&mut self) {
        let ids: Vec<ItemId> = self
            .visible_items()
            .iter()
            .filter(|i| i.safety == Safety::Safe && !i.actions.is_empty())
            .map(|i| i.id.clone())
            .collect();
        let all_on = ids.iter().all(|id| self.selected.contains(id));
        for id in ids {
            if all_on {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id);
            }
        }
    }

    pub fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.filter.clear();
        self.filtering = false;
        self.clamp();
    }

    /// Handles one key.
    pub fn on_key(&mut self, key: KeyEvent) -> Effect {
        self.message = None;
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Effect::Quit;
        }
        if self.show_help {
            self.show_help = false;
            return Effect::None;
        }
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.filtering = false;
                }
                KeyCode::Enter => self.filtering = false,
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Char(c) => self.filter.push(c),
                _ => {}
            }
            self.item_idx = 0;
            self.ai_idx = 0;
            self.project_idx = 0;
            return Effect::None;
        }
        if let PlanState::Confirm { input } = &mut self.plan_state {
            match key.code {
                KeyCode::Esc => self.plan_state = PlanState::Review,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => input.push(c),
                KeyCode::Enter => {
                    let word = input.trim().to_owned();
                    let has_review = self.plan.as_ref().is_some_and(Plan::has_review);
                    let ok = if has_review {
                        word == Confirmation::REVIEW_WORD
                    } else {
                        word.eq_ignore_ascii_case("y") || word == Confirmation::REVIEW_WORD
                    };
                    if ok {
                        let total = self
                            .plan
                            .as_ref()
                            .map(|p| p.entries.len() as u64)
                            .unwrap_or(0);
                        self.plan_state = PlanState::Running { done: 0, total };
                        return Effect::Execute(Confirmation::Typed { word });
                    }
                    self.message = Some(if has_review {
                        "Type reclaim to confirm, or press Esc.".into()
                    } else {
                        "Type y to confirm, or press Esc.".into()
                    });
                    input.clear();
                }
                _ => {}
            }
            return Effect::None;
        }
        if matches!(self.plan_state, PlanState::Running { .. }) {
            return Effect::None;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                if self.screen == Screen::Projects && self.project_open.is_some() {
                    self.project_open = None;
                    self.item_idx = 0;
                } else if key.code == KeyCode::Char('q') {
                    return Effect::Quit;
                } else if self.screen != Screen::Overview {
                    self.go(Screen::Overview);
                }
            }
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char(c @ '1'..='5') => {
                let idx = c as usize - '1' as usize;
                self.go(Screen::ALL[idx]);
            }
            KeyCode::Tab => {
                if self.screen == Screen::Overview {
                    self.focus = match self.focus {
                        Focus::Categories => Focus::Items,
                        Focus::Items => Focus::Categories,
                    };
                }
            }
            KeyCode::Left | KeyCode::Char('h') if self.screen == Screen::Overview => {
                self.focus = Focus::Categories;
            }
            KeyCode::Right | KeyCode::Char('l') if self.screen == Screen::Overview => {
                self.focus = Focus::Items;
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_by(1),
            KeyCode::Up | KeyCode::Char('k') => self.move_by(-1),
            KeyCode::PageDown => self.move_by(10),
            KeyCode::PageUp => self.move_by(-10),
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter.clear();
            }
            KeyCode::Char('s') => {
                self.sort = self.sort.next();
                self.message = Some(format!("Sorted by {}.", self.sort.label()));
            }
            KeyCode::Char('e') => self.show_explain = !self.show_explain,
            KeyCode::Char(' ') => {
                if self.screen == Screen::Overview && self.focus == Focus::Categories {
                    self.focus = Focus::Items;
                }
                self.toggle_current();
            }
            KeyCode::Char('a') => self.select_all_safe(),
            KeyCode::Enter => match self.screen {
                Screen::Overview if self.focus == Focus::Categories => self.focus = Focus::Items,
                Screen::Projects if self.project_open.is_none() => {
                    if let Some(p) = self.visible_projects().get(self.project_idx) {
                        self.project_open = Some(p.id.clone());
                        self.item_idx = 0;
                        self.filter.clear();
                    }
                }
                Screen::Plan
                    if self.plan_state == PlanState::Review
                        && self.plan.as_ref().is_some_and(|p| !p.entries.is_empty()) =>
                {
                    self.plan_state = PlanState::Confirm {
                        input: String::new(),
                    };
                }
                Screen::Plan if matches!(self.plan_state, PlanState::Finished(_)) => {
                    self.plan = None;
                    self.plan_state = PlanState::Review;
                    return Effect::Rescan;
                }
                _ => self.toggle_current(),
            },
            KeyCode::Char('c') => {
                if self.selected.is_empty() {
                    self.message =
                        Some("Select items with space first (a selects all safe ones).".into());
                } else {
                    return Effect::MakePlan(self.selected.iter().cloned().collect());
                }
            }
            KeyCode::Char('u') if self.screen == Screen::Quarantine => return Effect::Undo,
            KeyCode::Char('r') if self.screen == Screen::Quarantine => {
                if let Some(e) = self.quarantine.get(self.q_idx) {
                    return Effect::Restore(e.id.clone());
                }
            }
            KeyCode::Char('r') => return Effect::Rescan,
            _ => {}
        }
        Effect::None
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::path::PathBuf;

    pub fn item(n: u64, cat: Category, safety: Safety) -> Item {
        let integration = IntegrationId::from("t");
        let locator = Locator::path(PathBuf::from(format!("/h/code/p{n}/node_modules")));
        Item {
            id: ItemId::new(&integration, &locator),
            integration,
            category: cat,
            kind: "k".into(),
            title: format!("item {n}"),
            locator,
            size: Size::simple(n * 1000, n * 1000),
            project: None,
            last_used: None,
            safety,
            reasons: vec![Reason {
                code: ReasonCode::Regenerable,
                text: "Recreated automatically.".into(),
            }],
            actions: if safety == Safety::Keep {
                vec![]
            } else {
                vec![ActionKind::Quarantine]
            },
            details: serde_json::Value::Null,
            notes: vec![],
            restore_hint: None,
            identity: None,
        }
    }

    pub fn scan_info() -> ScanInfo {
        ScanInfo {
            id: ScanId::from("s"),
            started_at: Timestamp::now(),
            finished_at: Some(Timestamp::now()),
            roots: vec![],
            kipple_version: "test".into(),
            status: ScanStatus::Complete,
            entries_seen: 1000,
            duration_ms: 1200,
        }
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::from(KeyCode::Char(c))
    }

    pub fn app() -> App {
        let mut a = App::new(7);
        a.load(
            scan_info(),
            vec![
                item(3, Category::PackageCache, Safety::Safe),
                item(2, Category::PackageCache, Safety::Review),
                item(9, Category::ProjectArtifact, Safety::Safe),
                item(1, Category::ProjectArtifact, Safety::Keep),
            ],
            vec![],
        );
        a
    }

    #[test]
    fn select_and_make_plan() {
        let mut a = app();
        assert_eq!(a.current_category(), Some(Category::ProjectArtifact));
        assert_eq!(a.on_key(key('c')), Effect::None, "nothing selected");
        a.on_key(key(' '));
        assert_eq!(a.selected.len(), 1);
        a.on_key(key('j'));
        assert_eq!(a.visible_items().len(), 1, "keep items aren't listed");
        match a.on_key(key('c')) {
            Effect::MakePlan(ids) => assert_eq!(ids.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn all_safe_toggles_only_safe_items() {
        let mut a = app();
        a.focus = Focus::Categories;
        a.on_key(key('j'));
        assert_eq!(a.current_category(), Some(Category::PackageCache));
        a.on_key(key('a'));
        assert_eq!(a.selected.len(), 1);
        a.on_key(key('a'));
        assert!(a.selected.is_empty());
    }

    #[test]
    fn review_plans_need_reclaim() {
        let mut a = app();
        let mut plan_item = item(2, Category::PackageCache, Safety::Review);
        plan_item.safety = Safety::Review;
        a.plan = Some(Plan {
            id: PlanId::from("p"),
            created_at: Timestamp::now(),
            scan_id: ScanId::from("s"),
            entries: vec![PlanEntry {
                item: plan_item,
                action: ActionKind::Quarantine,
                preflight: vec![],
            }],
            totals: PlanTotals::default(),
            warnings: vec![],
        });
        a.go(Screen::Plan);
        a.on_key(KeyEvent::from(KeyCode::Enter));
        assert!(matches!(a.plan_state, PlanState::Confirm { .. }));
        a.on_key(key('y'));
        assert_eq!(a.on_key(KeyEvent::from(KeyCode::Enter)), Effect::None);
        for c in "reclaim".chars() {
            a.on_key(key(c));
        }
        assert!(matches!(
            a.on_key(KeyEvent::from(KeyCode::Enter)),
            Effect::Execute(_)
        ));
    }

    #[test]
    fn filter_narrows_items() {
        let mut a = app();
        a.focus = Focus::Categories;
        a.on_key(key('j'));
        a.on_key(key('/'));
        for c in "item 2".chars() {
            a.on_key(key(c));
        }
        a.on_key(KeyEvent::from(KeyCode::Enter));
        assert_eq!(a.visible_items().len(), 1);
        assert_eq!(a.visible_items()[0].title, "item 2");
    }

    #[test]
    fn number_keys_switch_screens_and_q_quits() {
        let mut a = app();
        a.on_key(key('5'));
        assert_eq!(a.screen, Screen::Quarantine);
        a.on_key(KeyEvent::from(KeyCode::Esc));
        assert_eq!(a.screen, Screen::Overview);
        assert_eq!(a.on_key(key('q')), Effect::Quit);
    }
}
