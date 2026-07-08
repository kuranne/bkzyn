use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct BackupConfig {
    pub include: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    pub includes: Option<HashMap<String, RuleMap>>,
    pub excludes: Option<HashMap<String, RuleMap>>,
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

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum RuleMap {
    AppList(Vec<String>),
    CategoryMap(HashMap<String, Vec<String>>),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_toml(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();
        f
    }

    #[test]
    fn test_load_valid_config() {
        let f = write_toml(
            r#"
exclude = ['.git']

[config]
include = ['zsh', 'git']
"#,
        );
        let cfg = BackupConfig::load(f.path()).unwrap();
        assert_eq!(cfg.exclude.unwrap(), vec![".git"]);
        let includes = cfg.items["config"].include.as_ref().unwrap();
        assert!(includes.contains(&"zsh".to_string()));
    }

    #[test]
    fn test_load_missing_file() {
        let result = BackupConfig::load("/tmp/bkzyn_does_not_exist_xyz.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_malformed_toml() {
        let f = write_toml("invalid [ toml {{{");
        let result = BackupConfig::load(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_categories_filters_correctly() {
        let f = write_toml(
            r#"
[config]
include = []

[dataHome]
include = []

[myapp]
include = []

[custom]
path = "~/custom"
include = []
"#,
        );
        let cfg = BackupConfig::load(f.path()).unwrap();
        let cats = cfg.categories();
        assert!(cats.contains_key(&"config".to_string()));
        assert!(cats.contains_key(&"dataHome".to_string()));
        assert!(cats.contains_key(&"custom".to_string()));
        assert!(!cats.contains_key(&"myapp".to_string()));
    }

    #[test]
    fn test_global_apps_excludes_categories() {
        let f = write_toml(
            r#"
[config]
include = []

[dataHome]
include = []

[myapp]
include = []
"#,
        );
        let cfg = BackupConfig::load(f.path()).unwrap();
        let apps = cfg.global_apps();
        assert!(apps.contains_key(&"myapp".to_string()));
        assert!(!apps.contains_key(&"config".to_string()));
        assert!(!apps.contains_key(&"dataHome".to_string()));
    }

    #[test]
    fn test_rulemap_applist_vs_categorymap() {
        // AppList: simple array under top-level [includes]
        let f = write_toml(
            r#"
[includes]
zsh = [".z*", "*.zsh"]
"#,
        );
        let cfg = BackupConfig::load(f.path()).unwrap();
        let includes = cfg.includes.as_ref().unwrap();
        assert!(matches!(includes["zsh"], RuleMap::AppList(_)));

        // CategoryMap: key → array under [includes.config]
        let f2 = write_toml(
            r#"
[includes.config]
myapp = ["*.cfg"]
"#,
        );
        let cfg2 = BackupConfig::load(f2.path()).unwrap();
        let includes2 = cfg2.includes.as_ref().unwrap();
        assert!(matches!(includes2["config"], RuleMap::CategoryMap(_)));
    }
}
