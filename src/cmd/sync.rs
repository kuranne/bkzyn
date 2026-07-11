use std::process::Command;

/// Synchronizes the repository with the remote (pull then push).
pub fn run(
    paths: &crate::AppPaths,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);
    let data_dir = paths.repo.join("data");

    if !data_dir.exists() {
        return Err("Data directory does not exist. Run setup or backup first.".into());
    }

    ui.status("INFO", "Sync", "Pulling latest changes from remote...");
    if !dry_run {
        let pull_status = Command::new("git")
            .current_dir(&data_dir)
            .arg("pull")
            .arg("--rebase")
            .status()?;

        if !pull_status.success() {
            ui.warn(
                "Sync",
                "Sync conflict detected. Aborting rebase to protect local data.",
            );
            let abort_status = Command::new("git")
                .current_dir(&data_dir)
                .arg("rebase")
                .arg("--abort")
                .status()?;

            if !abort_status.success() {
                ui.warn(
                    "Sync",
                    "Failed to abort rebase. Your git repository may be in an intermediate state.",
                );
            }

            return Err("Sync conflict detected. Please resolve manually or force push.".into());
        }
    }

    ui.status("INFO", "Sync", "Pushing local snapshots to remote...");
    if !dry_run {
        let push_status = Command::new("git")
            .current_dir(&data_dir)
            .arg("push")
            .status()?;

        if !push_status.success() {
            return Err("Failed to push changes to the remote repository.".into());
        }
    }

    ui.done("Successfully synchronized snapshots with remote");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppPaths;
    use std::fs;
    use tempfile::tempdir;

    fn setup_test_env() -> (tempfile::TempDir, AppPaths) {
        let dir = tempdir().unwrap();
        let base = dir.path().to_path_buf();
        let paths = AppPaths {
            repo: base.clone(),
            config: base.join("data").join("config"),
            data: base.join("data").join("share"),
            old: base.join("old"),
            xdg_config: base.join("xdg_config"),
            xdg_data: base.join("xdg_data"),
        };
        (dir, paths)
    }

    #[test]
    fn test_sync_missing_data_dir_fails() {
        let (_dir, paths) = setup_test_env();
        let result = run(&paths, false, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Data directory does not exist"));
    }

    #[test]
    fn test_sync_dry_run_succeeds_with_data_dir() {
        let (_dir, paths) = setup_test_env();
        fs::create_dir_all(paths.repo.join("data")).unwrap();
        let result = run(&paths, true, false);
        assert!(result.is_ok());
    }
}
