use crate::config::BackupConfig;
use chrono::Local;
use ignore::WalkBuilder;
use std::fs;
use tar::Builder;

/// Backs up local configurations to the repository config directory.
pub fn run(paths: &crate::AppPaths, dry_run: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let ui = crate::cli::CliManager::new(verbose);

    if !paths.old.exists() {
        fs::create_dir_all(&paths.old)?;
    }

    // 1. Backup existing config
    if paths.config.exists() {
        let date_str = Local::now().format("%Y-%m-%dT%H%M%S").to_string();
        let archive_name = format!("config_{}.tar.zst", date_str);
        let archive_path = paths.old.join(&archive_name);

        ui.status("INFO", "Backup", &format!("Backing up current config to {}", archive_path.display()));

        if !dry_run {
            let tar_zst_file = fs::File::create(&archive_path)?;
            let enc = zstd::Encoder::new(tar_zst_file, 3)?;
            let mut tar = Builder::new(enc);
            tar.append_dir_all("config", &paths.config)?;
            let enc = tar.into_inner()?;
            enc.finish()?;
        }
    }

    // 2. Read config
    let toml_path = paths.repo.join("backup.toml");
    if !toml_path.exists() {
        return Err("backup.toml not found in repository directory".into());
    }
    let config = BackupConfig::load(toml_path)?;

    // 3. Sync from XDG_CONFIG_HOME
    if !paths.config.exists() && !dry_run {
        fs::create_dir_all(&paths.config)?;
    }

    for app in config.configs {
        if app.contains('/') || app.contains('\\') || app == ".." || app == "." {
            ui.warn("Security", &format!("Skipping invalid app name '{}' to prevent path traversal.", app));
            continue;
        }

        let src_path = paths.xdg_config.join(&app);
        if !src_path.exists() {
            ui.status("SKIP", &app, &format!("Not found at {}", src_path.display()));
            continue;
        }

        let dest_path = paths.config.join(&app);
        ui.status("INFO", "Sync", &format!("Syncing {} to repository...", app));

        let item_config = config.items.get(&app);
        let mut exclude_builder = globset::GlobSetBuilder::new();
        let mut include_builder = globset::GlobSetBuilder::new();

        if let Some(cfg) = item_config {
            if let Some(excludes) = &cfg.exclude {
                for ex in excludes {
                    exclude_builder.add(globset::Glob::new(ex)?);
                }
            }
            if let Some(includes) = &cfg.include {
                for inc in includes {
                    include_builder.add(globset::Glob::new(inc)?);
                }
            }
        }

        let exclude_set = exclude_builder.build()?;
        let include_set = include_builder.build()?;

        let walker = WalkBuilder::new(&src_path).standard_filters(false).build();

        for result in walker {
            let entry = result?;

            // Skip directories and non-regular files (like sockets, fifos, etc.)
            let file_type = entry.file_type();
            if file_type.as_ref().map_or(true, |ft| {
                ft.is_dir() || (!ft.is_file() && !ft.is_symlink())
            }) {
                continue;
            }

            let rel_path = entry.path().strip_prefix(&src_path)?;
            let dest_file = dest_path.join(rel_path);

            // 1. Is it explicitly excluded?
            if exclude_set.is_match(rel_path) {
                continue;
            }

            // 2. Is it explicitly included OR does it already exist in the repo?
            let is_included = include_set.is_match(rel_path);
            let already_exists_in_repo = dest_file.exists();

            if !is_included && !already_exists_in_repo {
                continue;
            }

            ui.status("COPY", &app, &format!("{}", rel_path.display()));

            if !dry_run {
                if let Some(parent) = dest_file.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Gracefully catch copy errors so it doesn't crash the whole backup
                if let Err(e) = fs::copy(entry.path(), &dest_file) {
                    ui.warn("Copy", &format!("Failed to copy {} ({}) - skipping.", rel_path.display(), e));
                }
            }
        }
    }

    ui.done("Successful backup");
    Ok(())
}
