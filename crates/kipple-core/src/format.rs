//! Human formats shared by every interface (SPEC §11.3): sizes, ages, and
//! parsing of `--min-size` / `--older-than` values.

use crate::config::Units;

/// `12.4 GB` with 3 significant digits; bytes below 1 kB as `512 B`.
pub fn size(bytes: u64, units: Units) -> String {
    let (base, names): (f64, [&str; 6]) = match units {
        Units::Si => (1000.0, ["B", "kB", "MB", "GB", "TB", "PB"]),
        Units::Iec => (1024.0, ["B", "KiB", "MiB", "GiB", "TiB", "PiB"]),
    };
    if (bytes as f64) < base {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut idx = 0;
    while value >= base && idx < names.len() - 1 {
        value /= base;
        idx += 1;
    }
    // Rounding can carry to the next unit (999.96 kB → 1.00 MB).
    if value >= 999.5 && idx < names.len() - 1 {
        value /= base;
        idx += 1;
    }
    let text = if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    };
    format!("{text} {}", names[idx])
}

/// Relative age: `just now`, `5 minutes`, `3 days`, `7 months`, `2 years`.
pub fn age(secs: i64) -> String {
    let secs = secs.max(0);
    let plural = |n: i64, unit: &str| {
        if n == 1 {
            format!("1 {unit}")
        } else {
            format!("{n} {unit}s")
        }
    };
    match secs {
        s if s < 60 => "just now".to_owned(),
        s if s < 3600 => plural(s / 60, "minute"),
        s if s < 86_400 => plural(s / 3600, "hour"),
        s if s < 30 * 86_400 => plural(s / 86_400, "day"),
        s if s < 365 * 86_400 => plural(s / (30 * 86_400), "month"),
        s => plural(s / (365 * 86_400), "year"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ParseError(String);

/// Parses `500MB`, `2GB`, `1.5 GiB`, `4096`. Units are case-insensitive.
pub fn parse_size(s: &str) -> Result<u64, ParseError> {
    let t = s.trim();
    let split = t
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(t.len());
    let (num, unit) = t.split_at(split);
    let n: f64 = num
        .parse()
        .map_err(|_| ParseError(format!("`{s}` isn't a size (try 500MB or 2GB)")))?;
    let mult: f64 = match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" => 1.0,
        "k" | "kb" => 1e3,
        "m" | "mb" => 1e6,
        "g" | "gb" => 1e9,
        "t" | "tb" => 1e12,
        "kib" => 1024.0,
        "mib" => 1024f64.powi(2),
        "gib" => 1024f64.powi(3),
        "tib" => 1024f64.powi(4),
        other => return Err(ParseError(format!("unknown size unit `{other}` in `{s}`"))),
    };
    Ok((n * mult).round() as u64)
}

/// Parses `30d`, `6m` (months), `2w`, `1y`, `12h` into seconds.
pub fn parse_age(s: &str) -> Result<u64, ParseError> {
    let t = s.trim();
    let split = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    let (num, unit) = t.split_at(split);
    let n: u64 = num
        .parse()
        .map_err(|_| ParseError(format!("`{s}` isn't an age (try 30d, 6m or 1y)")))?;
    let unit_secs = match unit.trim() {
        "h" => 3600,
        "d" | "" => 86_400,
        "w" => 7 * 86_400,
        "m" | "mo" => 30 * 86_400,
        "y" => 365 * 86_400,
        other => {
            return Err(ParseError(format!(
                "unknown age unit `{other}` in `{s}` (use h, d, w, m or y)"
            )));
        }
    };
    Ok(n * unit_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_have_three_significant_digits() {
        assert_eq!(size(0, Units::Si), "0 B");
        assert_eq!(size(999, Units::Si), "999 B");
        assert_eq!(size(1_000, Units::Si), "1.00 kB");
        assert_eq!(size(12_400_000_000, Units::Si), "12.4 GB");
        assert_eq!(size(184_000_000_000, Units::Si), "184 GB");
        assert_eq!(size(999_960, Units::Si), "1.00 MB");
        assert_eq!(size(1_073_741_824, Units::Iec), "1.00 GiB");
    }

    #[test]
    fn ages() {
        assert_eq!(age(10), "just now");
        assert_eq!(age(3 * 86_400), "3 days");
        assert_eq!(age(86_400), "1 day");
        assert_eq!(age(7 * 30 * 86_400), "7 months");
        assert_eq!(age(800 * 86_400), "2 years");
    }

    #[test]
    fn parses_sizes_and_ages() {
        assert_eq!(parse_size("500MB").unwrap(), 500_000_000);
        assert_eq!(parse_size("1.5 GiB").unwrap(), 1_610_612_736);
        assert_eq!(parse_size("4096").unwrap(), 4096);
        assert!(parse_size("lots").is_err());
        assert_eq!(parse_age("30d").unwrap(), 30 * 86_400);
        assert_eq!(parse_age("6m").unwrap(), 180 * 86_400);
        assert!(parse_age("3x").is_err());
    }
}
