use std::fmt;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Locator;

macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
        )]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

string_id!(
    /// Stable integration id, e.g. `npm-cache`, `docker`, `codex`.
    IntegrationId
);
string_id!(
    /// `blake3(integration_id + "\0" + canonical_locator)`, first 16 bytes, hex.
    ItemId
);
string_id!(
    /// `blake3(canonical project root)`, first 16 bytes, hex.
    ProjectId
);
string_id!(
    /// ULID of a scan.
    ScanId
);
string_id!(
    /// ULID of a plan.
    PlanId
);
string_id!(
    /// ULID of a quarantine entry.
    QuarantineId
);

fn short_hash(parts: &[&[u8]]) -> String {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part);
    }
    let hash = hasher.finalize();
    hash.as_bytes()[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

impl ItemId {
    pub fn new(integration: &IntegrationId, locator: &Locator) -> Self {
        let key = locator.canonical_key();
        Self(short_hash(&[
            integration.0.as_bytes(),
            b"\0",
            key.as_bytes(),
        ]))
    }
}

impl ProjectId {
    pub fn for_root(root: &Path) -> Self {
        Self(short_hash(&[&path_bytes(root)]))
    }
}

impl ScanId {
    pub fn generate() -> Self {
        Self(ulid::Ulid::generate().to_string())
    }
}

impl PlanId {
    pub fn generate() -> Self {
        Self(ulid::Ulid::generate().to_string())
    }
}

impl QuarantineId {
    pub fn generate() -> Self {
        Self(ulid::Ulid::generate().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn item_id_is_stable_and_32_hex_chars() {
        let id = ItemId::new(
            &IntegrationId::from("npm-cache"),
            &Locator::Path {
                path: PathBuf::from("/home/a/.npm/_cacache"),
            },
        );
        assert_eq!(id.0.len(), 32);
        assert!(id.0.chars().all(|c| c.is_ascii_hexdigit()));
        let again = ItemId::new(
            &IntegrationId::from("npm-cache"),
            &Locator::Path {
                path: PathBuf::from("/home/a/.npm/_cacache"),
            },
        );
        assert_eq!(id, again);
    }

    #[test]
    fn item_id_depends_on_integration() {
        let loc = Locator::Path {
            path: PathBuf::from("/x"),
        };
        assert_ne!(
            ItemId::new(&IntegrationId::from("a"), &loc),
            ItemId::new(&IntegrationId::from("b"), &loc)
        );
    }
}
