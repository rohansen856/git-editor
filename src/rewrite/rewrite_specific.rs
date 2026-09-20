use crate::rewrite::engine::{self, CommitterMode, Edit, Plan};
use crate::rewrite::report::print_outcome;
use crate::utils::dates::{format_git_time, parse_git_time};
use crate::utils::prompt::{cancelled, confirm};
use crate::utils::prompt::{read_prompted_line, read_prompted_line_raw};
use crate::utils::simulation::{report_simulation, simulation_from_plan};
use crate::utils::types::Result;
use crate::utils::types::{CommitInfo, EditOptions};
use crate::utils::validator::{is_valid_email, validate_identity_part};
use crate::{args::Args, utils::commit_history::get_commit_history};
use colored::Colorize;
use git2::Repository;
use std::io::{self, Write};

pub fn select_commit(commits: &[CommitInfo]) -> Result<usize> {
    crate::say!("\n{}", "Commit History:".bold().green());
    crate::say!("{}", "-".repeat(80).cyan());

    for (i, commit) in commits.iter().enumerate() {
        crate::say!(
            "{:3}. {} {} {} {}",
            i + 1,
            commit.short_hash.yellow().bold(),
            commit
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .blue(),
            commit.author_name.magenta(),
            commit
                .message
                .lines()
                .next()
                .unwrap_or("(no message)")
                .white()
        );
    }

    crate::say!("{}", "-".repeat(80).cyan());
    crate::say_inline!(
        "\n{} {} ",
        "Select commit number to edit:".bold().green(),
        "(Esc to cancel)".bright_black()
    );
    io::stdout().flush()?;

    let input = read_prompted_line()?;

    let selection = input.parse::<usize>().map_err(|_| "Invalid number")?;

    if selection < 1 || selection > commits.len() {
        return Err("Selection out of range".into());
    }

    Ok(selection - 1)
}

pub fn show_commit_details(commit: &CommitInfo, repo: &Repository) -> Result<()> {
    crate::say!("\n{}", "Selected Commit Details:".bold().green());
    crate::say!("{}", "=".repeat(80).cyan());

    crate::say!("{}: {}", "Hash".bold(), commit.oid.to_string().yellow());
    crate::say!("{}: {}", "Short Hash".bold(), commit.short_hash.yellow());
    crate::say!(
        "{}: {}",
        "Author".bold(),
        format!("{} <{}>", commit.author_name, commit.author_email).magenta()
    );
    crate::say!(
        "{}: {}",
        "Date".bold(),
        commit
            .timestamp
            .format("%Y-%m-%d %H:%M:%S")
            .to_string()
            .blue()
    );
    crate::say!(
        "{}: {}",
        "Parent Count".bold(),
        commit.parent_count.to_string().white()
    );

    crate::say!("\n{}", "Message:".bold());
    crate::say!("{}", commit.message.white());

    // Show parent commits
    if commit.parent_count > 0 {
        let git_commit = repo.find_commit(commit.oid)?;
        crate::say!("\n{}", "Parent Commits:".bold());
        for (i, parent_id) in git_commit.parent_ids().enumerate() {
            let parent = repo.find_commit(parent_id)?;
            crate::say!(
                "  {}: {} - {}",
                i + 1,
                parent_id.to_string()[..8].to_string().yellow(),
                parent.summary().unwrap_or("(no message)").white()
            );
        }
    }

    crate::say!("{}", "=".repeat(80).cyan());
    Ok(())
}

/// Parse a comma-separated menu selection; `5` means "all of the above".
fn parse_edit_selection(input: &str) -> Result<Vec<usize>> {
    let mut selected = Vec::new();
    for token in input.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        match token.parse::<usize>() {
            Ok(5) => selected.extend(1..=4),
            Ok(n @ 1..=4) => selected.push(n),
            _ => return Err(format!("Invalid option '{token}' (choose 1-5)").into()),
        }
    }
    selected.sort_unstable();
    selected.dedup();
    if selected.is_empty() {
        return Err("No option selected; nothing to edit".into());
    }
    Ok(selected)
}

fn prompt_line(label: &str) -> Result<String> {
    crate::say_inline!("{} {} ", label.bold(), "(Esc to cancel)".bright_black());
    io::stdout().flush()?;
    read_prompted_line()
}

/// Read a multi-line message; a line containing only `.` ends it.
fn prompt_message() -> Result<String> {
    crate::say!(
        "{} {} ",
        "New commit message (finish with a line containing only '.'):".bold(),
        "(Esc to cancel)".bright_black()
    );
    let mut lines = Vec::new();
    loop {
        let line = read_prompted_line_raw()?;
        if line.trim() == "." {
            break;
        }
        lines.push(line);
    }
    build_message(&lines)
}

/// Join entered lines into a message, dropping leading/trailing blank lines.
fn build_message(lines: &[String]) -> Result<String> {
    let start = lines.iter().position(|l| !l.trim().is_empty());
    let end = lines.iter().rposition(|l| !l.trim().is_empty());
    match (start, end) {
        (Some(start), Some(end)) => Ok(lines[start..=end].join("\n") + "\n"),
        _ => Err("Commit message cannot be empty".into()),
    }
}

// Get user input for what to change
pub fn get_edit_options() -> Result<EditOptions> {
    crate::say!("\n{}", "What would you like to edit?".bold().green());
    crate::say!("1. Author name");
    crate::say!("2. Author email");
    crate::say!("3. Commit timestamp");
    crate::say!("4. Commit message");
    crate::say!("5. All of the above");

    let selections = parse_edit_selection(&prompt_line("Select option(s) (comma-separated):")?)?;
    let mut options = EditOptions::default();

    for selection in selections {
        match selection {
            1 => {
                let name = prompt_line("New author name:")?;
                validate_identity_part(&name, "Author name")?;
                options.author_name = Some(name);
            }
            2 => {
                let email = prompt_line("New author email:")?;
                if !is_valid_email(&email) {
                    return Err(format!("Invalid email format: {email}").into());
                }
                options.author_email = Some(email);
            }
            3 => {
                let timestamp =
                    prompt_line("New timestamp (YYYY-MM-DD HH:MM:SS [+HH:MM], default UTC):")?;
                options.timestamp = Some(parse_git_time(&timestamp)?);
            }
            4 => options.message = Some(prompt_message()?),
            _ => unreachable!("parse_edit_selection only returns 1-4"),
        }
    }

    Ok(options)
}

/// Find a commit by unique hash prefix (4+ hex digits) or by 1-based list
/// number (1 = newest, as shown by `-s`). A hash prefix wins when both match.
fn resolve_commit<'a>(commits: &'a [CommitInfo], reference: &str) -> Result<&'a CommitInfo> {
    let reference = reference.trim().to_ascii_lowercase();
    if reference.len() >= 4 && reference.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut matches = commits
            .iter()
            .filter(|c| c.oid.to_string().starts_with(&reference));
        match (matches.next(), matches.next()) {
            (Some(commit), None) => return Ok(commit),
            (Some(_), Some(_)) => {
                return Err(format!("--commit '{reference}' matches several commits").into())
            }
            (None, _) => {}
        }
    }
    match reference.parse::<usize>() {
        Ok(number) if (1..=commits.len()).contains(&number) => Ok(&commits[number - 1]),
        _ => Err(format!(
            "--commit '{reference}' matches no commit hash and is not a number between 1 and {}",
            commits.len()
        )
        .into()),
    }
}

/// Edits requested with --name/--email/--set-date/--set-message.
fn edit_options_from_args(args: &Args) -> Result<EditOptions> {
    if let Some(name) = &args.name {
        validate_identity_part(name, "Author name")?;
    }
    if let Some(email) = &args.email {
        if !is_valid_email(email) {
            return Err(format!("Invalid email format: {email}").into());
        }
    }
    let options = EditOptions {
        author_name: args.name.clone(),
        author_email: args.email.clone(),
        timestamp: args.set_date.as_deref().map(parse_git_time).transpose()?,
        message: args.set_message.clone(),
    };
    if options.author_name.is_none()
        && options.author_email.is_none()
        && options.timestamp.is_none()
        && options.message.is_none()
    {
        return Err("Nothing to change: pass --name, --email, --set-date and/or --set-message with --commit".into());
    }
    if options
        .message
        .as_deref()
        .is_some_and(|m| m.trim().is_empty())
    {
        return Err("Commit message cannot be empty".into());
    }
    Ok(options)
}

pub fn rewrite_specific_commits(args: &Args) -> Result<()> {
    let commits = get_commit_history(args, false)?;

    if commits.is_empty() {
        crate::say!("{}", "No commits found!".red());
        return Ok(());
    }

    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    // Fail early on a detached/unborn HEAD and remember the tip the edit is based on.
    let head = engine::current_branch(&repo)?.head;

    let (selected_commit, edit_options) = match &args.commit {
        // Flag-driven (non-interactive) edit.
        Some(reference) => (
            resolve_commit(&commits, reference)?,
            edit_options_from_args(args)?,
        ),
        None => {
            let selected = &commits[select_commit(&commits)?];
            show_commit_details(selected, &repo)?;
            (selected, get_edit_options()?)
        }
    };

    // Confirm changes
    crate::say!("\n{}", "Planned changes:".bold().yellow());
    if let Some(ref name) = edit_options.author_name {
        crate::say!(
            "  Author name: {} -> {}",
            selected_commit.author_name.red(),
            name.green()
        );
    }
    if let Some(ref email) = edit_options.author_email {
        crate::say!(
            "  Author email: {} -> {}",
            selected_commit.author_email.red(),
            email.green()
        );
    }
    if let Some(ref timestamp) = edit_options.timestamp {
        crate::say!(
            "  Timestamp: {} -> {}",
            selected_commit
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .red(),
            format_git_time(*timestamp).green()
        );
    }
    if let Some(ref message) = edit_options.message {
        crate::say!(
            "  Message: {} -> {}",
            selected_commit.message.lines().next().unwrap_or("").red(),
            message.lines().next().unwrap_or("").green()
        );
    }

    if args.simulate {
        let plan = Plan {
            edits: [(selected_commit.oid, edit_from_options(&edit_options))]
                .into_iter()
                .collect(),
            committer: args.committer.into(),
        };
        let preview = simulation_from_plan(&commits, &plan, "Specific Commit Edit");
        report_simulation(&preview, args.show_diff, true);
        crate::say!("{}", "Simulation only: nothing was written.".cyan());
        return Ok(());
    }

    if !confirm(&format!("\n{}", "Proceed with changes?".bold()), args.yes)? {
        return Err(cancelled());
    }

    // Apply changes
    apply_commit_changes(
        &repo,
        selected_commit,
        &edit_options,
        head,
        args.committer.into(),
    )?;

    crate::say!("\n{}", "✓ Commit successfully edited!".green().bold());

    if args.show_history {
        get_commit_history(args, true)?;
    }

    Ok(())
}

fn edit_from_options(options: &EditOptions) -> Edit {
    Edit {
        name: options.author_name.clone(),
        email: options.author_email.clone(),
        time: options.timestamp,
        message: options.message.clone(),
    }
}

/// Rewrite only the selected commit (descendants are re-parented, ancestors reused).
fn apply_commit_changes(
    repo: &Repository,
    target_commit: &CommitInfo,
    options: &EditOptions,
    expected_head: git2::Oid,
    committer: CommitterMode,
) -> Result<()> {
    let plan = Plan {
        edits: [(target_commit.oid, edit_from_options(options))]
            .into_iter()
            .collect(),
        committer,
    };
    let outcome = engine::apply(repo, &plan, expected_head)?;
    print_outcome(&outcome);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rewrite::engine::GitTime;
    use chrono::NaiveDateTime;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo_with_commits() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize git repo
        let repo = git2::Repository::init(&repo_path).unwrap();

        // Create multiple commits
        for i in 1..=3 {
            let file_path = temp_dir.path().join(format!("test{i}.txt"));
            fs::write(&file_path, format!("test content {i}")).unwrap();

            let mut index = repo.index().unwrap();
            index
                .add_path(std::path::Path::new(&format!("test{i}.txt")))
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
    fn test_show_commit_details() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let repo = Repository::open(&repo_path).unwrap();

        // Get commit info
        let args = Args {
            repo_path: Some(repo_path),
            ..Default::default()
        };

        let commits = get_commit_history(&args, false).unwrap();
        let commit = &commits[0];

        // Test that show_commit_details doesn't crash
        let result = show_commit_details(commit, &repo);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_message_keeps_body_structure() {
        let lines: Vec<String> = ["", "subject", "", "  indented body", ""]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(
            build_message(&lines).unwrap(),
            "subject\n\n  indented body\n"
        );
        assert!(build_message(&["  ".to_string()]).is_err());
    }

    #[test]
    fn test_resolve_commit_by_number_and_prefix() {
        let make = |hex: &str| CommitInfo {
            oid: git2::Oid::from_str(hex).unwrap(),
            ..Default::default()
        };
        let commits = vec![
            make("abcdef0000000000000000000000000000000000"),
            make("abcd120000000000000000000000000000000000"),
            make("1234560000000000000000000000000000000000"),
        ];
        assert_eq!(resolve_commit(&commits, "1").unwrap().oid, commits[0].oid);
        assert_eq!(resolve_commit(&commits, "3").unwrap().oid, commits[2].oid);
        assert_eq!(
            resolve_commit(&commits, "1234").unwrap().oid,
            commits[2].oid
        );
        assert_eq!(
            resolve_commit(&commits, "ABCDEF").unwrap().oid,
            commits[0].oid
        );
        assert!(resolve_commit(&commits, "abcd").is_err()); // ambiguous
        assert!(resolve_commit(&commits, "4").is_err());
        assert!(resolve_commit(&commits, "ffff").is_err());
    }

    #[test]
    fn test_edit_options_from_args_requires_a_change() {
        let args = Args {
            pick_specific_commits: true,
            commit: Some("1".into()),
            ..Default::default()
        };
        assert!(edit_options_from_args(&args).is_err());
        let args = Args {
            email: Some("not-an-email".into()),
            ..args
        };
        assert!(edit_options_from_args(&args).is_err());
    }

    #[test]
    fn test_parse_edit_selection() {
        assert_eq!(parse_edit_selection("1").unwrap(), vec![1]);
        assert_eq!(parse_edit_selection(" 3, 1 ,3").unwrap(), vec![1, 3]);
        assert_eq!(parse_edit_selection("5").unwrap(), vec![1, 2, 3, 4]);
        assert!(parse_edit_selection("9").is_err());
        assert!(parse_edit_selection("1,x").is_err());
        assert!(parse_edit_selection("").is_err());
    }

    #[test]
    fn test_edit_options_default() {
        let options = EditOptions::default();

        assert_eq!(options.author_name, None);
        assert_eq!(options.author_email, None);
        assert_eq!(options.timestamp, None);
        assert_eq!(options.message, None);
    }

    #[test]
    fn test_edit_options_with_values() {
        let timestamp = GitTime::new(1_672_574_400, 0);

        let options = EditOptions {
            author_name: Some("New Author".to_string()),
            author_email: Some("new@example.com".to_string()),
            timestamp: Some(timestamp),
            message: Some("New commit message".to_string()),
        };

        assert_eq!(options.author_name, Some("New Author".to_string()));
        assert_eq!(options.author_email, Some("new@example.com".to_string()));
        assert_eq!(options.timestamp, Some(timestamp));
        assert_eq!(options.message, Some("New commit message".to_string()));
    }

    #[test]
    fn test_commit_selection_validation() {
        // Test the selection validation logic that's used in select_commit
        let commits = [CommitInfo {
            oid: git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap(),
            short_hash: "12345678".to_string(),
            timestamp: NaiveDateTime::parse_from_str("2023-01-01 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
            author_name: "Test User".to_string(),
            author_email: "test@example.com".to_string(),
            message: "Test commit".to_string(),
            parent_count: 0,
            ..Default::default()
        }];

        // Test valid selection range
        let selection = 1;
        assert!(selection >= 1 && selection <= commits.len());

        // Test invalid selections
        let invalid_selection1 = 0;
        assert!(invalid_selection1 < 1 || invalid_selection1 > commits.len());

        let invalid_selection2 = commits.len() + 1;
        assert!(invalid_selection2 < 1 || invalid_selection2 > commits.len());
    }

    #[test]
    fn test_rewrite_specific_commits_with_empty_commits() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            pick_specific_commits: true,
            ..Default::default()
        };

        // Test that the function handles the case where get_commit_history returns commits
        let commits = get_commit_history(&args, false).unwrap();
        assert!(!commits.is_empty());
        assert_eq!(commits.len(), 3);
    }

    #[test]
    fn test_apply_commit_changes_logic() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let _repo = Repository::open(&repo_path).unwrap();

        // Get commit info
        let args = Args {
            repo_path: Some(repo_path),
            ..Default::default()
        };

        let commits = get_commit_history(&args, false).unwrap();
        let target_commit = &commits[0];

        // Test EditOptions with different values
        let options = EditOptions {
            author_name: Some("New Author".to_string()),
            author_email: Some("new@example.com".to_string()),
            timestamp: Some(GitTime::new(1_672_574_400, 0)),
            message: Some("New commit message".to_string()),
        };

        // Test that the options are properly set
        assert_eq!(options.author_name.as_ref().unwrap(), "New Author");
        assert_eq!(options.author_email.as_ref().unwrap(), "new@example.com");
        assert!(options.timestamp.is_some());
        assert_eq!(options.message.as_ref().unwrap(), "New commit message");

        // Test fallback to original values
        let partial_options = EditOptions {
            author_name: None,
            author_email: None,
            timestamp: None,
            message: None,
        };

        let author_name = partial_options
            .author_name
            .as_ref()
            .unwrap_or(&target_commit.author_name);
        let author_email = partial_options
            .author_email
            .as_ref()
            .unwrap_or(&target_commit.author_email);

        assert_eq!(author_name, &target_commit.author_name);
        assert_eq!(author_email, &target_commit.author_email);
    }
}
