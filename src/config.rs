use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct BackupConfig {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    #[serde(flatten)]
    pub items: HashMap<String, CategoryOrApp>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CategoryOrApp {
    pub path: Option<String>,
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    #[serde(flatten)]
    pub apps: HashMap<String, ItemConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ItemConfig {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
}

impl BackupConfig {
    /// Returns categories that are either built-in (config, dataHome) or have an explicit path.
    pub fn categories(&self) -> HashMap<&String, &CategoryOrApp> {
        self.items
            .iter()
            .filter(|(k, v)| k.as_str() == "config" || k.as_str() == "dataHome" || v.path.is_some())
            .collect()
    }

    /// Returns items that are neither built-in categories nor have an explicit path (treated as global apps).
    pub fn global_apps(&self) -> HashMap<&String, &CategoryOrApp> {
        self.items
            .iter()
            .filter(|(k, v)| k.as_str() != "config" && k.as_str() != "dataHome" && v.path.is_none())
            .collect()
    }

    /// Load the configuration from a file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: BackupConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
