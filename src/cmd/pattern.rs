use std::fs;

/// Adds an include or exclude pattern to an app in backup.toml
pub fn run(
    paths: &crate::AppPaths,
    app_name: &str,
    pattern: &str,
    is_include: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);
    let action_name = if is_include { "include" } else { "exclude" };

    let backup_toml_path = paths
        .get_backup_toml_path()
        .ok_or("Could not find backup.toml")?;

    ui.status(
        "INFO",
        "Config",
        &format!(
            "Adding '{}' to {} for app '{}'...",
            pattern, action_name, app_name
        ),
    );

    if !dry_run {
        let toml_str = fs::read_to_string(&backup_toml_path)?;
        if let Ok(mut doc) = toml_str.parse::<toml_edit::DocumentMut>() {
            // Ensure the app table exists: `[app_name]`
            if !doc.contains_table(app_name) {
                let table = toml_edit::table();
                doc[app_name] = toml_edit::Item::Table(table.into_table().unwrap());
            }

            // Get or create the `include` / `exclude` array
            let table = doc[app_name].as_table_mut().unwrap();

            if !table.contains_key(action_name) {
                let mut arr = toml_edit::Array::new();
                arr.push(pattern);
                table[action_name] = toml_edit::Item::Value(toml_edit::Value::Array(arr));
            } else if let Some(arr) = table[action_name].as_array_mut() {
                let mut found = false;
                for item in arr.iter() {
                    if item.as_str() == Some(pattern) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    arr.push(pattern);

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
                }
            }

            fs::write(&backup_toml_path, doc.to_string())?;

            // Sync to repo
            let repo_toml_path = paths.config.join("bkzyn").join("backup.toml");
            if backup_toml_path != repo_toml_path {
                let _ = fs::copy(&backup_toml_path, &repo_toml_path);
            }
        } else {
            return Err("Failed to parse backup.toml".into());
        }
    } else {
        ui.status("SKIP", "Config", "Dry run - no modifications made.");
    }

    ui.done(&format!(
        "Successfully added {} pattern to {} (run `bkzyn save` to commit)",
        action_name, app_name
    ));
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
        let result = run(&paths, "myapp", "*.log", true, false, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("backup.toml"));
    }

    #[test]
    fn test_pattern_add_include() {
        let (_dir, paths) = setup_test_env();
        let toml_path = write_backup_toml(&paths, "[myapp]\n");

        run(&paths, "myapp", "*.cfg", true, false, false).unwrap();

        let content = fs::read_to_string(&toml_path).unwrap();
        assert!(content.contains("include"));
        assert!(content.contains("*.cfg"));
    }

    #[test]
    fn test_pattern_add_exclude() {
        let (_dir, paths) = setup_test_env();
        let toml_path = write_backup_toml(&paths, "[myapp]\n");

        run(&paths, "myapp", "*.log", false, false, false).unwrap();

        let content = fs::read_to_string(&toml_path).unwrap();
        assert!(content.contains("exclude"));
        assert!(content.contains("*.log"));
    }

    #[test]
    fn test_pattern_duplicate_not_added_twice() {
        let (_dir, paths) = setup_test_env();
        let toml_path =
            write_backup_toml(&paths, "[myapp]\ninclude = [\"*.cfg\"]\n");

        // Add same pattern twice.
        run(&paths, "myapp", "*.cfg", true, false, false).unwrap();

        let content = fs::read_to_string(&toml_path).unwrap();
        // Pattern must appear exactly once.
        assert_eq!(content.matches("*.cfg").count(), 1);
    }

    #[test]
    fn test_pattern_dry_run_no_modification() {
        let (_dir, paths) = setup_test_env();
        let original = "[myapp]\n";
        let toml_path = write_backup_toml(&paths, original);

        run(&paths, "myapp", "*.cfg", true, true, false).unwrap();

        // File must be unchanged after dry-run.
        let content = fs::read_to_string(&toml_path).unwrap();
        assert_eq!(content, original);
    }
}

