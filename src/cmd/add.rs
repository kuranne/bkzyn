use std::fs;
#[cfg(not(unix))]
use std::path::Path;
use std::path::{Component, PathBuf};

/// Adds a new configuration to the backup repository and symlinks it.
pub fn run(
    paths: &crate::AppPaths,
    path_to_add: &PathBuf,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    let absolute_path = if path_to_add.is_absolute() {
        path_to_add.clone()
    } else {
        std::env::current_dir()?.join(path_to_add)
    };

    if !absolute_path.exists() {
        return Err(format!("Path does not exist: {}", absolute_path.display()).into());
    }

    // Check if the path is inside xdg_config (~/.config)
    let relative_path = match absolute_path.strip_prefix(&paths.xdg_config) {
        Ok(p) => p,
        Err(_) => return Err(
            "Only paths inside ~/.config (XDG_CONFIG_HOME) are currently supported by bkzyn add."
                .into(),
        ),
    };

    // Prevent adding nested subdirectories directly if the parent isn't tracked yet,
    // or just grab the top-level directory name inside ~/.config
    let top_level_name = match relative_path.components().next() {
        Some(Component::Normal(name)) => name,
        _ => return Err("Invalid path structure inside ~/.config".into()),
    };

    let source_path = paths.xdg_config.join(top_level_name);
    let target_path = paths.config.join(top_level_name);
    let app_name = top_level_name.to_string_lossy().into_owned();

    ui.status(
        "INFO",
        &app_name,
        &format!("Adding {} to backup repository...", source_path.display()),
    );

    if fs::symlink_metadata(&source_path)?.is_symlink() {
        ui.done(&format!(
            "{} is already a symlink (likely already backed up).",
            app_name
        ));
        return Ok(());
    }

    if target_path.exists() {
        return Err(format!(
            "Target {} already exists in the repository! Cannot overwrite.",
            target_path.display()
        )
        .into());
    }

    if !dry_run {
        // 1. Copy to the repository
        ui.status(
            "COPY",
            &app_name,
            &format!("{} -> {}", source_path.display(), target_path.display()),
        );

        if source_path.is_dir() {
            copy_dir_all(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path)?;
        }

        // 3. Update backup.toml
        if let Some(backup_toml_path) = paths.get_backup_toml_path() {
            ui.status(
                "INFO",
                "Config",
                &format!("Updating {}...", backup_toml_path.display()),
            );
            let toml_str = fs::read_to_string(&backup_toml_path)?;
            if let Ok(mut doc) = toml_str.parse::<toml_edit::DocumentMut>() {
                if let Some(config_table) = doc.get_mut("config").and_then(|i| i.as_table_mut()) {
                    if let Some(includes) = config_table
                        .get_mut("include")
                        .and_then(|i| i.as_array_mut())
                    {
                        let mut found = false;
                        for item in includes.iter() {
                            if item.as_str() == Some(&app_name) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            includes.push(&app_name);

                            // Re-sort alphabetically to keep it clean
                            let mut strings: Vec<String> = includes
                                .iter()
                                .filter_map(|i| i.as_str().map(|s| s.to_string()))
                                .collect();
                            strings.sort();

                            includes.clear();
                            for s in strings {
                                includes.push(s);
                            }

                            // Write to the source file
                            fs::write(&backup_toml_path, doc.to_string())?;

                            // Sync it instantly into the repository so the Git commit grabs it!
                            let repo_toml_path = paths.config.join("bkzyn").join("backup.toml");
                            if backup_toml_path != repo_toml_path {
                                let _ = fs::copy(&backup_toml_path, &repo_toml_path);
                            }
                        }
                    }
                }
            }
        }
    } else {
        ui.status("SKIP", &app_name, "Dry run - no files moved or symlinked.");
    }

    ui.done(&format!(
        "Successfully added {} (run `bkzyn save` to commit)",
        app_name
    ));
    Ok(())
}

fn copy_dir_all(
    src: impl AsRef<std::path::Path>,
    dst: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
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
        fs::create_dir_all(&paths.config).unwrap();
        fs::create_dir_all(&paths.xdg_config).unwrap();
        (dir, paths)
    }

    #[test]
    fn test_add_outside_xdg_fails() {
        let (_dir, paths) = setup_test_env();
        let outside_path = paths.repo.join("outside.txt");
        fs::write(&outside_path, "test").unwrap();

        let result = run(&paths, &outside_path, false, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Only paths inside"));
    }

    #[test]
    fn test_add_nonexistent_path_fails() {
        let (_dir, paths) = setup_test_env();
        let bad_path = paths.xdg_config.join("does_not_exist");

        let result = run(&paths, &bad_path, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_add_dir_copies_and_updates_toml() {
        let (_dir, paths) = setup_test_env();

        // Source app directory in xdg_config.
        let app_dir = paths.xdg_config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("settings.toml"), "value = 1").unwrap();

        // Provide a backup.toml so the update path runs.
        let bkzyn_dir = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&bkzyn_dir).unwrap();
        fs::write(bkzyn_dir.join("backup.toml"), "[config]\ninclude = []\n").unwrap();

        run(&paths, &app_dir, false, false).unwrap();

        // Dir must be copied into the repo.
        assert!(paths.config.join("myapp").join("settings.toml").exists());

        // backup.toml must now include "myapp".
        let toml_content = fs::read_to_string(bkzyn_dir.join("backup.toml")).unwrap();
        assert!(toml_content.contains("myapp"));
    }

    #[test]
    #[cfg(unix)]
    fn test_add_already_symlinked_returns_ok() {
        let (_dir, paths) = setup_test_env();

        // Make xdg_config/myapp a symlink (simulating already added).
        let app_dir = paths.xdg_config.join("myapp");
        let real_dir = paths.repo.join("real_myapp");
        fs::create_dir_all(&real_dir).unwrap();
        std::os::unix::fs::symlink(&real_dir, &app_dir).unwrap();

        let result = run(&paths, &app_dir, false, false);
        assert!(result.is_ok());
        // Repo target must NOT have been created (early return before copy).
        assert!(!paths.config.join("myapp").exists());
    }

    #[test]
    fn test_add_target_exists_in_repo_fails() {
        let (_dir, paths) = setup_test_env();

        let app_dir = paths.xdg_config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();

        // Pre-create the target in the repo to trigger the conflict error.
        fs::create_dir_all(paths.config.join("myapp")).unwrap();

        let result = run(&paths, &app_dir, false, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already exists in the repository"));
    }

    #[test]
    fn test_add_dry_run_no_side_effects() {
        let (_dir, paths) = setup_test_env();

        let app_dir = paths.xdg_config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("cfg.toml"), "x = 1").unwrap();

        let bkzyn_dir = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&bkzyn_dir).unwrap();
        fs::write(bkzyn_dir.join("backup.toml"), "[config]\ninclude = []\n").unwrap();

        run(&paths, &app_dir, true, false).unwrap();

        // Dry-run must not copy anything to the repo.
        assert!(!paths.config.join("myapp").exists());

        // backup.toml must be unchanged.
        let toml_content = fs::read_to_string(bkzyn_dir.join("backup.toml")).unwrap();
        assert!(!toml_content.contains("myapp"));
    }
}
