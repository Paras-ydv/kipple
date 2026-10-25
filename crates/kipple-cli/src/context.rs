//! Shared state for one CLI run.

use std::io::IsTerminal;

use anyhow::{Context as _, Result};
use kipple_core::config::{ColorMode, Config};
use kipple_core::dirs::Dirs;
use kipple_core::platform::{Env, Platform};
use serde::Serialize;

use kipple_core::config::Units;
use kipple_core::format::{parse_age, parse_size};
use kipple_core::model::ItemFilter;
use kipple_core::{Kipple, OpenOptions};

use crate::args::GlobalArgs;
use crate::output::Style;

#[expect(
    dead_code,
    reason = "platform and color are used by the scan commands (M3)"
)]
pub struct Context {
    pub global: GlobalArgs,
    pub env: Env,
    pub platform: Platform,
    pub dirs: Dirs,
    pub color: bool,
    pub units: Units,
}

impl Context {
    pub fn new(global: GlobalArgs, env: Env) -> Result<Self> {
        let platform = Platform::new(env.clone());
        let mut dirs = Dirs::resolve(&env, platform.os)?;
        if let Some(path) = &global.config {
            dirs.config_file = path.clone();
        }
        let color = !global.json
            && !global.no_color
            && env.get("NO_COLOR").is_none()
            && std::io::stdout().is_terminal();
        Ok(Self {
            global,
            env,
            platform,
            dirs,
            color,
            units: Units::Si,
        })
    }

    pub fn load_config(&self) -> Result<Config> {
        let config = Config::load(&self.dirs.config_file)?;
        Ok(config)
    }

    /// Applies `ui.color` from config.
    pub fn apply_config(&mut self, config: &Config) {
        match config.ui.color {
            ColorMode::Never => self.color = false,
            ColorMode::Always => {
                self.color =
                    !self.global.json && !self.global.no_color && self.env.get("NO_COLOR").is_none()
            }
            ColorMode::Auto => {}
        }
    }

    /// Opens the core with every built-in integration.
    pub fn open(&mut self) -> Result<Kipple> {
        let mut integrations = kipple_packs::builtin()?;
        integrations.extend(kipple_adapters::all());
        let mut opts = OpenOptions::new(self.env.clone(), integrations);
        opts.config_file = self.global.config.clone();
        let k = Kipple::open(opts)?;
        self.apply_config(k.config());
        self.units = k.config().ui.units;
        Ok(k)
    }

    pub fn style(&self) -> Style {
        Style {
            color: self.color,
            units: self.units,
        }
    }

    pub fn home(&self) -> Option<std::path::PathBuf> {
        self.env.home()
    }

    /// Global filter flags as an item filter.
    pub fn filter(&self) -> Result<ItemFilter> {
        let g = &self.global;
        Ok(ItemFilter {
            only: g.only.clone(),
            exclude: g.exclude.clone(),
            ids: Vec::new(),
            roots: g.root.clone(),
            older_than_secs: g.older_than.as_deref().map(parse_age).transpose()?,
            min_size: g.min_size.as_deref().map(parse_size).transpose()?,
            safe_only: g.safe_only,
            safety: Vec::new(),
            project: None,
        })
    }

    /// Prints `data` wrapped in the versioned envelope.
    pub fn print_json<T: Serialize>(&self, data: T) -> Result<()> {
        let envelope = kipple_core::Envelope::new(data);
        let text = serde_json::to_string_pretty(&envelope).context("serializing output")?;
        println!("{text}");
        Ok(())
    }
}
