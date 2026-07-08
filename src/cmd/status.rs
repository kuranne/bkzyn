use std::fs;
use std::process::Command;

/// Checks for differences between system configuration and the latest snapshot.
pub fn run(
    paths: &crate::AppPaths,
    _dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    if !paths.config.exists() {
        return Err("No tracked configurations found in the repository.".into());
    }

    ui.status(
        "INFO",
        "Status",
        "Comparing system configurations against snapshot...",
    );

    let mut differences_found = false;

    for entry in fs::read_dir(&paths.config)? {
        let entry = entry?;
        let repo_path = entry.path();

        if repo_path.is_dir() {
            let app_name = entry.file_name();
            let sys_path = paths.xdg_config.join(&app_name);

            if !sys_path.exists() {
                ui.warn(
                    "Status",
                    &format!(
                        "{} is missing from system (~/.config/{})",
                        app_name.to_string_lossy(),
                        app_name.to_string_lossy()
                    ),
                );
                differences_found = true;
                continue;
            }

            // Use git diff --no-index for a nice colored output of differences.
            // --no-pager prevents git from opening `less` on TTY environments.
            let status = Command::new("git")
                .arg("--no-pager")
                .arg("diff")
                .arg("--no-index")
                .arg("--color=always")
                .arg(&repo_path)
                .arg(&sys_path)
                .status()?;

            if !status.success() {
                // git diff --no-index exits with 1 if differences are found
                differences_found = true;
            }
        }
    }

    if !differences_found {
        ui.done("System is fully in sync with the latest snapshot. No changes detected.");
    } else {
        println!("\nUnsaved changes detected. Run `bkzyn backup` to sync changes into the repository, then `bkzyn save` to snapshot them.");
    }

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
        fs::create_dir_all(&paths.xdg_config).unwrap();
        (dir, paths)
    }

    #[test]
    fn test_status_missing_config_dir_fails() {
        let (_dir, paths) = setup_test_env();
        // paths.config does not exist.
        let result = run(&paths, false, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No tracked configurations found"));
    }

    #[test]
    fn test_status_in_sync_succeeds() {
        let (_dir, paths) = setup_test_env();
        // Create a repo app dir and a matching system dir with identical content.
        let repo_app = paths.config.join("myapp");
        let sys_app = paths.xdg_config.join("myapp");
        fs::create_dir_all(&repo_app).unwrap();
        fs::create_dir_all(&sys_app).unwrap();
        fs::write(repo_app.join("cfg.toml"), "x = 1").unwrap();
        fs::write(sys_app.join("cfg.toml"), "x = 1").unwrap();

        let result = run(&paths, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_warns_on_missing_system_app() {
        let (_dir, paths) = setup_test_env();
        // Repo has an app dir that does not exist on the system.
        let repo_app = paths.config.join("missingapp");
        fs::create_dir_all(&repo_app).unwrap();
        // xdg_config/missingapp is intentionally absent.

        let result = run(&paths, false, false);
        assert!(result.is_ok());
    }
}
