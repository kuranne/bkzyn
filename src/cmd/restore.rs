use std::fs;
#[cfg(not(unix))]
use std::path::Path;
use std::path::PathBuf;

/// Restores configuration symlinks from repository config directory to the system.
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

    // 1. Restore configuration symlinks

    for entry in fs::read_dir(&paths.config)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = paths.xdg_config.join(entry.file_name());
        let app_name = entry.file_name().to_string_lossy().into_owned();

        ui.status(
            "LINK",
            &app_name,
            &format!("{} -> {}", src_path.display(), dest_path.display()),
        );

        if !dry_run {
            if dest_path.exists() {
                // If it's a directory and not a symlink, back it up
                let meta = fs::symlink_metadata(&dest_path)?;
                if meta.is_dir() {
                    let backup_path = PathBuf::from(format!("{}.bak", dest_path.display()));
                    ui.status(
                        "BACKUP",
                        &app_name,
                        &format!("{} -> {}", dest_path.display(), backup_path.display()),
                    );
                    fs::rename(&dest_path, &backup_path)?;
                } else {
                    // For files or symlinks, just remove them to replace with symlink
                    if let Err(e) = fs::remove_file(&dest_path) {
                        ui.warn(
                            "Restore",
                            &format!("Failed to remove {}: {}", dest_path.display(), e),
                        );
                        continue;
                    }
                }
            }

            // Using symlink for everything as per standard dotfile manager practice
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&src_path, &dest_path)?;
            }
            #[cfg(not(unix))]
            {
                // Fallback to copy if not on unix
                if src_path.is_dir() {
                    copy_dir_all(&src_path, &dest_path)?;
                } else {
                    fs::copy(&src_path, &dest_path)?;
                }
            }
        }
    }

    ui.done("Successful restore");
    Ok(())
}

#[cfg(not(unix))]
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> std::io::Result<()> {
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
