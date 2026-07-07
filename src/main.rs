use bkzyn::cmd::{add, backup, log, pattern, restore, rollback, save, setup, status, sync};
use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "bkzyn")]
#[command(about = "A backup tool for dotfiles and configurations", long_about = None)]
struct Cli {
    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(
        long,
        global = true,
        help = "Run without making any modifications to the filesystem"
    )]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Back up local dotfiles to the repository
    Backup {
        /// Optional: set the github URL for the data repository before backing up
        #[arg(long)]
        set_url: Option<String>,
    },
    /// Install brew packages and set up configuration symlinks
    Setup,
    /// Restore configuration symlinks from repository to local system
    Restore,
    /// Move a configuration into the backup repository and symlink it back
    Add {
        /// The path to the file or directory in ~/.config to add
        path: std::path::PathBuf,
    },
    /// Add an include pattern for an app in backup.toml
    Include {
        /// The name of the app
        app: String,
        /// The pattern to include
        pattern: String,
    },
    /// Add an exclude pattern for an app in backup.toml
    Exclude {
        /// The name of the app
        app: String,
        /// The pattern to exclude
        pattern: String,
    },
    /// Save (commit) modifications to the Git repository
    Save {
        /// Optional specific category to save (e.g. "config")
        category: Option<String>,
        /// Optional commit message
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Synchronize snapshots with the remote repository (pull and push)
    Sync,
    /// View un-saved changes between system configuration and the repository
    Status,
    /// View a history of past snapshots
    Log,
    /// Revert the repository to a specific past snapshot
    Rollback {
        /// The snapshot ID (commit hash) to rollback to
        commit: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let paths = bkzyn::AppPaths::new().unwrap_or_else(|e| {
        eprintln!("Error resolving paths: {}", e);
        process::exit(1);
    });

    if cli.verbose {
        let ui = bkzyn::cli::CliManager::new(true);
        ui.status(
            "INFO",
            "Paths",
            &format!("Repository: {}", paths.repo.display()),
        );
        ui.status(
            "INFO",
            "Paths",
            &format!("XDG Config: {}", paths.xdg_config.display()),
        );
    }

    if let Err(e) = match &cli.command {
        Commands::Backup { set_url } => {
            backup::run(&paths, set_url.as_deref(), cli.dry_run, cli.verbose)
        }
        Commands::Setup => setup::run(&paths, cli.dry_run, cli.verbose),
        Commands::Restore => restore::run(&paths, cli.dry_run, cli.verbose),
        Commands::Add { path } => add::run(&paths, path, cli.dry_run, cli.verbose),
        Commands::Include { app, pattern: pat } => {
            pattern::run(&paths, app, pat, true, cli.dry_run, cli.verbose)
        }
        Commands::Exclude { app, pattern: pat } => {
            pattern::run(&paths, app, pat, false, cli.dry_run, cli.verbose)
        }
        Commands::Save { category, message } => save::run(
            &paths,
            category.as_deref(),
            message.as_deref(),
            cli.dry_run,
            cli.verbose,
        ),
        Commands::Sync => sync::run(&paths, cli.dry_run, cli.verbose),
        Commands::Status => status::run(&paths, cli.dry_run, cli.verbose),
        Commands::Log => log::run(&paths, cli.dry_run, cli.verbose),
        Commands::Rollback { commit } => rollback::run(&paths, commit, cli.dry_run, cli.verbose),
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
        // Flags before subcommand
        let args = vec!["bkzyn", "backup"];
        let cli = Cli::parse_from(args);
        assert!(!cli.verbose);
        assert!(!cli.dry_run);
        assert!(matches!(cli.command, Commands::Backup { set_url: _ }));

        // Flags after subcommand
        let args = vec!["bkzyn", "backup", "-v", "--dry-run"];
        let cli = Cli::parse_from(args);
        assert!(matches!(cli.command, Commands::Backup { set_url: _ }));
        assert!(cli.dry_run);
        assert!(cli.verbose);
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
