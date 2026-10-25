//! Extra details for walk-matched items of specific packs (INTEGRATIONS.md
//! §4.1–4.2).

use std::path::Path;

use kipple_core::integration::ItemDraft;
use serde_json::{Map, Value, json};

fn details(draft: &mut ItemDraft) -> &mut Map<String, Value> {
    if !draft.details.is_object() {
        draft.details = json!({});
    }
    match &mut draft.details {
        Value::Object(m) => m,
        _ => unreachable!("set to an object above"),
    }
}

pub fn enrich(pack_id: &str, draft: &mut ItemDraft, path: &Path) {
    let Some(parent) = path.parent() else { return };
    match pack_id {
        "node-modules" => node_modules(draft, parent),
        "python-venv" => python_venv(draft, path, parent),
        _ => {}
    }
}

/// Lockfile → package manager and restore command.
const LOCKFILES: &[(&str, &str, &str)] = &[
    ("pnpm-lock.yaml", "pnpm", "pnpm install"),
    ("yarn.lock", "yarn", "yarn install"),
    ("bun.lockb", "bun", "bun install"),
    ("bun.lock", "bun", "bun install"),
    ("package-lock.json", "npm", "npm install"),
    ("npm-shrinkwrap.json", "npm", "npm install"),
];

fn node_modules(draft: &mut ItemDraft, project: &Path) {
    let name = std::fs::read_to_string(project.join("package.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.get("name").and_then(Value::as_str).map(str::to_owned));
    // Lockfiles may sit at a workspace root above the package.
    let lock = project.ancestors().take(4).find_map(|dir| {
        LOCKFILES
            .iter()
            .find(|(f, _, _)| dir.join(f).is_file())
            .copied()
    });
    let d = details(draft);
    if let Some(n) = name {
        d.insert("package".into(), json!(n));
    }
    let (manager, hint) = match lock {
        Some((_, m, h)) => (m, h),
        None => ("npm", "npm install"),
    };
    d.insert("package_manager".into(), json!(manager));
    draft.restore_hint = Some(hint.to_owned());
}

fn python_venv(draft: &mut ItemDraft, venv: &Path, project: &Path) {
    let cfg = std::fs::read_to_string(venv.join("pyvenv.cfg")).unwrap_or_default();
    let mut version = None;
    let mut creator = None;
    for line in cfg.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim().to_owned());
        match k {
            "version" | "version_info" => version = version.or(Some(v)),
            "uv" => creator = Some("uv".to_owned()),
            "virtualenv" => creator = creator.or(Some("virtualenv".to_owned())),
            _ => {}
        }
    }
    let hint = if project.join("uv.lock").is_file() {
        "uv sync"
    } else if project.join("poetry.lock").is_file() {
        "poetry install"
    } else if project.join("Pipfile.lock").is_file() {
        "pipenv install"
    } else if project.join("requirements.txt").is_file() {
        "pip install -r requirements.txt"
    } else {
        "pip install -e ."
    };
    let d = details(draft);
    if let Some(v) = version {
        d.insert("python_version".into(), json!(v));
    }
    d.insert(
        "created_by".into(),
        json!(creator.unwrap_or_else(|| "venv".into())),
    );
    draft.restore_hint = Some(hint.to_owned());
}
