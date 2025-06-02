mod args;
mod datetime;
mod validator;
mod rewrite;
mod types;

use args::Args;
use clap::Parser;
use validator::validate_inputs;
use datetime::generate_timestamps;
use rewrite::rewrite_commits;
use types::Result;

fn main() -> Result<()> {
    let args = Args::parse();
    if let Err(e) = validate_inputs(&args) {
        eprintln!("Validation error: {}", e);
        return Err(e);
    }

    let timestamps = generate_timestamps(&args)?;
    rewrite_commits(&args, timestamps)?;
    Ok(())
}
