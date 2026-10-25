//! Shared state for one CLI run.

use std::io::IsTerminal;

use anyhow::{Context as _, Result};
use kipple_core::config::{ColorMode, Config};
use kipple_core::dirs::Dirs;
use kipple_core::platform::{Env, Platform};
use serde::Serialize;

use crate::args::GlobalArgs;

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
        })
    }

    pub fn load_config(&self) -> Result<Config> {
        let config = Config::load(&self.dirs.config_file)?;
        Ok(config)
    }

    /// Applies `ui.color` from config.
    #[expect(dead_code, reason = "used by the scan commands (M3)")]
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

    /// Prints `data` wrapped in the versioned envelope.
    pub fn print_json<T: Serialize>(&self, data: T) -> Result<()> {
        let envelope = kipple_core::Envelope::new(data);
        let text = serde_json::to_string_pretty(&envelope).context("serializing output")?;
        println!("{text}");
        Ok(())
    }
}
