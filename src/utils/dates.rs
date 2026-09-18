//! Parsing of user-supplied dates.
//!
//! Accepted forms:
//! * `YYYY-MM-DD HH:MM:SS` — interpreted as UTC (offset `+0000`)
//! * `YYYY-MM-DD HH:MM:SS +HH:MM` / `+HHMM` / `Z` — explicit offset
//! * RFC 3339, e.g. `2024-01-01T10:00:00+05:30`

use crate::rewrite::engine::GitTime;
use crate::utils::types::Result;
use chrono::{DateTime, FixedOffset, NaiveDateTime};

pub const DATE_FORMAT_HELP: &str =
    "YYYY-MM-DD HH:MM:SS (UTC), optionally followed by an offset like +05:30, or RFC 3339";

/// Parse a date into a git timestamp (seconds + offset).
pub fn parse_git_time(input: &str) -> Result<GitTime> {
    let input = input.trim();
    if let Ok(naive) = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
        return Ok(GitTime::new(naive.and_utc().timestamp(), 0));
    }
    let with_offset = parse_with_offset(input)
        .or_else(|| DateTime::parse_from_rfc3339(input).ok())
        .ok_or_else(|| format!("Invalid date '{input}' (expected {DATE_FORMAT_HELP})"))?;
    Ok(GitTime::new(
        with_offset.timestamp(),
        with_offset.offset().local_minus_utc() / 60,
    ))
}

/// The instant as naive UTC, for display and range arithmetic.
pub fn parse_utc(input: &str) -> Result<NaiveDateTime> {
    let time = parse_git_time(input)?;
    Ok(DateTime::from_timestamp(time.seconds, 0)
        .ok_or("date out of range")?
        .naive_utc())
}

/// Render a git timestamp as `YYYY-MM-DD HH:MM:SS +HHMM` in its own offset.
pub fn format_git_time(time: GitTime) -> String {
    FixedOffset::east_opt(time.offset_minutes * 60)
        .and_then(|tz| DateTime::from_timestamp(time.seconds, 0).map(|t| t.with_timezone(&tz)))
        .map_or_else(
            || time.seconds.to_string(),
            |t| t.format("%Y-%m-%d %H:%M:%S %z").to_string(),
        )
}

fn parse_with_offset(input: &str) -> Option<DateTime<FixedOffset>> {
    let normalized = input
        .strip_suffix('Z')
        .map(|s| format!("{} +00:00", s.trim_end()));
    let input = normalized.as_deref().unwrap_or(input);
    ["%Y-%m-%d %H:%M:%S %:z", "%Y-%m-%d %H:%M:%S %z"]
        .iter()
        .find_map(|fmt| DateTime::parse_from_str(input, fmt).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_dates_are_utc() {
        let t = parse_git_time("2024-01-01 00:00:00").unwrap();
        assert_eq!(t, GitTime::new(1_704_067_200, 0));
    }

    #[test]
    fn explicit_offsets_are_kept() {
        let expected = GitTime::new(1_704_067_200 - 330 * 60, 330);
        assert_eq!(
            parse_git_time("2024-01-01 00:00:00 +05:30").unwrap(),
            expected
        );
        assert_eq!(
            parse_git_time("2024-01-01 00:00:00 +0530").unwrap(),
            expected
        );
        assert_eq!(
            parse_git_time("2024-01-01T00:00:00+05:30").unwrap(),
            expected
        );
        assert_eq!(
            parse_git_time("2024-01-01 00:00:00 -05:00").unwrap(),
            GitTime::new(1_704_067_200 + 300 * 60, -300)
        );
        assert_eq!(
            parse_git_time("2024-01-01 00:00:00Z").unwrap(),
            GitTime::new(1_704_067_200, 0)
        );
    }

    #[test]
    fn invalid_dates_are_rejected() {
        for bad in ["2023-13-45 25:61:61", "yesterday", "2024-01-01", ""] {
            assert!(parse_git_time(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn format_round_trips() {
        let t = parse_git_time("2024-01-01 10:00:00 +05:30").unwrap();
        assert_eq!(format_git_time(t), "2024-01-01 10:00:00 +0530");
        assert_eq!(parse_git_time(&format_git_time(t)).unwrap(), t);
    }

    #[test]
    fn utc_view_of_offset_date() {
        let utc = parse_utc("2024-01-01 05:30:00 +05:30").unwrap();
        assert_eq!(utc.to_string(), "2024-01-01 00:00:00");
    }
}
