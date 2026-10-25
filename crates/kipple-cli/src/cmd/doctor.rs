//! `kipple doctor` and `kipple completions`.

use std::process::ExitCode;

use anyhow::Result;
use clap::CommandFactory;
use kipple_core::model::Detection;

use crate::args::Cli;
use crate::context::Context;
use crate::output::{Align, Table, short_path};

pub fn doctor(ctx: &mut Context) -> Result<ExitCode> {
    let k = ctx.open()?;
    let report = k.doctor()?;
    if ctx.global.json {
        ctx.print_json(&report)?;
        return Ok(ExitCode::SUCCESS);
    }
    let style = ctx.style();
    let home = ctx.home();
    let p = &report.platform;
    println!(
        "{}",
        style.bold(&format!(
            "kipple {} · {} {}{}",
            kipple_core::VERSION,
            match p.os {
                kipple_core::model::Os::Macos => "macOS",
                kipple_core::model::Os::Linux => "Linux",
                kipple_core::model::Os::Windows => "Windows",
                kipple_core::model::Os::Wsl => "WSL",
            },
            p.arch,
            if p.wsl { " (WSL)" } else { "" }
        ))
    );
    println!(
        "Config  {}{}",
        short_path(&report.config_path, home.as_deref()),
        if report.config_exists {
            ""
        } else {
            " (not created; defaults apply)"
        }
    );
    println!(
        "Data    {}\n",
        short_path(&report.data_dir, home.as_deref())
    );

    let mut t = Table::new(&[
        ("Integration", Align::Left),
        ("Status", Align::Left),
        ("Details", Align::Left),
    ]);
    let mut fixes = Vec::new();
    for i in &report.integrations {
        let status = match (&i.detection, i.enabled) {
            (Detection::Unsupported { .. }, _) => "problem",
            (Detection::Present { .. }, true) => "found",
            (Detection::Present { .. }, false) => "off",
            (Detection::NotPresent, _) => "not found",
        };
        let mut details: Vec<String> = Vec::new();
        if let Detection::Present { version: Some(v) } = &i.detection {
            details.push(format!("version {v}"));
        }
        let found: Vec<String> = i
            .locations
            .iter()
            .filter(|l| l.exists)
            .map(|l| short_path(&l.path, home.as_deref()))
            .collect();
        if !found.is_empty() {
            details.push(found.join(", "));
        }
        let missing: Vec<&str> = i
            .tools
            .iter()
            .filter(|t| t.found.is_none())
            .map(|t| t.name.as_str())
            .collect();
        if !missing.is_empty() && status == "found" {
            details.push(format!("no {}", missing.join(", ")));
        }
        t.row(vec![
            i.info.id.to_string(),
            status.to_owned(),
            style.dim(&details.join(" · ")),
        ]);
        for f in &i.fixes {
            fixes.push(format!("{}: {f}", i.info.id));
        }
    }
    print!("{}", t.render(&style));
    if !fixes.is_empty() || !report.warnings.is_empty() {
        println!("\nWhat to do");
        for f in fixes.iter().chain(report.warnings.iter()) {
            println!("  {f}");
        }
    }
    Ok(ExitCode::SUCCESS)
}

pub fn completions(shell: clap_complete::Shell) -> Result<ExitCode> {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "kipple", &mut std::io::stdout());
    Ok(ExitCode::SUCCESS)
}
