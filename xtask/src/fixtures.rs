//! `cargo xtask fixtures [ids…]`: generates fixtures into one sandbox home
//! that is kept, and prints the environment to use it.

use anyhow::{Result, bail};
use kipple_core::testing::{Fixture, TestHome, fixtures_dir};

pub fn run(ids: &[String]) -> Result<()> {
    let mut all: Vec<String> = std::fs::read_dir(fixtures_dir())?
        .flatten()
        .filter(|e| e.path().join("fixture.toml").is_file())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .collect();
    all.sort();
    let chosen: Vec<String> = if ids.is_empty() {
        all.clone()
    } else {
        ids.to_vec()
    };
    let mut home = TestHome::new()?;
    for id in &chosen {
        if !all.contains(id) {
            bail!("no fixture named `{id}`");
        }
        Fixture::named(id)?.generate(&mut home)?;
    }
    let env: Vec<String> = home
        .env()
        .iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                k.to_string_lossy(),
                shell_quote(&v.to_string_lossy())
            )
        })
        .collect();
    let path = home.keep();
    eprintln!("generated {} fixtures in {}", chosen.len(), path.display());
    eprintln!("run kipple against it with:\n");
    println!("env -i {} kipple scan", env.join(" "));
    Ok(())
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
