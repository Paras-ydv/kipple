//! `kipple doctor` (SPEC §11.2): what each integration sees and how to fix
//! what it can't.

use crate::integration::Ctx;
use crate::kipple::{Kipple, Result};
use crate::model::*;

fn full_disk_access_hint(os: Os) -> &'static str {
    match os {
        Os::Macos => {
            " On macOS, give your terminal Full Disk Access in System Settings, Privacy & Security."
        }
        _ => " Check the folder's permissions.",
    }
}

impl Kipple {
    /// Checks every integration. Read-only.
    pub fn doctor(&self) -> Result<DoctorReport> {
        let cancel = CancelToken::new();
        let procs = self.platform().processes.snapshot();
        let ctx = Ctx::new(self.platform(), self.config(), self.dirs(), &cancel, &procs);
        let enabled_by_user = &self.config().integrations.enable;
        let disabled_by_user = &self.config().integrations.disable;
        let os = self.platform().os;

        let mut integrations = Vec::new();
        for adapter in self.adapters() {
            let info = adapter.info();
            let id = info.id.as_str().to_owned();
            let os_ok = crate::platform::os::os_matches(&info.os, os, self.platform().wsl);
            let detection = if os_ok {
                adapter.detect(&ctx)
            } else {
                Detection::Unsupported {
                    reason: "Not available on this system.".into(),
                }
            };
            let enabled = os_ok
                && !disabled_by_user.contains(&id)
                && (!adapter.opt_in() || enabled_by_user.contains(&id));
            let locations: Vec<LocationCheck> = if os_ok {
                adapter
                    .locations(&ctx)
                    .into_iter()
                    .map(|path| {
                        let exists = path.symlink_metadata().is_ok();
                        let readable =
                            exists && (path.is_file() || std::fs::read_dir(&path).is_ok());
                        LocationCheck {
                            path,
                            exists,
                            readable,
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let tools: Vec<ToolCheck> = adapter
                .tools()
                .into_iter()
                .map(|name| ToolCheck {
                    found: self.platform().runner.which(&name),
                    name,
                })
                .collect();

            let mut fixes = Vec::new();
            if os_ok {
                if let Detection::Unsupported { reason } = &detection {
                    fixes.push(reason.clone());
                }
                for l in locations.iter().filter(|l| l.exists && !l.readable) {
                    fixes.push(format!(
                        "kipple can't read {}.{}",
                        l.path.display(),
                        full_disk_access_hint(os)
                    ));
                }
                let has_data = locations.iter().any(|l| l.exists);
                if has_data {
                    for t in tools.iter().filter(|t| t.found.is_none()) {
                        fixes.push(format!(
                            "Install `{}` or put it on your PATH so kipple can use the tool's own cleanup.",
                            t.name
                        ));
                    }
                }
                if adapter.opt_in() && !enabled && detection.is_present() {
                    fixes.push(format!(
                        "This is off by default. Turn it on with `enable = [\"{id}\"]` under [integrations] in your config."
                    ));
                }
                if disabled_by_user.contains(&id) && detection.is_present() {
                    fixes.push(format!(
                        "You turned this off in your config (`disable = [\"{id}\"]`)."
                    ));
                }
            }
            integrations.push(DoctorIntegration {
                info,
                enabled,
                detection,
                locations,
                tools,
                fixes,
            });
        }

        let mut warnings = Vec::new();
        let platform = self.platform().info();
        if !platform.trash_available {
            warnings.push(
                "There's no trash on this system, so items on other drives can't be cleaned."
                    .into(),
            );
        }
        Ok(DoctorReport {
            platform,
            config_path: self.dirs().config_file.clone(),
            config_exists: self.dirs().config_file.exists(),
            data_dir: self.dirs().data_dir.clone(),
            integrations,
            warnings,
        })
    }
}
