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

    #[test]
    fn test_get_backup_toml_path_resolution() {
        let dir = tempdir().unwrap();
        let base = dir.path();

        let repo_dir = base.join("repo");
        let xdg_config_dir = base.join("xdg_config");

        fs::create_dir_all(&repo_dir).unwrap();
        fs::create_dir_all(&xdg_config_dir).unwrap();

        let paths = AppPaths {
            repo: repo_dir.clone(),
            xdg_config: xdg_config_dir.clone(),
            config: base.join("config"),
            data: base.join("data"),
            old: base.join("old"),
            xdg_data: base.join("xdg_data"),
        };

        // 1. None of them exist
        // (Note: dirs::home_dir() might exist, but ~/.config/bkzyn/backup.toml likely doesn't in CI)
        if paths.get_backup_toml_path().is_some() {
            // If the user actually has ~/.config/bkzyn/backup.toml on their real system,
            // we skip the `None` assertion to avoid flaky local tests.
        } else {
            assert_eq!(paths.get_backup_toml_path(), None);
        }

        // 2. Only XDG exists
        let xdg_bkzyn = xdg_config_dir.join("bkzyn");
        fs::create_dir_all(&xdg_bkzyn).unwrap();
        let xdg_toml = xdg_bkzyn.join("backup.toml");
        fs::write(&xdg_toml, "").unwrap();

        assert_eq!(paths.get_backup_toml_path(), Some(xdg_toml.clone()));

        // 3. Both Repo and XDG exist
        #[cfg(debug_assertions)]
        {
            let repo_toml = repo_dir.join("backup.toml");
            fs::write(&repo_toml, "").unwrap();

            // In debug mode, repo takes precedence
            assert_eq!(paths.get_backup_toml_path(), Some(repo_toml));
        }

        #[cfg(not(debug_assertions))]
        {
            let repo_toml = repo_dir.join("backup.toml");
            fs::write(&repo_toml, "").unwrap();

            // In release mode, repo is ignored, XDG still takes precedence
            assert_eq!(paths.get_backup_toml_path(), Some(xdg_toml));
        }
    }
}
