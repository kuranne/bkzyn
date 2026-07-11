use std::fs;
#[cfg(not(unix))]
use std::path::Path;
use std::path::{Component, PathBuf};

/// Adds a new configuration to the backup repository and registers it.
pub fn run(
    paths: &crate::AppPaths,
    paths_to_add: Vec<PathBuf>,
    ignores: Option<&[String]>,
    dry_run: bool,
    verbose: bool,
    secret: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    let backup_toml_path = paths.get_backup_toml_path();

    for path_to_add in paths_to_add {
        let absolute_path = if path_to_add.is_absolute() {
            path_to_add.clone()
        } else {
            std::env::current_dir()?.join(&path_to_add)
        };

        if !absolute_path.exists() {
            ui.warn(
                "Add",
                &format!("Path does not exist: {}", absolute_path.display()),
            );
            continue;
        }

        // Check if the path is inside xdg_config (~/.config)
        let relative_path = match absolute_path.strip_prefix(&paths.xdg_config) {
            Ok(p) => p,
            Err(_) => {
                ui.warn(
                    "Add",
                    &format!(
                        "Path {} is not inside ~/.config (XDG_CONFIG_HOME). Skipping.",
                        absolute_path.display()
                    ),
                );
                continue;
            }
        };

        let mut components = relative_path.components();
        let top_level_name = match components.next() {
            Some(Component::Normal(name)) => name.to_string_lossy().into_owned(),
            _ => {
                ui.warn("Add", "Invalid path structure inside ~/.config. Skipping.");
                continue;
            }
        };

        // Determine if we are adding the entire app folder or a specific deep file
        let is_deep_file = relative_path.components().count() > 1;

        let source_path = &absolute_path;
        let target_path = paths.config.join(&top_level_name).join(
            relative_path
                .strip_prefix(&top_level_name)
                .unwrap_or(std::path::Path::new("")),
        );

        ui.status(
            "INFO",
            &top_level_name,
            &format!("Adding {} to backup repository...", source_path.display()),
        );

        if fs::symlink_metadata(source_path)?.is_symlink() {
            ui.status("SKIP", &top_level_name, "Already a symlink.");
            continue;
        }

        if target_path.exists() {
            ui.status(
                "SKIP",
                &top_level_name,
                &format!(
                    "Target {} already exists in the repository. Skipping.",
                    target_path.display()
                ),
            );
            continue;
        }

        if !dry_run {
            if !secret {
                ui.status(
                    "COPY",
                    &top_level_name,
                    &format!("{} -> {}", source_path.display(), target_path.display()),
                );

                // Setup ignore matcher if ignores are provided
                let mut exclude_set = None;
                if let Some(ig) = ignores {
                    let mut builder = globset::GlobSetBuilder::new();
                    for pattern in ig {
                        if let Ok(glob) = globset::Glob::new(pattern) {
                            builder.add(glob);
                        }
                    }
                    exclude_set = builder.build().ok();
                }

                if source_path.is_dir() {
                    // Copy directory but apply ignores
                    if let Err(e) =
                        copy_dir_with_ignores(source_path, &target_path, source_path, &exclude_set)
                    {
                        ui.warn("Copy", &format!("Failed to copy directory: {}", e));
                        continue;
                    }
                } else {
                    if let Some(parent) = target_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(source_path, &target_path)?;
                }
            } else {
                ui.status(
                    "INFO",
                    &top_level_name,
                    "Adding secret path without copying in plaintext. Run bkzyn backup to encrypt.",
                );
            }

            // Update backup.toml if it exists
            if let Some(backup_toml_path) = &backup_toml_path {
                ui.status(
                    "INFO",
                    "Config",
                    &format!("Updating {}...", backup_toml_path.display()),
                );

                let toml_str = fs::read_to_string(backup_toml_path)?;
                if let Ok(mut doc) = toml_str.parse::<toml_edit::DocumentMut>() {
                    let array_name = if secret { "secrets" } else { "whitelists" };

                    // 1. Ensure app is in [config] list
                    if !doc.contains_table("config") {
                        doc["config"] =
                            toml_edit::Item::Table(toml_edit::table().into_table().unwrap());
                    }
                    let config_table = doc["config"].as_table_mut().unwrap();

                    if !config_table.contains_key(array_name) {
                        config_table[array_name] = toml_edit::Item::Value(toml_edit::Value::Array(
                            toml_edit::Array::new(),
                        ));
                    }

                    if let Some(whitelist) = config_table[array_name].as_array_mut() {
                        let mut found = false;
                        for item in whitelist.iter() {
                            if item.as_str() == Some(&top_level_name) {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            whitelist.push(&top_level_name);
                        }
                    }

                    // 2. If it's a deep file, ensure it's listed for the app
                    // Use the standard [config.myapp] table instead of flattened string
                    if is_deep_file {
                        let relative_to_app = relative_path
                            .strip_prefix(&top_level_name)
                            .unwrap()
                            .to_string_lossy()
                            .to_string();

                        if !config_table.contains_table(&top_level_name) {
                            let table = toml_edit::Table::new();
                            config_table.insert(&top_level_name, toml_edit::Item::Table(table));
                        }
                        let app_table = config_table[&top_level_name].as_table_mut().unwrap();

                        if !app_table.contains_key(array_name) {
                            app_table[array_name] = toml_edit::Item::Value(
                                toml_edit::Value::Array(toml_edit::Array::new()),
                            );
                        }
                        if let Some(wl) = app_table[array_name].as_array_mut() {
                            let mut found = false;
                            for item in wl.iter() {
                                if item.as_str() == Some(&relative_to_app) {
                                    found = true;
                                    break;
                                }
                            }
                            if !found {
                                wl.push(relative_to_app);
                            }
                        }
                    }

                    // 3. Inject ignores if provided
                    if let Some(ig) = ignores {
                        if !config_table.contains_table(&top_level_name) {
                            let table = toml_edit::Table::new();
                            config_table.insert(&top_level_name, toml_edit::Item::Table(table));
                        }
                        let app_table = config_table[&top_level_name].as_table_mut().unwrap();

                        if !app_table.contains_key("ignores") {
                            app_table["ignores"] = toml_edit::Item::Value(toml_edit::Value::Array(
                                toml_edit::Array::new(),
                            ));
                        }

                        if let Some(ig_arr) = app_table["ignores"].as_array_mut() {
                            for pattern in ig {
                                let mut found = false;
                                for item in ig_arr.iter() {
                                    if item.as_str() == Some(pattern) {
                                        found = true;
                                        break;
                                    }
                                }
                                if !found {
                                    ig_arr.push(pattern);
                                }
                            }
                        }
                    }

                    // Write to the source file
                    fs::write(backup_toml_path, doc.to_string())?;

                    // Sync it instantly into the repository
                    let repo_toml_path = paths.config.join("bkzyn").join("backup.toml");
                    if backup_toml_path != &repo_toml_path {
                        let _ = fs::copy(backup_toml_path, &repo_toml_path);
                    }
                }
            }
        } else {
            ui.status("SKIP", &top_level_name, "Dry run - no files moved.");
        }
    }

    ui.done("Successfully added paths (run `bkzyn save` to commit)");
    Ok(())
}

fn copy_dir_with_ignores(
    src: impl AsRef<std::path::Path>,
    dst: impl AsRef<std::path::Path>,
    base_src: impl AsRef<std::path::Path>,
    ignores: &Option<globset::GlobSet>,
) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(ig) = ignores {
            if let Ok(rel_path) = path.strip_prefix(&base_src) {
                if ig.is_match(rel_path) {
                    continue; // Skip ignored
                }
            }
        }

        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_with_ignores(
                path,
                dst.as_ref().join(entry.file_name()),
                base_src.as_ref(),
                ignores,
            )?;
        } else {
            fs::copy(path, dst.as_ref().join(entry.file_name()))?;
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

        // Warning generated and skipped, returning Ok(())
        let result = run(&paths, vec![outside_path], None, false, false, false);
        assert!(result.is_ok()); // Skip logic implemented
    }

    #[test]
    fn test_add_nonexistent_path_fails() {
        let (_dir, paths) = setup_test_env();
        let bad_path = paths.xdg_config.join("does_not_exist");

        // Warning generated and skipped, returning Ok(())
        let result = run(&paths, vec![bad_path], None, false, false, false);
        assert!(result.is_ok());
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
        fs::write(bkzyn_dir.join("backup.toml"), "[config]\nwhitelist = []\n").unwrap();

        run(&paths, vec![app_dir], None, false, false, false).unwrap();

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

        let result = run(&paths, vec![app_dir], None, false, false, false);
        assert!(result.is_ok());
        // Repo target must NOT have been created (early return before copy).
        assert!(!paths.config.join("myapp").exists());
    }

    #[test]
    fn test_add_target_exists_in_repo_skips() {
        let (_dir, paths) = setup_test_env();

        let app_dir = paths.xdg_config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();

        // Pre-create the target in the repo to trigger the conflict.
        fs::create_dir_all(paths.config.join("myapp")).unwrap();

        // Does not error anymore, just skips!
        let result = run(&paths, vec![app_dir], None, false, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_dry_run_no_side_effects() {
        let (_dir, paths) = setup_test_env();

        let app_dir = paths.xdg_config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("cfg.toml"), "x = 1").unwrap();

        let bkzyn_dir = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&bkzyn_dir).unwrap();
        fs::write(bkzyn_dir.join("backup.toml"), "[config]\nwhitelist = []\n").unwrap();

        run(&paths, vec![app_dir], None, true, false, false).unwrap();

        // Dry-run must not copy anything to the repo.
        assert!(!paths.config.join("myapp").exists());

        // backup.toml must be unchanged.
        let toml_content = fs::read_to_string(bkzyn_dir.join("backup.toml")).unwrap();
        assert!(!toml_content.contains("myapp"));
    }
}
