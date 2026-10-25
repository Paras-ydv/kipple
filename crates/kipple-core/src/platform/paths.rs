//! Path expansion and normalization (SPEC §16).

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use crate::platform::Env;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExpandError {
    #[error("`{0}` is not set")]
    UnsetVariable(String),
    #[error("unknown path variable `{{{0}}}`")]
    UnknownVar(String),
    #[error("home directory is unknown")]
    NoHome,
}

/// Default for XDG variables when unset.
fn xdg_default(env: &Env, name: &str) -> Option<PathBuf> {
    let home = env.home()?;
    Some(match name {
        "XDG_CONFIG_HOME" => home.join(".config"),
        "XDG_DATA_HOME" => home.join(".local/share"),
        "XDG_CACHE_HOME" => home.join(".cache"),
        "XDG_STATE_HOME" => home.join(".local/state"),
        _ => return None,
    })
}

fn lookup(env: &Env, name: &str) -> Result<String, ExpandError> {
    if let Some(v) = env.get_str(name) {
        return Ok(v.to_owned());
    }
    if let Some(p) = xdg_default(env, name) {
        return Ok(p.to_string_lossy().into_owned());
    }
    if (name == "HOME" || name == "USERPROFILE")
        && let Some(h) = env.home()
    {
        return Ok(h.to_string_lossy().into_owned());
    }
    Err(ExpandError::UnsetVariable(name.to_owned()))
}

fn is_var_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Expands `~`, `$VAR`, `${VAR}`, `%VAR%` and `{var}` (from `vars`).
///
/// Unset environment variables are an error, so a path is never silently
/// resolved to somewhere unexpected.
pub fn expand(
    input: &str,
    env: &Env,
    vars: &BTreeMap<String, PathBuf>,
) -> Result<PathBuf, ExpandError> {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    if rest == "~" || rest.starts_with("~/") || rest.starts_with("~\\") {
        let home = env.home().ok_or(ExpandError::NoHome)?;
        out.push_str(&home.to_string_lossy());
        rest = &rest[1..];
    }
    let chars: Vec<char> = rest.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '$' => {
                if chars.get(i + 1) == Some(&'{') {
                    let end = chars[i + 2..].iter().position(|&c| c == '}');
                    if let Some(end) = end {
                        let name: String = chars[i + 2..i + 2 + end].iter().collect();
                        out.push_str(&lookup(env, &name)?);
                        i += end + 3;
                        continue;
                    }
                } else {
                    let len = chars[i + 1..]
                        .iter()
                        .take_while(|c| is_var_char(**c))
                        .count();
                    if len > 0 {
                        let name: String = chars[i + 1..i + 1 + len].iter().collect();
                        out.push_str(&lookup(env, &name)?);
                        i += len + 1;
                        continue;
                    }
                }
                out.push(c);
                i += 1;
            }
            '%' => {
                let end = chars[i + 1..].iter().position(|&c| c == '%');
                match end {
                    Some(end)
                        if end > 0 && chars[i + 1..i + 1 + end].iter().all(|c| is_var_char(*c)) =>
                    {
                        let name: String = chars[i + 1..i + 1 + end].iter().collect();
                        out.push_str(&lookup(env, &name)?);
                        i += end + 2;
                    }
                    _ => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            '{' => {
                let end = chars[i + 1..].iter().position(|&c| c == '}');
                match end {
                    Some(end)
                        if end > 0 && chars[i + 1..i + 1 + end].iter().all(|c| is_var_char(*c)) =>
                    {
                        let name: String = chars[i + 1..i + 1 + end].iter().collect();
                        let value = vars
                            .get(&name)
                            .ok_or_else(|| ExpandError::UnknownVar(name.clone()))?;
                        out.push_str(&value.to_string_lossy());
                        i += end + 2;
                    }
                    _ => {
                        out.push(c);
                        i += 1;
                    }
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    Ok(normalize_separators(&out))
}

/// Uses the platform separator.
pub fn normalize_separators(s: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(s.replace('/', "\\"))
    } else {
        PathBuf::from(s)
    }
}

/// Removes `.` and resolves `..` lexically, without touching the filesystem.
pub fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Canonicalizes everything but the last component, so a symlink at `path`
/// itself is not followed. Falls back to lexical normalization for parts that
/// don't exist.
pub fn canonicalize_parent(path: &Path) -> PathBuf {
    let path = lexical_normalize(path);
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => {
            match parent.canonicalize() {
                Ok(p) => strip_verbatim(p).join(name),
                Err(_) => canonicalize_parent(parent).join(name),
            }
        }
        _ => path,
    }
}

/// Removes Windows `\\?\` prefixes produced by `canonicalize`.
pub fn strip_verbatim(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

/// Replaces the home prefix with `~` for display.
pub fn shorten_home(path: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home
        && let Ok(rest) = path.strip_prefix(home)
    {
        if rest.as_os_str().is_empty() {
            return "~".to_owned();
        }
        let sep = std::path::MAIN_SEPARATOR;
        return format!("~{sep}{}", rest.display());
    }
    path.display().to_string()
}

/// Whether paths on this OS compare case-insensitively by default.
pub const CASE_INSENSITIVE: bool = cfg!(any(target_os = "macos", windows));

fn comparable(path: &Path) -> String {
    let s = lexical_normalize(path).to_string_lossy().into_owned();
    let s = if cfg!(windows) {
        s.replace('/', "\\")
    } else {
        s
    };
    if CASE_INSENSITIVE {
        s.to_lowercase()
    } else {
        s
    }
}

/// `path` equals `base` or lies inside it, comparing lexically and
/// case-insensitively where the default filesystem is.
pub fn is_within(path: &Path, base: &Path) -> bool {
    let p = comparable(path);
    let b = comparable(base);
    let b = b.trim_end_matches(std::path::MAIN_SEPARATOR);
    if p == b {
        return true;
    }
    p.strip_prefix(b)
        .is_some_and(|rest| rest.starts_with(std::path::MAIN_SEPARATOR))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Env {
        Env::from_pairs([
            ("HOME", "/home/u"),
            ("USERPROFILE", "/home/u"),
            ("FOO", "/opt/foo"),
        ])
    }

    fn vars() -> BTreeMap<String, PathBuf> {
        BTreeMap::from([("cache".to_owned(), PathBuf::from("/c"))])
    }

    #[test]
    fn expands_all_forms() {
        let e = env();
        let v = vars();
        assert_eq!(
            expand("~/.npm", &e, &v).unwrap(),
            normalize_separators("/home/u/.npm")
        );
        assert_eq!(
            expand("$FOO/x", &e, &v).unwrap(),
            normalize_separators("/opt/foo/x")
        );
        assert_eq!(
            expand("${FOO}x", &e, &v).unwrap(),
            normalize_separators("/opt/foox")
        );
        assert_eq!(
            expand("%FOO%/x", &e, &v).unwrap(),
            normalize_separators("/opt/foo/x")
        );
        assert_eq!(
            expand("{cache}/y", &e, &v).unwrap(),
            normalize_separators("/c/y")
        );
    }

    #[test]
    fn xdg_defaults_to_home() {
        assert_eq!(
            expand("$XDG_CACHE_HOME/pip", &env(), &vars()).unwrap(),
            normalize_separators("/home/u/.cache/pip")
        );
    }

    #[test]
    fn unset_variable_is_an_error() {
        assert_eq!(
            expand("$NOPE/x", &env(), &vars()),
            Err(ExpandError::UnsetVariable("NOPE".into()))
        );
        assert_eq!(
            expand("{nope}", &env(), &vars()),
            Err(ExpandError::UnknownVar("nope".into()))
        );
    }

    #[test]
    fn literal_percent_and_braces_survive() {
        assert_eq!(
            expand("a%b c%{x y}", &env(), &vars()).unwrap(),
            normalize_separators("a%b c%{x y}")
        );
    }

    #[test]
    fn lexical_normalize_resolves_dots() {
        assert_eq!(
            lexical_normalize(Path::new("/a/./b/../c")),
            PathBuf::from("/a/c")
        );
    }

    #[test]
    fn within_checks_component_boundaries() {
        assert!(is_within(Path::new("/a/b/c"), Path::new("/a/b")));
        assert!(is_within(Path::new("/a/b"), Path::new("/a/b/")));
        assert!(!is_within(Path::new("/a/bc"), Path::new("/a/b")));
        assert!(!is_within(Path::new("/a"), Path::new("/a/b")));
    }

    #[test]
    fn shorten_home_uses_tilde() {
        let home = Path::new("/home/u");
        assert_eq!(shorten_home(Path::new("/home/u"), Some(home)), "~");
        assert_eq!(
            shorten_home(Path::new("/home/u/x"), Some(home)),
            format!("~{}x", std::path::MAIN_SEPARATOR)
        );
        assert_eq!(shorten_home(Path::new("/other"), Some(home)), "/other");
    }
}
