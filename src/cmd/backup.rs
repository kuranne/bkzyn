use crate::config::BackupConfig;
use chrono::Local;
use ignore::WalkBuilder;
use std::fs;
use tar::Builder;

/// Backs up local configurations to the repository config directory.
pub fn run(
    paths: &crate::AppPaths,
    set_url: Option<&str>,
    dry_run: bool,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    let data_dir = paths.repo.join("data");

    // Set Git URL if provided
    if let Some(url) = set_url {
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
            std::process::Command::new("git")
                .current_dir(&data_dir)
                .arg("init")
                .status()?;
        }

        let status = std::process::Command::new("git")
            .current_dir(&data_dir)
            .args(["remote", "add", "origin", url])
            .status()?;

        if !status.success() {
            // If origin already exists, try set-url
            let set_status = std::process::Command::new("git")
                .current_dir(&data_dir)
                .args(["remote", "set-url", "origin", url])
                .status()?;
            if !set_status.success() {
                ui.warn("Backup", "Failed to set remote URL for data repository");
            }
        }
        ui.status(
            "INFO",
            "Backup",
            &format!("Set data repository URL to {}", url),
        );
    }

    if !paths.old.exists() {
        fs::create_dir_all(&paths.old)?;
    }

    // 1. Backup existing data categories
    let data_dir = paths.repo.join("data");
    if data_dir.exists() {
        let date_str = Local::now().format("%Y-%m-%dT%H%M%S").to_string();

        for entry in fs::read_dir(&data_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let cat_name = entry.file_name();
                let cat_name_str = cat_name.to_string_lossy();
                let cat_old_dir = paths.old.join(&cat_name);

                if !cat_old_dir.exists() {
                    fs::create_dir_all(&cat_old_dir)?;
                }

                let archive_name = format!("{}.tar.zst", date_str);
                let archive_path = cat_old_dir.join(&archive_name);

                ui.status(
                    "INFO",
                    "Backup",
                    &format!("Archiving {} to {}", cat_name_str, archive_path.display()),
                );

                if !dry_run {
                    let tar_zst_file = fs::File::create(&archive_path)?;
                    let enc = zstd::Encoder::new(tar_zst_file, 3)?;
                    let mut tar = Builder::new(enc);
                    tar.append_dir_all(&cat_name, entry.path())?;
                    let enc = tar.into_inner()?;
                    enc.finish()?;

                    // Clean up old archives, keeping only the 5 most recent
                    if let Ok(entries) = fs::read_dir(&cat_old_dir) {
                        let mut archives: Vec<std::path::PathBuf> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| e.path())
                            .filter(|p| p.extension().is_some_and(|ext| ext == "zst"))
                            .collect();

                        // Sort alphabetically (date format YYYY-MM-DDTHHMMSS makes this chronological)
                        archives.sort();

                        if archives.len() > 5 {
                            for archive in archives.iter().take(archives.len() - 5) {
                                let _ = fs::remove_file(archive);
                            }
                        }
                    }
                }
            }
        }
    }

    // 2. Read config
    let toml_path = paths
        .get_backup_toml_path()
        .ok_or("backup.toml not found in configuration or repository directory")?;
    let config = BackupConfig::load(toml_path)?;

    // 3. Sync categories
    for (cat_name, cat_cfg) in config.categories() {
        let apps_to_backup = cat_cfg.whitelist.clone().unwrap_or_default();

        let mut src_base = match cat_name.as_str() {
            "config" => paths.xdg_config.clone(),
            "dataHome" => paths.xdg_data.clone(),
            _ => std::path::PathBuf::new(),
        };

        if let Some(custom_path) = &cat_cfg.path {
            if custom_path.starts_with("~/") {
                if let Some(home) = dirs::home_dir() {
                    src_base = home.join(custom_path.strip_prefix("~/").unwrap());
                } else {
                    src_base = std::path::PathBuf::from(custom_path);
                }
            } else {
                src_base = std::path::PathBuf::from(custom_path);
            }
        }

        if src_base.as_os_str().is_empty() {
            ui.warn(
                "Config",
                &format!(
                    "Category '{}' has no default path and no path defined. Skipping.",
                    cat_name
                ),
            );
            continue;
        }

        let dest_base = paths.repo.join("data").join(cat_name);
        if !dest_base.exists() && !dry_run {
            fs::create_dir_all(&dest_base)?;
        }

        for app in apps_to_backup {
            if app.contains('/') || app.contains('\\') || app == ".." || app == "." {
                ui.warn(
                    "Security",
                    &format!(
                        "Skipping invalid app name '{}' to prevent path traversal.",
                        app
                    ),
                );
                continue;
            }

            let src_path = src_base.join(&app);
            if !src_path.exists() {
                ui.status(
                    "SKIP",
                    &app,
                    &format!("Not found at {}", src_path.display()),
                );
                continue;
            }

            let dest_path = dest_base.join(&app);
            ui.status("INFO", "Sync", &format!("Syncing {} to repository...", app));

            let mut ignore_builder = globset::GlobSetBuilder::new();
            let mut whitelist_builder = globset::GlobSetBuilder::new();

            // 1. Global rules
            if let Some(ignores) = &config.ignores {
                for ex in ignores {
                    ignore_builder.add(globset::Glob::new(ex)?);
                }
            }
            if let Some(whitelists) = &config.whitelist {
                for inc in whitelists {
                    whitelist_builder.add(globset::Glob::new(inc)?);
                }
            }

            // 2. Global app rules
            if let Some(app_cfg) = config.global_apps().get(&app) {
                if let Some(ignores) = &app_cfg.ignores {
                    for ex in ignores {
                        ignore_builder.add(globset::Glob::new(ex)?);
                    }
                }
                if let Some(whitelists) = &app_cfg.whitelist {
                    for inc in whitelists {
                        whitelist_builder.add(globset::Glob::new(inc)?);
                    }
                }
            }

            // 3. Section rules
            if let Some(ignores) = &cat_cfg.ignores {
                for ex in ignores {
                    ignore_builder.add(globset::Glob::new(ex)?);
                }
            }
            if let Some(whitelists) = &cat_cfg.whitelist {
                for inc in whitelists {
                    whitelist_builder.add(globset::Glob::new(inc)?);
                }
            }

            // 4. Section app rules
            if let Some(app_cfg) = cat_cfg.apps.get(&app) {
                if let Some(ignores) = &app_cfg.ignores {
                    for ex in ignores {
                        ignore_builder.add(globset::Glob::new(ex)?);
                    }
                }
                if let Some(whitelists) = &app_cfg.whitelist {
                    for inc in whitelists {
                        whitelist_builder.add(globset::Glob::new(inc)?);
                    }
                }
            }

            // 5. Plural whitelist/ignores blocks
            use crate::config::RuleMap;
            if let Some(ignores_map) = &config.ignores_map {
                if let Some(RuleMap::AppList(list)) = ignores_map.get(&app) {
                    for ex in list {
                        ignore_builder.add(globset::Glob::new(ex)?);
                    }
                }
                if let Some(RuleMap::CategoryMap(cmap)) = ignores_map.get(cat_name) {
                    if let Some(list) = cmap.get(&app) {
                        for ex in list {
                            ignore_builder.add(globset::Glob::new(ex)?);
                        }
                    }
                }
            }

            if let Some(whitelists) = &config.whitelists {
                if let Some(RuleMap::AppList(list)) = whitelists.get(&app) {
                    for inc in list {
                        whitelist_builder.add(globset::Glob::new(inc)?);
                    }
                }
                if let Some(RuleMap::CategoryMap(cmap)) = whitelists.get(cat_name) {
                    if let Some(list) = cmap.get(&app) {
                        for inc in list {
                            whitelist_builder.add(globset::Glob::new(inc)?);
                        }
                    }
                }
            }

            let ignore_set = ignore_builder.build()?;
            let whitelist_set = whitelist_builder.build()?;

            let walker = WalkBuilder::new(&src_path).standard_filters(false).build();

            for result in walker {
                let entry = result?;

                // Skip directories and non-regular files (like sockets, fifos, etc.)
                let file_type = entry.file_type();
                if file_type
                    .as_ref()
                    .is_none_or(|ft| ft.is_dir() || (!ft.is_file() && !ft.is_symlink()))
                {
                    continue;
                }

                let rel_path = entry.path().strip_prefix(&src_path)?;
                let dest_file = dest_path.join(rel_path);

                let is_whitelisted = whitelist_set.is_match(rel_path);

                // 1. Is it explicitly ignored?
                let is_ignored = ignore_set.is_match(rel_path);

                // Ignore it UNLESS it is explicitly whitelisted (whitelist overrides ignore)
                if is_ignored && !is_whitelisted {
                    continue;
                }

                // 2. Is it explicitly whitelisted OR does it already exist in the repo?
                let already_exists_in_repo = dest_file.exists();

                if !is_whitelisted && !already_exists_in_repo {
                    continue;
                }

                ui.status("COPY", &app, &format!("{}", rel_path.display()));

                if !dry_run {
                    if let Some(parent) = dest_file.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    // Gracefully catch copy errors so it doesn't crash the whole backup
                    if let Err(e) = fs::copy(entry.path(), &dest_file) {
                        ui.warn(
                            "Copy",
                            &format!("Failed to copy {} ({}) - skipping.", rel_path.display(), e),
                        );
                    }
                }
            }

            // Sweep destination for zombie files (files that exist in backup but were deleted from system)
            if dest_path.exists() {
                let dest_walker = WalkBuilder::new(&dest_path).standard_filters(false).build();
                let mut to_delete = Vec::new();

                for entry in dest_walker.flatten() {
                    let file_type = entry.file_type();
                    if file_type.as_ref().is_none_or(|ft| ft.is_dir()) {
                        continue;
                    }

                    if let Ok(rel_path) = entry.path().strip_prefix(&dest_path) {
                        let src_file = src_path.join(rel_path);

                        if !src_file.exists() && !ignore_set.is_match(rel_path) {
                            to_delete.push((rel_path.to_path_buf(), entry.path().to_path_buf()));
                        }
                    }
                }

                for (rel, full) in to_delete {
                    ui.status(
                        "DELETE",
                        &app,
                        &format!("Zombie file removed: {}", rel.display()),
                    );
                    if !dry_run {
                        let _ = fs::remove_file(&full);
                    }
                }
            }
        }
    }

    ui.done("Successful backup");
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
        fs::create_dir_all(&paths.data).unwrap();
        fs::create_dir_all(&paths.xdg_config).unwrap();
        fs::create_dir_all(&paths.xdg_data).unwrap();
        (dir, paths)
    }

    fn write_config(paths: &AppPaths, toml_str: &str) {
        let backup_dir = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&backup_dir).unwrap();
        fs::write(backup_dir.join("backup.toml"), toml_str).unwrap();
    }

    #[test]
    fn test_backup_missing_toml() {
        let (_dir, paths) = setup_test_env();
        let result = run(&paths, None, false, false);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "backup.toml not found in configuration or repository directory"
        );
    }

    #[test]
    fn test_backup_normal_and_ignore() {
        let (_dir, paths) = setup_test_env();

        // Set up the local xdg config
        let app_dir = paths.xdg_config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("file.txt"), "hello").unwrap();
        fs::write(app_dir.join("secret.key"), "do not backup").unwrap();

        // First attempt: no whitelist pattern, so it won't backup anything unless the repo dir exists
        let toml_str = r#"
[config]
whitelist = ["myapp"]
[config.myapp]
ignores = ["*.key"]
"#;
        write_config(&paths, toml_str);

        // Pre-create the repo dir so it counts as "already exists" for whitelist logic
        fs::create_dir_all(paths.config.join("myapp")).unwrap();

        // Second attempt: explicit whitelist pattern
        let toml_str = r#"
[config]
whitelist = ["myapp"]
[config.myapp]
whitelist = ["file.txt"]
ignores = ["*.key"]
"#;
        write_config(&paths, toml_str);

        run(&paths, None, false, false).unwrap();

        // Check if file.txt was backed up
        assert!(paths.config.join("myapp").join("file.txt").exists());
        // Check if secret.key was skipped
        assert!(!paths.config.join("myapp").join("secret.key").exists());
    }

    #[test]
    fn test_backup_weird_path_traversal() {
        let (_dir, paths) = setup_test_env();

        let toml_str = r#"
[config]
whitelist = ["../escaped", "normal"]
"#;
        write_config(&paths, toml_str);

        let escaped_dir = paths.xdg_config.join("../escaped");
        fs::create_dir_all(&escaped_dir).unwrap();
        fs::write(escaped_dir.join("hack.txt"), "hacked").unwrap();

        let normal_dir = paths.xdg_config.join("normal");
        fs::create_dir_all(&normal_dir).unwrap();

        // The path traversal should trigger the security warning and skip it, continuing fine.
        run(&paths, None, false, false).unwrap();

        // Ensure we didn't back it up into the repo under a literal directory
        assert!(!paths.config.join("escaped").exists());
        assert!(!paths.config.join("..").join("escaped_backup").exists());
    }

    #[test]
    fn test_backup_archives_old_state() {
        let (_dir, paths) = setup_test_env();
        fs::create_dir_all(paths.config.join("oldapp")).unwrap();
        fs::write(paths.config.join("oldapp").join("old.txt"), "old").unwrap();

        let toml_str = r#"
[config]
whitelist = []
"#;
        write_config(&paths, toml_str);

        run(&paths, None, false, false).unwrap();

        // Ensure an archive was created in paths.old/config
        let mut config_old_dir = fs::read_dir(paths.old.join("config")).unwrap();
        let archive = config_old_dir.next().unwrap().unwrap();
        assert!(archive.file_name().to_string_lossy().ends_with(".tar.zst"));
    }

    #[test]
    fn test_backup_plural_ignores_whitelists() {
        let (_dir, paths) = setup_test_env();

        let app_dir = paths.xdg_config.join("myapp");
        fs::create_dir_all(&app_dir).unwrap();
        fs::write(app_dir.join("keep.txt"), "keep").unwrap();
        fs::write(app_dir.join("drop.txt"), "drop").unwrap();

        let toml_str = r#"
[config]
whitelist = ["myapp"]

[whitelists.config]
myapp = ["keep.txt"]

[ignores_map.config]
myapp = ["drop.txt"]
"#;
        write_config(&paths, toml_str);

        // Pre-create the repo dir
        fs::create_dir_all(paths.config.join("myapp")).unwrap();

        run(&paths, None, false, false).unwrap();

        assert!(paths.config.join("myapp").join("keep.txt").exists());
        assert!(!paths.config.join("myapp").join("drop.txt").exists());
    }
}
