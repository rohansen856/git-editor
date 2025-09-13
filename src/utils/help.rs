use colored::*;

pub fn print_help() {
    println!(
        "{}",
        "Git Editor - Git History Rewriting Tool".green().bold()
    );
    println!();
    println!("{}", "A powerful Rust-based command-line utility designed to safely rewrite Git commit metadata.".white());
    println!();
    println!("{}", "USAGE:".yellow().bold());
    println!("    {} [OPTIONS]", "git-editor".cyan());
    println!();
    println!("{}", "OPERATION MODES:".yellow().bold());
    println!("  {} Full History Rewrite (default)", "•".green());
    println!("    Requires: --email, --name, --begin, --end");
    println!("    Example: git-editor --email user@example.com --name \"User\" --begin \"2023-01-01 00:00:00\" --end \"2023-01-07 23:59:59\"");
    println!();
    println!("  {} Show History", "•".green());
    println!("    Flag: -s, --show-history");
    println!("    Example: git-editor -s");
    println!();
    println!("  {} Pick Specific Commits", "•".green());
    println!("    Flag: -p, --pick-specific-commits");
    println!("    Example: git-editor -p");
    println!();
    println!("  {} Range Editing", "•".green());
    println!("    Flag: -x, --range");
    println!("    Example: git-editor -x");
    println!();
    println!("  {} Simulation Mode (Dry-Run)", "•".green());
    println!("    Flag: --simulate");
    println!("    Shows what changes would be made without applying them");
    println!("    Example: git-editor --simulate --name \"Author\" --email \"author@example.com\"");
    println!("    Example: git-editor --simulate --show-diff --name \"Author\" --email \"author@example.com\"");
    println!();
    println!("{}", "OPTIONS:".yellow().bold());
    println!(
        "  {:<25} Path to Git repository (defaults to current directory)",
        "-r, --repo-path <PATH>".cyan()
    );
    println!(
        "  {:<25} Email for rewritten commits",
        "--email <EMAIL>".cyan()
    );
    println!(
        "  {:<25} Name for rewritten commits",
        "-n, --name <NAME>".cyan()
    );
    println!(
        "  {:<25} Start date (YYYY-MM-DD HH:MM:SS)",
        "-b, --begin <DATE>".cyan()
    );
    println!(
        "  {:<25} End date (YYYY-MM-DD HH:MM:SS)",
        "-e, --end <DATE>".cyan()
    );
    println!("  {:<25} Show commit history", "-s, --show-history".cyan());
    println!(
        "  {:<25} Interactive commit selection",
        "-p, --pick-specific-commits".cyan()
    );
    println!("  {:<25} Interactive range editing", "-x, --range".cyan());
    println!(
        "  {:<25} Dry-run mode - preview changes without applying",
        "--simulate".cyan()
    );
    println!(
        "  {:<25} Show detailed diff in simulation (requires --simulate)",
        "--show-diff".cyan()
    );
    println!("  {:<25} Print help information", "-h, --help".cyan());
    println!("  {:<25} Print version information", "-V, --version".cyan());
    println!();
    println!(
        "{}",
        "For more detailed usage information, use: git-editor --help"
            .white()
            .italic()
    );
}
