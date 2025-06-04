use crate::args::Args;
use crate::types::Result;
use chrono::{Duration, NaiveDateTime};
use rand::Rng;

pub fn generate_timestamps(args: &Args) -> Result<Vec<NaiveDateTime>> {
    let start_dt =
        NaiveDateTime::parse_from_str(args.start.as_ref().unwrap(), "%Y-%m-%d %H:%M:%S")?;
    let end_dt = NaiveDateTime::parse_from_str(args.end.as_ref().unwrap(), "%Y-%m-%d %H:%M:%S")?;

    if start_dt >= end_dt {
        eprintln!("Start datetime must be before end datetime");
        std::process::exit(1);
    }

    let total_commits = count_commits(args.repo_path.as_ref().unwrap())?;
    if total_commits == 0 {
        eprintln!("No commits found in repository");
        std::process::exit(1);
    }

    let min_span = Duration::hours(3 * (total_commits as i64 - 1));
    let total_span = end_dt - start_dt;

    if total_span < min_span {
        eprintln!("Date range too small for {} commits", total_commits);
        std::process::exit(1);
    }

    let slack = total_span - min_span;
    let mut rng = rand::rng();
    let mut weights: Vec<f64> = (0..(total_commits - 1)).map(|_| rng.random()).collect();
    let sum: f64 = weights.iter().sum();

    for w in &mut weights {
        *w = (*w / sum) * slack.num_seconds() as f64;
    }

    let mut timestamps = Vec::with_capacity(total_commits);
    let mut current = start_dt;
    timestamps.push(current);

    for w in &weights {
        let secs = w.round() as i64 + 3 * 3600;
        current += Duration::seconds(secs);
        timestamps.push(current);
    }

    Ok(timestamps)
}

fn count_commits(repo_path: &str) -> Result<usize> {
    let repo = git2::Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
    Ok(revwalk.count())
}
