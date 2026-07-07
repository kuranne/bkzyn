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

    let mut backup_toml_path = paths.xdg_config.join("bkzyn").join("backup.toml");
    if !backup_toml_path.exists() {
        backup_toml_path = paths.config.join("bkzyn").join("backup.toml");
    }

    if !backup_toml_path.exists() {
        return Err(format!(
            "Could not find backup.toml at {}",
            backup_toml_path.display()
        )
        .into());
    }

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
