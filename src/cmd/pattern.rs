use crate::config::BackupConfig;
use std::fs;
use std::path::{Component, PathBuf};

/// Adds ignore paths to an app in backup.toml
pub fn run(
    paths: &crate::AppPaths,
    paths_to_ignore: Vec<PathBuf>,
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

    for target in paths_to_ignore {
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
                        "Ignore",
                        &format!(
                            "Path {} does not have a valid app directory.",
                            target.display()
                        ),
                    );
                    continue;
                }
            };

            let pattern = components.as_path().to_string_lossy().into_owned();
            if pattern.is_empty() {
                ui.warn("Ignore", &format!("Path {} is a top-level app directory. Ignoring the entire app is not supported by this command.", target.display()));
                continue;
            }

            ui.status(
                "INFO",
                "Config",
                &format!(
                    "Adding '{}' to ignores for app '{}' in category '{}'...",
                    pattern, app_name, cat_name
                ),
            );

            if !dry_run {
                let has_legacy = doc.contains_table(&app_name);

                if !doc.contains_table(&cat_name) {
                    let table = toml_edit::Table::new();
                    doc.insert(&cat_name, toml_edit::Item::Table(table));
                }
                let cat_table = doc[&cat_name].as_table_mut().unwrap();

                let mut use_legacy_root = false;
                if cat_name == "config" && !cat_table.contains_table(&app_name) && has_legacy {
                    use_legacy_root = true;
                }

                let app_table = if use_legacy_root {
                    doc[&app_name].as_table_mut().unwrap()
                } else {
                    if !cat_table.contains_table(&app_name) {
                        let table = toml_edit::Table::new();
                        cat_table.insert(&app_name, toml_edit::Item::Table(table));
                    }
                    cat_table[&app_name].as_table_mut().unwrap()
                };

                if !app_table.contains_key("ignores") {
                    app_table["ignores"] =
                        toml_edit::Item::Value(toml_edit::Value::Array(toml_edit::Array::new()));
                }

                if let Some(arr) = app_table["ignores"].as_array_mut() {
                    let mut found = false;
                    for item in arr.iter() {
                        if item.as_str() == Some(&pattern) {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        arr.push(&pattern);
                        modified = true;

                        // Optional: Sort alphabetically
                        let mut strings: Vec<String> = arr
                            .iter()
                            .filter_map(|i| i.as_str().map(|s| s.to_string()))
                            .collect();
                        strings.sort();

                        arr.clear();
                        for s in strings {
                            arr.push(s);
                        }
                    } else {
                        ui.status(
                            "SKIP",
                            "Config",
                            &format!(
                                "Pattern '{}' already exists for app '{}'.",
                                pattern, app_name
                            ),
                        );
                    }
                }
            } else {
                ui.status("SKIP", "Config", "Dry run - no modifications made.");
            }
        } else {
            ui.warn(
                "Ignore",
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
        ui.done("Successfully updated ignores in backup.toml (run `bkzyn save` to commit)");
    } else if !modified && !dry_run {
        ui.status("INFO", "Config", "No changes made to backup.toml.");
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
    fn test_pattern_missing_toml_fails() {
        let (_dir, paths) = setup_test_env();
        let target = paths.xdg_config.join("myapp").join("pattern.log");
        let result = run(&paths, vec![target], false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("backup.toml"));
    }

    #[test]
    fn test_pattern_add_ignore() {
        let (_dir, paths) = setup_test_env();
        let toml_path = write_backup_toml(&paths, "[config]\n[myapp]\n");
        let target = paths.xdg_config.join("myapp").join("pattern.log");

        run(&paths, vec![target], false, false).unwrap();

        let content = fs::read_to_string(&toml_path).unwrap();
        assert!(content.contains("ignores"));
        assert!(content.contains("pattern.log"));
    }

    #[test]
    fn test_pattern_duplicate_not_added_twice() {
        let (_dir, paths) = setup_test_env();
        let toml_path =
            write_backup_toml(&paths, "[config]\n[myapp]\nignores = [\"pattern.log\"]\n");
        let target = paths.xdg_config.join("myapp").join("pattern.log");

        // Add same pattern twice.
        run(&paths, vec![target], false, false).unwrap();

        let content = fs::read_to_string(&toml_path).unwrap();
        // Pattern must appear exactly once.
        assert_eq!(content.matches("pattern.log").count(), 1);
    }

    #[test]
    fn test_pattern_dry_run_no_modification() {
        let (_dir, paths) = setup_test_env();
        let original = "[config]\n[myapp]\n";
        let toml_path = write_backup_toml(&paths, original);
        let target = paths.xdg_config.join("myapp").join("pattern.log");

        run(&paths, vec![target], true, false).unwrap();

        // File must be unchanged after dry-run.
        let content = fs::read_to_string(&toml_path).unwrap();
        assert_eq!(content, original);
    }
}
