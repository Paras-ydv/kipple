//! Scan benchmark on a generated tree (SPEC §5.3, §22 Q1).
//!
//! `cargo bench -p kipple-core --bench walk -- [entries] [runs]`
//!
//! Builds a tree of roughly `entries` files and folders shaped like a home
//! folder full of projects, then runs the full scan path (walk, matching,
//! sizing) and a walk-only pass, and prints p50 and p90.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use kipple_core::config::Config;
use kipple_core::dirs::Dirs;
use kipple_core::integration::{Adapter, ArtifactRule, Ctx, IntegrationError, ItemDraft, ItemSink};
use kipple_core::model::*;
use kipple_core::platform::{Env, Platform, ProcessSnapshot};
use kipple_core::scan::walker::WalkOptions;
use kipple_core::scan::{self, ScanInput};

/// Artifact rules similar in cost to the built-in packs: named rules with
/// sibling checks plus the two generic `*` rules that stat every folder.
struct BenchRules;

impl Adapter for BenchRules {
    fn info(&self) -> IntegrationInfo {
        IntegrationInfo {
            id: IntegrationId::from("bench"),
            name: "bench".into(),
            kind: IntegrationKind::Pack,
            category: Category::ProjectArtifact,
            description: String::new(),
            docs_url: None,
            status: IntegrationStatus::Stable,
            os: vec![],
            in_use_processes: vec![],
        }
    }

    fn detect(&self, _: &Ctx) -> Detection {
        Detection::present()
    }

    fn artifact_rules(&self) -> Vec<ArtifactRule> {
        let rule = |dir: &str, sib: &[&str], inside: &[&str], tag: bool, prio: i32| ArtifactRule {
            integration: IntegrationId::from("bench"),
            dir: dir.into(),
            requires_sibling_any: sib.iter().map(|s| s.to_string()).collect(),
            requires_sibling_all: vec![],
            requires_inside_any: inside.iter().map(|s| s.to_string()).collect(),
            requires_gitignored: false,
            requires_cachedir_tag: tag,
            template: ItemDraft::new("k", "{name}", Locator::path("/")),
            priority: prio,
        };
        vec![
            rule("node_modules", &["package.json"], &[], false, 100),
            rule("target", &["Cargo.toml"], &[], false, 100),
            rule("{.next,.turbo,.vite}", &["package.json"], &[], false, 100),
            rule(
                "{__pycache__,.pytest_cache,.mypy_cache}",
                &[],
                &[],
                false,
                100,
            ),
            rule("*", &[], &["pyvenv.cfg"], false, 800),
            rule("*", &[], &[], true, 1000),
        ]
    }

    fn discover(&self, _: &Ctx, _: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        Ok(())
    }
}

fn write(p: &Path, n: usize) {
    std::fs::write(p, vec![b'x'; n]).expect("write");
}

/// ~250 entries per project: sources, git-ish folders, node_modules.
fn generate(root: &Path, entries: usize) -> usize {
    let projects = (entries / 250).max(1);
    let mut count = 0;
    for p in 0..projects {
        let dir = root.join(format!("code/group{}/proj{p}", p % 40));
        std::fs::create_dir_all(dir.join("src/components")).expect("mkdir");
        write(&dir.join("package.json"), 40);
        count += 4;
        for f in 0..30 {
            write(&dir.join(format!("src/components/c{f}.tsx")), 200);
        }
        count += 30;
        for pkg in 0..20 {
            let pd = dir.join(format!("node_modules/pkg{pkg}/lib"));
            std::fs::create_dir_all(&pd).expect("mkdir");
            write(&pd.parent().expect("parent").join("package.json"), 50);
            for f in 0..8 {
                write(&pd.join(format!("f{f}.js")), 300);
            }
            count += 11;
        }
        // Folders the walk must descend through (docs, assets).
        for d in 0..4 {
            let dd = dir.join(format!("assets/a{d}"));
            std::fs::create_dir_all(&dd).expect("mkdir");
            for f in 0..2 {
                write(&dd.join(format!("img{f}.png")), 100);
            }
            count += 3;
        }
    }
    count
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx]
}

fn main() {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();
    let entries: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(500_000);
    let runs: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(5);

    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().canonicalize().expect("canonical");
    let t = Instant::now();
    let made = generate(&root, entries);
    eprintln!(
        "generated {made} entries in {:.1}s",
        t.elapsed().as_secs_f64()
    );

    let env = Env::from_pairs([
        ("HOME", root.as_os_str()),
        ("KIPPLE_TEST_SANDBOX", root.as_os_str()),
    ]);
    let platform = Platform::new(env.clone());
    let config = Config::default();
    let dirs = Dirs::resolve(&env, platform.os).expect("dirs");
    let procs = ProcessSnapshot::default();

    for mode in ["scan", "walk"] {
        let integrations: Vec<Arc<dyn Adapter>> = if mode == "scan" {
            vec![Arc::new(BenchRules)]
        } else {
            vec![]
        };
        let mut times = Vec::new();
        let mut items = 0;
        let mut seen = 0;
        // One warm-up run, then measured runs.
        for run in 0..=runs {
            let cancel = CancelToken::new();
            let ctx = Ctx::new(&platform, &config, &dirs, &cancel, &procs);
            let t = Instant::now();
            let out = scan::run(
                &ctx,
                ScanInput {
                    integrations: integrations.clone(),
                    roots: vec![root.clone()],
                    exclude: vec![],
                    skip: vec![],
                    walk: WalkOptions::default(),
                },
                &NoopSink::default(),
            )
            .expect("scan");
            if run > 0 {
                times.push(t.elapsed());
            }
            items = out.items.len();
            seen = out.entries_seen;
        }
        times.sort();
        println!(
            "{mode}  p50 {:>7.1} ms   p90 {:>7.1} ms   entries walked {seen:>8}   items {items}",
            percentile(&times, 0.5).as_secs_f64() * 1000.0,
            percentile(&times, 0.9).as_secs_f64() * 1000.0,
        );
    }
}
