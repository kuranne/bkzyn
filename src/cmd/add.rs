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
        let mut backup_toml_path = paths.xdg_config.join("bkzyn").join("backup.toml");
        if !backup_toml_path.exists() {
            // Check repo if not in XDG
            backup_toml_path = paths.config.join("bkzyn").join("backup.toml");
        }

        if backup_toml_path.exists() {
            ui.status(
                "INFO",
                "Config",
                &format!("Updating {}...", backup_toml_path.display()),
            );
            let toml_str = fs::read_to_string(&backup_toml_path)?;
            if let Ok(mut doc) = toml_str.parse::<toml_edit::DocumentMut>() {
                if let Some(configs) = doc.get_mut("configs").and_then(|i| i.as_array_mut()) {
                    let mut found = false;
                    for item in configs.iter() {
                        if item.as_str() == Some(&app_name) {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        configs.push(&app_name);

                        // Re-sort alphabetically to keep it clean
                        let mut strings: Vec<String> = configs
                            .iter()
                            .filter_map(|i| i.as_str().map(|s| s.to_string()))
                            .collect();
                        strings.sort();

                        configs.clear();
                        for s in strings {
                            configs.push(s);
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
