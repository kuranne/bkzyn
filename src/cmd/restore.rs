use crate::config::BackupConfig;
use ignore::WalkBuilder;
use minijinja::Environment;
use std::fs;
use std::path::{Path, PathBuf};

/// Restores and copies configurations from repository config directory to the system.
pub fn run(
    paths: &crate::AppPaths,
    target_paths: Vec<PathBuf>,
    dry_run: bool,
    verbose: bool,
    strict: bool,
    skip_secrets: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    let toml_path = paths
        .get_backup_toml_path()
        .ok_or("backup.toml not found in configuration or repository directory")?;
    let config = BackupConfig::load(toml_path)?;

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

    env.add_global("host", minijinja::Value::from_serialize(&host_context));

    // Determine category paths
    // Map of cat_name -> (repo_dir, host_dir)
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

        if host_dir.as_os_str().is_empty() {
            continue;
        }

        let repo_dir = paths.repo.join("data").join(cat_name);
        category_dirs.insert(cat_name.clone(), (repo_dir, host_dir));
    }

    if target_paths.is_empty() {
        ui.status("INFO", "Restore", "Restoring all categories from backup...");
        for (repo_dir, host_dir) in category_dirs.values() {
            if !repo_dir.exists() {
                continue;
            }
            if !dry_run {
                fs::create_dir_all(host_dir)?;
            }
            restore_directory(
                repo_dir,
                repo_dir,
                host_dir,
                &env,
                &ui,
                dry_run,
                strict,
                skip_secrets,
            )?;
        }
    } else {
        ui.status("INFO", "Restore", "Restoring specific target paths...");
        for target in target_paths {
            let target_abs = if target.is_absolute() {
                target.clone()
            } else {
                std::env::current_dir()?.join(&target)
            };

            let mut matched = false;
            for (repo_dir, host_dir) in category_dirs.values() {
                if let Ok(rel_path) = target_abs.strip_prefix(host_dir) {
                    matched = true;
                    let repo_target = repo_dir.join(rel_path);

                    let mut tmpl_target = repo_target.clone();
                    let file_name = tmpl_target.file_name().unwrap_or_default().to_os_string();
                    let mut tmpl_name = file_name.clone();
                    tmpl_name.push(".tmpl");
                    tmpl_target.set_file_name(tmpl_name);

                    if repo_target.exists() && repo_target.is_dir() {
                        restore_directory(
                            repo_dir,
                            &repo_target,
                            &target_abs,
                            &env,
                            &ui,
                            dry_run,
                            strict,
                            skip_secrets,
                        )?;
                    } else if repo_target.exists() || tmpl_target.exists() {
                        let actual_src = if tmpl_target.exists() {
                            tmpl_target
                        } else {
                            repo_target
                        };
                        let app_name = rel_path
                            .components()
                            .next()
                            .map(|c| c.as_os_str().to_string_lossy().into_owned())
                            .unwrap_or_default();
                        restore_file(
                            &actual_src,
                            &target_abs,
                            &app_name,
                            &env,
                            &ui,
                            dry_run,
                            strict,
                            skip_secrets,
                        )?;
                    } else {
                        ui.warn(
                            "Restore",
                            &format!(
                                "Path {} does not exist in backup repository.",
                                target.display()
                            ),
                        );
                    }
                    break;
                }
            }
            if !matched {
                ui.warn(
                    "Restore",
                    &format!(
                        "Path {} does not match any known backup category host path.",
                        target.display()
                    ),
                );
            }
        }
    }

    ui.done("Successful restore");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn restore_directory(
    base_repo_dir: &Path,
    repo_dir: &Path,
    host_dir: &Path,
    env: &Environment,
    ui: &crate::cli::CliManager,
    dry_run: bool,
    strict: bool,
    skip_secrets: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let walker = WalkBuilder::new(repo_dir).standard_filters(false).build();

    for result in walker {
        let entry = result?;
        let src_path = entry.path();
        let file_type = entry.file_type();
        if file_type
            .as_ref()
            .is_none_or(|ft| ft.is_dir() || (!ft.is_file() && !ft.is_symlink()))
        {
            continue;
        }

        let rel_to_base = src_path.strip_prefix(base_repo_dir).unwrap_or(src_path);
        let app_name = rel_to_base
            .components()
            .next()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_default();

        let rel_path = src_path.strip_prefix(repo_dir)?;
        let is_tmpl = src_path.extension().is_some_and(|ext| ext == "tmpl");
        let is_gpg = src_path.extension().is_some_and(|ext| ext == "gpg")
            && src_path.to_string_lossy().ends_with(".tar.zst.gpg");

        let mut dest_rel_path = rel_path.to_path_buf();
        if is_tmpl {
            dest_rel_path.set_extension("");
        } else if is_gpg {
            let p_str = dest_rel_path.to_string_lossy().replace(".tar.zst.gpg", "");
            dest_rel_path = PathBuf::from(p_str);
        }
        let dest_path = host_dir.join(&dest_rel_path);

        restore_file(
            src_path,
            &dest_path,
            &app_name,
            env,
            ui,
            dry_run,
            strict,
            skip_secrets,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn restore_file(
    src_path: &Path,
    dest_path: &Path,
    app_name: &str,
    env: &Environment,
    ui: &crate::cli::CliManager,
    dry_run: bool,
    strict: bool,
    skip_secrets: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_gpg = src_path.extension().is_some_and(|ext| ext == "gpg")
        && src_path.to_string_lossy().ends_with(".tar.zst.gpg");

    if is_gpg {
        if skip_secrets {
            ui.status(
                "SKIP",
                app_name,
                &format!(
                    "Secret {} skipped due to --skip-secrets",
                    dest_path.display()
                ),
            );
            return Ok(());
        }
        if !crate::command_exists("gpg") {
            ui.warn(
                "Security",
                &format!("GPG not installed, skipping secret {}", dest_path.display()),
            );
            return Ok(());
        }

        ui.status(
            "DECRYPT",
            app_name,
            &format!("Decrypting {}", dest_path.display()),
        );
        if !dry_run {
            let temp_dir = tempfile::tempdir()?;
            let temp_tar = temp_dir.path().join("secret.tar.zst");
            let status = std::process::Command::new("gpg")
                .args(["--batch", "--yes", "--decrypt", "-o"])
                .arg(&temp_tar)
                .arg(src_path)
                .status()?;
            if !status.success() {
                ui.warn(
                    "Security",
                    &format!("Failed to decrypt {}", src_path.display()),
                );
                return Ok(());
            }

            let tar_zst_file = fs::File::open(&temp_tar)?;
            let dec = zstd::Decoder::new(tar_zst_file)?;
            let mut tar = tar::Archive::new(dec);

            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
                tar.unpack(parent)?;
            }
        }
        return Ok(());
    }

    let is_tmpl = src_path.extension().is_some_and(|ext| ext == "tmpl");

    if is_tmpl {
        ui.status(
            "RENDER",
            app_name,
            &format!("{} -> {}", src_path.display(), dest_path.display()),
        );
        let template_content = fs::read_to_string(src_path)?;
        match env.render_str(&template_content, minijinja::context!()) {
            Ok(rendered) => {
                if !dry_run {
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    if dest_path.exists() {
                        let meta = fs::symlink_metadata(dest_path)?;
                        if meta.is_dir() {
                            let backup_path = PathBuf::from(format!("{}.bak", dest_path.display()));
                            fs::rename(dest_path, &backup_path)?;
                        } else {
                            fs::remove_file(dest_path)?;
                        }
                    }
                    fs::write(dest_path, rendered)?;
                }
            }
            Err(e) => {
                if strict {
                    return Err(
                        format!("Failed to render template {}: {}", src_path.display(), e).into(),
                    );
                } else {
                    ui.warn(
                        "Render",
                        &format!("Failed to render template {}: {}", src_path.display(), e),
                    );
                }
            }
        }
    } else {
        ui.status(
            "COPY",
            app_name,
            &format!("{} -> {}", src_path.display(), dest_path.display()),
        );
        if !dry_run {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            if dest_path.exists() {
                let meta = fs::symlink_metadata(dest_path)?;
                if meta.is_dir() {
                    let backup_path = PathBuf::from(format!("{}.bak", dest_path.display()));
                    fs::rename(dest_path, &backup_path)?;
                } else {
                    fs::remove_file(dest_path)?;
                }
            }
            fs::copy(src_path, dest_path)?;
        }
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
        fs::create_dir_all(&paths.xdg_data).unwrap();

        let bkzyn_dir = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&bkzyn_dir).unwrap();
        fs::write(
            bkzyn_dir.join("backup.toml"),
            r#"
[config]
whitelists = ["myapp"]
"#,
        )
        .unwrap();

        (dir, paths)
    }

    #[test]
    fn test_restore_missing_config_dir() {
        let (_dir, paths) = setup_test_env();
        fs::remove_dir_all(&paths.config).unwrap(); // remove it
        let result = run(&paths, Vec::new(), false, false, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_restore_normal_file() {
        let (_dir, paths) = setup_test_env();
        let app_dir = paths.config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("file.txt"), "hello world").unwrap();

        run(&paths, Vec::new(), false, false, false, false).unwrap();

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

        run(&paths, Vec::new(), false, false, false, false).unwrap();

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

        run(&paths, Vec::new(), false, false, false, false).unwrap();

        let dest_bad = paths.xdg_config.join("myapp").join("bad");
        assert!(!dest_bad.exists()); // failed to render, so not created

        let dest_good = paths.xdg_config.join("myapp").join("good.txt");
        assert!(dest_good.exists()); // but the rest of the files still worked
    }

    #[test]
    fn test_restore_template_fail_with_strict_aborts() {
        let (_dir, paths) = setup_test_env();
        let app_dir = paths.config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        // invalid template syntax
        fs::write(
            app_dir.join("bad.tmpl"),
            "{{ undefined.var.that.causes.error",
        )
        .unwrap();

        let result = run(&paths, Vec::new(), false, false, true, false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to render template"));
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

        run(&paths, Vec::new(), false, false, false, false).unwrap();

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
