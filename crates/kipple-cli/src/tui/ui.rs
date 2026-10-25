//! Rendering (UI.md §4, Terminal UI board).

use kipple_core::config::Units;
use kipple_core::format;
use kipple_core::model::*;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};

use super::app::{App, Focus, PlanState, Screen};
use super::theme::{Theme, category_legend};

pub struct View<'a> {
    pub app: &'a App,
    pub theme: Theme,
    pub units: Units,
    pub home: Option<std::path::PathBuf>,
}

fn pad(s: &str, n: usize) -> String {
    let count = s.chars().count();
    if count > n {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    } else {
        format!("{s}{}", " ".repeat(n - count))
    }
}

fn lpad(s: &str, n: usize) -> String {
    let count = s.chars().count();
    if count >= n {
        s.to_owned()
    } else {
        format!("{}{s}", " ".repeat(n - count))
    }
}

impl View<'_> {
    fn size(&self, b: u64) -> String {
        format::size(b, self.units)
    }

    fn size_or_dash(&self, b: u64) -> String {
        if b == 0 {
            self.theme.dash().to_owned()
        } else {
            self.size(b)
        }
    }

    fn short(&self, p: &std::path::Path) -> String {
        kipple_core::platform::paths::shorten_home(p, self.home.as_deref())
    }

    fn block(&self) -> Block<'static> {
        Block::bordered()
            .border_set(self.theme.border_set())
            .border_style(self.theme.border())
    }

    pub fn draw(&self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(Block::new().style(self.theme.base()), area);
        let [header, tabs, _, body, status, keys] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .areas(area);
        self.header(f, header);
        self.tabs(f, tabs);
        match self.app.screen {
            Screen::Overview => self.overview(f, body),
            Screen::Projects => self.projects(f, body),
            Screen::AiHistory => self.ai(f, body),
            Screen::Plan => self.plan(f, body),
            Screen::Quarantine => self.quarantine(f, body),
        }
        self.status(f, status);
        self.keys(f, keys);
        if self.app.show_help {
            self.help(f, area);
        }
    }

    fn header(&self, f: &mut Frame, area: Rect) {
        let left = match &self.app.summary {
            Some(s) => format!(
                " kipple   {} reclaimable   {} safe   {} review",
                self.size(s.reclaimable),
                self.size(s.by_safety.safe),
                self.size(s.by_safety.review)
            ),
            None => " kipple".to_owned(),
        };
        let right = match (&self.app.scanning, &self.app.scan) {
            (Some(n), _) => format!(
                "scanning… {} files ",
                super::super::cmd::scan::compact_count(*n)
            ),
            (None, Some(s)) => {
                let secs = Timestamp::now().as_second()
                    - s.finished_at.unwrap_or(s.started_at).as_second();
                match format::age(secs).as_str() {
                    "just now" => "scanned just now ".to_owned(),
                    age => format!("scanned {age} ago "),
                }
            }
            _ => String::new(),
        };
        let gap =
            (area.width as usize).saturating_sub(left.chars().count() + right.chars().count());
        let text = format!("{left}{}{right}", " ".repeat(gap));
        f.render_widget(Paragraph::new(text).style(self.theme.header()), area);
    }

    fn tabs(&self, f: &mut Frame, area: Rect) {
        let mut spans = vec![Span::raw(" ")];
        for (i, s) in Screen::ALL.iter().enumerate() {
            let label = format!("{} {}", i + 1, s.label());
            let style = if *s == self.app.screen {
                self.theme.active_tab()
            } else {
                self.theme.muted()
            };
            spans.push(Span::styled(label, style));
            spans.push(Span::raw("   "));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn strip(&self, f: &mut Frame, area: Rect, cats: &[CategorySummary]) {
        let [bar, legend] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
        let total: u64 = cats.iter().map(|c| c.reclaimable).sum();
        let width = bar.width.saturating_sub(2) as usize;
        let mut spans = vec![Span::raw(" ")];
        let mut used = 0;
        let shown: Vec<&CategorySummary> = cats.iter().filter(|c| c.reclaimable > 0).collect();
        for (i, c) in shown.iter().enumerate() {
            let mut w = if total == 0 {
                0
            } else {
                (c.reclaimable as u128 * width as u128 / total as u128) as usize
            };
            if i + 1 == shown.len() {
                w = width.saturating_sub(used);
            }
            w = w.max(1).min(width.saturating_sub(used));
            used += w;
            let fill = if self.theme.color {
                self.theme.block_char().repeat(w)
            } else {
                // Without color, the first letter tells bands apart.
                category_legend(c.category)
                    .chars()
                    .next()
                    .unwrap_or('#')
                    .to_string()
                    .repeat(w)
            };
            spans.push(Span::styled(fill, self.theme.category(c.category)));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), bar);
        let mut lspans = vec![Span::raw(" ")];
        for c in &shown {
            lspans.push(Span::styled(
                self.theme.block_char().to_owned(),
                self.theme.category(c.category),
            ));
            lspans.push(Span::styled(
                format!(" {}   ", category_legend(c.category)),
                self.theme.muted(),
            ));
        }
        f.render_widget(Paragraph::new(Line::from(lspans)), legend);
    }

    fn overview(&self, f: &mut Frame, area: Rect) {
        let cats = self.app.categories();
        if self.app.summary.is_none() {
            let msg = match self.app.scanning {
                Some(n) => format!(
                    "Scanning… {} files so far. Nothing is changed.",
                    super::super::cmd::scan::compact_count(n)
                ),
                None => "No scan yet. Press r to scan.".to_owned(),
            };
            f.render_widget(
                Paragraph::new(msg)
                    .style(self.theme.soft())
                    .block(self.block()),
                area,
            );
            return;
        }
        let [strip, _, main] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(3),
        ])
        .areas(area);
        self.strip(f, strip, &cats);
        let left_w = if area.width >= 100 {
            52
        } else {
            (area.width * 45 / 100).max(30)
        };
        let [left, right] = Layout::horizontal([Constraint::Length(left_w), Constraint::Min(20)])
            .spacing(1)
            .areas(main);
        self.categories(f, left, &cats);
        let explain_h = if self.app.show_explain && right.height >= 12 {
            6
        } else {
            0
        };
        let [list, explain] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(explain_h)]).areas(right);
        let title = self
            .app
            .current_category()
            .map(|c| c.label())
            .unwrap_or("Items");
        self.item_list(
            f,
            list,
            title,
            self.app.item_idx,
            self.app.focus == Focus::Items,
        );
        if explain_h > 0 {
            self.explain(f, explain);
        }
    }

    fn categories(&self, f: &mut Frame, area: Rect, cats: &[CategorySummary]) {
        let inner_w = area.width.saturating_sub(2) as usize;
        let wide = inner_w >= 48;
        let name_w = if wide {
            17
        } else {
            inner_w.saturating_sub(13).max(8)
        };
        let mut lines = Vec::new();
        let header = if wide {
            format!(
                " {}{}{}{}",
                pad("Category", 17),
                lpad("Items", 6),
                lpad("Reclaimable", 14),
                lpad("Safe", 10)
            )
        } else {
            format!(" {}{}", pad("Category", name_w), lpad("Reclaimable", 12))
        };
        lines.push(Line::styled(header, self.theme.muted()));
        let body_h = area.height.saturating_sub(4) as usize;
        for (i, c) in cats.iter().enumerate().take(body_h) {
            let current = i == self.app.cat_idx;
            let pointer = if current { self.theme.pointer() } else { " " };
            let text = if wide {
                format!(
                    "{pointer}{}{}{}{}",
                    pad(c.category.label(), 17),
                    lpad(&c.items.to_string(), 6),
                    lpad(&self.size(c.reclaimable), 14),
                    lpad(&self.size_or_dash(c.by_safety.safe), 10)
                )
            } else {
                format!(
                    "{pointer}{}{}",
                    pad(c.category.label(), name_w),
                    lpad(&self.size(c.reclaimable), 12)
                )
            };
            let style = if current && self.app.focus == Focus::Categories {
                self.theme.selected_row()
            } else if current {
                self.theme.bright()
            } else {
                self.theme.text()
            };
            lines.push(Line::styled(pad(&text, inner_w), style));
        }
        let pad_lines = body_h.saturating_sub(cats.len().min(body_h));
        for _ in 0..pad_lines {
            lines.push(Line::raw(""));
        }
        let items: u64 = cats.iter().map(|c| c.items).sum();
        lines.push(Line::styled(
            format!(" {} categories   {} items", cats.len(), items),
            self.theme.muted(),
        ));
        f.render_widget(Paragraph::new(lines).block(self.block()), area);
    }

    fn item_list(&self, f: &mut Frame, area: Rect, title: &str, cursor: usize, focused: bool) {
        let items = self.app.visible_items();
        let inner_w = area.width.saturating_sub(2) as usize;
        let size_w = 10;
        let safety_w = 9;
        let name_w = inner_w.saturating_sub(5 + size_w + safety_w);
        let mut lines = vec![Line::styled(
            format!(
                " {}{}   {}",
                pad(title, name_w + 4),
                lpad("Size", size_w),
                "Safety"
            ),
            self.theme.muted(),
        )];
        let body_h = area.height.saturating_sub(3) as usize;
        if items.is_empty() {
            let msg = if self.app.filter.is_empty() {
                "Nothing here."
            } else {
                "No items match the filter."
            };
            lines.push(Line::styled(format!(" {msg}"), self.theme.soft()));
        }
        let start = cursor.saturating_sub(body_h.saturating_sub(1));
        for (i, item) in items.iter().enumerate().skip(start).take(body_h) {
            let on = self.app.selected.contains(&item.id);
            let row_style = if i == cursor && focused {
                self.theme.selected_row()
            } else {
                Style::new()
            };
            let check_style = if on {
                self.theme.checked()
            } else {
                self.theme.muted()
            };
            let spans = vec![
                Span::styled(
                    format!(" {} ", self.theme.check(on)),
                    check_style.patch(row_style),
                ),
                Span::styled(pad(&item.title, name_w), self.theme.text().patch(row_style)),
                Span::styled(
                    lpad(&self.size(item.size.reclaimable), size_w),
                    self.theme.text().patch(row_style),
                ),
                Span::styled("   ", row_style),
                Span::styled(
                    pad(item.safety.as_str(), safety_w - 3),
                    self.theme.safety(item.safety).patch(row_style),
                ),
            ];
            lines.push(Line::from(spans));
        }
        f.render_widget(Paragraph::new(lines).block(self.block()), area);
    }

    fn explain(&self, f: &mut Frame, area: Rect) {
        let Some(item) = self.app.current_item() else {
            f.render_widget(Paragraph::new("").block(self.block()), area);
            return;
        };
        let mut lines = vec![Line::from(vec![
            Span::styled(
                format!("{}   {}   ", item.title, self.size(item.size.reclaimable)),
                self.theme.bright(),
            ),
            Span::styled(item.safety.as_str(), self.theme.safety(item.safety)),
        ])];
        for r in item.reasons.iter().take(2) {
            lines.push(Line::styled(r.text.clone(), self.theme.soft()));
        }
        let action = match item.actions.first() {
            Some(a) if a.undoable() => {
                format!("Action: {a} (undo for {} days)", self.app.keep_days)
            }
            Some(a) => format!("Action: {a} (no undo)"),
            None => "kipple won't act on this item.".to_owned(),
        };
        lines.push(Line::styled(action, self.theme.muted()));
        let block = self
            .block()
            .padding(ratatui::widgets::Padding::horizontal(1));
        f.render_widget(
            Paragraph::new(lines)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .block(block),
            area,
        );
    }

    fn projects(&self, f: &mut Frame, area: Rect) {
        if let Some(id) = &self.app.project_open {
            let name = self
                .app
                .projects
                .iter()
                .find(|p| &p.id == id)
                .map(|p| p.name.clone())
                .unwrap_or_default();
            let explain_h = if self.app.show_explain && area.height >= 14 {
                6
            } else {
                0
            };
            let [list, explain] =
                Layout::vertical([Constraint::Min(3), Constraint::Length(explain_h)]).areas(area);
            self.item_list(f, list, &name, self.app.item_idx, true);
            if explain_h > 0 {
                self.explain(f, explain);
            }
            return;
        }
        let projects = self.app.visible_projects();
        let inner_w = area.width.saturating_sub(2) as usize;
        let name_w = (inner_w / 2).clamp(16, 44);
        let mut lines = vec![Line::styled(
            format!(
                " {}{}{}   {}",
                pad("Project", name_w),
                pad("Last used", 12),
                lpad("Reclaimable", 12),
                "Breakdown"
            ),
            self.theme.muted(),
        )];
        if projects.is_empty() {
            lines.push(Line::styled(
                " No projects hold reclaimable items.",
                self.theme.soft(),
            ));
        }
        let body_h = area.height.saturating_sub(3) as usize;
        let start = self
            .app
            .project_idx
            .saturating_sub(body_h.saturating_sub(1));
        for (i, p) in projects.iter().enumerate().skip(start).take(body_h) {
            let mut parts: Vec<(u64, String)> = p
                .by_category
                .iter()
                .filter(|(_, b)| **b > 0)
                .map(|(c, b)| (*b, format!("{} {}", category_legend(*c), self.size(*b))))
                .collect();
            parts.sort_by_key(|x| std::cmp::Reverse(x.0));
            let label = if p.exists {
                self.short(&p.root)
            } else {
                format!("{} (deleted)", self.short(&p.root))
            };
            let age = match &p.last_used {
                Some(lu) => format::age(Timestamp::now().as_second() - lu.at.as_second()),
                None => self.theme.dash().to_owned(),
            };
            let text = format!(
                " {}{}{}   {}",
                pad(&label, name_w),
                pad(&age, 12),
                lpad(&self.size(p.total.reclaimable), 12),
                parts
                    .into_iter()
                    .take(3)
                    .map(|x| x.1)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let style = if i == self.app.project_idx {
                self.theme.selected_row()
            } else {
                self.theme.text()
            };
            lines.push(Line::styled(pad(&text, inner_w), style));
        }
        f.render_widget(Paragraph::new(lines).block(self.block()), area);
    }

    fn ai(&self, f: &mut Frame, area: Rect) {
        let explain_h = if self.app.show_explain && area.height >= 14 {
            6
        } else {
            0
        };
        let [list, explain] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(explain_h)]).areas(area);
        if self.app.visible_items().is_empty() && self.app.filter.is_empty() {
            f.render_widget(
                Paragraph::new(" No AI tool history found.")
                    .style(self.theme.soft())
                    .block(self.block()),
                list,
            );
            return;
        }
        self.item_list(f, list, "Sessions", self.app.ai_idx, true);
        if explain_h > 0 {
            self.explain(f, explain);
        }
    }

    fn plan(&self, f: &mut Frame, area: Rect) {
        let Some(plan) = &self.app.plan else {
            f.render_widget(
                Paragraph::new(
                    " No plan yet. Select items with space on another screen, then press c.",
                )
                .style(self.theme.soft())
                .block(self.block()),
                area,
            );
            return;
        };
        let inner_w = area.width.saturating_sub(2) as usize;
        let mut lines = vec![Line::styled(
            format!(
                " Plan · {} items · reclaims {}",
                plan.totals.entries,
                self.size(plan.totals.reclaimable)
            ),
            self.theme.bright(),
        )];
        if plan.totals.freed_now > 0 {
            lines.push(Line::styled(
                format!(
                    " {} is freed right away by the tools that made it. That part can't be undone.",
                    self.size(plan.totals.freed_now)
                ),
                self.theme.soft(),
            ));
        }
        if plan.totals.held_in_quarantine > 0 {
            lines.push(Line::styled(
                format!(
                    " {} waits in quarantine for {} days so you can undo it, then it is deleted.",
                    self.size(plan.totals.held_in_quarantine),
                    self.app.keep_days
                ),
                self.theme.soft(),
            ));
        }
        lines.push(Line::raw(""));
        let size_w = 10;
        let name_w = inner_w.saturating_sub(size_w + 22);
        let body_h = area.height.saturating_sub(lines.len() as u16 + 4) as usize;
        let start = self.app.plan_idx.saturating_sub(body_h.saturating_sub(1));
        for (i, e) in plan.entries.iter().enumerate().skip(start).take(body_h) {
            let style = if i == self.app.plan_idx {
                self.theme.selected_row()
            } else {
                Style::new()
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {}", pad(e.item.safety.as_str(), 8)),
                    self.theme.safety(e.item.safety).patch(style),
                ),
                Span::styled(pad(&e.item.title, name_w), self.theme.text().patch(style)),
                Span::styled(
                    lpad(&self.size(e.item.size.reclaimable), size_w),
                    self.theme.text().patch(style),
                ),
                Span::styled(
                    format!("  {}", pad(&e.action.to_string(), 12)),
                    self.theme.muted().patch(style),
                ),
            ]));
        }
        for w in &plan.warnings {
            lines.push(Line::styled(format!(" {w}"), self.theme.muted()));
        }
        f.render_widget(Paragraph::new(lines).block(self.block()), area);
    }

    fn quarantine(&self, f: &mut Frame, area: Rect) {
        let inner_w = area.width.saturating_sub(2) as usize;
        let mut lines = vec![Line::styled(
            format!(
                " {}{}{}",
                lpad("Size", 10),
                pad("   Deleted in", 15),
                "Item"
            ),
            self.theme.muted(),
        )];
        if self.app.quarantine.is_empty() {
            lines.push(Line::styled(" Quarantine is empty.", self.theme.soft()));
        }
        let now = Timestamp::now().as_second();
        let body_h = area.height.saturating_sub(3) as usize;
        let start = self.app.q_idx.saturating_sub(body_h.saturating_sub(1));
        for (i, e) in self
            .app
            .quarantine
            .iter()
            .enumerate()
            .skip(start)
            .take(body_h)
        {
            let left = e.expires_at.as_second() - now;
            let left = if left <= 0 {
                "next run".to_owned()
            } else {
                format::age(left)
            };
            let from = e
                .original_paths
                .first()
                .map(|p| self.short(p))
                .unwrap_or_default();
            let text = format!(
                " {}   {}{}  {}",
                lpad(&self.size(e.size.on_disk), 10),
                pad(&left, 12),
                e.item.title,
                from
            );
            let style = if i == self.app.q_idx {
                self.theme.selected_row()
            } else {
                self.theme.text()
            };
            lines.push(Line::styled(pad(&text, inner_w), style));
        }
        f.render_widget(Paragraph::new(lines).block(self.block()), area);
    }

    fn status(&self, f: &mut Frame, area: Rect) {
        let app = self.app;
        let line = if app.filtering {
            Line::from(vec![
                Span::styled(" / ", self.theme.key()),
                Span::styled(app.filter.clone(), self.theme.bright()),
            ])
        } else if let PlanState::Confirm { input } = &app.plan_state {
            let word = if app.plan.as_ref().is_some_and(Plan::has_review) {
                "reclaim"
            } else {
                "y"
            };
            Line::from(vec![
                Span::styled(
                    format!(" Type {word} and press Enter to go ahead: "),
                    self.theme.soft(),
                ),
                Span::styled(input.clone(), self.theme.bright()),
            ])
        } else if let PlanState::Running { done, total } = &app.plan_state {
            Line::styled(
                format!(" Working… {done} of {total} done. Nothing else runs until it finishes."),
                self.theme.soft(),
            )
        } else if let PlanState::Finished(r) = &app.plan_state {
            Line::styled(
                format!(
                    " Freed {} now, {} in quarantine. {} done, {} skipped, {} failed. Enter rescans.",
                    self.size(r.freed_now),
                    self.size(r.held_in_quarantine),
                    r.done,
                    r.skipped,
                    r.failed
                ),
                self.theme.soft(),
            )
        } else if let Some(m) = &app.message {
            Line::styled(format!(" {m}"), self.theme.soft())
        } else {
            let left = vec![
                Span::styled(" Selected ", self.theme.soft()),
                Span::styled(self.size(app.selected_bytes()), self.theme.emphasis_size()),
                Span::styled(
                    match app.selected.len() {
                        1 => " in 1 item".to_owned(),
                        n => format!(" in {n} items"),
                    },
                    self.theme.soft(),
                ),
            ];
            let right = vec![
                Span::styled("press ", self.theme.soft()),
                Span::styled("c", self.theme.key()),
                Span::styled(" to make a plan ", self.theme.soft()),
            ];
            let used: usize = left
                .iter()
                .chain(right.iter())
                .map(|s| s.content.chars().count())
                .sum();
            let mut spans = left;
            spans.push(Span::raw(
                " ".repeat((area.width as usize).saturating_sub(used)),
            ));
            spans.extend(right);
            Line::from(spans)
        };
        f.render_widget(Paragraph::new(line), area);
    }

    fn keys(&self, f: &mut Frame, area: Rect) {
        let [rule, row] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
        f.render_widget(
            Paragraph::new(self.theme.rule_char().repeat(area.width as usize))
                .style(self.theme.rule()),
            rule,
        );
        let keys: &[(&str, &str)] = match self.app.screen {
            Screen::Plan => &[
                ("enter", "confirm"),
                ("j/k", "move"),
                ("esc", "back"),
                ("?", "help"),
                ("q", "quit"),
            ],
            Screen::Quarantine => &[
                ("u", "undo last clean"),
                ("r", "restore"),
                ("j/k", "move"),
                ("?", "help"),
                ("q", "quit"),
            ],
            Screen::Projects if self.app.project_open.is_none() => &[
                ("enter", "open"),
                ("/", "filter"),
                ("s", "sort"),
                ("?", "help"),
                ("q", "quit"),
            ],
            _ => &[
                ("space", "select"),
                ("a", "all safe"),
                ("c", "make plan"),
                ("?", "help"),
                ("q", "quit"),
                ("e", "explain"),
                ("/", "filter"),
                ("s", "sort"),
            ],
        };
        // Hints that don't fit are left out; `?` lists them all.
        let gap = if area.width < 100 { "  " } else { "   " };
        let mut spans = vec![Span::raw(" ")];
        let mut used = 1;
        for (k, l) in keys {
            let w = k.chars().count() + l.chars().count() + 1 + gap.len();
            if used + w > area.width as usize {
                break;
            }
            used += w;
            spans.push(Span::styled(*k, self.theme.key()));
            spans.push(Span::styled(format!(" {l}{gap}"), self.theme.muted()));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), row);
    }

    fn help(&self, f: &mut Frame, area: Rect) {
        let rows = [
            ("1-5", "switch screens"),
            ("j/k or arrows", "move"),
            ("h/l or tab", "categories or items"),
            ("space", "select an item"),
            ("a", "select all safe items here"),
            ("enter", "open, or confirm a plan"),
            ("e", "show or hide the explanation"),
            ("c", "make a plan from the selection"),
            ("/", "filter"),
            ("s", "sort by size, age or name"),
            ("r", "scan again (restore on Quarantine)"),
            ("u", "undo the last clean (Quarantine)"),
            ("q", "back, then quit"),
        ];
        let w = 52.min(area.width);
        let h = (rows.len() as u16 + 4).min(area.height);
        let rect = Rect::new(
            area.x + (area.width - w) / 2,
            area.y + (area.height - h) / 2,
            w,
            h,
        );
        let lines: Vec<Line> = rows
            .iter()
            .map(|(k, d)| {
                Line::from(vec![
                    Span::styled(format!(" {}", pad(k, 16)), self.theme.key()),
                    Span::styled(*d, self.theme.soft()),
                ])
            })
            .collect();
        f.render_widget(Clear, rect);
        f.render_widget(
            Paragraph::new(lines).style(self.theme.base()).block(
                self.block()
                    .title(Span::styled(" Keys ", self.theme.bright())),
            ),
            rect,
        );
    }
}
