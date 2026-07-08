use ignore::WalkBuilder;
use minijinja::Environment;
use std::fs;
use std::path::PathBuf;

/// Restores and copies configurations from repository config directory to the system.
pub fn run(
    paths: &crate::AppPaths,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    if !paths.config.exists() {
        return Err("No config directory found in the repository.".into());
    }

    ui.status(
        "INFO",
        "Restore",
        &format!("Restoring configs to {}", paths.xdg_config.display()),
    );

    if !dry_run {
        fs::create_dir_all(&paths.xdg_config)?;
    }

    // Load host variables for templating
    let mut env = Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    let host_toml = paths.xdg_config.join("bkzyn").join("host.toml");
    let host_context: toml::Value = if host_toml.exists() {
        let content = fs::read_to_string(&host_toml)?;
        toml::from_str(&content)?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    // Add context to env as global so templates can access it
    env.add_global("host", minijinja::Value::from_serialize(&host_context));

    // Walk repository config directory
    let walker = WalkBuilder::new(&paths.config)
        .standard_filters(false)
        .build();

    for result in walker {
        let entry = result?;
        let src_path = entry.path();

        let file_type = entry.file_type();
        if file_type.as_ref().is_none_or(|ft| ft.is_dir()) {
            continue;
        }

        let rel_path = src_path.strip_prefix(&paths.config)?;
        let app_name = rel_path
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_default();

        let is_tmpl = src_path.extension().is_some_and(|ext| ext == "tmpl");
        let mut dest_rel_path = rel_path.to_path_buf();
        if is_tmpl {
            dest_rel_path.set_extension(""); // remove .tmpl
        }

        let dest_path = paths.xdg_config.join(&dest_rel_path);

        if !dry_run {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if dest_path.exists() {
                let meta = fs::symlink_metadata(&dest_path)?;
                if meta.is_dir() {
                    let backup_path = PathBuf::from(format!("{}.bak", dest_path.display()));
                    fs::rename(&dest_path, &backup_path)?;
                } else {
                    fs::remove_file(&dest_path)?;
                }
            }

            if is_tmpl {
                ui.status(
                    "RENDER",
                    &app_name,
                    &format!("{} -> {}", rel_path.display(), dest_rel_path.display()),
                );
                let template_content = fs::read_to_string(src_path)?;
                match env.render_str(&template_content, minijinja::context!()) {
                    Ok(rendered) => {
                        fs::write(&dest_path, rendered)?;
                    }
                    Err(e) => {
                        ui.warn(
                            "Render",
                            &format!("Failed to render template {}: {}", rel_path.display(), e),
                        );
                    }
                }
            } else {
                ui.status(
                    "COPY",
                    &app_name,
                    &format!("{} -> {}", src_path.display(), dest_path.display()),
                );
                fs::copy(src_path, &dest_path)?;
            }
        } else {
            if is_tmpl {
                ui.status(
                    "RENDER",
                    &app_name,
                    &format!("{} -> {}", rel_path.display(), dest_rel_path.display()),
                );
            } else {
                ui.status(
                    "COPY",
                    &app_name,
                    &format!("{} -> {}", src_path.display(), dest_path.display()),
                );
            }
        }
    }

    ui.done("Successful restore");
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
        fs::create_dir_all(&paths.xdg_data).unwrap();
        (dir, paths)
    }

    #[test]
    fn test_restore_missing_config_dir() {
        let (_dir, paths) = setup_test_env();
        fs::remove_dir_all(&paths.config).unwrap(); // remove it
        let result = run(&paths, false, false);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "No config directory found in the repository."
        );
    }

    #[test]
    fn test_restore_normal_file() {
        let (_dir, paths) = setup_test_env();
        let app_dir = paths.config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("file.txt"), "hello world").unwrap();

        run(&paths, false, false).unwrap();

        let dest_file = paths.xdg_config.join("myapp").join("file.txt");
        assert!(dest_file.exists());
        #[cfg(unix)]
        assert!(fs::symlink_metadata(&dest_file).unwrap().is_file());
        assert_eq!(fs::read_to_string(dest_file).unwrap(), "hello world");
    }

    #[test]
    fn test_restore_template_success() {
        let (_dir, paths) = setup_test_env();
        let app_dir = paths.config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(
            app_dir.join("config.toml.tmpl"),
            "key = \"{{ host.my_val }}\"",
        )
        .unwrap();

        let backup_dir = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(backup_dir.join("host.toml"), "my_val = \"super_secret\"").unwrap();

        run(&paths, false, false).unwrap();

        let dest_file = paths.xdg_config.join("myapp").join("config.toml");
        assert!(dest_file.exists());
        #[cfg(unix)]
        assert!(!fs::symlink_metadata(&dest_file).unwrap().is_symlink()); // rendered, not a symlink
        assert_eq!(
            fs::read_to_string(dest_file).unwrap(),
            "key = \"super_secret\""
        );
    }

    #[test]
    fn test_restore_template_fail_warns_but_continues() {
        let (_dir, paths) = setup_test_env();
        let app_dir = paths.config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        // invalid template syntax
        fs::write(
            app_dir.join("bad.tmpl"),
            "{{ undefined.var.that.causes.error",
        )
        .unwrap();
        fs::write(app_dir.join("good.txt"), "fine").unwrap();

        run(&paths, false, false).unwrap();

        let dest_bad = paths.xdg_config.join("myapp").join("bad");
        assert!(!dest_bad.exists()); // failed to render, so not created

        let dest_good = paths.xdg_config.join("myapp").join("good.txt");
        assert!(dest_good.exists()); // but the rest of the files still worked
    }

    #[test]
    fn test_restore_overwrite_directory_with_file() {
        let (_dir, paths) = setup_test_env();
        let app_dir = paths.config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("file.txt"), "new content").unwrap();

        // Dest already exists as a directory
        let dest_dir = paths.xdg_config.join("myapp").join("file.txt");
        fs::create_dir_all(&dest_dir).unwrap();

        run(&paths, false, false).unwrap();

        let dest_file = paths.xdg_config.join("myapp").join("file.txt");
        assert!(dest_file.exists());
        assert!(dest_file.is_file());
        assert_eq!(fs::read_to_string(&dest_file).unwrap(), "new content");

        // Ensure the old directory was backed up to file.txt.bak
        let dest_dir_bak = paths.xdg_config.join("myapp").join("file.txt.bak");
        assert!(dest_dir_bak.exists());
        assert!(dest_dir_bak.is_dir());
    }
}
