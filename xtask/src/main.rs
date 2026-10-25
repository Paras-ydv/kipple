//! Development tasks: `cargo xtask <task>`.

mod schema;

use std::path::PathBuf;

use anyhow::{Result, bail};

const HELP: &str = "usage: cargo xtask <task>

tasks:
  schema [--check]   export JSON Schemas to docs/schema/
  help               show this message";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|a| a == name);
    match args.first().map(String::as_str) {
        None | Some("help") => {
            println!("{HELP}");
            Ok(())
        }
        Some("schema") => schema::run(&workspace_root(), flag("--check")),
        Some(other) => bail!("unknown task: {other}\n\n{HELP}"),
    }
}
