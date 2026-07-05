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
    let paths = bkzyn::AppPaths::new().unwrap_or_else(|e| {
        eprintln!("Error resolving paths: {}", e);
        process::exit(1);
    });

    if let Err(e) = match &cli.command {
        Commands::Backup => backup::run(&paths, cli.dry_run, cli.verbose),
        Commands::Setup => setup::run(&paths, cli.dry_run, cli.verbose),
        Commands::Restore => restore::run(&paths, cli.dry_run, cli.verbose),
    } {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_backup_command() {
        let args = vec!["bkzyn", "backup"];
        let cli = Cli::parse_from(args);
        assert!(!cli.verbose);
        assert!(!cli.dry_run);
        assert!(matches!(cli.command, Commands::Backup));
    }

    #[test]
    fn test_setup_command() {
        let args = vec!["bkzyn", "--verbose", "setup"];
        let cli = Cli::parse_from(args);
        assert!(cli.verbose);
        assert!(!cli.dry_run);
        assert!(matches!(cli.command, Commands::Setup));
    }

    #[test]
    fn test_restore_command_dry_run() {
        let args = vec!["bkzyn", "--dry-run", "restore"];
        let cli = Cli::parse_from(args);
        assert!(!cli.verbose);
        assert!(cli.dry_run);
        assert!(matches!(cli.command, Commands::Restore));
    }
}
