//! Human output (SPEC §11.3): tables, sizes, ages, safety words, colors.

use kipple_core::config::Units;
use kipple_core::model::{Category, LastUsed, Safety, Timestamp};
use kipple_core::platform::paths;

/// Styling for one run. Colors only when enabled; meaning never depends on
/// them (safety is always written as a word).
#[derive(Debug, Clone, Copy)]
pub struct Style {
    pub color: bool,
    pub units: Units,
}

const RESET: &str = "\x1b[0m";

impl Style {
    fn paint(&self, code: &str, s: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{s}{RESET}")
        } else {
            s.to_owned()
        }
    }

    pub fn dim(&self, s: &str) -> String {
        self.paint("2", s)
    }

    pub fn bold(&self, s: &str) -> String {
        self.paint("1", s)
    }

    /// Safety colors from UI.md §2.3 (dark theme values).
    pub fn safety(&self, s: Safety) -> String {
        let code = match s {
            Safety::Safe => "38;2;111;192;143",
            Safety::Review => "38;2;230;167;64",
            Safety::Keep => "38;2;229;122;112",
        };
        self.paint(code, s.as_str())
    }

    pub fn size(&self, bytes: u64) -> String {
        kipple_core::format::size(bytes, self.units)
    }

    /// Size or an em dash for zero.
    pub fn size_or_dash(&self, bytes: u64) -> String {
        if bytes == 0 {
            "—".to_owned()
        } else {
            self.size(bytes)
        }
    }
}

pub fn age_of(lu: Option<&LastUsed>) -> String {
    match lu {
        Some(lu) => kipple_core::format::age(Timestamp::now().as_second() - lu.at.as_second()),
        None => "—".to_owned(),
    }
}

pub fn short_path(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    paths::shorten_home(path, home)
}

/// Short label for project breakdowns: "deps", "docker", "ai", …
pub fn category_short(c: Category) -> &'static str {
    match c {
        Category::ProjectArtifact => "deps",
        Category::PackageCache => "caches",
        Category::Toolchain => "toolchains",
        Category::Container => "docker",
        Category::Mobile => "mobile",
        Category::AiHistory => "ai",
        Category::Model => "models",
        Category::IdeCache => "ide",
        Category::BuildCache => "build",
        Category::Logs => "logs",
        Category::System => "system",
    }
}

/// Breakdown value: unitless GB like the spec ("docker 8.9") from 0.1 GB,
/// a full size below that.
pub fn compact_gb(bytes: u64, style: &Style) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 10.0 {
        format!("{gb:.0}")
    } else if gb >= 0.1 {
        format!("{gb:.1}")
    } else {
        style.size(bytes)
    }
}

/// Visible width, ignoring ANSI escapes.
fn width(s: &str) -> usize {
    let mut w = 0;
    let mut in_esc = false;
    for c in s.chars() {
        match (in_esc, c) {
            (false, '\x1b') => in_esc = true,
            (true, 'm') => in_esc = false,
            (true, _) => {}
            (false, _) => w += 1,
        }
    }
    w
}

/// Truncates to `max` characters with an ellipsis.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// A simple aligned table. Header in sentence case, dimmed.
pub struct Table {
    headers: Vec<(String, Align)>,
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new(headers: &[(&str, Align)]) -> Self {
        Self {
            headers: headers.iter().map(|(h, a)| ((*h).to_owned(), *a)).collect(),
            rows: Vec::new(),
        }
    }

    pub fn row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    pub fn render(&self, style: &Style) -> String {
        let n = self.headers.len();
        let mut widths: Vec<usize> = self.headers.iter().map(|(h, _)| width(h)).collect();
        for r in &self.rows {
            for (i, c) in r.iter().enumerate().take(n) {
                widths[i] = widths[i].max(width(c));
            }
        }
        let fmt_row = |cells: Vec<String>| -> String {
            let mut line = String::new();
            for (i, c) in cells.iter().enumerate().take(n) {
                let pad = widths[i].saturating_sub(width(c));
                let last = i + 1 == n;
                match self.headers[i].1 {
                    Align::Left if last => line.push_str(c),
                    Align::Left => {
                        line.push_str(c);
                        line.push_str(&" ".repeat(pad));
                    }
                    Align::Right => {
                        line.push_str(&" ".repeat(pad));
                        line.push_str(c);
                    }
                }
                if !last {
                    line.push_str("   ");
                }
            }
            line.trim_end().to_owned()
        };
        let mut out = String::new();
        let header: Vec<String> = self.headers.iter().map(|(h, _)| h.clone()).collect();
        out.push_str(&style.dim(&fmt_row(header)));
        out.push('\n');
        for r in &self.rows {
            out.push_str(&fmt_row(r.clone()));
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_aligns_columns() {
        let style = Style {
            color: false,
            units: Units::Si,
        };
        let mut t = Table::new(&[("Name", Align::Left), ("Size", Align::Right)]);
        t.row(vec!["a".into(), "1.00 GB".into()]);
        t.row(vec!["longer".into(), "12 B".into()]);
        assert_eq!(
            t.render(&style),
            "Name        Size\na        1.00 GB\nlonger      12 B\n"
        );
    }

    #[test]
    fn width_ignores_ansi() {
        let style = Style {
            color: true,
            units: Units::Si,
        };
        assert_eq!(width(&style.safety(Safety::Safe)), 4);
    }

    #[test]
    fn truncates() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
    }
}
