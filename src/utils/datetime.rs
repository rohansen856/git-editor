use crate::args::Args;
use crate::utils::dates::parse_utc;
use crate::utils::types::Result;
use chrono::{Duration, NaiveDateTime};
use rand::Rng;
use uuid::Uuid;

pub fn generate_timestamps(args: &mut Args) -> Result<Vec<NaiveDateTime>> {
    let start_dt = parse_utc(args.start.as_deref().ok_or("Missing --begin")?)?;
    let end_dt = parse_utc(args.end.as_deref().ok_or("Missing --end")?)?;

    if start_dt >= end_dt {
        return Err("Start datetime must be before end datetime".into());
    }

    if url::Url::parse(args.repo_path.as_ref().unwrap()).is_ok()
        && !std::path::Path::new(args.repo_path.as_ref().unwrap()).exists()
    {
        let tmp_dir = std::env::temp_dir().join(format!("git_editor_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_dir)?;

        let status = std::process::Command::new("git")
            .args([
                "clone",
                args.repo_path.as_ref().unwrap(),
                &tmp_dir.to_string_lossy(),
            ])
            .status()?;

        if !status.success() {
            return Err("Failed to clone repository".into());
        }

        // Update repo_path to point to the cloned repository
        args.repo_path = Some(tmp_dir.to_string_lossy().to_string());
    }
    let total_commits = count_commits(args.repo_path.as_ref().unwrap())?;
    if total_commits == 0 {
        return Err("No commits found in repository".into());
    }

    spread_timestamps(start_dt, end_dt, total_commits, args.skip_range_check)
}

/// Minimum gap between generated commits by default.
const DEFAULT_GAP_SECS: i64 = 3 * 3600;
/// Minimum gap with `--skip-range-check` when it fits.
const PACKED_GAP_SECS: i64 = 5 * 60;

/// `count` strictly increasing timestamps from `start` to exactly `end`.
///
/// Gaps are at least 3 hours (or, with `skip_range_check`, at least 5 minutes
/// when that fits and otherwise as even as possible, but never below one second),
/// with the remaining slack spread randomly. Everything is whole seconds, so the
/// last timestamp is `end` exactly.
pub fn spread_timestamps(
    start: NaiveDateTime,
    end: NaiveDateTime,
    count: usize,
    skip_range_check: bool,
) -> Result<Vec<NaiveDateTime>> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if count == 1 {
        return Ok(vec![start]);
    }
    let gaps = (count - 1) as i64;
    let span = (end - start).num_seconds();

    let min_gap = if span >= DEFAULT_GAP_SECS * gaps {
        DEFAULT_GAP_SECS
    } else if !skip_range_check {
        return Err(format!(
            "Date range too small for {count} commits. Need at least {} hours between start and end dates.\n\
            Tip: Pass --skip-range-check to skip this validation and distribute commits evenly across the given range.",
            DEFAULT_GAP_SECS * gaps / 3600
        )
        .into());
    } else if span >= PACKED_GAP_SECS * gaps {
        PACKED_GAP_SECS
    } else if span >= gaps {
        // Too tight for random gaps: spread evenly, every gap at least one second.
        let gap_lengths = even_split(span, gaps as usize);
        return Ok(accumulate(start, &gap_lengths));
    } else {
        return Err(format!(
            "Date range too small for {count} commits: need at least {gaps} seconds so every commit gets a distinct timestamp"
        )
        .into());
    };

    let slack = span - min_gap * gaps;
    let mut rng = rand::rng();
    let weights: Vec<f64> = (0..gaps)
        .map(|_| rng.random::<f64>() + f64::EPSILON)
        .collect();
    let gap_lengths: Vec<i64> = random_split(slack, &weights)
        .into_iter()
        .map(|extra| extra + min_gap)
        .collect();
    Ok(accumulate(start, &gap_lengths))
}

/// Split `total` into `parts` integers that differ by at most one.
fn even_split(total: i64, parts: usize) -> Vec<i64> {
    let base = total / parts as i64;
    let remainder = (total % parts as i64) as usize;
    (0..parts)
        .map(|i| base + i64::from(i < remainder))
        .collect()
}

/// Split `total` proportionally to `weights` into integers that sum to `total` exactly.
fn random_split(total: i64, weights: &[f64]) -> Vec<i64> {
    let sum: f64 = weights.iter().sum();
    let shares: Vec<f64> = weights.iter().map(|w| w / sum * total as f64).collect();
    let mut parts: Vec<i64> = shares.iter().map(|s| s.floor() as i64).collect();
    let mut missing = total - parts.iter().sum::<i64>();
    // Hand out the rounding remainder to the largest fractional parts.
    let mut order: Vec<usize> = (0..parts.len()).collect();
    order.sort_by(|&a, &b| {
        let fa = shares[a] - shares[a].floor();
        let fb = shares[b] - shares[b].floor();
        fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
    });
    for i in order {
        if missing <= 0 {
            break;
        }
        parts[i] += 1;
        missing -= 1;
    }
    parts
}

fn accumulate(start: NaiveDateTime, gaps: &[i64]) -> Vec<NaiveDateTime> {
    let mut current = start;
    let mut out = Vec::with_capacity(gaps.len() + 1);
    out.push(current);
    for gap in gaps {
        current += Duration::seconds(*gap);
        out.push(current);
    }
    out
}

fn count_commits(repo_path: &str) -> Result<usize> {
    let repo = git2::Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
    Ok(revwalk.count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize git repo
        let repo = git2::Repository::init(&repo_path).unwrap();

        // Create a test file
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        // Add and commit file
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let sig = git2::Signature::new(
            "Test User",
            "test@example.com",
            &git2::Time::new(1234567890, 0),
        )
        .unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_count_commits() {
        let (_temp_dir, repo_path) = create_test_repo();
        let count = count_commits(&repo_path).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_generate_timestamps_invalid_date_format() {
        let (_temp_dir, repo_path) = create_test_repo();
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("invalid-date".to_string()),
            end: Some("2023-01-02 00:00:00".to_string()),
            ..Default::default()
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_timestamps_valid_range() {
        let (_temp_dir, repo_path) = create_test_repo();
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-10 00:00:00".to_string()),
            ..Default::default()
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();
        assert_eq!(timestamps.len(), 1); // One commit in test repo

        let start_dt =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_dt =
            NaiveDateTime::parse_from_str("2023-01-10 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        assert!(timestamps[0] >= start_dt);
        assert!(timestamps[0] <= end_dt);
    }

    #[test]
    fn test_generate_timestamps_preserves_order() {
        let (_temp_dir, repo_path) = create_test_repo();
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-10 00:00:00".to_string()),
            ..Default::default()
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();

        // Check that timestamps are in ascending order
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1]);
        }
    }

    fn create_test_repo_with_n_commits(n: usize) -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();
        let repo = git2::Repository::init(&repo_path).unwrap();

        for i in 1..=n {
            let file_path = temp_dir.path().join(format!("file{i}.txt"));
            fs::write(&file_path, format!("content {i}")).unwrap();

            let mut index = repo.index().unwrap();
            index
                .add_path(std::path::Path::new(&format!("file{i}.txt")))
                .unwrap();
            index.write().unwrap();

            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();

            let sig = git2::Signature::new(
                "Test User",
                "test@example.com",
                &git2::Time::new(1234567890 + i as i64 * 3600, 0),
            )
            .unwrap();

            let parents = if i == 1 {
                vec![]
            } else {
                let head = repo.head().unwrap();
                let parent_commit = head.peel_to_commit().unwrap();
                vec![parent_commit]
            };

            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("Commit {i}"),
                &tree,
                &parents.iter().collect::<Vec<_>>(),
            )
            .unwrap();
        }

        (temp_dir, repo_path)
    }

    #[test]
    fn test_small_range_error_mentions_skip_flag() {
        let (_temp_dir, repo_path) = create_test_repo_with_n_commits(5);
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 01:00:00".to_string()), // 1 hour for 5 commits
            ..Default::default()
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("--skip-range-check"),
            "Error message should mention --skip-range-check flag, got: {err_msg}"
        );
    }

    #[test]
    fn test_skip_range_check_produces_correct_count() {
        let (_temp_dir, repo_path) = create_test_repo_with_n_commits(5);
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 01:00:00".to_string()), // 1 hour for 5 commits
            skip_range_check: true,
            ..Default::default()
        };

        let result = generate_timestamps(&mut args);
        assert!(
            result.is_ok(),
            "skip_range_check should succeed: {:?}",
            result.err()
        );

        let timestamps = result.unwrap();
        assert_eq!(timestamps.len(), 5);
    }

    #[test]
    fn test_skip_range_check_respects_5min_gap_and_bounds() {
        let (_temp_dir, repo_path) = create_test_repo_with_n_commits(3);
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 02:00:00".to_string()), // 2 hours for 3 commits (enough for 5-min gaps)
            skip_range_check: true,
            ..Default::default()
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();
        let start_dt =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_dt =
            NaiveDateTime::parse_from_str("2023-01-01 02:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        // All timestamps should be within bounds
        for ts in &timestamps {
            assert!(*ts >= start_dt, "Timestamp {ts} is before start");
            assert!(*ts <= end_dt, "Timestamp {ts} is after end");
        }

        // Each consecutive gap should be >= 5 minutes
        for i in 1..timestamps.len() {
            let gap = timestamps[i] - timestamps[i - 1];
            assert!(
                gap >= Duration::minutes(5),
                "Gap between timestamps[{i}] and [{prev}] is {gap_min} mins, expected >= 5",
                prev = i - 1,
                gap_min = gap.num_minutes()
            );
        }

        // Timestamps should be in ascending order
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1]);
        }
    }

    #[test]
    fn test_skip_range_check_single_commit() {
        let (_temp_dir, repo_path) = create_test_repo(); // single commit
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 00:01:00".to_string()), // 1 minute
            skip_range_check: true,
            ..Default::default()
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();
        assert_eq!(timestamps.len(), 1);
    }

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    fn assert_strictly_increasing_within(
        ts: &[NaiveDateTime],
        start: NaiveDateTime,
        end: NaiveDateTime,
    ) {
        assert_eq!(ts.first(), Some(&start));
        assert_eq!(ts.last(), Some(&end));
        assert!(ts.windows(2).all(|w| w[0] < w[1]), "{ts:?}");
    }

    #[test]
    fn test_spread_hits_both_bounds_with_3h_gaps() {
        let (start, end) = (dt("2024-01-01 00:00:00"), dt("2024-01-03 00:00:00"));
        for _ in 0..50 {
            let ts = spread_timestamps(start, end, 7, false).unwrap();
            assert_strictly_increasing_within(&ts, start, end);
            assert!(ts
                .windows(2)
                .all(|w| (w[1] - w[0]).num_seconds() >= 3 * 3600));
        }
    }

    #[test]
    fn test_spread_tiny_range_never_duplicates() {
        let (start, end) = (dt("2024-01-01 00:00:00"), dt("2024-01-01 00:00:04"));
        let ts = spread_timestamps(start, end, 5, true).unwrap();
        assert_strictly_increasing_within(&ts, start, end);
        assert!(spread_timestamps(start, end, 6, true).is_err());
    }

    #[test]
    fn test_random_split_sums_exactly() {
        let parts = random_split(1001, &[0.3, 0.3, 0.4]);
        assert_eq!(parts.iter().sum::<i64>(), 1001);
        assert_eq!(even_split(10, 3), vec![4, 3, 3]);
    }
}
