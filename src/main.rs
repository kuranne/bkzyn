use clap::{Parser, Subcommand};
use bkzyn::cmd::{backup, setup, restore};
use std::process;

#[derive(Parser)]
#[command(name = "bkzyn")]
#[command(about = "A backup tool for dotfiles and configurations", long_about = None)]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[arg(long, help = "Run without making any modifications to the filesystem")]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Back up local dotfiles to the repository
    Backup,
    /// Install brew packages and set up configuration symlinks
    Setup,
    /// Restore configuration symlinks from repository to local system
    Restore,
}

fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Backup => backup::run(cli.dry_run, cli.verbose),
        Commands::Setup => setup::run(cli.dry_run, cli.verbose),
        Commands::Restore => restore::run(cli.dry_run, cli.verbose),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
