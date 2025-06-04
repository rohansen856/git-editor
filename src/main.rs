mod args;
mod datetime;
mod rewrite;
mod types;
mod utils;
mod validator;

use args::Args;
use clap::Parser;
use datetime::generate_timestamps;
use rewrite::rewrite_commits;
use types::Result;
use validator::validate_inputs;

fn main() -> Result<()> {
    let mut args = Args::parse();

    args.ensure_all_args_present();

    if let Err(e) = validate_inputs(&args) {
        eprintln!("Validation error: {}", e);
        return Err(e);
    }

    let timestamps = generate_timestamps(&args)?;
    rewrite_commits(&args, timestamps)?;
    Ok(())
}
