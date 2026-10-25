use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};
use kipple_core::config::Config;
use serde::Serialize;

use crate::args::ConfigCommand;
use crate::context::Context;

#[derive(Serialize)]
struct PathOutput {
    config_file: String,
    exists: bool,
    data_dir: String,
}

#[derive(Serialize)]
struct ValidateOutput {
    config_file: String,
    valid: bool,
    error: Option<String>,
}

pub fn run(ctx: &Context, cmd: &ConfigCommand) -> Result<ExitCode> {
    let file = &ctx.dirs.config_file;
    match cmd {
        ConfigCommand::Path => {
            if ctx.global.json {
                ctx.print_json(PathOutput {
                    config_file: file.display().to_string(),
                    exists: file.exists(),
                    data_dir: ctx.dirs.data_dir.display().to_string(),
                })?;
            } else {
                println!("{}", file.display());
            }
        }
        ConfigCommand::Show => {
            let config = ctx.load_config()?;
            if ctx.global.json {
                ctx.print_json(&config)?;
            } else {
                if !file.exists() && !ctx.global.quiet {
                    println!(
                        "# {} doesn't exist; these are the defaults.\n",
                        file.display()
                    );
                }
                print!("{}", config.to_toml());
            }
        }
        ConfigCommand::Validate => {
            let result = Config::load(file);
            let error = result.as_ref().err().map(ToString::to_string);
            if ctx.global.json {
                ctx.print_json(ValidateOutput {
                    config_file: file.display().to_string(),
                    valid: error.is_none(),
                    error: error.clone(),
                })?;
            } else if let Some(e) = &error {
                eprintln!("{e}");
            } else if !file.exists() {
                println!("No config file at {}. Defaults apply.", file.display());
            } else {
                println!("{} is valid.", file.display());
            }
            if error.is_some() {
                return Ok(ExitCode::from(1));
            }
        }
        ConfigCommand::Edit => {
            if ctx.global.json {
                bail!("`config edit` is interactive and can't be used with --json");
            }
            let editor = ctx
                .env
                .get_str("VISUAL")
                .or_else(|| ctx.env.get_str("EDITOR"))
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    if cfg!(windows) {
                        "notepad".into()
                    } else {
                        "vi".into()
                    }
                });
            if let Some(dir) = file.parent() {
                std::fs::create_dir_all(dir)
                    .with_context(|| format!("creating {}", dir.display()))?;
            }
            if !file.exists() {
                std::fs::write(file, Config::default().to_toml())
                    .with_context(|| format!("writing {}", file.display()))?;
            }
            // The editor setting may carry arguments (`code --wait`).
            let mut parts = editor.split_whitespace();
            let program = parts.next().unwrap_or("vi");
            let status = std::process::Command::new(program)
                .args(parts)
                .arg(file)
                .status()
                .with_context(|| format!("starting {program}"))?;
            if !status.success() {
                bail!("{program} exited with an error");
            }
            if let Err(e) = Config::load(file) {
                eprintln!("{e}");
                return Ok(ExitCode::from(1));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
