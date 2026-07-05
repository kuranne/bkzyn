//! Core functions and path management for the bkzyn backup tool.

pub mod cmd;
pub mod config;
pub mod cli;

/// Paths configuration for the application.
pub struct AppPaths {
    /// Directory where the repository is stored.
    pub repo: std::path::PathBuf,
    /// Directory where the configuration files are backed up or stored.
    pub config: std::path::PathBuf,
    /// Directory where older backups are archived.
    pub old: std::path::PathBuf,
    /// The user's local configuration directory on the system.
    pub xdg_config: std::path::PathBuf,
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
                config: repo.join("config"),
                old: repo.join(".old"),
                repo,
                xdg_config: std::env::var("XDG_CONFIG_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| home.join(".config")),
            })
        } else {
            let repo = std::env::var("XDG_DATA_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| home.join(".local/share"))
                .join("backup");
            Ok(Self {
                config: std::env::var("XDG_CONFIG_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| home.join(".config"))
                    .join("backup"),
                old: repo.join(".old"),
                repo,
                xdg_config: std::env::var("XDG_CONFIG_HOME")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| home.join(".config")),
            })
        }
    }
}
