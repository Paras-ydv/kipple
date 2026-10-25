//! Share card renderer (SPEC §14, UI.md §5): a 1200×630 PNG.
//!
//! The SVG is a port of the reference implementation
//! `packages/design/src/share/shareCard.ts` (and the stack geometry in
//! `packages/design/src/stack.ts`); keep the two in step. It is rendered
//! with `resvg` and the bundled static Mona Sans fonts (SIL OFL). No system
//! fonts are loaded, so the output doesn't depend on the machine.

use std::fmt::Write as _;
use std::sync::{Arc, OnceLock};

use kipple_core::model::Category;
use resvg::tiny_skia;
use resvg::usvg;

pub const WIDTH: u32 = 1200;
pub const HEIGHT: u32 = 630;
pub const URL: &str = "github.com/Paras-ydv/kipple";

const REGULAR: &[u8] = include_bytes!("../assets/fonts/MonaSans-Regular.ttf");
const BOLD: &[u8] = include_bytes!("../assets/fonts/MonaSans-Bold.ttf");
const DISPLAY: &[u8] = include_bytes!("../assets/fonts/MonaSansDisplaySemiExpanded-ExtraBold.ttf");
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// "My Mac was hiding 184 GB of kipple."
    Found,
    /// "I just reclaimed 76.5 GB of kipple." (a clean in the last 24 hours)
    Reclaimed,
}

/// What the card shows. Never usernames or paths (UI.md §5.5).
#[derive(Debug, Clone, PartialEq)]
pub struct CardData {
    pub variant: Variant,
    /// Total found, or total freed.
    pub total: u64,
    /// "Mac", "PC" or "computer".
    pub device: String,
    /// Bytes found per category, or bytes freed per category.
    pub categories: Vec<(Category, u64)>,
    /// Reclaimed variant: bytes still left after the clean.
    pub remaining: u64,
    /// `--with-projects`: project folder names and sizes, shown instead of
    /// the category chips.
    pub projects: Vec<(String, u64)>,
}

/// Colour and chip label per category (`packages/design/src/tokens.ts`).
fn token(c: Category) -> (&'static str, &'static str) {
    match c {
        Category::Container => ("#2F6BFF", "Docker"),
        Category::AiHistory => ("#8B5CF6", "AI chat history"),
        Category::PackageCache => ("#FFB020", "caches"),
        Category::ProjectArtifact => ("#3CCB5A", "build output"),
        Category::Mobile => ("#12C8C2", "mobile"),
        Category::Model => ("#FF5C8A", "models"),
        Category::Toolchain => ("#FF7A45", "toolchains"),
        Category::BuildCache => ("#A3D635", "build caches"),
        Category::IdeCache => ("#4CC9F0", "IDE caches"),
        Category::Logs => ("#B0B8C8", "logs"),
        Category::System => ("#7B8591", "system"),
    }
}

const GB: f64 = 1e9;
const MB: f64 = 1e6;

/// The design's byte format: "184 GB", "107.5 GB", "512 MB"
/// (`packages/design/src/format.ts`).
pub fn format_bytes(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= GB || (b / MB).round() >= 1000.0 {
        let gb = (b / GB * 10.0).round() / 10.0;
        if gb >= 100.0 && gb.fract() == 0.0 {
            format!("{gb:.0} GB")
        } else {
            format!("{gb:.1} GB")
        }
    } else if b >= MB {
        format!("{} MB", (b / MB).round())
    } else if b >= 1000.0 {
        format!("{} KB", (b / 1000.0).round())
    } else {
        format!("{bytes} bytes")
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Numbers as the reference prints them: up to two decimals, no trailing
/// zeros.
fn num(n: f64) -> String {
    let r = (n * 100.0).round() / 100.0;
    if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        let s = format!("{r:.2}");
        s.trim_end_matches('0').to_owned()
    }
}

/// Text width in px from the font's glyph advances (skrifa, as resvg uses).
fn text_width(font: &[u8], text: &str, size: f32) -> f32 {
    use skrifa::MetadataProvider;
    use skrifa::instance::{LocationRef, Size};
    let Ok(face) = skrifa::FontRef::new(font) else {
        return text.chars().count() as f32 * size * 0.56;
    };
    let charmap = face.charmap();
    let metrics = face.glyph_metrics(Size::new(size), LocationRef::default());
    text.chars()
        .map(|c| {
            charmap
                .map(c)
                .and_then(|g| metrics.advance_width(g))
                .unwrap_or(size * 0.5)
        })
        .sum()
}

pub fn headline(data: &CardData) -> (String, String) {
    let size = format_bytes(data.total);
    match data.variant {
        Variant::Found => (
            format!("My {} was hiding", data.device),
            format!("{size} of kipple."),
        ),
        Variant::Reclaimed => ("I just reclaimed".to_owned(), format!("{size} of kipple.")),
    }
}

// ---- Stack geometry (packages/design/src/stack.ts) -------------------------

struct Layer {
    color: String,
    value: f64,
    clear: bool,
}

struct Slab {
    color: String,
    clear: bool,
    top: f64,
    h: f64,
    d: f64,
    x: f64,
    w: f64,
    cx: f64,
    cy: f64,
}

struct Layout {
    slabs: Vec<Slab>,
    bottom: f64,
    d: f64,
    w: f64,
    cx: f64,
}

/// JavaScript's `Math.round`.
fn round_js(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn metrics(w: f64) -> (f64, f64, f64) {
    let d = round_js(w * 0.13);
    (d, round_js(d * 0.4), round_js(w * 0.06))
}

fn fit_heights(values: &[f64], avail: f64, hmin: f64) -> Vec<f64> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }
    if avail <= n as f64 * hmin {
        return vec![(avail / n as f64).max(0.0); n];
    }
    let mut fixed = vec![false; n];
    loop {
        let free_sum: f64 = values
            .iter()
            .zip(&fixed)
            .filter(|(_, f)| !**f)
            .map(|(v, _)| v)
            .sum();
        let fixed_count = fixed.iter().filter(|f| **f).count() as f64;
        let s = if free_sum > 0.0 {
            (avail - fixed_count * hmin) / free_sum
        } else {
            0.0
        };
        let mut changed = false;
        for (i, v) in values.iter().enumerate() {
            if !fixed[i] && v * s < hmin {
                fixed[i] = true;
                changed = true;
            }
        }
        if !changed {
            return values
                .iter()
                .zip(&fixed)
                .map(|(v, f)| if *f { hmin } else { v * s })
                .collect();
        }
    }
}

fn layout_stack(
    mut list: Vec<Layer>,
    cx: f64,
    w: f64,
    y0: f64,
    y1: f64,
    keep_order: bool,
) -> Layout {
    let (d, gap, hmin) = metrics(w);
    list.retain(|l| l.value > 0.0);
    if !keep_order {
        list.sort_by(|a, b| b.value.total_cmp(&a.value));
    }
    let n = list.len() as f64;
    let avail = y1 - y0 - n * d - (n - 1.0).max(0.0) * gap;
    let values: Vec<f64> = list.iter().map(|l| l.value).collect();
    let heights = fit_heights(&values, avail, hmin);
    let mut y = y1;
    let mut slabs = Vec::new();
    for (l, h) in list.into_iter().zip(heights) {
        let top = y - (d + h);
        slabs.push(Slab {
            color: l.color,
            clear: l.clear,
            top,
            h,
            d,
            x: cx - w / 2.0,
            w,
            cx,
            cy: top + (d + h) / 2.0,
        });
        y = top - gap;
    }
    Layout {
        slabs,
        bottom: y1,
        d,
        w,
        cx,
    }
}

fn parse_hex(hex: &str) -> (f64, f64, f64) {
    let n = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
    (
        ((n >> 16) & 255) as f64,
        ((n >> 8) & 255) as f64,
        (n & 255) as f64,
    )
}

fn mix(a: &str, b: &str, t: f64) -> (u8, u8, u8) {
    let (r1, g1, b1) = parse_hex(a);
    let (r2, g2, b2) = parse_hex(b);
    let c = |x: f64, y: f64| round_js(x + (y - x) * t) as u8;
    (c(r1, r2), c(g1, g2), c(b1, b2))
}

fn rgba((r, g, b): (u8, u8, u8), a: f64) -> String {
    format!("rgba({r},{g},{b},{})", num(a))
}

fn stack_defs(id: &str) -> String {
    format!(
        r##"<filter id="{id}-glow" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="18"/></filter><filter id="{id}-floor" x="-50%" y="-200%" width="200%" height="500%"><feGaussianBlur stdDeviation="10"/></filter><linearGradient id="{id}-streak" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".5"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>"##
    )
}

fn stack_svg(layout: &Layout, id: &str) -> String {
    let mut defs = String::new();
    let mut glows = String::new();
    let mut slabs = String::new();
    let floor = format!(
        r##"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="#000" fill-opacity=".45" filter="url(#{id}-floor)"/>"##,
        num(layout.cx),
        num(layout.bottom + layout.d * 0.35),
        num(layout.w * 0.7),
        num(layout.d * 0.35)
    );
    for (i, s) in layout.slabs.iter().enumerate() {
        let r = 14.0_f64.min(s.w * 0.08);
        let (top, bottom, stroke, lid_fill, lid_stroke) = if s.clear {
            (
                "rgba(255,255,255,0.08)".to_owned(),
                "rgba(255,255,255,0.04)".to_owned(),
                "rgba(255,255,255,0.3)",
                "rgba(255,255,255,0.14)".to_owned(),
                "rgba(255,255,255,0.3)",
            )
        } else {
            (
                rgba(mix(&s.color, "#FFFFFF", 0.2), 0.92),
                rgba(mix(&s.color, "#000000", 0.38), 0.92),
                "rgba(255,255,255,0.28)",
                rgba(mix(&s.color, "#FFFFFF", 0.45), 0.96),
                "rgba(255,255,255,0.55)",
            )
        };
        let gid = format!("{id}-b{i}");
        let _ = write!(
            defs,
            r##"<linearGradient id="{gid}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient>"##
        );
        if !s.clear {
            let _ = write!(
                glows,
                r##"<ellipse cx="{}" cy="{}" rx="{}" ry="{}" fill="{}" fill-opacity="0.28" filter="url(#{id}-glow)"/>"##,
                num(s.cx),
                num(s.cy),
                num(s.w * 0.55),
                num((s.d + s.h) * 0.6),
                s.color
            );
        }
        let _ = write!(
            slabs,
            r##"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="url(#{gid})" stroke="{stroke}" stroke-width="1"/><rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="{lid_fill}" stroke="{lid_stroke}" stroke-width="1"/>"##,
            num(s.x),
            num(s.top + s.d / 2.0),
            num(s.w),
            num(s.h + s.d / 2.0),
            num(r),
            num(s.x),
            num(s.top),
            num(s.w),
            num(s.d),
            num(s.d / 2.0),
        );
        let streak_h = (s.h - s.d / 2.0 - 10.0).max(0.0);
        if streak_h > 0.0 {
            let _ = write!(
                slabs,
                r##"<rect x="{}" y="{}" width="{}" height="{}" rx="2" fill="url(#{id}-streak)"/>"##,
                num(s.x + s.w * 0.06),
                num(s.top + s.d + 4.0),
                num((s.w * 0.025).max(3.0)),
                num(streak_h)
            );
        }
    }
    format!("<defs>{defs}</defs>{floor}{glows}{slabs}")
}

// ---- Card ------------------------------------------------------------------

/// The card as SVG (UI.md §5).
pub fn render_svg(data: &CardData) -> String {
    let id = "kc";
    let (line1, line2) = headline(data);

    // Headline steps down 4 px until both lines fit 600 px (minimum 48).
    let mut size = 64.0_f32;
    while size > 48.0
        && text_width(DISPLAY, &line1, size).max(text_width(DISPLAY, &line2, size)) > 600.0
    {
        size -= 4.0;
    }
    let base1 = 196.0_f32;
    let base2 = base1 + (size * 1.02).round();

    let mut cats: Vec<(Category, u64)> = data
        .categories
        .iter()
        .copied()
        .filter(|(_, b)| *b > 0)
        .collect();
    cats.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    // Chips: three largest categories, or project names with --with-projects.
    let chip_y = 496.0_f64;
    let mut x = 64.0_f64;
    let mut chips = String::new();
    let entries: Vec<(Option<&str>, String, String)> = if data.projects.is_empty() {
        cats.iter()
            .take(3)
            .map(|(c, b)| {
                let (color, label) = token(*c);
                (Some(color), format_bytes(*b), label.to_owned())
            })
            .collect()
    } else {
        data.projects
            .iter()
            .take(3)
            .map(|(name, b)| (None, format_bytes(*b), name.clone()))
            .collect()
    };
    for (color, value, label) in entries {
        let text_x = if color.is_some() { x + 33.0 } else { x + 14.0 };
        let w = (text_x - x)
            + text_width(BOLD, &value, 17.0) as f64
            + text_width(REGULAR, &format!(" {label}"), 17.0) as f64
            + 14.0;
        let _ = write!(
            chips,
            r##"<rect x="{}" y="{}" width="{w:.1}" height="42" rx="18" fill="url(#{id}-chip)" stroke="rgba(255,255,255,.18)" stroke-width="1"/><line x1="{}" y1="{}" x2="{:.1}" y2="{}" stroke="rgba(255,255,255,.4)" stroke-width="1"/>"##,
            num(x),
            num(chip_y),
            num(x + 18.0),
            num(chip_y + 1.0),
            x + w - 18.0,
            num(chip_y + 1.0),
        );
        if let Some(c) = color {
            let _ = write!(
                chips,
                r##"<circle cx="{}" cy="{}" r="5" fill="{c}"/>"##,
                num(x + 19.0),
                num(chip_y + 21.0)
            );
        }
        let _ = write!(
            chips,
            r##"<text x="{}" y="{}" font-family="'Mona Sans', sans-serif" font-size="17" fill="#F4F5F8"><tspan font-weight="700">{}</tspan><tspan font-weight="400"> {}</tspan></text>"##,
            num(text_x),
            num(chip_y + 27.0),
            esc(&value),
            esc(&label)
        );
        x += w + 10.0;
    }

    // Stack on the right.
    let mut layers: Vec<Layer> = cats
        .iter()
        .map(|(c, b)| Layer {
            color: token(*c).0.to_owned(),
            value: *b as f64,
            clear: false,
        })
        .collect();
    if data.variant == Variant::Reclaimed && data.remaining > 0 {
        layers.push(Layer {
            color: "#FFFFFF".into(),
            value: data.remaining as f64,
            clear: true,
        });
    }
    let layout = layout_stack(
        layers,
        930.0,
        240.0,
        70.0,
        556.0,
        data.variant == Variant::Reclaimed,
    );

    let glow = |cx: i32, cy: i32, r: i32, color: &str, op: &str, n: &str| {
        format!(
            r##"<radialGradient id="{id}-{n}" cx="{cx}" cy="{cy}" r="{r}" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="{color}" stop-opacity="{op}"/><stop offset="1" stop-color="{color}" stop-opacity="0"/></radialGradient>"##
        )
    };

    let mut s = String::new();
    let _ = write!(
        s,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}" viewBox="0 0 {WIDTH} {HEIGHT}" font-family="'Mona Sans', sans-serif"><defs>"##
    );
    s.push_str(&glow(930, 300, 460, "#347CFF", "0.42", "g1"));
    s.push_str(&glow(1120, 640, 420, "#B25CFF", "0.38", "g2"));
    s.push_str(&glow(760, 640, 360, "#FF8454", "0.26", "g3"));
    s.push_str(&glow(40, -40, 520, "#347CFF", "0.16", "g4"));
    let _ = write!(
        s,
        r##"<linearGradient id="{id}-chip" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fff" stop-opacity=".12"/><stop offset="1" stop-color="#fff" stop-opacity=".04"/></linearGradient>"##
    );
    s.push_str(&stack_defs(&format!("{id}-s")));
    let _ = write!(
        s,
        r##"</defs><rect width="{WIDTH}" height="{HEIGHT}" fill="#06080F"/>"##
    );
    for g in ["g1", "g2", "g3", "g4"] {
        let _ = write!(
            s,
            r##"<rect width="{WIDTH}" height="{HEIGHT}" fill="url(#{id}-{g})"/>"##
        );
    }
    let _ = write!(
        s,
        r##"<text x="64" y="86" font-family="'Mona Sans Expanded', 'Mona Sans', sans-serif" font-size="26" font-weight="800" font-stretch="125%" letter-spacing="-0.78" fill="#F4F5F8">kipple</text>"##
    );
    let _ = write!(
        s,
        r##"<text font-family="'Mona Sans Display SemiExpanded', 'Mona Sans', sans-serif" font-size="{}" font-weight="800" font-stretch="108%" letter-spacing="{:.2}" fill="#F4F5F8"><tspan x="64" y="{}">{}</tspan><tspan x="64" y="{}">{}</tspan></text>"##,
        num(size as f64),
        -0.04 * size,
        num(base1 as f64),
        esc(&line1),
        num(base2 as f64),
        esc(&line2)
    );
    s.push_str(&chips);
    let _ = write!(
        s,
        r##"<text x="64" y="574" font-size="16" font-weight="400" fill="rgba(255,255,255,.62)">Found with kipple, an open-source disk cleaner for developers. <tspan fill="#FFFFFF">{URL}</tspan></text>"##
    );
    s.push_str(&stack_svg(&layout, &format!("{id}-s")));
    s.push_str("</svg>");
    s
}

fn fontdb() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        for font in [REGULAR, BOLD, DISPLAY, EXPANDED] {
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
            variant: Variant::Found,
            total: 184_000_000_000,
            device: "Mac".into(),
            categories: vec![
                (Category::Container, 61_200_000_000),
                (Category::AiHistory, 38_900_000_000),
                (Category::PackageCache, 27_500_000_000),
                (Category::ProjectArtifact, 24_000_000_000),
                (Category::Mobile, 21_600_000_000),
                (Category::Model, 10_800_000_000),
            ],
            remaining: 0,
            projects: vec![],
        }
    }

    fn text_of(svg: &str) -> String {
        svg.split('<')
            .filter_map(|p| p.split_once('>').map(|(_, t)| t.trim()))
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn fonts_match_the_design_package() {
        let design =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/design/fonts");
        if !design.is_dir() {
            return; // Published crate: the design package isn't there.
        }
        for (name, bytes) in [
            ("MonaSans-Regular.ttf", REGULAR),
            ("MonaSans-Bold.ttf", BOLD),
            ("MonaSansDisplaySemiExpanded-ExtraBold.ttf", DISPLAY),
            ("MonaSansExpanded-ExtraBold.ttf", EXPANDED),
        ] {
            assert_eq!(
                std::fs::read(design.join(name)).unwrap(),
                bytes,
                "{name} differs"
            );
        }
    }

    #[test]
    fn bundled_fonts_have_the_families_the_svg_names() {
        let families: std::collections::BTreeSet<String> = fontdb()
            .faces()
            .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()))
            .collect();
        for f in [
            "Mona Sans",
            "Mona Sans Expanded",
            "Mona Sans Display SemiExpanded",
        ] {
            assert!(families.contains(f), "{f} missing from {families:?}");
        }
    }

    #[test]
    fn formats_like_the_design() {
        assert_eq!(format_bytes(184_000_000_000), "184 GB");
        assert_eq!(format_bytes(107_500_000_000), "107.5 GB");
        assert_eq!(format_bytes(24_000_000_000), "24.0 GB");
        assert_eq!(format_bytes(512_000_000), "512 MB");
    }

    #[test]
    fn stack_fills_the_area_largest_at_the_bottom() {
        let layers = vec![
            Layer {
                color: "#2F6BFF".into(),
                value: 61.2,
                clear: false,
            },
            Layer {
                color: "#FF5C8A".into(),
                value: 0.1,
                clear: false,
            },
            Layer {
                color: "#8B5CF6".into(),
                value: 38.9,
                clear: false,
            },
        ];
        let l = layout_stack(layers, 930.0, 240.0, 70.0, 556.0, false);
        assert_eq!(l.d, 31.0);
        assert_eq!(l.slabs[0].color, "#2F6BFF");
        assert!(
            (l.slabs[2].top - 70.0).abs() < 0.01,
            "top {}",
            l.slabs[2].top
        );
        assert_eq!(l.slabs[2].h, 14.0, "tiny layers get the minimum height");
    }

    #[test]
    fn found_card_text() {
        let text = text_of(&render_svg(&sample()));
        assert!(
            text.contains("My Mac was hiding 184 GB of kipple."),
            "{text}"
        );
        assert!(text.contains("61.2 GB Docker"));
        assert!(text.contains("38.9 GB AI chat history"));
        assert!(text.contains("27.5 GB caches"));
        assert!(!text.contains("build output"), "only three chips");
    }

    #[test]
    fn reclaimed_variant_and_projects() {
        let mut d = sample();
        d.variant = Variant::Reclaimed;
        d.total = 76_500_000_000;
        d.remaining = 107_500_000_000;
        d.projects = vec![("nomad-app".into(), 14_200_000_000)];
        let svg = render_svg(&d);
        let text = text_of(&svg);
        assert!(
            text.contains("I just reclaimed 76.5 GB of kipple."),
            "{text}"
        );
        assert!(text.contains("14.2 GB nomad-app"));
        assert!(
            svg.contains("rgba(255,255,255,0.14)"),
            "clear slab for what is left"
        );
    }

    #[test]
    fn png_is_deterministic() {
        let a = render_png(&sample()).unwrap();
        assert_eq!(a, render_png(&sample()).unwrap());
        assert_eq!(&a[1..4], b"PNG");
    }

    /// The SVG is the deterministic source; rasterizing can differ by a few
    /// bits between CPU architectures.
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
}
