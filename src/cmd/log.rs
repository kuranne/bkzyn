use std::process::Command;

/// Displays the git log of snapshots.
pub fn run(
    paths: &crate::AppPaths,
    _dry_run: bool,
    _verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = paths.repo.join("data");

    if !data_dir.exists() {
        return Err("Data directory does not exist. Run setup or backup first.".into());
    }

    // Run git log with pretty formatting.
    // --no-pager prevents git from opening `less` on TTY environments.
    let status = Command::new("git")
        .current_dir(&data_dir)
        .arg("--no-pager")
        .arg("log")
        .arg("--color=always")
        .arg("--pretty=format:%C(yellow)%h%Creset - %C(cyan)%ad%Creset - %s %C(green)(%cr)%Creset")
        .arg("--date=short")
        .status()?;

    if !status.success() {
        return Err("Failed to view snapshot log.".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppPaths;
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
    fn test_log_missing_data_dir_fails() {
        let (_dir, paths) = setup_test_env();
        let result = run(&paths, false, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Data directory does not exist"));
    }
}

