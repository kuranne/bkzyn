use crate::config::BackupConfig;
use std::fs;
use std::path::{Component, PathBuf};

/// Removes tracked configurations from the backup repository and untracks them in backup.toml
pub fn run(
    paths: &crate::AppPaths,
    paths_to_remove: Vec<PathBuf>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    let backup_toml_path = paths
        .get_backup_toml_path()
        .ok_or("Could not find backup.toml")?;

    let config = BackupConfig::load(&backup_toml_path)?;

    // Determine category paths on the host
    let mut category_dirs = std::collections::HashMap::new();
    for (cat_name, cat_cfg) in config.categories() {
        let mut host_dir = match cat_name.as_str() {
            "config" => paths.xdg_config.clone(),
            "dataHome" => paths.xdg_data.clone(),
            _ => std::path::PathBuf::new(),
        };

        if let Some(custom_path) = &cat_cfg.path {
            if custom_path.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    host_dir = home.join(custom_path.strip_prefix("~/").unwrap());
                } else {
                    host_dir = std::path::PathBuf::from(custom_path);
                }
            } else {
                host_dir = std::path::PathBuf::from(custom_path);
            }
        }

        if !host_dir.as_os_str().is_empty() {
            category_dirs.insert(cat_name.clone(), host_dir);
        }
    }

    let toml_str = fs::read_to_string(&backup_toml_path)?;
    let mut doc = toml_str.parse::<toml_edit::DocumentMut>()?;
    let mut modified = false;

    for target in paths_to_remove {
        let absolute_path = if target.is_absolute() {
            target.clone()
        } else {
            std::env::current_dir()?.join(&target)
        };

        let mut matched_category = None;
        let mut rel_path = None;

        for (cat_name, host_dir) in &category_dirs {
            if let Ok(rp) = absolute_path.strip_prefix(host_dir) {
                matched_category = Some(cat_name.clone());
                rel_path = Some(rp.to_path_buf());
                break;
            }
        }

        if let (Some(cat_name), Some(relative_path)) = (matched_category, rel_path) {
            let mut components = relative_path.components();
            let app_name = match components.next() {
                Some(Component::Normal(name)) => name.to_string_lossy().into_owned(),
                _ => {
                    ui.warn(
                        "Remove",
                        &format!(
                            "Path {} does not have a valid app directory.",
                            target.display()
                        ),
                    );
                    continue;
                }
            };

            let is_deep_file = relative_path.components().count() > 1;

            let target_repo_path = paths.repo.join("data").join(&cat_name).join(&relative_path);
            let legacy_repo_path = paths.config.join(&relative_path); // Fallback for root level [config]

            let mut repo_path_to_delete = None;
            if target_repo_path.exists() || target_repo_path.is_symlink() {
                repo_path_to_delete = Some(target_repo_path);
            } else if cat_name == "config"
                && (legacy_repo_path.exists() || legacy_repo_path.is_symlink())
            {
                repo_path_to_delete = Some(legacy_repo_path);
            }

            ui.status(
                "INFO",
                &app_name,
                &format!(
                    "Untracking {} from category '{}'...",
                    target.display(),
                    cat_name
                ),
            );

            if !dry_run {
                if let Some(repo_path) = repo_path_to_delete {
                    ui.status(
                        "DELETE",
                        &app_name,
                        &format!("Removing {} from repository", repo_path.display()),
                    );
                    if repo_path.is_dir() {
                        if let Err(e) = fs::remove_dir_all(&repo_path) {
                            ui.warn("Remove", &format!("Failed to delete directory: {}", e));
                        }
                    } else {
                        if let Err(e) = fs::remove_file(&repo_path) {
                            ui.warn("Remove", &format!("Failed to delete file: {}", e));
                        }
                    }
                } else {
                    ui.status("SKIP", &app_name, "File or folder not found in repository.");
                }

                // Untrack from backup.toml
                let full_key = format!("{}.{}", cat_name, app_name);

                // Remove from app-specific table [cat_name.app_name] or fallback to [app_name] for config
                let final_key = if doc.contains_table(&full_key) {
                    full_key
                } else if cat_name == "config" && doc.contains_table(&app_name) {
                    app_name.clone()
                } else {
                    full_key
                };

                if doc.contains_table(&final_key) {
                    let app_table = doc[&final_key].as_table_mut().unwrap();
                    if is_deep_file {
                        let relative_to_app = relative_path
                            .strip_prefix(&app_name)
                            .unwrap()
                            .to_string_lossy()
                            .to_string();

                        if let Some(wl) = app_table
                            .get_mut("whitelists")
                            .and_then(|v| v.as_array_mut())
                        {
                            let original_len = wl.len();
                            wl.retain(|v| v.as_str() != Some(&relative_to_app));
                            if wl.len() < original_len {
                                modified = true;
                                ui.status(
                                    "INFO",
                                    "Config",
                                    &format!(
                                        "Removed '{}' from whitelists for app '{}'",
                                        relative_to_app, app_name
                                    ),
                                );
                            }
                        }

                        if let Some(ig) =
                            app_table.get_mut("ignores").and_then(|v| v.as_array_mut())
                        {
                            let original_len = ig.len();
                            ig.retain(|v| v.as_str() != Some(&relative_to_app));
                            if ig.len() < original_len {
                                modified = true;
                                ui.status(
                                    "INFO",
                                    "Config",
                                    &format!(
                                        "Removed '{}' from ignores for app '{}'",
                                        relative_to_app, app_name
                                    ),
                                );
                            }
                        }
                    } else {
                        // Entire app is being removed
                        // If it's just the root app (not a deep file), we should completely remove it from the whitelists array in [category]
                        if let Some(cat_table) =
                            doc.get_mut(&cat_name).and_then(|i| i.as_table_mut())
                        {
                            if let Some(wl) = cat_table
                                .get_mut("whitelists")
                                .and_then(|v| v.as_array_mut())
                            {
                                let original_len = wl.len();
                                wl.retain(|v| v.as_str() != Some(&app_name));
                                if wl.len() < original_len {
                                    modified = true;
                                    ui.status(
                                        "INFO",
                                        "Config",
                                        &format!(
                                            "Removed app '{}' from [{}] whitelists",
                                            app_name, cat_name
                                        ),
                                    );
                                }
                            }
                        }

                        // Remove the entire app block [cat_name.app_name]
                        if doc.remove(&final_key).is_some() {
                            modified = true;
                            ui.status(
                                "INFO",
                                "Config",
                                &format!("Removed config block [{}] entirely.", final_key),
                            );
                        }
                    }
                } else if !is_deep_file {
                    // Check if it's listed directly in the category's whitelists
                    if let Some(cat_table) = doc.get_mut(&cat_name).and_then(|i| i.as_table_mut()) {
                        if let Some(wl) = cat_table
                            .get_mut("whitelists")
                            .and_then(|v| v.as_array_mut())
                        {
                            let original_len = wl.len();
                            wl.retain(|v| v.as_str() != Some(&app_name));
                            if wl.len() < original_len {
                                modified = true;
                                ui.status(
                                    "INFO",
                                    "Config",
                                    &format!(
                                        "Removed app '{}' from [{}] whitelists",
                                        app_name, cat_name
                                    ),
                                );
                            }
                        }
                    }
                }
            } else {
                ui.status("SKIP", &app_name, "Dry run - no modifications made.");
            }
        } else {
            ui.warn(
                "Remove",
                &format!(
                    "Path {} does not match any known backup category host path.",
                    target.display()
                ),
            );
        }
    }

    if modified && !dry_run {
        fs::write(&backup_toml_path, doc.to_string())?;

        // Sync to repo
        let repo_toml_path = paths.config.join("bkzyn").join("backup.toml");
        if backup_toml_path != repo_toml_path {
            let _ = fs::copy(&backup_toml_path, &repo_toml_path);
        }
        ui.done("Successfully removed paths (run `bkzyn save` to commit)");
    } else if !modified && !dry_run {
        ui.status(
            "INFO",
            "Config",
            "No tracking changes were made to backup.toml.",
        );
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
        fs::create_dir_all(&paths.config).unwrap();
        fs::create_dir_all(&paths.xdg_config).unwrap();
        (dir, paths)
    }

    /// Writes a backup.toml into xdg_config/bkzyn/ and returns its path.
    fn write_backup_toml(paths: &AppPaths, content: &str) -> std::path::PathBuf {
        let dir = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("backup.toml");
        fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn test_remove_missing_toml_fails() {
        let (_dir, paths) = setup_test_env();
        let target = paths.xdg_config.join("myapp").join("pattern.log");
        let result = run(&paths, vec![target], false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("backup.toml"));
    }

    #[test]
    fn test_remove_deep_file() {
        let (_dir, paths) = setup_test_env();
        let toml_path = write_backup_toml(
            &paths,
            "[config]\n[myapp]\nwhitelists = [\"pattern.log\"]\n",
        );
        let target = paths.xdg_config.join("myapp").join("pattern.log");

        // Create dummy file in repo
        let repo_app = paths.config.join("myapp");
        fs::create_dir_all(&repo_app).unwrap();
        fs::write(repo_app.join("pattern.log"), "test").unwrap();

        run(&paths, vec![target], false, false).unwrap();

        let content = fs::read_to_string(&toml_path).unwrap();
        assert!(!content.contains("\"pattern.log\""));
        assert!(!repo_app.join("pattern.log").exists()); // Should be deleted
    }

    #[test]
    fn test_remove_entire_app() {
        let (_dir, paths) = setup_test_env();
        let toml_path = write_backup_toml(
            &paths,
            "[config]\nwhitelists = [\"myapp\"]\n[myapp]\nwhitelists = [\"pattern.log\"]\n",
        );
        let target = paths.xdg_config.join("myapp");

        let repo_app = paths.config.join("myapp");
        fs::create_dir_all(&repo_app).unwrap();
        fs::write(repo_app.join("pattern.log"), "test").unwrap();

        run(&paths, vec![target], false, false).unwrap();

        let content = fs::read_to_string(&toml_path).unwrap();
        assert!(!content.contains("\"myapp\"")); // Should be removed from config whitelists
        assert!(!content.contains("[myapp]")); // The entire block should be gone
        assert!(!repo_app.exists()); // The entire dir in repo should be deleted
    }

    #[test]
    fn test_remove_dry_run_no_modification() {
        let (_dir, paths) = setup_test_env();
        let original = "[config]\n[myapp]\nwhitelists = [\"pattern.log\"]\n";
        let toml_path = write_backup_toml(&paths, original);
        let target = paths.xdg_config.join("myapp").join("pattern.log");

        let repo_app = paths.config.join("myapp");
        fs::create_dir_all(&repo_app).unwrap();
        fs::write(repo_app.join("pattern.log"), "test").unwrap();

        run(&paths, vec![target], true, false).unwrap();

        // File must be unchanged after dry-run.
        let content = fs::read_to_string(&toml_path).unwrap();
        assert_eq!(content, original);
        assert!(repo_app.join("pattern.log").exists());
    }
}
