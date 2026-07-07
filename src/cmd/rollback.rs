use std::process::Command;

/// Rolls back the repository data to a specific commit.
pub fn run(
    paths: &crate::AppPaths,
    commit: &str,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);
    let data_dir = paths.repo.join("data");

    if !data_dir.exists() {
        return Err("Data directory does not exist. Run setup or backup first.".into());
    }

    ui.status("INFO", "Rollback", &format!("Reverting repository state to snapshot {}", commit));
    
    if !dry_run {
        // We use git restore to safely copy the files from the commit into the working directory
        // without altering the commit history or creating a detached HEAD.
        let status = Command::new("git")
            .current_dir(&data_dir)
            .arg("restore")
            .arg(format!("--source={}", commit))
            .arg("--worktree")
            .arg("--staged")
            .arg(".")
            .status()?;

        if !status.success() {
            return Err(format!("Failed to rollback to {}. Ensure the snapshot ID is valid.", commit).into());
        }
    }

    ui.done(&format!("Successfully reverted repository to {}. Run `bkzyn restore` to apply these configurations to your system.", commit));
    Ok(())
}
