//! Colors and glyphs for the TUI, from the Terminal UI design board. Color
//! never carries meaning on its own: safety is always written as a word.

use kipple_core::model::{Category, Safety};
use ratatui::style::{Color, Modifier, Style};

const fn hex(rgb: u32) -> Color {
    Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

pub const BG: Color = hex(0x1A1F26);
pub const FG: Color = hex(0xD9DEE3);
pub const BRIGHT: Color = hex(0xE9ECEF);
pub const MUTED: Color = hex(0x8E99A6);
pub const SOFT: Color = hex(0xA3ADB8);
pub const BORDER: Color = hex(0x3A4350);
pub const RULE: Color = hex(0x323A45);
pub const SELECT_BG: Color = hex(0x2E3742);
pub const SAFE: Color = hex(0x6FC08F);
pub const REVIEW: Color = hex(0xE6A740);
pub const KEEP: Color = hex(0xE57A70);

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub color: bool,
    pub ascii: bool,
}

impl Theme {
    fn fg(&self, c: Color) -> Style {
        if self.color {
            Style::new().fg(c)
        } else {
            Style::new()
        }
    }

    pub fn base(&self) -> Style {
        if self.color {
            Style::new().fg(FG).bg(BG)
        } else {
            Style::new()
        }
    }

    pub fn text(&self) -> Style {
        self.fg(FG)
    }

    pub fn bright(&self) -> Style {
        self.fg(BRIGHT).add_modifier(Modifier::BOLD)
    }

    pub fn muted(&self) -> Style {
        self.fg(MUTED)
    }

    pub fn soft(&self) -> Style {
        self.fg(SOFT)
    }

    pub fn border(&self) -> Style {
        self.fg(BORDER)
    }

    pub fn rule(&self) -> Style {
        self.fg(RULE)
    }

    /// Inverted bar at the top.
    pub fn header(&self) -> Style {
        if self.color {
            Style::new().fg(BG).bg(BRIGHT).add_modifier(Modifier::BOLD)
        } else {
            Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        }
    }

    pub fn active_tab(&self) -> Style {
        self.fg(BRIGHT)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    }

    /// Highlighted row.
    pub fn selected_row(&self) -> Style {
        if self.color {
            Style::new().bg(SELECT_BG).fg(Color::White)
        } else {
            Style::new().add_modifier(Modifier::REVERSED)
        }
    }

    pub fn safety(&self, s: Safety) -> Style {
        self.fg(match s {
            Safety::Safe => SAFE,
            Safety::Review => REVIEW,
            Safety::Keep => KEEP,
        })
    }

    pub fn checked(&self) -> Style {
        self.fg(SAFE)
    }

    pub fn key(&self) -> Style {
        self.fg(BRIGHT).add_modifier(Modifier::BOLD)
    }

    pub fn emphasis_size(&self) -> Style {
        self.fg(SAFE).add_modifier(Modifier::BOLD)
    }

    /// Mineral color per category, from the board.
    pub fn category(&self, c: Category) -> Style {
        self.fg(category_color(c))
    }

    pub fn pointer(&self) -> &'static str {
        if self.ascii { ">" } else { "▸" }
    }

    pub fn block_char(&self) -> &'static str {
        if self.ascii { "#" } else { "█" }
    }

    pub fn check(&self, on: bool) -> &'static str {
        if on { "[x]" } else { "[ ]" }
    }

    pub fn dash(&self) -> &'static str {
        if self.ascii { "-" } else { "—" }
    }

    pub fn border_set(&self) -> ratatui::symbols::border::Set<'static> {
        if self.ascii {
            ratatui::symbols::border::Set {
                top_left: "+",
                top_right: "+",
                bottom_left: "+",
                bottom_right: "+",
                vertical_left: "|",
                vertical_right: "|",
                horizontal_top: "-",
                horizontal_bottom: "-",
            }
        } else {
            ratatui::symbols::border::ROUNDED
        }
    }

    pub fn rule_char(&self) -> &'static str {
        if self.ascii { "-" } else { "─" }
    }
}

pub fn category_color(c: Category) -> Color {
    match c {
        Category::Container => hex(0x2F6BFF),
        Category::AiHistory => hex(0x8B5CF6),
        Category::PackageCache => hex(0xFFB020),
        Category::ProjectArtifact | Category::BuildCache => hex(0x3CCB5A),
        Category::Mobile => hex(0x12C8C2),
        Category::Model => hex(0xFF5C8A),
        Category::Toolchain => hex(0xC97F6B),
        Category::IdeCache | Category::Logs | Category::System => hex(0x7B8591),
    }
}

/// Short lowercase legend label.
pub fn category_legend(c: Category) -> &'static str {
    match c {
        Category::Container => "containers",
        Category::AiHistory => "ai history",
        Category::PackageCache => "caches",
        Category::ProjectArtifact => "artifacts",
        Category::BuildCache => "build caches",
        Category::Mobile => "mobile",
        Category::Model => "models",
        Category::Toolchain => "toolchains",
        Category::IdeCache => "ide caches",
        Category::Logs => "logs",
        Category::System => "system",
    }
}

/// UTF-8 unless `KIPPLE_ASCII=1` or the locale says otherwise.
pub fn wants_ascii(env: &kipple_core::platform::Env) -> bool {
    if env.get_str("KIPPLE_ASCII").is_some_and(|v| v != "0") {
        return true;
    }
    let locale = env
        .get_str("LC_ALL")
        .or_else(|| env.get_str("LC_CTYPE"))
        .or_else(|| env.get_str("LANG"));
    match locale {
        Some(l) => {
            let l = l.to_ascii_lowercase();
            !(l.contains("utf-8") || l.contains("utf8")) && l != "c.utf-8"
        }
        None => false,
    }
}
