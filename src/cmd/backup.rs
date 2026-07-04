use crate::config::BackupConfig;
use chrono::Local;
use ignore::WalkBuilder;
use std::fs;
use std::path::PathBuf;
use tar::Builder;

pub fn run(dry_run: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo_dir = std::env::current_dir()?;
    let old_dir = repo_dir.join(".old");
    let config_dir = repo_dir.join("config");

    if !old_dir.exists() {
        fs::create_dir_all(&old_dir)?;
    }

    // 1. Backup existing ./config
    if config_dir.exists() {
        let date_str = Local::now().format("%Y-%m-%dT%H%M%S").to_string();
        let archive_name = format!("config_{}.tar.zst", date_str);
        let archive_path = old_dir.join(&archive_name);

        if verbose {
            println!(
                "--> Backing up current config to {}",
                archive_path.display()
            );
        }

        if !dry_run {
            let tar_zst_file = fs::File::create(&archive_path)?;
            let enc = zstd::Encoder::new(tar_zst_file, 3)?.auto_finish();
            let mut tar = Builder::new(enc);
            tar.append_dir_all("config", &config_dir)?;
            tar.finish()?;
        }
    }

    // 2. Read config
    let toml_path = repo_dir.join("backup.toml");
    if !toml_path.exists() {
        return Err("backup.toml not found in the current directory".into());
    }
    let config = BackupConfig::load(toml_path)?;

    // 3. Sync from XDG_CONFIG_HOME
    let xdg_config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".config"));

    if !config_dir.exists() && !dry_run {
        fs::create_dir_all(&config_dir)?;
    }

    for app in config.configs {
        let src_path = xdg_config_home.join(&app);
        if !src_path.exists() {
            if verbose {
                println!("--> Skipping {} (not found at {})", app, src_path.display());
            }
            continue;
        }

        let dest_path = config_dir.join(&app);
        if verbose {
            println!("--> Syncing {} to repository...", app);
        }

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

            if verbose {
                println!("    Copying {}", rel_path.display());
            }

            if !dry_run {
                if let Some(parent) = dest_file.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Gracefully catch copy errors so it doesn't crash the whole backup
                if let Err(e) = fs::copy(entry.path(), &dest_file) {
                    if verbose {
                        println!(
                            "    Warning: Failed to copy {} ({}) - skipping.",
                            rel_path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
