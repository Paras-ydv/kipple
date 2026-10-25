//! Default English text for every reason code (SPEC §7.3). Integrations may
//! override the text. Placeholders: `{days}`, `{project}`, `{tool}`, `{path}`,
//! `{count}`, `{size}`, `{names}`.

use std::collections::BTreeMap;

use crate::model::{Reason, ReasonCode};

pub fn template(code: ReasonCode) -> &'static str {
    match code {
        ReasonCode::Regenerable => "Your build tools recreate this automatically.",
        ReasonCode::Redownloadable => "This is a download cache; it is fetched again when needed.",
        ReasonCode::InactiveProject => "The project hasn't been used for {days} days.",
        ReasonCode::RecentlyUsed => {
            "The project was used in the last {days} days, so you may still need this."
        }
        ReasonCode::RecentlyWritten => {
            "This changed in the last few minutes, so something may still be using it."
        }
        ReasonCode::ActiveWriter => "Something is writing to this right now.",
        ReasonCode::ToolRunning => "{tool} is running. Quit it first, then scan again.",
        ReasonCode::InUseByContainer => "A container uses this.",
        ReasonCode::UnusedByAnyEmulator => "No emulator uses this system image.",
        ReasonCode::UsedByEmulator => "Used by the emulator {names}.",
        ReasonCode::ContainsUserData => "This may hold data you created that can't be recreated.",
        ReasonCode::ChatHistory => {
            "This is chat history; once removed you can't read or resume it."
        }
        ReasonCode::MayContainSecrets => "Chat logs often contain pasted keys or passwords.",
        ReasonCode::LargeRedownload => "Removing this means a large download to get it back.",
        ReasonCode::OrphanedProject => "The project folder it belongs to no longer exists.",
        ReasonCode::MigratedCopy => "A newer copy of this exists in the current location.",
        ReasonCode::BackupCopy => "This is a backup copy the app made of its own database.",
        ReasonCode::FreePages => {
            "The database has {size} of unused space that compaction can return."
        }
        ReasonCode::ProtectedPath => "This is in a protected location ({path}).",
        ReasonCode::OutsideRoots => "This is outside the folders kipple was asked to scan.",
        ReasonCode::UnknownFormat => {
            "kipple doesn't recognise this storage format, so it won't touch it."
        }
        ReasonCode::NeedsElevation => {
            "Removing this needs administrator rights; kipple doesn't use sudo."
        }
        ReasonCode::NativeToolMissing => {
            "{tool} isn't installed or is too old, and kipple only removes this with it."
        }
        ReasonCode::SharedHardlinks => {
            "{size} of this is shared with other files and won't be freed."
        }
        ReasonCode::PinnedByProject => "A project pins this version.",
        ReasonCode::Experimental => {
            "kipple's support for this tool is new, so it asks you to review first."
        }
        ReasonCode::UserOverride => "Your config sets this safety level.",
        ReasonCode::ContainsSource => "This folder holds source code or a git repository.",
    }
}

/// Fills `{name}` placeholders. Unknown placeholders are left as-is.
pub fn render(code: ReasonCode, vars: &BTreeMap<&str, String>) -> String {
    let mut text = template(code).to_owned();
    for (k, v) in vars {
        text = text.replace(&format!("{{{k}}}"), v);
    }
    text
}

/// A reason with the default text and no placeholders filled.
pub fn reason(code: ReasonCode) -> Reason {
    Reason {
        code,
        text: template(code).to_owned(),
    }
}

/// A reason with placeholders filled.
pub fn reason_with(code: ReasonCode, vars: &[(&str, String)]) -> Reason {
    let map: BTreeMap<&str, String> = vars.iter().cloned().collect();
    Reason {
        code,
        text: render(code, &map),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_is_one_plain_sentence() {
        for code in ReasonCode::ALL {
            let t = template(code);
            assert!(t.ends_with('.'), "{code}: must end with a period");
            assert!(!t.contains('!'), "{code}: no exclamation marks");
            assert!(t.len() < 120, "{code}: keep it short");
        }
    }

    #[test]
    fn renders_placeholders() {
        let r = reason_with(ReasonCode::InactiveProject, &[("days", "45".into())]);
        assert_eq!(r.text, "The project hasn't been used for 45 days.");
    }
}
