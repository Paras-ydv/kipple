//! Share card renderer (SPEC §14, UI.md §5): a 1200×630 PNG from an SVG
//! built here, rendered with `resvg` and the bundled Mona Sans fonts (SIL
//! OFL). No system fonts are loaded, so output is the same everywhere.

use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};

use kipple_core::config::Units;
use kipple_core::model::Category;
use resvg::tiny_skia;
use resvg::usvg;

pub const WIDTH: u32 = 1200;
pub const HEIGHT: u32 = 630;

const REGULAR: &[u8] = include_bytes!("../assets/fonts/MonaSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../assets/fonts/MonaSans-Bold.ttf");
const EXTRA_BOLD: &[u8] = include_bytes!("../assets/fonts/MonaSans-ExtraBold.ttf");
const EXPANDED: &[u8] = include_bytes!("../assets/fonts/MonaSansExpanded-ExtraBold.ttf");

#[derive(Debug, thiserror::Error)]
pub enum ShareError {
    #[error("couldn't build the card: {0}")]
    Svg(String),
    #[error("couldn't render the card")]
    Render,
    #[error("couldn't encode the PNG: {0}")]
    Png(String),
}

/// What the card shows. No paths or usernames unless `projects` is filled
/// (`--with-projects`, folder names only).
#[derive(Debug, Clone, PartialEq)]
pub struct CardData {
    /// Reclaimable bytes found.
    pub total: u64,
    /// Freed by a clean in the last 24 hours, for the before/after variant.
    pub freed: Option<u64>,
    /// Bytes per category, any order.
    pub categories: Vec<(Category, u64)>,
    /// "Mac", "PC" or "machine".
    pub device: String,
    /// Project folder names (opt-in).
    pub projects: Vec<String>,
    pub units: Units,
    pub url: String,
}

/// Board palette.
fn color(c: Category) -> &'static str {
    match c {
        Category::Container => "#2F6BFF",
        Category::AiHistory => "#8B5CF6",
        Category::PackageCache => "#FFB020",
        Category::ProjectArtifact | Category::BuildCache => "#3CCB5A",
        Category::Mobile => "#12C8C2",
        Category::Model => "#FF5C8A",
        Category::Toolchain => "#C97F6B",
        Category::IdeCache | Category::Logs | Category::System => "#7B8591",
    }
}

fn chip_label(c: Category) -> &'static str {
    match c {
        Category::Container => "Docker",
        Category::AiHistory => "AI chat history",
        Category::PackageCache => "caches",
        Category::ProjectArtifact => "dependencies and builds",
        Category::BuildCache => "build caches",
        Category::Mobile => "emulators and simulators",
        Category::Model => "models",
        Category::Toolchain => "old toolchains",
        Category::IdeCache => "editor caches",
        Category::Logs => "logs",
        Category::System => "system files",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Text width in px from real glyph advances (skrifa, as resvg uses).
fn text_width(font: &[u8], text: &str, size: f32, letter_spacing: f32) -> f32 {
    use skrifa::MetadataProvider;
    use skrifa::instance::{LocationRef, Size};
    let Ok(face) = skrifa::FontRef::new(font) else {
        return text.chars().count() as f32 * size * 0.56;
    };
    let charmap = face.charmap();
    let metrics = face.glyph_metrics(Size::new(size), LocationRef::default());
    let advance: f32 = text
        .chars()
        .map(|c| {
            charmap
                .map(c)
                .and_then(|g| metrics.advance_width(g))
                .unwrap_or(size * 0.5)
        })
        .sum();
    advance + letter_spacing * text.chars().count().saturating_sub(1) as f32
}

/// Greedy word wrap to `max` px.
fn wrap(font: &[u8], text: &str, size: f32, spacing: f32, max: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if text_width(font, &candidate, size, spacing) > max && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            line = word.to_owned();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn size(b: u64, units: Units) -> String {
    kipple_core::format::size(b, units)
}

/// The card as SVG.
pub fn render_svg(data: &CardData) -> String {
    let mut cats: Vec<(Category, u64)> = data
        .categories
        .iter()
        .copied()
        .filter(|(_, b)| *b > 0)
        .collect();
    cats.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut s = String::new();
    let _ = write!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}">
<defs>
  <radialGradient id="glowA" cx="0.78" cy="0.30" r="0.55"><stop offset="0" stop-color="#2F6BFF" stop-opacity="0.38"/><stop offset="1" stop-color="#2F6BFF" stop-opacity="0"/></radialGradient>
  <radialGradient id="glowB" cx="0.92" cy="0.85" r="0.5"><stop offset="0" stop-color="#8B5CF6" stop-opacity="0.32"/><stop offset="1" stop-color="#8B5CF6" stop-opacity="0"/></radialGradient>
  <radialGradient id="glowC" cx="0.05" cy="1.0" r="0.6"><stop offset="0" stop-color="#12C8C2" stop-opacity="0.14"/><stop offset="1" stop-color="#12C8C2" stop-opacity="0"/></radialGradient>
  <linearGradient id="glass" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#FFFFFF" stop-opacity="0.12"/><stop offset="1" stop-color="#FFFFFF" stop-opacity="0.04"/></linearGradient>
  <linearGradient id="sheen" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#FFFFFF" stop-opacity="0.30"/><stop offset="0.45" stop-color="#FFFFFF" stop-opacity="0.04"/><stop offset="1" stop-color="#FFFFFF" stop-opacity="0"/></linearGradient>
</defs>
<rect width="{WIDTH}" height="{HEIGHT}" fill="#06080F"/>
<rect width="{WIDTH}" height="{HEIGHT}" fill="url(#glowA)"/>
<rect width="{WIDTH}" height="{HEIGHT}" fill="url(#glowB)"/>
<rect width="{WIDTH}" height="{HEIGHT}" fill="url(#glowC)"/>
"##
    );

    // The stack: one glass slab per category, heights by size.
    let (x, w, top, bottom, gap) = (790.0_f32, 300.0_f32, 70.0_f32, 560.0_f32, 8.0_f32);
    let shown: Vec<&(Category, u64)> = cats.iter().take(7).collect();
    let total: u64 = shown.iter().map(|(_, b)| *b).sum();
    let avail = bottom - top - gap * shown.len().saturating_sub(1) as f32;
    let mut y = top;
    for (i, (c, b)) in shown.iter().enumerate() {
        let h = if total == 0 {
            0.0
        } else {
            (*b as f32 / total as f32 * avail).max(18.0)
        };
        let h = if i + 1 == shown.len() {
            (bottom - y).max(18.0)
        } else {
            h
        };
        let r = 16.0_f32.min(h / 2.0);
        let col = color(*c);
        let _ = writeln!(
            s,
            r##"<g><rect x="{x}" y="{y:.1}" width="{w}" height="{h:.1}" rx="{r:.1}" fill="{col}" fill-opacity="0.34"/><rect x="{x}" y="{y:.1}" width="{w}" height="{h:.1}" rx="{r:.1}" fill="url(#glass)" stroke="#FFFFFF" stroke-opacity="0.18"/><rect x="{x}" y="{y:.1}" width="{w}" height="{h:.1}" rx="{r:.1}" fill="url(#sheen)" opacity="0.6"/><line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y1:.1}" stroke="#FFFFFF" stroke-opacity="0.4"/></g>"##,
            x1 = x + r,
            x2 = x + w - r,
            y1 = y + 0.5,
        );
        y += h + gap;
    }

    // Wordmark.
    let _ = writeln!(
        s,
        r##"<text x="64" y="88" font-family="Mona Sans Expanded" font-weight="800" font-size="26" letter-spacing="-0.78" fill="#F4F5F8">kipple</text>"##
    );

    // Headline, wrapped to 600 px.
    let total_text = size(data.total, data.units);
    let headline = match data.freed {
        Some(f) => format!(
            "I just freed {} of kipple on my {}.",
            size(f, data.units),
            data.device
        ),
        None => format!("My {} was hiding {total_text} of kipple.", data.device),
    };
    let (hs, hspace) = (64.0_f32, -2.56_f32);
    let mut lines = wrap(EXTRA_BOLD, &headline, hs, hspace, 600.0);
    lines.truncate(4);
    for (i, l) in lines.iter().enumerate() {
        let ly = 196.0 + i as f32 * hs * 1.02;
        let _ = writeln!(
            s,
            r##"<text x="64" y="{ly:.1}" font-family="Mona Sans" font-weight="800" font-size="{hs}" letter-spacing="{hspace}" fill="#F4F5F8">{}</text>"##,
            esc(l)
        );
    }

    // Chips: top three categories.
    let (cy, ch, fs) = (482.0_f32, 40.0_f32, 17.0_f32);
    let mut cx = 64.0_f32;
    for (c, b) in cats.iter().take(3) {
        let num = size(*b, data.units);
        let label = chip_label(*c);
        let num_w = text_width(BOLD, &num, fs, 0.0);
        let label_w = text_width(REGULAR, label, fs, 0.0);
        let cw = 14.0 + 10.0 + 9.0 + num_w + 5.0 + label_w + 14.0;
        if cx + cw > 700.0 {
            break;
        }
        let ty = cy + ch / 2.0 + fs * 0.35;
        let _ = writeln!(
            s,
            r##"<g><rect x="{cx:.1}" y="{cy}" width="{cw:.1}" height="{ch}" rx="18" fill="url(#glass)" stroke="#FFFFFF" stroke-opacity="0.18"/><line x1="{l1:.1}" y1="{cy1:.1}" x2="{l2:.1}" y2="{cy1:.1}" stroke="#FFFFFF" stroke-opacity="0.4"/><circle cx="{dx:.1}" cy="{dy:.1}" r="5" fill="{col}"/><text x="{tx:.1}" y="{ty:.1}" font-family="Mona Sans" font-size="{fs}" fill="#F4F5F8"><tspan font-weight="700">{num}</tspan><tspan font-weight="400" dx="5">{label}</tspan></text></g>"##,
            l1 = cx + 18.0,
            l2 = cx + cw - 18.0,
            cy1 = cy + 0.5,
            dx = cx + 14.0 + 5.0,
            dy = cy + ch / 2.0,
            col = color(*c),
            tx = cx + 14.0 + 10.0 + 9.0,
            num = esc(&num),
            label = esc(label),
        );
        cx += cw + 10.0;
    }

    // Opt-in project names.
    if !data.projects.is_empty() {
        let names: Vec<&str> = data.projects.iter().take(4).map(String::as_str).collect();
        let _ = writeln!(
            s,
            r##"<text x="64" y="458" font-family="Mona Sans" font-size="15" fill="#FFFFFF" fill-opacity="0.62">Heaviest: {}</text>"##,
            esc(&names.join(", "))
        );
    }

    // Footer.
    let _ = write!(
        s,
        r##"<text x="64" y="562" font-family="Mona Sans" font-size="16"><tspan fill="#FFFFFF" fill-opacity="0.62">Found with kipple, an open-source disk cleaner for developers. </tspan><tspan fill="#FFFFFF">{}</tspan></text>
</svg>
"##,
        esc(&data.url)
    );
    s
}

fn fontdb() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        for font in [REGULAR, BOLD, EXTRA_BOLD, EXPANDED] {
            db.load_font_data(font.to_vec());
        }
        db.set_sans_serif_family("Mona Sans");
        Arc::new(db)
    })
    .clone()
}

/// The card as PNG bytes.
pub fn render_png(data: &CardData) -> Result<Vec<u8>, ShareError> {
    let svg = render_svg(data);
    let opt = usvg::Options {
        fontdb: fontdb(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg, &opt).map_err(|e| ShareError::Svg(e.to_string()))?;
    let mut pixmap = tiny_skia::Pixmap::new(WIDTH, HEIGHT).ok_or(ShareError::Render)?;
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap
        .encode_png()
        .map_err(|e| ShareError::Png(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> CardData {
        CardData {
            total: 184_000_000_000,
            freed: None,
            categories: vec![
                (Category::Container, 61_200_000_000),
                (Category::AiHistory, 38_900_000_000),
                (Category::PackageCache, 27_500_000_000),
                (Category::ProjectArtifact, 24_000_000_000),
                (Category::Mobile, 21_600_000_000),
                (Category::Model, 10_800_000_000),
            ],
            device: "Mac".into(),
            projects: vec![],
            units: Units::Si,
            url: "github.com/Paras-ydv/kipple".into(),
        }
    }

    #[test]
    fn bundled_fonts_have_the_expected_families() {
        let db = fontdb();
        let families: std::collections::BTreeSet<String> = db
            .faces()
            .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
            .collect();
        assert!(families.contains("Mona Sans"), "{families:?}");
        assert!(families.contains("Mona Sans Expanded"), "{families:?}");
    }

    #[test]
    fn headline_wraps_like_the_board() {
        let lines = wrap(
            EXTRA_BOLD,
            "My Mac was hiding 184 GB of kipple.",
            64.0,
            -2.56,
            600.0,
        );
        assert_eq!(lines, vec!["My Mac was hiding", "184 GB of kipple."]);
    }

    #[test]
    fn svg_has_no_paths_or_usernames_by_default() {
        let svg = render_svg(&sample());
        assert!(svg.contains("My Mac was hiding"));
        assert!(svg.contains("61.2 GB"));
        assert!(svg.contains(">Docker<"));
        assert!(!svg.contains("Heaviest"));
        assert!(!svg.contains("/Users/"));
    }

    #[test]
    fn png_is_deterministic() {
        let a = render_png(&sample()).unwrap();
        let b = render_png(&sample()).unwrap();
        assert_eq!(a, b);
        assert_eq!(&a[1..4], b"PNG");
    }

    /// The SVG is the deterministic source of the card; rasterizing can
    /// differ by a few bits between CPU architectures.
    #[test]
    fn svg_snapshot() {
        insta::assert_snapshot!(render_svg(&sample()));
    }

    #[test]
    #[ignore = "writes the sample card to $KIPPLE_CARD_OUT for a visual check"]
    fn write_sample() {
        if let Some(out) = std::env::var_os("KIPPLE_CARD_OUT") {
            std::fs::write(out, render_png(&sample()).unwrap()).unwrap();
        }
    }

    #[test]
    fn freed_variant() {
        let mut d = sample();
        d.freed = Some(76_500_000_000);
        let svg = render_svg(&d);
        let text: String = svg
            .split('<')
            .filter_map(|part| part.split_once('>').map(|(_, t)| t.trim()))
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            text.contains("I just freed 76.5 GB of kipple on my Mac."),
            "{text}"
        );
    }
}
