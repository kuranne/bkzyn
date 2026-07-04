use std::fs;
#[cfg(not(unix))]
use std::path::Path;
use std::path::PathBuf;

pub fn run(dry_run: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let repo_dir = std::env::current_dir()?;
    let config_dir = repo_dir.join("config");

    let xdg_config_home = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".config"));

    if !config_dir.exists() {
        return Err("No config directory found in the repository.".into());
    }

    if verbose {
        println!("--> Restoring configs to {}", xdg_config_home.display());
    }

    if !dry_run {
        fs::create_dir_all(&xdg_config_home)?;
    }

    for entry in fs::read_dir(config_dir)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = xdg_config_home.join(entry.file_name());

        if verbose {
            println!(
                "    Restoring {} -> {}",
                src_path.display(),
                dest_path.display()
            );
        }

        if !dry_run {
            if dest_path.exists() {
                // If it's a directory and not a symlink, back it up
                let meta = fs::symlink_metadata(&dest_path)?;
                if meta.is_dir() {
                    let backup_path = PathBuf::from(format!("{}.bak", dest_path.display()));
                    if verbose {
                        println!(
                            "        [Backup] {} -> {}",
                            dest_path.display(),
                            backup_path.display()
                        );
                    }
                    fs::rename(&dest_path, &backup_path)?;
                } else {
                    // For files or symlinks, just remove them to replace with symlink
                    fs::remove_file(&dest_path).unwrap_or_else(|_| {
                        if meta.is_dir() {
                            fs::remove_dir_all(&dest_path).unwrap()
                        }
                    });
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
