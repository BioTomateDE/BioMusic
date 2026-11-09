// ai slop
//! Serde support for `Option<Duration>` written as a string in one of two forms:
//!
//! - clock, whole numbers only: `"02:12"` (mm:ss) or `"1:02:03"` (h:mm:ss)
//! - decimal number with a unit: `"52.745s"`, `"38425.5ms"`, `"1500us"` (also `µs`)
//!
//! Plain numbers (`52.7`) and unit-less strings are rejected on purpose.
//!
//! Serializing writes `"mm:ss"` (or `"h:mm:ss"` from one hour up) for whole seconds,
//! and `"<seconds>s"` with an exact fraction otherwise, e.g. `"52.745s"`.
//! Values round-trip losslessly (nanosecond precision).
//!
//! ```ignore
//! #[derive(Serialize, Deserialize)]
//! struct Config {
//!     #[serde(default, with = "crate::duration_secs", skip_serializing_if = "Option::is_none")]
//!     end: Option<Duration>,
//! }
//! ```

use std::fmt;
use std::time::Duration;

use serde::Serializer;
use serde::de::{self, Deserializer, Visitor};

const NANOS_PER_SEC: u128 = 1_000_000_000;

/// Parse `"02:12"`, `"1:02:03"`, `"52.745s"`, `"38425.5ms"` or `"1500us"`.
/// Digits beyond nanosecond precision are truncated.
pub fn parse_duration(input: &str) -> Result<Duration, String> {
    let s = input.trim();
    if s.contains(':') {
        parse_clock(input, s)
    } else {
        parse_with_unit(input, s)
    }
}

fn expected(input: &str) -> String {
    format!(
        "invalid duration {input:?}: expected \"mm:ss\" / \"h:mm:ss\" (whole numbers) \
         or a number with unit s, ms or us (e.g. \"52.745s\")"
    )
}

fn parse_digits(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn parse_clock(input: &str, s: &str) -> Result<Duration, String> {
    let nums: Option<Vec<u64>> = s.split(':').map(parse_digits).collect();
    let nums = nums.ok_or_else(|| expected(input))?;

    let secs = match nums[..] {
        [m, sec] if sec < 60 => m.checked_mul(60).and_then(|v| v.checked_add(sec)),
        [h, m, sec] if m < 60 && sec < 60 => h
            .checked_mul(60)
            .and_then(|v| v.checked_add(m))
            .and_then(|v| v.checked_mul(60))
            .and_then(|v| v.checked_add(sec)),
        _ => {
            return Err(format!(
                "invalid duration {input:?}: seconds and minutes must be below 60"
            ));
        }
    };
    secs.map(Duration::from_secs)
        .ok_or_else(|| format!("duration {input:?} is too large"))
}

fn parse_with_unit(input: &str, s: &str) -> Result<Duration, String> {
    // Longer suffixes first: "s" is a suffix of "ms" and "us".
    let (number, unit_nanos): (&str, u128) = if let Some(n) = s.strip_suffix("ms") {
        (n, 1_000_000)
    } else if let Some(n) = s.strip_suffix("us").or_else(|| s.strip_suffix("µs")) {
        (n, 1_000)
    } else if let Some(n) = s.strip_suffix('s') {
        (n, 1_000_000_000)
    } else {
        return Err(expected(input));
    };
    let number = number.trim_end();

    // Exact decimal arithmetic (no f64), so "38425.5ms" is exactly 38.4255 s.
    let (int_s, frac_s) = number.split_once('.').unwrap_or((number, ""));
    if int_s.is_empty() && frac_s.is_empty() {
        return Err(expected(input));
    }
    let int: u128 = if int_s.is_empty() {
        0
    } else {
        parse_digits(int_s).ok_or_else(|| expected(input))?.into()
    };
    if !frac_s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(expected(input));
    }
    let frac_digits = &frac_s[..frac_s.len().min(18)];
    let frac: u128 = if frac_digits.is_empty() {
        0
    } else {
        frac_digits.parse().map_err(|_| expected(input))?
    };

    let total_nanos = int * unit_nanos + frac * unit_nanos / 10u128.pow(frac_digits.len() as u32);
    let secs = u64::try_from(total_nanos / NANOS_PER_SEC).map_err(|_| format!("duration {input:?} is too large"))?;
    Ok(Duration::new(secs, (total_nanos % NANOS_PER_SEC) as u32))
}

/// `"mm:ss"` (or `"h:mm:ss"` from one hour up) for whole seconds, `"<secs>.<frac>s"` otherwise.
pub fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    match d.subsec_nanos() {
        0 if total < 3600 => format!("{:02}:{:02}", total / 60, total % 60),
        0 => format!("{}:{:02}:{:02}", total / 3600, total % 3600 / 60, total % 60),
        n => format!("{total}.{}s", format!("{n:09}").trim_end_matches('0')),
    }
}

struct StrVisitor;

impl Visitor<'_> for StrVisitor {
    type Value = Duration;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a string like \"02:12\", \"52.745s\", \"38425.5ms\" or \"1500us\"")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Duration, E> {
        parse_duration(v).map_err(E::custom)
    }
}

struct OptionVisitor;

impl<'de> Visitor<'de> for OptionVisitor {
    type Value = Option<Duration>;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an optional duration string like \"02:12\" or \"52.745s\"")
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        Ok(None)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_str(StrVisitor).map(Some)
    }
}

/// Use as `#[serde(default, with = "crate::duration_secs", skip_serializing_if = "Option::is_none")]`
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Duration>, D::Error> {
    d.deserialize_option(OptionVisitor)
}

pub fn serialize<S: Serializer>(d: &Option<Duration>, s: S) -> Result<S::Ok, S::Error> {
    match d {
        Some(d) => s.serialize_some(&format_duration(*d)),
        None => s.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Cfg {
        #[serde(default, with = "super", skip_serializing_if = "Option::is_none")]
        end: Option<Duration>,
    }

    fn cfg(end: Option<Duration>) -> Cfg {
        Cfg { end }
    }

    #[test]
    fn clock_forms() {
        assert_eq!(parse_duration("02:12").unwrap(), Duration::from_secs(132));
        assert_eq!(parse_duration("2:12").unwrap(), Duration::from_secs(132));
        assert_eq!(parse_duration("00:06").unwrap(), Duration::from_secs(6));
        assert_eq!(parse_duration("90:00").unwrap(), Duration::from_secs(5400));
        assert_eq!(parse_duration("1:02:03").unwrap(), Duration::from_secs(3723));
        assert_eq!(parse_duration(" 02:12 ").unwrap(), Duration::from_secs(132));
    }

    #[test]
    fn unit_forms_are_exact() {
        assert_eq!(parse_duration("52.745s").unwrap(), Duration::new(52, 745_000_000));
        assert_eq!(parse_duration("38425.5ms").unwrap(), Duration::new(38, 425_500_000));
        assert_eq!(parse_duration("1500us").unwrap(), Duration::from_micros(1500));
        assert_eq!(parse_duration("1.5µs").unwrap(), Duration::from_nanos(1500));
        assert_eq!(parse_duration("2.5 s").unwrap(), Duration::from_millis(2500));
        assert_eq!(parse_duration(".5s").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("52s").unwrap(), Duration::from_secs(52));
        assert_eq!(parse_duration("0.000000001s").unwrap(), Duration::from_nanos(1));
    }

    #[test]
    fn rejects_everything_else() {
        for bad in [
            "", "s", "ms", ".s", "52", "52.5", "02:12.5", "1:75", "1:75:00", "1:2:3:4", "02:", ":12", "5m", "5h",
            "-5s", "1e3s", "1.2.3s", "NaN s", "inf s", "1:-5", "1.5:30",
        ] {
            assert!(parse_duration(bad).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn toml_deserialize() {
        assert_eq!(
            toml::from_str::<Cfg>("end = \"02:12\"").unwrap(),
            cfg(Some(Duration::from_secs(132)))
        );
        assert_eq!(
            toml::from_str::<Cfg>("end = \"38425.5ms\"").unwrap(),
            cfg(Some(Duration::new(38, 425_500_000)))
        );
        assert_eq!(toml::from_str::<Cfg>("").unwrap(), cfg(None));

        // Bare numbers are not allowed.
        for bad in ["end = 52.7", "end = 5", "end = true"] {
            let err = toml::from_str::<Cfg>(bad).unwrap_err().to_string();
            assert!(err.contains("a string like"), "{bad}: {err}");
        }
        let err = toml::from_str::<Cfg>("end = \"52\"").unwrap_err().to_string();
        assert!(err.contains("expected"), "{err}");
    }

    #[test]
    fn formatting() {
        assert_eq!(format_duration(Duration::from_secs(132)), "02:12");
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00");
        assert_eq!(format_duration(Duration::from_secs(3723)), "1:02:03");
        assert_eq!(format_duration(Duration::new(52, 745_000_000)), "52.745s");
        assert_eq!(format_duration(Duration::from_millis(500)), "0.5s");
        assert_eq!(format_duration(Duration::from_nanos(1)), "0.000000001s");
    }

    #[test]
    fn toml_serialize_roundtrip() {
        for d in [
            Duration::from_secs(132),
            Duration::from_secs(3723),
            Duration::new(52, 745_000_000),
            Duration::new(3, 1),
            Duration::from_micros(1500),
        ] {
            let c = cfg(Some(d));
            let text = toml::to_string(&c).unwrap();
            assert_eq!(toml::from_str::<Cfg>(&text).unwrap(), c, "{text}");
        }
        assert_eq!(
            toml::to_string(&cfg(Some(Duration::from_secs(132)))).unwrap(),
            "end = \"02:12\"\n"
        );
        assert_eq!(
            toml::to_string(&cfg(Some(Duration::new(52, 745_000_000)))).unwrap(),
            "end = \"52.745s\"\n"
        );
        assert_eq!(toml::to_string(&cfg(None)).unwrap(), "");
    }
}
