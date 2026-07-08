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

    ui.status(
        "INFO",
        "Rollback",
        &format!("Reverting repository state to snapshot {}", commit),
    );

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
            return Err(format!(
                "Failed to rollback to {}. Ensure the snapshot ID is valid.",
                commit
            )
            .into());
        }
    }

    ui.done(&format!("Successfully reverted repository to {}. Run `bkzyn restore` to apply these configurations to your system.", commit));
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
    fn test_rollback_missing_data_dir_fails() {
        let (_dir, paths) = setup_test_env();
        let result = run(&paths, "HEAD~1", false, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Data directory does not exist"));
    }

    #[test]
    fn test_rollback_dry_run_succeeds_with_data_dir() {
        let (_dir, paths) = setup_test_env();
        fs::create_dir_all(paths.repo.join("data")).unwrap();
        let result = run(&paths, "HEAD~1", true, false);
        assert!(result.is_ok());
    }
}
