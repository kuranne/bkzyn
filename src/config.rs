use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct BackupConfig {
    pub configs: Vec<String>,
    #[serde(flatten)]
    pub items: HashMap<String, ConfigItem>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ConfigItem {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

impl BackupConfig {
    /// Load the configuration from a file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: BackupConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
