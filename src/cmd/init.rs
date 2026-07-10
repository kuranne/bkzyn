use inquire::{MultiSelect, Text};
use std::fs;
use toml_edit::DocumentMut;

/// Interactively builds and updates the tracking configuration (`backup.toml`)
pub fn run(
    paths: &crate::AppPaths,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);
    ui.status("INFO", "Init", "Scanning for configuration directories...");

    let mut apps = Vec::new();
    if let Ok(entries) = fs::read_dir(&paths.xdg_config) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        apps.push(name.to_string());
                    }
                }
            }
        }
    }
    apps.sort();

    if apps.is_empty() {
        ui.warn(
            "Init",
            &format!("No directories found in {}", paths.xdg_config.display()),
        );
        return Ok(());
    }

    let selected_apps =
        match MultiSelect::new("Which apps would you like to track and backup?", apps).prompt() {
            Ok(s) => s,
            Err(_) => return Ok(()),
        };

    if selected_apps.is_empty() {
        ui.status("INFO", "Init", "No apps selected. Exiting.");
        return Ok(());
    }

    let backup_toml_path = paths.xdg_config.join("bkzyn").join("backup.toml");
    let mut toml_str = String::new();
    if backup_toml_path.exists() {
        toml_str = fs::read_to_string(&backup_toml_path)?;
    }
    let mut doc = toml_str.parse::<DocumentMut>().unwrap_or_default();

    if !doc.contains_table("config") {
        doc["config"] = toml_edit::Item::Table(toml_edit::table().into_table().unwrap());
    }
    let config_table = doc["config"].as_table_mut().unwrap();

    if !config_table.contains_key("whitelists") {
        config_table["whitelists"] =
            toml_edit::Item::Value(toml_edit::Value::Array(toml_edit::Array::new()));
    }
    for app in selected_apps {
        // Ensure added to global whitelists
        if let Some(global_whitelists) = config_table["whitelists"].as_array_mut() {
            let mut found = false;
            for item in global_whitelists.iter() {
                if item.as_str() == Some(&app) {
                    found = true;
                    break;
                }
            }
            if !found {
                global_whitelists.push(&app);
            }
        }

        // Ask for ignores
        let ignores_input = Text::new(&format!(
            "Any files to ignore in {}? (comma-separated globs, or leave empty)",
            app
        ))
        .prompt()
        .unwrap_or_default();
        let ignores: Vec<String> = ignores_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Ask for secrets
        let secrets_input = Text::new(&format!(
            "Any files to treat as secrets in {}? (comma-separated globs, or leave empty)",
            app
        ))
        .prompt()
        .unwrap_or_default();
        let secrets: Vec<String> = secrets_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if !ignores.is_empty() || !secrets.is_empty() {
            if !config_table.contains_table(&app) {
                let table = toml_edit::Table::new();
                config_table.insert(&app, toml_edit::Item::Table(table));
            }
            let app_table = config_table[&app].as_table_mut().unwrap();

            if !ignores.is_empty() {
                if !app_table.contains_key("ignores") {
                    app_table["ignores"] =
                        toml_edit::Item::Value(toml_edit::Value::Array(toml_edit::Array::new()));
                }
                let arr = app_table["ignores"].as_array_mut().unwrap();
                for ig in ignores {
                    let mut found = false;
                    for item in arr.iter() {
                        if item.as_str() == Some(&ig) {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        arr.push(ig);
                    }
                }
            }

            if !secrets.is_empty() {
                if !app_table.contains_key("secrets") {
                    app_table["secrets"] =
                        toml_edit::Item::Value(toml_edit::Value::Array(toml_edit::Array::new()));
                }
                let arr = app_table["secrets"].as_array_mut().unwrap();
                for sec in secrets {
                    let mut found = false;
                    for item in arr.iter() {
                        if item.as_str() == Some(&sec) {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        arr.push(sec);
                    }
                }
            }
        }
    }

    if !dry_run {
        if let Some(parent) = backup_toml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&backup_toml_path, doc.to_string())?;

        // Sync it to repo
        let repo_toml_path = paths.config.join("bkzyn").join("backup.toml");
        if let Some(parent) = repo_toml_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let _ = fs::copy(&backup_toml_path, &repo_toml_path);

        ui.done(
            "Interactive setup complete! Run `bkzyn backup` to apply and `bkzyn save` to commit.",
        );
    } else {
        ui.done("Dry-run finished. No changes made.");
    }

    Ok(())
}
