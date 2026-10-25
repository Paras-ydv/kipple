//! `cargo xtask schema [--check]`: JSON Schemas for every public type into
//! `docs/schema/` (SPEC §3.4).

use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use kipple_core::model::*;
use schemars::{JsonSchema, schema_for};

fn render<T: JsonSchema>() -> Result<String> {
    let schema = schema_for!(T);
    Ok(serde_json::to_string_pretty(&schema)? + "\n")
}

fn all() -> Result<Vec<(&'static str, String)>> {
    Ok(vec![
        ("item", render::<Item>()?),
        ("project", render::<ProjectSummary>()?),
        ("plan", render::<Plan>()?),
        ("scan-result", render::<ScanResult>()?),
        ("report", render::<Report>()?),
        ("explanation", render::<Explanation>()?),
        ("exec-report", render::<ExecReport>()?),
        ("quarantine-entry", render::<QuarantineEntry>()?),
        ("restore-report", render::<RestoreReport>()?),
        ("purge-report", render::<PurgeReport>()?),
        ("journal-entry", render::<JournalEntry>()?),
        ("size-point", render::<SizePoint>()?),
        ("doctor-report", render::<DoctorReport>()?),
        ("integration", render::<IntegrationStatusReport>()?),
        ("scan-event", render::<ScanEvent>()?),
        ("exec-event", render::<ExecEvent>()?),
        ("item-filter", render::<ItemFilter>()?),
        ("plan-request", render::<PlanRequest>()?),
    ])
}

pub fn run(root: &Path, check: bool) -> Result<()> {
    let dir = root.join("docs/schema");
    fs::create_dir_all(&dir)?;
    let mut stale = Vec::new();
    for (name, json) in all()? {
        let path = dir.join(format!("{name}.json"));
        let current = fs::read_to_string(&path).unwrap_or_default();
        if current != json {
            if check {
                stale.push(name);
            } else {
                fs::write(&path, json).with_context(|| format!("writing {}", path.display()))?;
                println!("wrote {}", path.display());
            }
        }
    }
    if !stale.is_empty() {
        bail!(
            "schemas out of date: {} (run `cargo xtask schema`)",
            stale.join(", ")
        );
    }
    Ok(())
}
