use bkzyn::cmd::{
    add, backup, init, log, pattern, remove, restore, rollback, save, setup, status, sync,
};
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
        /// Skip encrypting and backing up secrets
        #[arg(long)]
        skip_secrets: bool,
    },
    /// Install brew packages and set up configuration files
    Setup {
        /// Optional custom ZDOTDIR path. If flag is passed without value, defaults to $XDG_CONFIG_HOME/zsh.
        #[arg(long, num_args = 0..=1, default_missing_value = "DEFAULT_ZDOTDIR")]
        zdotdir: Option<String>,

        /// Disable ZSH Bootstraps check
        #[arg(long)]
        no_check_zsh: bool,

        /// Skip decrypting and restoring secrets
        #[arg(long)]
        skip_secrets: bool,
    },
    /// Check templates for missing variables
    #[command(visible_aliases = ["template-check"])]
    Check,
    /// Restore configuration files from repository to local system
    Restore {
        /// Optional specific paths to restore (e.g. ~/.config/tmux)
        paths: Vec<std::path::PathBuf>,
        /// Abort the restore if a template is missing variables
        #[arg(long)]
        strict: bool,
        /// Skip decrypting and restoring secrets
        #[arg(long)]
        skip_secrets: bool,
    },
    /// Interactively setup tracking configuration (wizard mode)
    #[command(visible_aliases = ["wizard"])]
    Init,
    /// Add new configurations to the backup repository
    Add {
        /// The paths to the files or directories to add
        paths: Vec<std::path::PathBuf>,
        /// Optional glob patterns to ignore when adding a directory
        #[arg(short = 'i', long = "ignore", num_args = 1..)]
        ignores: Option<Vec<String>>,
        /// Mark this path as a secret to be encrypted during backup
        #[arg(long)]
        secret: bool,
    },
    /// Remove configurations from the backup repository and stop tracking them
    #[command(visible_aliases = ["rm"])]
    Remove {
        /// The paths to the files or directories to remove
        paths: Vec<std::path::PathBuf>,
        /// Remove this path from the secrets list instead of whitelists
        #[arg(long)]
        secret: bool,
    },
    /// Add paths to the ignore list in backup.toml
    Ignore {
        /// The paths to the files or directories to ignore
        paths: Vec<std::path::PathBuf>,
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
        Commands::Check => restore::run(&paths, vec![], true, cli.verbose, true, true),
        Commands::Init => init::run(&paths, cli.dry_run, cli.verbose),
        Commands::Backup {
            set_url,
            skip_secrets,
        } => backup::run(
            &paths,
            set_url.as_deref(),
            cli.dry_run,
            cli.verbose,
            *skip_secrets,
        ),
        Commands::Add {
            paths: p,
            ignores,
            secret,
        } => add::run(
            &paths,
            p.clone(),
            ignores.as_deref(),
            cli.dry_run,
            cli.verbose,
            *secret,
        ),
        Commands::Remove { paths: p, secret } => {
            remove::run(&paths, p.clone(), cli.dry_run, cli.verbose, *secret)
        }
        Commands::Setup {
            zdotdir,
            no_check_zsh,
            skip_secrets,
        } => setup::run(
            &paths,
            zdotdir.as_deref(),
            *no_check_zsh,
            cli.dry_run,
            cli.verbose,
            *skip_secrets,
        ),
        Commands::Restore {
            paths: p,
            strict,
            skip_secrets,
        } => restore::run(
            &paths,
            p.clone(),
            cli.dry_run,
            cli.verbose,
            *strict,
            *skip_secrets,
        ),
        Commands::Ignore { paths: p } => pattern::run(&paths, p.clone(), cli.dry_run, cli.verbose),
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
        assert!(matches!(cli.command, Commands::Backup { .. }));

        // Flags after subcommand
        let args = vec!["bkzyn", "backup", "-v", "--dry-run"];
        let cli = Cli::parse_from(args);
        assert!(matches!(cli.command, Commands::Backup { .. }));
        assert!(cli.dry_run);
        assert!(cli.verbose);
    }

    #[test]
    fn test_setup_command() {
        let args = vec!["bkzyn", "--verbose", "setup"];
        let cli = Cli::parse_from(args);
        assert!(cli.verbose);
        assert!(!cli.dry_run);
        assert!(matches!(cli.command, Commands::Setup { .. }));
    }

    #[test]
    fn test_restore_command_dry_run() {
        let args = vec!["bkzyn", "--dry-run", "restore"];
        let cli = Cli::parse_from(args);
        assert!(!cli.verbose);
        assert!(cli.dry_run);
        assert!(matches!(cli.command, Commands::Restore { .. }));
    }
}
