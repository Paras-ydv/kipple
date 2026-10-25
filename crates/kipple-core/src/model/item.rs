use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ActionKind, IntegrationId, ItemId, ProjectId, Timestamp};

/// What kind of junk an item is. Drives grouping and colors.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    ProjectArtifact,
    PackageCache,
    Toolchain,
    Container,
    Mobile,
    AiHistory,
    Model,
    IdeCache,
    BuildCache,
    Logs,
    System,
}

impl Category {
    pub const ALL: [Category; 11] = [
        Category::ProjectArtifact,
        Category::PackageCache,
        Category::Toolchain,
        Category::Container,
        Category::Mobile,
        Category::AiHistory,
        Category::Model,
        Category::IdeCache,
        Category::BuildCache,
        Category::Logs,
        Category::System,
    ];

    /// The kebab-case id used in config, packs and `--only`.
    pub fn id(self) -> &'static str {
        match self {
            Category::ProjectArtifact => "project-artifact",
            Category::PackageCache => "package-cache",
            Category::Toolchain => "toolchain",
            Category::Container => "container",
            Category::Mobile => "mobile",
            Category::AiHistory => "ai-history",
            Category::Model => "model",
            Category::IdeCache => "ide-cache",
            Category::BuildCache => "build-cache",
            Category::Logs => "logs",
            Category::System => "system",
        }
    }

    /// Plural label for tables, sentence case.
    pub fn label(self) -> &'static str {
        match self {
            Category::ProjectArtifact => "Project artifacts",
            Category::PackageCache => "Package caches",
            Category::Toolchain => "Toolchains",
            Category::Container => "Containers",
            Category::Mobile => "Mobile",
            Category::AiHistory => "AI history",
            Category::Model => "Models",
            Category::IdeCache => "IDE caches",
            Category::BuildCache => "Build caches",
            Category::Logs => "Logs",
            Category::System => "System",
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for Category {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Category::ALL
            .into_iter()
            .find(|c| c.id() == s)
            .ok_or_else(|| format!("unknown category `{s}`"))
    }
}

/// How safe an item is to remove. Ordered from least to most strict.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Safety {
    /// Regenerates automatically or holds no user data.
    Safe,
    /// Probably fine; the user should look first.
    Review,
    /// Protected, in use or unknown. Never actionable.
    Keep,
}

impl Safety {
    pub fn as_str(self) -> &'static str {
        match self {
            Safety::Safe => "safe",
            Safety::Review => "review",
            Safety::Keep => "keep",
        }
    }

    /// The stricter of two levels.
    pub fn stricter(self, other: Safety) -> Safety {
        self.max(other)
    }
}

impl fmt::Display for Safety {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Safety {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "safe" => Ok(Safety::Safe),
            "review" => Ok(Safety::Review),
            "keep" => Ok(Safety::Keep),
            _ => Err(format!(
                "unknown safety `{s}` (expected safe, review or keep)"
            )),
        }
    }
}

/// Stable machine codes for reasons (SPEC §7.3).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReasonCode {
    Regenerable,
    Redownloadable,
    InactiveProject,
    RecentlyUsed,
    RecentlyWritten,
    ActiveWriter,
    ToolRunning,
    InUseByContainer,
    UnusedByAnyEmulator,
    UsedByEmulator,
    ContainsUserData,
    ChatHistory,
    MayContainSecrets,
    LargeRedownload,
    OrphanedProject,
    MigratedCopy,
    BackupCopy,
    FreePages,
    ProtectedPath,
    OutsideRoots,
    UnknownFormat,
    NeedsElevation,
    NativeToolMissing,
    SharedHardlinks,
    PinnedByProject,
    /// Integration is experimental; safety capped at Review.
    Experimental,
    /// A user override in config changed the safety.
    UserOverride,
    /// Contains source files or a `.git` directory.
    ContainsSource,
}

impl ReasonCode {
    pub const ALL: [ReasonCode; 28] = [
        ReasonCode::Regenerable,
        ReasonCode::Redownloadable,
        ReasonCode::InactiveProject,
        ReasonCode::RecentlyUsed,
        ReasonCode::RecentlyWritten,
        ReasonCode::ActiveWriter,
        ReasonCode::ToolRunning,
        ReasonCode::InUseByContainer,
        ReasonCode::UnusedByAnyEmulator,
        ReasonCode::UsedByEmulator,
        ReasonCode::ContainsUserData,
        ReasonCode::ChatHistory,
        ReasonCode::MayContainSecrets,
        ReasonCode::LargeRedownload,
        ReasonCode::OrphanedProject,
        ReasonCode::MigratedCopy,
        ReasonCode::BackupCopy,
        ReasonCode::FreePages,
        ReasonCode::ProtectedPath,
        ReasonCode::OutsideRoots,
        ReasonCode::UnknownFormat,
        ReasonCode::NeedsElevation,
        ReasonCode::NativeToolMissing,
        ReasonCode::SharedHardlinks,
        ReasonCode::PinnedByProject,
        ReasonCode::Experimental,
        ReasonCode::UserOverride,
        ReasonCode::ContainsSource,
    ];

    /// The stable SCREAMING_SNAKE_CASE code.
    pub fn as_str(self) -> &'static str {
        match self {
            ReasonCode::Regenerable => "REGENERABLE",
            ReasonCode::Redownloadable => "REDOWNLOADABLE",
            ReasonCode::InactiveProject => "INACTIVE_PROJECT",
            ReasonCode::RecentlyUsed => "RECENTLY_USED",
            ReasonCode::RecentlyWritten => "RECENTLY_WRITTEN",
            ReasonCode::ActiveWriter => "ACTIVE_WRITER",
            ReasonCode::ToolRunning => "TOOL_RUNNING",
            ReasonCode::InUseByContainer => "IN_USE_BY_CONTAINER",
            ReasonCode::UnusedByAnyEmulator => "UNUSED_BY_ANY_EMULATOR",
            ReasonCode::UsedByEmulator => "USED_BY_EMULATOR",
            ReasonCode::ContainsUserData => "CONTAINS_USER_DATA",
            ReasonCode::ChatHistory => "CHAT_HISTORY",
            ReasonCode::MayContainSecrets => "MAY_CONTAIN_SECRETS",
            ReasonCode::LargeRedownload => "LARGE_REDOWNLOAD",
            ReasonCode::OrphanedProject => "ORPHANED_PROJECT",
            ReasonCode::MigratedCopy => "MIGRATED_COPY",
            ReasonCode::BackupCopy => "BACKUP_COPY",
            ReasonCode::FreePages => "FREE_PAGES",
            ReasonCode::ProtectedPath => "PROTECTED_PATH",
            ReasonCode::OutsideRoots => "OUTSIDE_ROOTS",
            ReasonCode::UnknownFormat => "UNKNOWN_FORMAT",
            ReasonCode::NeedsElevation => "NEEDS_ELEVATION",
            ReasonCode::NativeToolMissing => "NATIVE_TOOL_MISSING",
            ReasonCode::SharedHardlinks => "SHARED_HARDLINKS",
            ReasonCode::PinnedByProject => "PINNED_BY_PROJECT",
            ReasonCode::Experimental => "EXPERIMENTAL",
            ReasonCode::UserOverride => "USER_OVERRIDE",
            ReasonCode::ContainsSource => "CONTAINS_SOURCE",
        }
    }
}

impl fmt::Display for ReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ReasonCode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ReasonCode::ALL
            .into_iter()
            .find(|c| c.as_str() == s)
            .ok_or_else(|| format!("unknown reason code `{s}`"))
    }
}

/// Why an item has its safety level. One plain sentence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Reason {
    pub code: ReasonCode,
    pub text: String,
}

/// Where an item lives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Locator {
    Path { path: PathBuf },
    Paths { paths: Vec<PathBuf> },
    Docker { kind: String, id: String },
    Logical { store: String, key: String },
}

impl Locator {
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Locator::Path { path: path.into() }
    }

    /// The string hashed into the item id.
    pub fn canonical_key(&self) -> String {
        match self {
            Locator::Path { path } => format!("path:{}", path.to_string_lossy()),
            Locator::Paths { paths } => {
                let mut parts: Vec<_> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                parts.sort();
                format!("paths:{}", parts.join("\n"))
            }
            Locator::Docker { kind, id } => format!("docker:{kind}:{id}"),
            Locator::Logical { store, key } => format!("logical:{store}:{key}"),
        }
    }

    /// Filesystem paths the item occupies, if any.
    pub fn paths(&self) -> Vec<&PathBuf> {
        match self {
            Locator::Path { path } => vec![path],
            Locator::Paths { paths } => paths.iter().collect(),
            Locator::Docker { .. } | Locator::Logical { .. } => Vec::new(),
        }
    }

    /// The first path, used for display and project attribution.
    pub fn primary_path(&self) -> Option<&PathBuf> {
        self.paths().into_iter().next()
    }
}

/// Honest sizes (SPEC §5.5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Size {
    /// Sum of file lengths.
    pub apparent: u64,
    /// Allocated blocks.
    pub on_disk: u64,
    /// What deleting frees, after hardlink and shared-store accounting.
    pub reclaimable: u64,
    pub file_count: Option<u64>,
}

impl Size {
    /// A size where everything allocated is reclaimable.
    pub fn simple(apparent: u64, on_disk: u64) -> Self {
        Self {
            apparent,
            on_disk,
            reclaimable: on_disk,
            file_count: None,
        }
    }

    pub fn add(&mut self, other: &Size) {
        self.apparent += other.apparent;
        self.on_disk += other.on_disk;
        self.reclaimable += other.reclaimable;
        self.file_count = match (self.file_count, other.file_count) {
            (Some(a), Some(b)) => Some(a + b),
            (a, b) => a.or(b),
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LastUsedSource {
    GitCommit,
    GitIndex,
    SourceMtime,
    ItemMtime,
    ToolMetadata,
    Atime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LastUsed {
    #[schemars(with = "String")]
    pub at: Timestamp,
    pub source: LastUsedSource,
}

impl LastUsed {
    /// The newer of two optional values.
    pub fn newest(a: Option<LastUsed>, b: Option<LastUsed>) -> Option<LastUsed> {
        match (a, b) {
            (Some(a), Some(b)) => Some(if b.at > a.at { b } else { a }),
            (a, b) => a.or(b),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectRef {
    pub id: ProjectId,
    pub root: PathBuf,
    pub name: String,
}

/// Filesystem identity captured at scan time; re-checked before acting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Identity {
    /// Device and inode (Unix).
    Inode { dev: u64, ino: u64 },
    /// Creation time in nanoseconds since the epoch (Windows fallback).
    Created { nanos: i128 },
}

/// One reclaimable thing reported by an integration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Item {
    pub id: ItemId,
    pub integration: IntegrationId,
    pub category: Category,
    /// Integration-specific kind, e.g. `node_modules`, `docker_image`.
    pub kind: String,
    /// Human label.
    pub title: String,
    pub locator: Locator,
    pub size: Size,
    pub project: Option<ProjectRef>,
    pub last_used: Option<LastUsed>,
    pub safety: Safety,
    /// At least one.
    pub reasons: Vec<Reason>,
    /// Allowed actions, preferred first. Empty when `safety` is `Keep`.
    pub actions: Vec<ActionKind>,
    /// Integration-specific extras.
    #[serde(default)]
    pub details: serde_json::Value,
    #[serde(default)]
    pub notes: Vec<String>,
    /// Command that regenerates the item, shown by `explain`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub restore_hint: Option<String>,
    /// Identity of the primary path at scan time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
}

impl Item {
    pub fn has_reason(&self, code: ReasonCode) -> bool {
        self.reasons.iter().any(|r| r.code == code)
    }
}
