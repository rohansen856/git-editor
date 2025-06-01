use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    pub repo_path: String,
    #[arg(long)]
    pub email: String,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub start: String,
    #[arg(long)]
    pub end: String,
}
