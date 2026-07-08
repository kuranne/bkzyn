use std::process::Command;

/// Saves (commits and optionally pushes) changes to the backup repository
pub fn run(
    paths: &crate::AppPaths,
    category: Option<&str>,
    message: Option<&str>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    let commit_message = message.unwrap_or("Update configurations");

    ui.status(
        "INFO",
        "Git",
        &format!(
            "Saving changes to repository at {}...",
            paths.repo.display()
        ),
    );

    if !dry_run {
        let backup_repo = paths.repo.join("data");

        // Auto-initialize git if the data folder was wiped
        if !backup_repo.join(".git").exists() {
            if !backup_repo.exists() {
                std::fs::create_dir_all(&backup_repo)?;
            }
            Command::new("git")
                .current_dir(&backup_repo)
                .arg("init")
                .status()?;
            ui.status("INFO", "Git", "Initialized new git repository in data/");
        }

        // 1. Git add
        let add_path = if let Some(cat) = category {
            cat.to_string()
        } else {
            ".".to_string()
        };

        let add_status = Command::new("git")
            .current_dir(&backup_repo)
            .args(["add", &add_path])
            .status()?;

        if !add_status.success() {
            return Err("Failed to execute `git add`".into());
        }

        // 2. Git commit
        let commit_status = Command::new("git")
            .current_dir(&backup_repo)
            .args(["commit", "-m", commit_message])
            .status()?;

        if !commit_status.success() {
            ui.warn(
                "Git",
                "Git commit returned a non-zero status. (Maybe there were no changes to commit?)",
            );
        } else {
            ui.status(
                "INFO",
                "Git",
                &format!("Committed with message: \"{}\"", commit_message),
            );
        }
    } else {
        ui.status(
            "SKIP",
            "Git",
            &format!(
                "Dry run - would have committed with message: \"{}\"",
                commit_message
            ),
        );
    }

    ui.done("Successfully saved changes");
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
    fn test_save_dry_run_success() {
        let (_dir, paths) = setup_test_env();
        let result = run(&paths, None, None, true, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_fails_git_add() {
        let (_dir, paths) = setup_test_env();
        let backup_repo = paths.repo.join("data");
        fs::create_dir_all(&backup_repo).unwrap();
        Command::new("git")
            .current_dir(&backup_repo)
            .arg("init")
            .status()
            .unwrap();

        let result = run(
            &paths,
            Some("nonexistent_file_that_does_not_exist"),
            None,
            false,
            false,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to execute `git add`"
        );
    }

    #[test]
    fn test_save_successful_commit_with_message() {
        let (_dir, paths) = setup_test_env();
        let backup_repo = paths.repo.join("data");
        fs::create_dir_all(&backup_repo).unwrap();

        // Init repo and configure git identity so commit doesn't fail.
        for args in [
            vec!["init"],
            vec!["config", "user.email", "test@example.com"],
            vec!["config", "user.name", "Test"],
        ] {
            Command::new("git")
                .current_dir(&backup_repo)
                .args(&args)
                .status()
                .unwrap();
        }

        // Stage a file so the commit is non-empty.
        fs::write(backup_repo.join("test.txt"), "hello").unwrap();

        let result = run(&paths, None, Some("my custom message"), false, false);
        assert!(result.is_ok());
    }
}
