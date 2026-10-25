//! The generic pack engine: turns a [`Pack`] into an [`Adapter`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use kipple_core::integration::{Adapter, ArtifactRule, Ctx, IntegrationError, ItemDraft, ItemSink};
use kipple_core::model::{
    ActionKind, Category, Check, Detection, IntegrationId, IntegrationInfo, IntegrationKind, Item,
    LastUsed, LastUsedSource, Locator, Os, Outcome, ReasonCode, Safety, Timestamp,
};
use kipple_core::platform::{Env, os, paths};
use kipple_core::safety::reasons::{reason, reason_with};

use crate::enrich;
use crate::schema::{Each, Measure, Pack};

/// A loaded pack.
#[derive(Debug, Clone)]
pub struct PackAdapter {
    pub pack: Pack,
}

/// Resolved `{var}` values.
pub type Vars = BTreeMap<String, PathBuf>;

impl PackAdapter {
    pub fn new(pack: Pack) -> Self {
        Self { pack }
    }

    fn os_value(def: &crate::schema::VarDef, os: Os) -> Option<&str> {
        let v = match os {
            Os::Macos => def.macos.as_deref(),
            Os::Linux | Os::Wsl => def.linux.as_deref(),
            Os::Windows => def.windows.as_deref(),
        };
        v.or(def.default.as_deref())
    }

    /// Resolves vars: `command` → `env` → OS key → `default`.
    pub fn resolve_vars(&self, ctx: &Ctx) -> Vars {
        self.resolve_vars_with(&ctx.platform.env, ctx.platform.os, |argv| {
            ctx.command_path(argv)
        })
    }

    pub fn resolve_vars_with(
        &self,
        env: &Env,
        os: Os,
        run: impl Fn(&[String]) -> Option<PathBuf>,
    ) -> Vars {
        let mut vars = Vars::new();
        for (name, def) in &self.pack.vars {
            let append = |p: PathBuf| match &def.append {
                Some(a) => p.join(a),
                None => p,
            };
            let from_cmd = def.command.as_ref().and_then(|argv| run(argv)).map(append);
            let from_env = || {
                def.env
                    .as_ref()
                    .and_then(|e| env.get_path(e))
                    .map(|p| append(paths::normalize_separators(&p.to_string_lossy())))
            };
            let from_os =
                || Self::os_value(def, os).and_then(|s| paths::expand(s, env, &vars).ok());
            if let Some(p) = from_cmd.or_else(from_env).or_else(from_os) {
                vars.insert(name.clone(), p);
            }
        }
        vars
    }

    fn info_category(&self) -> Category {
        self.pack.category
    }

    /// Location base paths that exist (globs expanded).
    fn resolved_locations(&self, ctx: &Ctx, vars: &Vars) -> Vec<(usize, PathBuf)> {
        let mut out = Vec::new();
        for (idx, loc) in self.pack.locations.iter().enumerate() {
            if !os::os_matches(&loc.os, ctx.platform.os, ctx.platform.wsl) {
                continue;
            }
            let Ok(expanded) = paths::expand(&loc.path, &ctx.platform.env, vars) else {
                continue;
            };
            for p in expand_glob(&expanded) {
                out.push((idx, p));
            }
        }
        out
    }

    /// Parses an action list, dropping native actions whose program is
    /// missing. Returns the actions and missing tool names.
    fn actions(&self, ctx: &Ctx, list: &[String]) -> (Vec<ActionKind>, Vec<String>) {
        let mut actions = Vec::new();
        let mut missing = Vec::new();
        for a in list {
            let Ok(action) = a.parse::<ActionKind>() else {
                continue;
            };
            if let ActionKind::Native { command_id } = &action {
                let Some(cmd) = self.pack.commands.get(command_id) else {
                    continue;
                };
                let program = cmd.run.first().map(String::as_str).unwrap_or_default();
                if ctx.platform.runner.which(program).is_none() {
                    missing.push(program.to_owned());
                    continue;
                }
            }
            actions.push(action);
        }
        (actions, missing)
    }

    fn command_argv(&self, ctx: &Ctx, command_id: &str, item: &Item) -> Option<Vec<String>> {
        let cmd = self.pack.commands.get(command_id)?;
        let mut vars = self.resolve_vars(ctx);
        if let Some(p) = item.locator.primary_path() {
            vars.insert("path".into(), p.clone());
        }
        cmd.run
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                if i == 0 {
                    Some(arg.clone())
                } else {
                    substitute_vars(arg, &vars)
                }
            })
            .collect()
    }
}

/// Replaces `{var}` in a command argument. Unknown vars fail.
fn substitute_vars(arg: &str, vars: &Vars) -> Option<String> {
    let mut out = String::new();
    let mut rest = arg;
    while let Some(start) = rest.find('{') {
        let end = rest[start..].find('}')? + start;
        out.push_str(&rest[..start]);
        let name = &rest[start + 1..end];
        out.push_str(&vars.get(name)?.to_string_lossy());
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

fn has_glob(s: &str) -> bool {
    s.contains(['*', '?', '['])
}

/// Expands a path whose components may contain globs, one level at a time.
/// Returns existing paths only.
pub fn expand_glob(path: &Path) -> Vec<PathBuf> {
    let s = path.to_string_lossy();
    if !has_glob(&s) {
        return if path.symlink_metadata().is_ok() {
            vec![path.to_path_buf()]
        } else {
            vec![]
        };
    }
    let mut current: Vec<PathBuf> = vec![PathBuf::new()];
    for comp in path.components() {
        let c = comp.as_os_str().to_string_lossy();
        if has_glob(&c) {
            let Ok(glob) = globset::Glob::new(&c) else {
                return vec![];
            };
            let m = glob.compile_matcher();
            let mut next = Vec::new();
            for base in &current {
                if let Ok(rd) = std::fs::read_dir(base) {
                    let mut names: Vec<PathBuf> = rd
                        .flatten()
                        .filter(|e| m.is_match(e.file_name()))
                        .map(|e| e.path())
                        .collect();
                    names.sort();
                    next.extend(names);
                }
            }
            current = next;
        } else {
            for p in &mut current {
                p.push(comp.as_os_str());
            }
        }
    }
    current.retain(|p| p.symlink_metadata().is_ok());
    current
}

fn mtime(p: &Path) -> Option<SystemTime> {
    p.symlink_metadata().and_then(|m| m.modified()).ok()
}

impl Adapter for PackAdapter {
    fn info(&self) -> IntegrationInfo {
        IntegrationInfo {
            id: IntegrationId::from(self.pack.id.as_str()),
            name: self.pack.name.clone(),
            kind: IntegrationKind::Pack,
            category: self.pack.category,
            description: self.pack.description.clone(),
            docs_url: self.pack.docs_url.clone(),
            status: self.pack.status,
            os: self.pack.os.clone(),
            in_use_processes: self.pack.in_use_processes.clone(),
        }
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        let d = &self.pack.detect;
        if d.any_command.is_empty() && d.any_path.is_empty() {
            if self.pack.locations.is_empty() {
                return Detection::present();
            }
            let vars = self.resolve_vars(ctx);
            return if self.resolved_locations(ctx, &vars).is_empty() {
                Detection::NotPresent
            } else {
                Detection::present()
            };
        }
        if d.any_command
            .iter()
            .any(|c| ctx.platform.runner.which(c).is_some())
        {
            return Detection::present();
        }
        let vars = self.resolve_vars(ctx);
        let any_path = d.any_path.iter().any(|p| {
            paths::expand(p, &ctx.platform.env, &vars)
                .map(|p| !expand_glob(&p).is_empty())
                .unwrap_or(false)
        });
        if any_path {
            Detection::present()
        } else {
            Detection::NotPresent
        }
    }

    fn artifact_rules(&self) -> Vec<ArtifactRule> {
        let id = IntegrationId::from(self.pack.id.as_str());
        self.pack
            .artifacts
            .iter()
            .map(|a| {
                let title = a.title.clone().unwrap_or_else(|| "{name}".to_owned());
                let mut t = ItemDraft::new(a.kind.clone(), title.clone(), Locator::path("/"));
                t.category = Some(self.info_category());
                t.safety = a.safety;
                t.reasons = a.reasons.iter().map(|c| reason(*c)).collect();
                // Native tool checks need a context; artifact actions are
                // core actions in practice (quarantine/trash).
                t.actions = a.actions.iter().filter_map(|s| s.parse().ok()).collect();
                t.restore_hint = a.restore_hint.clone();
                t.age_rule = self.pack.category == Category::ProjectArtifact;
                t.aggregate = a.aggregate.map(|_| title.replace("{name}", &a.kind));
                t.safety_outside_project = a.safety_outside_project;
                let generic = a.dir == "*";
                ArtifactRule {
                    integration: id.clone(),
                    dir: a.dir.clone(),
                    requires_sibling_any: a.requires_sibling_any.clone(),
                    requires_sibling_all: a.requires_sibling_all.clone(),
                    requires_inside_any: a.requires_inside_any.clone(),
                    requires_gitignored: a.requires_gitignored,
                    requires_cachedir_tag: a.requires_cachedir_tag,
                    template: t,
                    priority: a.priority.unwrap_or(if generic { 900 } else { 100 }),
                }
            })
            .collect()
    }

    fn locations(&self, ctx: &Ctx) -> Vec<PathBuf> {
        let vars = self.resolve_vars(ctx);
        self.resolved_locations(ctx, &vars)
            .into_iter()
            .map(|(_, p)| p)
            .collect()
    }

    fn tools(&self) -> Vec<String> {
        let mut t: Vec<String> = self
            .pack
            .commands
            .values()
            .filter_map(|c| c.run.first().cloned())
            .chain(self.pack.detect.any_command.iter().cloned())
            .collect();
        t.sort();
        t.dedup();
        t
    }

    fn discover(&self, ctx: &Ctx, sink: &mut dyn ItemSink) -> Result<(), IntegrationError> {
        if self.pack.locations.is_empty() {
            return Ok(());
        }
        let vars = self.resolve_vars(ctx);
        for (idx, base) in self.resolved_locations(ctx, &vars) {
            let loc = &self.pack.locations[idx];
            let (actions, missing) = self.actions(ctx, &loc.actions);
            let min_size = loc
                .min_size
                .as_deref()
                .and_then(|s| kipple_core::format::parse_size(s).ok());
            let targets: Vec<PathBuf> = match loc.each {
                Each::Itself => vec![base.clone()],
                Each::Child => {
                    let mut children: Vec<PathBuf> = std::fs::read_dir(&base)
                        .map(|rd| rd.flatten().map(|e| e.path()).collect())
                        .unwrap_or_default();
                    children.sort();
                    children
                }
            };
            // Newest children by mtime, for keep_latest.
            let mut newest: Vec<PathBuf> = Vec::new();
            if let (Each::Child, Some(n)) = (loc.each, loc.keep_latest) {
                let mut by_time: Vec<(Option<SystemTime>, PathBuf)> =
                    targets.iter().map(|p| (mtime(p), p.clone())).collect();
                by_time.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                newest = by_time
                    .into_iter()
                    .take(n as usize)
                    .map(|(_, p)| p)
                    .collect();
            }
            for path in targets {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let parent = path
                    .parent()
                    .and_then(Path::file_name)
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut d = ItemDraft::new(
                    loc.kind.clone(),
                    loc.title
                        .replace("{name}", &name)
                        .replace("{parent}", &parent),
                    Locator::path(&path),
                );
                d.category = Some(self.pack.category);
                d.safety = loc.safety;
                d.reasons = loc.reasons.iter().map(|c| reason(*c)).collect();
                d.actions = actions.clone();
                d.restore_hint = loc.restore_hint.clone();
                d.min_size = min_size;
                if let Some(t) = mtime(&path).and_then(|t| Timestamp::try_from(t).ok()) {
                    d.last_used = Some(LastUsed {
                        at: t,
                        source: LastUsedSource::ItemMtime,
                    });
                }
                if newest.contains(&path) {
                    d.safety = Safety::Keep;
                    d.reasons.insert(
                        0,
                        kipple_core::model::Reason {
                            code: ReasonCode::RecentlyUsed,
                            text: "This is the newest version, which your tools use.".to_owned(),
                        },
                    );
                    d.actions.clear();
                }
                if d.actions.is_empty() && d.safety != Safety::Keep {
                    d.safety = Safety::Keep;
                    let tool = missing
                        .first()
                        .cloned()
                        .unwrap_or_else(|| self.pack.name.clone());
                    d.reasons.insert(
                        0,
                        reason_with(ReasonCode::NativeToolMissing, &[("tool", tool)]),
                    );
                }
                sink.item(d);
            }
        }
        Ok(())
    }

    fn enrich(&self, draft: &mut ItemDraft, path: &Path) {
        enrich::enrich(&self.pack.id, draft, path);
    }

    fn preflight(&self, ctx: &Ctx, _item: &Item, action: &ActionKind) -> Vec<Check> {
        let ActionKind::Native { command_id } = action else {
            return Vec::new();
        };
        let Some(cmd) = self.pack.commands.get(command_id) else {
            return vec![Check::fail(
                "command",
                format!("unknown command `{command_id}`"),
            )];
        };
        let mut checks = Vec::new();
        let program = cmd.run.first().cloned().unwrap_or_default();
        checks.push(match ctx.platform.runner.which(&program) {
            Some(_) => Check::pass("tool", format!("{program} is installed")),
            None => Check::fail("tool", format!("{program} isn't installed")),
        });
        let procs = ctx.platform.processes.snapshot();
        for p in &cmd.requires_closed {
            if procs.any_running(p) {
                let name = p.split('|').next().unwrap_or(p);
                checks.push(Check::fail("closed", format!("Quit {name} first.")));
            }
        }
        checks
    }

    fn apply(
        &self,
        ctx: &Ctx,
        item: &Item,
        action: &ActionKind,
    ) -> Result<Outcome, IntegrationError> {
        let ActionKind::Native { command_id } = action else {
            return Err(IntegrationError::Unsupported(format!(
                "packs only run native commands, not `{action}`"
            )));
        };
        let cmd =
            self.pack.commands.get(command_id).ok_or_else(|| {
                IntegrationError::other(format!("unknown command `{command_id}`"))
            })?;
        let argv = self
            .command_argv(ctx, command_id, item)
            .ok_or_else(|| IntegrationError::other("command uses an unknown variable"))?;
        let paths: Vec<PathBuf> = item.locator.paths().into_iter().cloned().collect();
        let before = (cmd.measure == Some(Measure::BeforeAfter)).then(|| {
            kipple_core::scan::sizes::measure(&paths, ctx.cancel)
                .finish()
                .0
                .on_disk
        });
        let out = ctx
            .platform
            .runner
            .run(&argv, None, Duration::from_secs(cmd.timeout_secs))?;
        if !out.success() {
            let msg = out
                .stderr
                .lines()
                .chain(out.stdout.lines())
                .find(|l| !l.trim().is_empty())
                .unwrap_or("no output")
                .trim()
                .to_owned();
            return Ok(Outcome::Failed {
                error: format!("`{}` failed: {msg}", argv.join(" ")),
            });
        }
        let freed = match before {
            Some(b) => {
                let after = kipple_core::scan::sizes::measure(&paths, ctx.cancel)
                    .finish()
                    .0
                    .on_disk;
                b.saturating_sub(after)
            }
            None => item.size.reclaimable,
        };
        Ok(Outcome::Done { freed })
    }

    fn describe_action(&self, action: &ActionKind) -> Option<String> {
        let ActionKind::Native { command_id } = action else {
            return None;
        };
        let cmd = self.pack.commands.get(command_id)?;
        Some(format!(
            "Runs `{}`, the tool's own cleanup. This can't be undone, but the tool downloads what it needs again.",
            kipple_core::platform::command::display_argv(&cmd.run)
        ))
    }

    fn opt_in(&self) -> bool {
        self.pack.opt_in
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_and_validate;

    fn npm() -> PackAdapter {
        let text = crate::builtin_sources()
            .iter()
            .find(|(f, _)| *f == "npm-cache.toml")
            .map(|(_, t)| *t)
            .unwrap();
        PackAdapter::new(parse_and_validate(text).unwrap())
    }

    fn home_env() -> Env {
        Env::from_pairs([
            ("HOME", "/h"),
            ("USERPROFILE", "/h"),
            ("LOCALAPPDATA", "/h/AppData/Local"),
        ])
    }

    #[test]
    fn os_default_when_nothing_else() {
        let p = npm();
        let no_cmd = |_: &[String]| None;
        let lin = p.resolve_vars_with(&home_env(), Os::Linux, no_cmd);
        assert_eq!(
            lin["npm_cache"],
            paths::normalize_separators("/h/.npm/_cacache")
        );
        let win = p.resolve_vars_with(&home_env(), Os::Windows, no_cmd);
        assert_eq!(
            win["npm_cache"],
            paths::normalize_separators("/h/AppData/Local/npm-cache/_cacache")
        );
    }

    #[test]
    fn env_beats_os_default_and_gets_appended() {
        let mut env = home_env();
        env.set("npm_config_cache", "/custom/npm");
        let vars = npm().resolve_vars_with(&env, Os::Macos, |_: &[String]| None);
        assert_eq!(
            vars["npm_cache"],
            PathBuf::from("/custom/npm").join("_cacache")
        );
    }

    #[test]
    fn command_beats_everything() {
        let mut env = home_env();
        env.set("npm_config_cache", "/custom/npm");
        let vars = npm().resolve_vars_with(&env, Os::Linux, |argv: &[String]| {
            assert_eq!(argv, ["npm", "config", "get", "cache"]);
            Some(PathBuf::from("/from/command"))
        });
        assert_eq!(
            vars["npm_cache"],
            PathBuf::from("/from/command").join("_cacache")
        );
    }

    #[test]
    fn substitutes_command_vars() {
        let vars = Vars::from([("cache".to_owned(), PathBuf::from("/c"))]);
        assert_eq!(
            substitute_vars("--dir={cache}", &vars).as_deref(),
            Some("--dir=/c")
        );
        assert_eq!(substitute_vars("{nope}", &vars), None);
    }

    #[cfg(unix)]
    #[test]
    fn native_action_runs_mock_and_measures() {
        use kipple_core::testing::TestHome;
        let home = TestHome::new().unwrap();
        let cache = home.join(".npm/_cacache");
        std::fs::create_dir_all(cache.join("x")).unwrap();
        std::fs::write(cache.join("x/blob"), vec![0u8; 20_000]).unwrap();
        home.mock_command(
            "npm",
            &format!(
                "if [ \"$1\" = cache ]; then rm -rf '{}/x'; exit 0; fi\nexit 1",
                cache.display()
            ),
        )
        .unwrap();
        let platform = kipple_core::platform::Platform::new(home.env().clone());
        let config = kipple_core::config::Config::default();
        let dirs = kipple_core::dirs::Dirs::resolve(home.env(), platform.os).unwrap();
        let cancel = kipple_core::model::CancelToken::new();
        let procs = kipple_core::platform::ProcessSnapshot::default();
        let ctx = Ctx::new(&platform, &config, &dirs, &cancel, &procs);
        let mut drafts: Vec<ItemDraft> = Vec::new();
        npm().discover(&ctx, &mut drafts).unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].actions[0], ActionKind::native("npm-cache-clean"));
        let integration = IntegrationId::from("npm-cache");
        let item = Item {
            id: kipple_core::model::ItemId::new(&integration, &drafts[0].locator),
            integration,
            category: Category::PackageCache,
            kind: drafts[0].kind.clone(),
            title: drafts[0].title.clone(),
            locator: drafts[0].locator.clone(),
            size: kipple_core::model::Size::default(),
            project: None,
            last_used: None,
            safety: Safety::Safe,
            reasons: vec![],
            actions: drafts[0].actions.clone(),
            details: serde_json::Value::Null,
            notes: vec![],
            restore_hint: None,
            identity: None,
        };
        let outcome = npm()
            .apply(&ctx, &item, &ActionKind::native("npm-cache-clean"))
            .unwrap();
        match outcome {
            Outcome::Done { freed } => assert!(freed >= 20_000, "freed {freed}"),
            other => panic!("unexpected {other:?}"),
        }
        assert!(!cache.join("x").exists());
    }
}
