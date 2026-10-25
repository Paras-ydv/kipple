//! Helpers shared by the AI history adapters (INTEGRATIONS.md §4.10–4.13).

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, SystemTime};

use kipple_core::model::{LastUsed, LastUsedSource, Reason, ReasonCode, Timestamp};
use kipple_core::safety::reasons::{reason, reason_with};

use crate::secrets;

/// Titles are cut to this many characters (SPEC §11.3).
pub const TITLE_CHARS: usize = 60;

/// How many lines and bytes we read from the start of a transcript looking
/// for metadata. Content past that is never read.
pub const HEAD_LINES: usize = 400;
pub const HEAD_BYTES: u64 = 4 * 1024 * 1024;

/// Bytes read from the end of a transcript to find the last timestamp.
const TAIL_BYTES: u64 = 64 * 1024;

/// A one-line, secret-masked title of at most [`TITLE_CHARS`] characters.
/// Returns `None` for empty text.
pub fn title(text: &str) -> Option<String> {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return None;
    }
    let masked = secrets::mask(&flat);
    if masked.chars().count() <= TITLE_CHARS {
        return Some(masked);
    }
    let mut out: String = masked.chars().take(TITLE_CHARS - 1).collect();
    out = out.trim_end().to_owned();
    out.push('…');
    Some(out)
}

/// The reasons every chat-history item carries (SPEC §7.5).
pub fn chat_reasons() -> Vec<Reason> {
    vec![
        reason(ReasonCode::ChatHistory),
        reason(ReasonCode::MayContainSecrets),
    ]
}

/// A `TOOL_RUNNING` reason with its own text.
pub fn tool_running(text: &str) -> Reason {
    Reason {
        code: ReasonCode::ToolRunning,
        text: text.to_owned(),
    }
}

/// A `NATIVE_TOOL_MISSING` reason for `tool`.
pub fn tool_missing(tool: &str) -> Reason {
    reason_with(ReasonCode::NativeToolMissing, &[("tool", tool.to_owned())])
}

/// Parses an RFC 3339 timestamp such as `2026-01-15T10:00:00.000Z`.
pub fn parse_ts(s: &str) -> Option<Timestamp> {
    s.parse::<Timestamp>().ok()
}

pub fn tool_used(at: Timestamp) -> LastUsed {
    LastUsed {
        at,
        source: LastUsedSource::ToolMetadata,
    }
}

/// Short id for titles: the first 8 characters.
pub fn short_id(id: &str) -> String {
    id.chars().take(8).collect()
}

/// `YYYY-MM-DD` of a timestamp, in UTC.
pub fn day(ts: Timestamp) -> String {
    ts.strftime("%Y-%m-%d").to_string()
}

pub fn mtime(path: &Path) -> Option<SystemTime> {
    path.symlink_metadata().and_then(|m| m.modified()).ok()
}

/// Seconds since `t` (0 for times in the future).
pub fn age_secs(t: SystemTime) -> u64 {
    SystemTime::now()
        .duration_since(t)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Whether any of `paths` is modified within `within`.
pub fn modified_within(path: &Path, within: Duration) -> bool {
    mtime(path).is_some_and(|t| age_secs(t) < within.as_secs())
}

/// Reads JSON lines from the start of `path`, calling `f` for each parsed
/// object until it returns `false` or the head limits are reached.
/// Returns how many lines parsed as JSON objects.
pub fn scan_head(path: &Path, mut f: impl FnMut(&serde_json::Value) -> bool) -> usize {
    let Ok(file) = File::open(path) else {
        return 0;
    };
    let reader = BufReader::new(file.take(HEAD_BYTES));
    let mut parsed = 0;
    for line in reader.lines().take(HEAD_LINES) {
        let Ok(line) = line else { break };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if !v.is_object() {
            continue;
        }
        parsed += 1;
        if !f(&v) {
            break;
        }
    }
    parsed
}

/// The newest `timestamp` field among the last lines of a JSONL file.
pub fn last_timestamp(path: &Path) -> Option<Timestamp> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(TAIL_BYTES);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    file.take(TAIL_BYTES).read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    text.lines()
        .rev()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find_map(|v| v.get("timestamp")?.as_str().and_then(parse_ts))
}

/// File sizes of `paths`, then again after `delay`. True when any grew,
/// which means something is writing to it right now (SPEC §7.2).
pub fn growing(paths: &[&Path], delay: Duration) -> Vec<bool> {
    let len = |p: &Path| p.metadata().map(|m| m.len()).unwrap_or(0);
    let before: Vec<u64> = paths.iter().map(|p| len(p)).collect();
    if !paths.is_empty() && !delay.is_zero() {
        std::thread::sleep(delay);
    }
    paths.iter().zip(before).map(|(p, b)| len(p) > b).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_flat_masked_and_short() {
        assert_eq!(title("  fix\n the   bug ").as_deref(), Some("fix the bug"));
        assert_eq!(title(" \n "), None);
        let long = "a ".repeat(100);
        let t = title(&long).unwrap();
        assert!(t.chars().count() <= TITLE_CHARS);
        assert!(t.ends_with('…'));
        assert_eq!(
            title("deploy with AKIAIOSFODNN7EXAMPLE now").as_deref(),
            Some("deploy with AKIA**** now")
        );
    }

    #[test]
    fn parses_timestamps() {
        let t = parse_ts("2026-01-15T10:00:00.000Z").unwrap();
        assert_eq!(day(t), "2026-01-15");
        assert!(parse_ts("yesterday").is_none());
    }
}
