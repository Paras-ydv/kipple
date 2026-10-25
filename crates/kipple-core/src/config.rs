//! `config.toml` loading and validation (SPEC §10.3).

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Category, Safety};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("{path}: unknown key `{key}`{}", suggestion_suffix(.suggestion))]
    UnknownKey {
        path: PathBuf,
        key: String,
        suggestion: Option<String>,
    },
    #[error("{path}: {message}")]
    Invalid { path: PathBuf, message: String },
}

fn suggestion_suffix(s: &Option<String>) -> String {
    s.as_ref()
        .map(|s| format!(" (did you mean `{s}`?)"))
        .unwrap_or_default()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub scan: ScanConfig,
    pub safety: SafetyConfig,
    pub quarantine: QuarantineConfig,
    pub integrations: IntegrationsConfig,
    pub exec: ExecConfig,
    pub ui: UiConfig,
    pub retention: RetentionConfig,
    pub large_files: LargeFilesConfig,
    pub archive: ArchiveConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ScanConfig {
    pub roots: Vec<String>,
    pub exclude: Vec<String>,
    pub include_network: bool,
    pub follow_symlinks: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            roots: vec!["~".to_owned()],
            exclude: Vec::new(),
            include_network: false,
            follow_symlinks: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SafetyConfig {
    /// Projects used within this many days keep their artifacts at Review.
    pub recent_days: u32,
    /// Extra protected paths.
    pub protect: Vec<String>,
    #[serde(rename = "override")]
    pub overrides: Vec<SafetyOverride>,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            recent_days: 14,
            protect: Vec::new(),
            overrides: Vec::new(),
        }
    }
}

/// `[[safety.override]]`: by integration id, kind or path glob.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SafetyOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glob: Option<String>,
    pub safety: Safety,
    /// Allow this override to make an item less strict.
    #[serde(default)]
    pub allow_relax: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct QuarantineConfig {
    pub keep_days: u32,
}

impl Default for QuarantineConfig {
    fn default() -> Self {
        Self { keep_days: 7 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IntegrationsConfig {
    pub disable: Vec<String>,
    /// Opt-in integrations (disabled by default).
    pub enable: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExecConfig {
    /// Integrations executed in parallel.
    pub parallelism: u32,
}

impl Default for ExecConfig {
    fn default() -> Self {
        Self { parallelism: 2 }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Units {
    /// GB, powers of 1000.
    #[default]
    Si,
    /// GiB, powers of 1024.
    Iec,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    pub units: Units,
    pub color: ColorMode,
}

/// [v1] Keep-clean rules. Parsed and validated now; evaluated by `kipple auto`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetentionConfig {
    #[serde(rename = "rule")]
    pub rules: Vec<RetentionRule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionRule {
    pub name: String,
    #[serde(rename = "match")]
    pub matcher: RetentionMatch,
    pub action: String,
    #[serde(default)]
    pub auto_confirm: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetentionMatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_inactive_days: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub older_than_days: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LargeFilesConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_size: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArchiveConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
}

// ---------------------------------------------------------------------------
// Unknown-key detection with suggestions.

enum Shape {
    Leaf,
    /// A free-form table (e.g. `[retention.rule.match]` values).
    Table(&'static [(&'static str, Shape)]),
    ArrayOfTables(&'static [(&'static str, Shape)]),
}

const OVERRIDE: &[(&str, Shape)] = &[
    ("integration", Shape::Leaf),
    ("kind", Shape::Leaf),
    ("glob", Shape::Leaf),
    ("safety", Shape::Leaf),
    ("allow_relax", Shape::Leaf),
];

const RETENTION_MATCH: &[(&str, Shape)] = &[
    ("integration", Shape::Leaf),
    ("category", Shape::Leaf),
    ("kind", Shape::Leaf),
    ("project_inactive_days", Shape::Leaf),
    ("older_than_days", Shape::Leaf),
];

const RETENTION_RULE: &[(&str, Shape)] = &[
    ("name", Shape::Leaf),
    ("match", Shape::Table(RETENTION_MATCH)),
    ("action", Shape::Leaf),
    ("auto_confirm", Shape::Leaf),
];

const ROOT: &[(&str, Shape)] = &[
    (
        "scan",
        Shape::Table(&[
            ("roots", Shape::Leaf),
            ("exclude", Shape::Leaf),
            ("include_network", Shape::Leaf),
            ("follow_symlinks", Shape::Leaf),
        ]),
    ),
    (
        "safety",
        Shape::Table(&[
            ("recent_days", Shape::Leaf),
            ("protect", Shape::Leaf),
            ("override", Shape::ArrayOfTables(OVERRIDE)),
        ]),
    ),
    ("quarantine", Shape::Table(&[("keep_days", Shape::Leaf)])),
    (
        "integrations",
        Shape::Table(&[("disable", Shape::Leaf), ("enable", Shape::Leaf)]),
    ),
    ("exec", Shape::Table(&[("parallelism", Shape::Leaf)])),
    (
        "ui",
        Shape::Table(&[("units", Shape::Leaf), ("color", Shape::Leaf)]),
    ),
    (
        "retention",
        Shape::Table(&[("rule", Shape::ArrayOfTables(RETENTION_RULE))]),
    ),
    ("large_files", Shape::Table(&[("min_size", Shape::Leaf)])),
    ("archive", Shape::Table(&[("dir", Shape::Leaf)])),
];

/// Finds the first unknown key and the closest known one.
fn check_keys(
    table: &toml::Table,
    shape: &'static [(&'static str, Shape)],
    prefix: &str,
) -> Option<(String, Option<String>)> {
    for (key, value) in table {
        let full = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        let Some((_, child)) = shape.iter().find(|(name, _)| name == key) else {
            let suggestion = shape
                .iter()
                .map(|(name, _)| (*name, strsim::jaro_winkler(key, name)))
                .filter(|(_, score)| *score > 0.8)
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(name, _)| {
                    if prefix.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{prefix}.{name}")
                    }
                });
            return Some((full, suggestion));
        };
        match (child, value) {
            (Shape::Table(fields), toml::Value::Table(t)) => {
                if let Some(found) = check_keys(t, fields, &full) {
                    return Some(found);
                }
            }
            (Shape::ArrayOfTables(fields), toml::Value::Array(items)) => {
                for item in items {
                    if let toml::Value::Table(t) = item
                        && let Some(found) = check_keys(t, fields, &full)
                    {
                        return Some(found);
                    }
                }
            }
            // Type mismatches are reported by serde with a better message.
            _ => {}
        }
    }
    None
}

impl Config {
    /// Parses and validates `text` read from `path` (for messages).
    pub fn parse(text: &str, path: &Path) -> Result<Self, ConfigError> {
        let table: toml::Table = text
            .parse()
            .map_err(|e: toml::de::Error| ConfigError::Parse {
                path: path.to_path_buf(),
                message: e.message().to_owned(),
            })?;
        if let Some((key, suggestion)) = check_keys(&table, ROOT, "") {
            return Err(ConfigError::UnknownKey {
                path: path.to_path_buf(),
                key,
                suggestion,
            });
        }
        let config: Config = toml::from_str(text).map_err(|e| ConfigError::Parse {
            path: path.to_path_buf(),
            message: e.message().to_owned(),
        })?;
        config.validate(path)?;
        Ok(config)
    }

    /// Loads `path`, or defaults when it doesn't exist.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text, path),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    fn validate(&self, path: &Path) -> Result<(), ConfigError> {
        let invalid = |message: String| ConfigError::Invalid {
            path: path.to_path_buf(),
            message,
        };
        if self.scan.roots.is_empty() {
            return Err(invalid("scan.roots must list at least one path".into()));
        }
        for glob in &self.scan.exclude {
            globset::Glob::new(glob)
                .map_err(|e| invalid(format!("scan.exclude: bad glob `{glob}`: {e}")))?;
        }
        if self.safety.recent_days == 0 {
            return Err(invalid("safety.recent_days must be at least 1".into()));
        }
        if self.quarantine.keep_days == 0 {
            return Err(invalid("quarantine.keep_days must be at least 1".into()));
        }
        if !(1..=16).contains(&self.exec.parallelism) {
            return Err(invalid("exec.parallelism must be between 1 and 16".into()));
        }
        for (i, o) in self.safety.overrides.iter().enumerate() {
            if o.integration.is_none() && o.kind.is_none() && o.glob.is_none() {
                return Err(invalid(format!(
                    "safety.override[{i}] needs integration, kind or glob"
                )));
            }
            if let Some(glob) = &o.glob {
                globset::Glob::new(glob).map_err(|e| {
                    invalid(format!("safety.override[{i}]: bad glob `{glob}`: {e}"))
                })?;
            }
        }
        for rule in &self.retention.rules {
            if !matches!(
                rule.action.as_str(),
                "quarantine" | "trash" | "native" | "archive"
            ) {
                return Err(invalid(format!(
                    "retention rule `{}`: action must be quarantine, trash, native or archive",
                    rule.name
                )));
            }
        }
        Ok(())
    }

    /// Validates and writes the config to `path` (atomically). Comments in
    /// an existing file are not kept.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let text = self.to_toml();
        // Round-trip through the parser so only valid configs are written.
        Self::parse(&text, path)?;
        let io = |source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text).map_err(io)?;
        std::fs::rename(&tmp, path).map_err(io)
    }

    /// The effective config as TOML.
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }
}

impl fmt::Display for Units {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Units::Si => "si",
            Units::Iec => "iec",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Config, ConfigError> {
        Config::parse(text, Path::new("config.toml"))
    }

    const SPEC_EXAMPLE: &str = r#"
[scan]
roots = ["~"]
exclude = ["~/Library/Mobile Documents", "**/huge-dataset/**"]
include_network = false
follow_symlinks = false

[safety]
recent_days = 14
protect = ["~/work/client-x"]

[[safety.override]]
integration = "docker"
kind = "docker_volume"
safety = "keep"

[quarantine]
keep_days = 7

[integrations]
disable = ["old-installers"]
enable = ["large-files"]

[retention]
[[retention.rule]]
name = "stale node_modules"
match = { integration = "node-modules", project_inactive_days = 60 }
action = "quarantine"
[[retention.rule]]
name = "old codex sessions"
match = { integration = "codex", older_than_days = 45 }
action = "native"

[ui]
units = "si"
color = "auto"
"#;

    #[test]
    fn parses_spec_example() {
        let config = parse(SPEC_EXAMPLE).unwrap();
        assert_eq!(config.safety.overrides[0].safety, Safety::Keep);
        assert_eq!(config.retention.rules.len(), 2);
        assert_eq!(config.integrations.disable, ["old-installers"]);
    }

    #[test]
    fn round_trips() {
        let config = parse(SPEC_EXAMPLE).unwrap();
        let again = parse(&config.to_toml()).unwrap();
        assert_eq!(config, again);
        assert_eq!(
            parse(&Config::default().to_toml()).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn empty_file_is_defaults() {
        assert_eq!(parse("").unwrap(), Config::default());
    }

    #[test]
    fn unknown_key_suggests_closest() {
        let err = parse("[scan]\nrots = [\"~\"]\n").unwrap_err();
        assert_eq!(
            err.to_string(),
            "config.toml: unknown key `scan.rots` (did you mean `scan.roots`?)"
        );
        let err = parse("[quarantine]\nkeepdays = 3\n").unwrap_err();
        assert!(
            err.to_string()
                .contains("did you mean `quarantine.keep_days`")
        );
        let err =
            parse("[[safety.override]]\nintegraton = \"x\"\nsafety = \"keep\"\n").unwrap_err();
        assert!(err.to_string().contains("safety.override.integration"));
    }

    #[test]
    fn unknown_key_without_close_match() {
        let err = parse("[zzz]\n").unwrap_err();
        assert_eq!(err.to_string(), "config.toml: unknown key `zzz`");
    }

    #[test]
    fn rejects_invalid_values() {
        assert!(parse("[quarantine]\nkeep_days = 0\n").is_err());
        assert!(parse("[safety]\nrecent_days = 0\n").is_err());
        assert!(parse("[ui]\nunits = \"bytes\"\n").is_err());
        assert!(parse("[[safety.override]]\nsafety = \"keep\"\n").is_err());
        assert!(parse("[exec]\nparallelism = 0\n").is_err());
    }

    #[test]
    fn save_round_trips_and_rejects_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/config.toml");
        let mut c = Config::default();
        c.safety.recent_days = 21;
        c.integrations.disable = vec!["npm-cache".into()];
        c.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), c);
        c.quarantine.keep_days = 0;
        assert!(c.save(&path).is_err());
        assert_eq!(
            Config::load(&path).unwrap().quarantine.keep_days,
            7,
            "bad config not written"
        );
    }

    #[test]
    fn missing_file_is_defaults() {
        let config = Config::load(Path::new("/definitely/not/here/config.toml")).unwrap();
        assert_eq!(config, Config::default());
    }
}
