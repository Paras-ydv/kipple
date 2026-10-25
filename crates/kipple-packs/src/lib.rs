//! Pack (TOML) loader and generic pack engine (INTEGRATIONS.md §2).

mod engine;
mod enrich;
pub mod schema;
mod validate;

use std::sync::Arc;

use kipple_core::integration::Adapter;

pub use engine::{PackAdapter, Vars, expand_glob};
pub use schema::Pack;
pub use validate::{parse_and_validate, validate};

mod builtin {
    include!(concat!(env!("OUT_DIR"), "/builtin.rs"));
}

#[derive(Debug, thiserror::Error)]
#[error("built-in pack {file} is invalid: {}", errors.join("; "))]
pub struct InvalidPack {
    pub file: String,
    pub errors: Vec<String>,
}

/// Raw text of every built-in pack, by file name.
pub fn builtin_sources() -> &'static [(&'static str, &'static str)] {
    builtin::BUILTIN
}

/// Every built-in pack, parsed and validated.
pub fn builtin_packs() -> Result<Vec<Pack>, InvalidPack> {
    builtin::BUILTIN
        .iter()
        .map(|(file, text)| {
            parse_and_validate(text).map_err(|errors| InvalidPack {
                file: (*file).to_owned(),
                errors,
            })
        })
        .collect()
}

/// Built-in packs as adapters.
pub fn builtin() -> Result<Vec<Arc<dyn Adapter>>, InvalidPack> {
    Ok(builtin_packs()?
        .into_iter()
        .map(|p| Arc::new(PackAdapter::new(p)) as Arc<dyn Adapter>)
        .collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_builtin_pack_is_valid_and_unique() {
        let packs = super::builtin_packs().unwrap();
        let mut ids: Vec<&str> = packs.iter().map(|p| p.id.as_str()).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "duplicate pack ids");
        for (file, _) in super::builtin_sources() {
            let pack = packs.iter().find(|p| format!("{}.toml", p.id) == *file);
            assert!(pack.is_some(), "{file} must be named after its id");
        }
    }
}
