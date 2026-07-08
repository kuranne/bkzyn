//! Core functions and path management for the bkzyn backup tool.

pub mod cli;
pub mod cmd;
pub mod config;

/// Paths configuration for the application.
pub struct AppPaths {
    /// Directory where the repository is stored.
    pub repo: std::path::PathBuf,
    /// Directory where the configuration files are backed up or stored.
    pub config: std::path::PathBuf,
    /// Directory where data files (.local/share) are backed up or stored.
    pub data: std::path::PathBuf,
    /// Directory where older backups are archived.
    pub old: std::path::PathBuf,
    /// The user's local configuration directory on the system.
    pub xdg_config: std::path::PathBuf,
    /// The user's local data directory on the system.
    pub xdg_data: std::path::PathBuf,
}

impl AppPaths {
    /// Creates a new `AppPaths` instance with resolved system paths.
    pub fn new() -> Result<Self, std::io::Error> {
        let home = dirs::home_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
        })?;

        if cfg!(debug_assertions) {
            let repo = std::env::current_dir()?;
            Ok(Self {
                config: repo.join("data").join("config"),
                data: repo.join("data").join("share"),
                old: repo.join(".old"),
                repo,
                xdg_config: std::env::var("XDG_CONFIG_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| home.join(".config")),
                xdg_data: std::env::var("XDG_DATA_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| home.join(".local/share")),
            })
        } else {
            let mut repo = std::env::var("XDG_DATA_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| home.join(".local/share"))
                .join("bkzyn");

            // Resolve the symlink so we execute Git and file modifications in the true source directory
            if let Ok(real_path) = std::fs::canonicalize(&repo) {
                repo = real_path;
            }

            Ok(Self {
                config: repo.join("data").join("config"),
                data: repo.join("data").join("share"),
                old: repo.join(".old"),
                repo,
                xdg_config: std::env::var("XDG_CONFIG_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| home.join(".config")),
                xdg_data: std::env::var("XDG_DATA_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| home.join(".local/share")),
            })
        }
    }

    /// Returns the resolved path to backup.toml based on the environment (Debug vs Release)
    pub fn get_backup_toml_path(&self) -> Option<std::path::PathBuf> {
        #[cfg(debug_assertions)]
        {
            let path = self.repo.join("backup.toml");
            if path.exists() {
                return Some(path);
            }
        }

        let path = self.xdg_config.join("bkzyn").join("backup.toml");
        if path.exists() {
            return Some(path);
        }

        if let Some(home) = dirs::home_dir() {
            let path = home.join(".config").join("bkzyn").join("backup.toml");
            if path.exists() {
                return Some(path);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    /// Builds an `AppPaths` fully isolated inside `base` so no real `~/.config` is reachable.
    fn isolated_paths(base: &std::path::Path) -> AppPaths {
        AppPaths {
            repo: base.join("repo"),
            xdg_config: base.join("xdg_config"),
            config: base.join("config"),
            data: base.join("data"),
            old: base.join("old"),
            xdg_data: base.join("xdg_data"),
        }
    }

    #[test]
    fn test_get_backup_toml_returns_none_when_missing() {
        let dir = tempdir().unwrap();
        let paths = isolated_paths(dir.path());
        // Neither repo/backup.toml nor xdg_config/bkzyn/backup.toml exists,
        // and paths.xdg_config points into the tempdir so ~/.config is never reached.
        assert_eq!(paths.get_backup_toml_path(), None);
    }

    #[test]
    fn test_get_backup_toml_returns_xdg_path() {
        let dir = tempdir().unwrap();
        let paths = isolated_paths(dir.path());
        let xdg_bkzyn = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&xdg_bkzyn).unwrap();
        let xdg_toml = xdg_bkzyn.join("backup.toml");
        fs::write(&xdg_toml, "").unwrap();
        assert_eq!(paths.get_backup_toml_path(), Some(xdg_toml));
    }

    #[test]
    #[cfg(debug_assertions)]
    fn test_get_backup_toml_repo_takes_precedence_in_debug() {
        let dir = tempdir().unwrap();
        let paths = isolated_paths(dir.path());

        // Create both xdg and repo copies.
        let xdg_bkzyn = paths.xdg_config.join("bkzyn");
        fs::create_dir_all(&xdg_bkzyn).unwrap();
        fs::write(xdg_bkzyn.join("backup.toml"), "").unwrap();

        fs::create_dir_all(&paths.repo).unwrap();
        let repo_toml = paths.repo.join("backup.toml");
        fs::write(&repo_toml, "").unwrap();

        // In debug mode the repo copy has priority.
        assert_eq!(paths.get_backup_toml_path(), Some(repo_toml));
    }

    #[test]
    fn test_app_paths_new_succeeds() {
        // Smoke test: new() must not panic and must return a valid struct.
        let result = AppPaths::new();
        assert!(result.is_ok());
    }
}
