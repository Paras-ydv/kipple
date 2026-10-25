//! Pack validation (INTEGRATIONS.md §2.6).

use std::collections::BTreeSet;

use kipple_core::model::ActionKind;

use crate::schema::{Each, Pack};

/// Parses and checks a pack. Returns every problem found.
pub fn parse_and_validate(text: &str) -> Result<Pack, Vec<String>> {
    let pack: Pack = toml::from_str(text).map_err(|e| vec![e.message().to_owned()])?;
    let errors = validate(&pack);
    if errors.is_empty() {
        Ok(pack)
    } else {
        Err(errors)
    }
}

fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

/// `{name}` references in a string.
fn var_refs(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else {
            break;
        };
        out.push(rest[start + 1..start + end].to_owned());
        rest = &rest[start + end + 1..];
    }
    out
}

pub fn validate(pack: &Pack) -> Vec<String> {
    let mut errors = Vec::new();
    let mut err = |m: String| errors.push(m);
    if !is_kebab(&pack.id) {
        err(format!("id `{}` must be kebab-case", pack.id));
    }
    if pack.name.trim().is_empty() {
        err("name is empty".into());
    }
    if pack.description.trim().is_empty() {
        err("description is empty".into());
    }
    if pack.locations.is_empty() && pack.artifacts.is_empty() {
        err("a pack needs at least one [[locations]] or [[artifacts]]".into());
    }
    let vars: BTreeSet<&str> = pack.vars.keys().map(String::as_str).collect();
    for (name, def) in &pack.vars {
        if name == "name" || name == "path" {
            err(format!("var name `{name}` is reserved"));
        }
        if def.macos.is_none()
            && def.linux.is_none()
            && def.windows.is_none()
            && def.default.is_none()
            && def.env.is_none()
            && def.command.is_none()
        {
            err(format!("var `{name}` has no source"));
        }
        if def.command.as_ref().is_some_and(Vec::is_empty) {
            err(format!("var `{name}`: command is empty"));
        }
    }
    let check_refs = |s: &str, ctx: &str, allow_name: bool, errs: &mut Vec<String>| {
        for r in var_refs(s) {
            let ok = vars.contains(r.as_str()) || (allow_name && (r == "name" || r == "parent"));
            if !ok {
                errs.push(format!("{ctx}: unknown variable `{{{r}}}`"));
            }
        }
    };
    let mut ref_errors = Vec::new();
    let check_actions = |list: &[String], ctx: &str, errs: &mut Vec<String>| {
        if list.is_empty() {
            errs.push(format!("{ctx}: actions is empty"));
        }
        for a in list {
            match a.parse::<ActionKind>() {
                Ok(ActionKind::Native { command_id })
                    if !pack.commands.contains_key(&command_id) =>
                {
                    errs.push(format!("{ctx}: action `{a}` refers to an unknown command"));
                }
                Ok(ActionKind::Compact { .. } | ActionKind::Archive) => {
                    errs.push(format!("{ctx}: packs can't use `{a}`"));
                }
                Ok(_) => {}
                Err(e) => errs.push(format!("{ctx}: {e}")),
            }
        }
    };
    for (i, loc) in pack.locations.iter().enumerate() {
        let ctx = format!("locations[{i}]");
        check_refs(&loc.path, &ctx, false, &mut ref_errors);
        check_refs(&loc.title, &ctx, loc.each == Each::Child, &mut ref_errors);
        check_actions(&loc.actions, &ctx, &mut ref_errors);
        if loc.reasons.is_empty() {
            ref_errors.push(format!("{ctx}: needs at least one reason"));
        }
        if loc.keep_latest.is_some() && loc.each != Each::Child {
            ref_errors.push(format!("{ctx}: keep_latest needs each = \"child\""));
        }
        if let Some(s) = &loc.min_size
            && let Err(e) = kipple_core::format::parse_size(s)
        {
            ref_errors.push(format!("{ctx}: min_size: {e}"));
        }
        if !loc.path.starts_with('{')
            && !loc.path.starts_with('~')
            && !loc.path.starts_with('$')
            && !loc.path.starts_with('%')
        {
            ref_errors.push(format!(
                "{ctx}: path must start with a variable, `~`, `$VAR` or `%VAR%`"
            ));
        }
    }
    for (i, a) in pack.artifacts.iter().enumerate() {
        let ctx = format!("artifacts[{i}]");
        if a.dir.contains(['/', '\\']) {
            ref_errors.push(format!("{ctx}: dir must be a single name or glob"));
        }
        if let Err(e) = globset::Glob::new(&a.dir) {
            ref_errors.push(format!("{ctx}: bad dir glob: {e}"));
        }
        check_actions(&a.actions, &ctx, &mut ref_errors);
        if a.reasons.is_empty() {
            ref_errors.push(format!("{ctx}: needs at least one reason"));
        }
        if a.dir == "*" && a.requires_inside_any.is_empty() && !a.requires_cachedir_tag {
            ref_errors.push(format!(
                "{ctx}: dir \"*\" needs requires_inside_any or requires_cachedir_tag"
            ));
        }
        if let Some(t) = &a.title {
            check_refs(t, &ctx, true, &mut ref_errors);
        }
    }
    for (id, cmd) in &pack.commands {
        let ctx = format!("commands.{id}");
        if cmd.run.is_empty() {
            ref_errors.push(format!("{ctx}: run is empty"));
        }
        if let Some(program) = cmd.run.first()
            && program.contains(['/', '\\', ' '])
        {
            ref_errors.push(format!(
                "{ctx}: the program must be a bare name found on PATH"
            ));
        }
        for arg in cmd.run.iter().skip(1) {
            for r in var_refs(arg) {
                if !vars.contains(r.as_str()) && r != "path" {
                    ref_errors.push(format!("{ctx}: unknown variable `{{{r}}}`"));
                }
            }
        }
        if cmd.timeout_secs == 0 || cmd.timeout_secs > 3600 {
            ref_errors.push(format!("{ctx}: timeout_secs must be 1–3600"));
        }
    }
    errors.extend(ref_errors);
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC_EXAMPLE: &str = r#"
id = "npm-cache"
name = "npm cache"
category = "package-cache"
description = "npm's download cache. Safe to remove; npm re-downloads packages when needed."
detect.any_command = ["npm"]
detect.any_path = ["{npm_cache}"]

[vars]
npm_cache.macos   = "~/.npm/_cacache"
npm_cache.linux   = "~/.npm/_cacache"
npm_cache.windows = "%LOCALAPPDATA%/npm-cache/_cacache"
npm_cache.env     = "npm_config_cache"
npm_cache.append  = "_cacache"

[[locations]]
path = "{npm_cache}"
kind = "npm_cache"
title = "npm download cache"
safety = "safe"
reasons = ["REDOWNLOADABLE"]
actions = ["native:npm-cache-clean", "quarantine"]

[commands.npm-cache-clean]
run = ["npm", "cache", "clean", "--force"]
timeout_secs = 300
"#;

    #[test]
    fn spec_example_is_valid() {
        let pack = parse_and_validate(SPEC_EXAMPLE).unwrap();
        assert_eq!(pack.id, "npm-cache");
    }

    #[test]
    fn reports_problems() {
        let bad = SPEC_EXAMPLE
            .replace("native:npm-cache-clean", "native:nope")
            .replace("{npm_cache}\"\nkind", "{missing}\"\nkind");
        let errs = parse_and_validate(&bad).unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("unknown command")),
            "{errs:?}"
        );
        assert!(
            errs.iter()
                .any(|e| e.contains("unknown variable `{missing}`")),
            "{errs:?}"
        );
    }

    #[test]
    fn unknown_keys_are_errors() {
        let bad = SPEC_EXAMPLE.replace("timeout_secs", "timeout");
        assert!(parse_and_validate(&bad).is_err());
    }
}
