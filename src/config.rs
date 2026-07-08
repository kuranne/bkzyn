use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct BackupConfig {
    pub whitelist: Option<Vec<String>>,
    pub ignores: Option<Vec<String>>,
    pub whitelists: Option<HashMap<String, RuleMap>>,
    pub ignores_map: Option<HashMap<String, RuleMap>>,
    #[serde(flatten)]
    pub items: HashMap<String, CategoryOrApp>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CategoryOrApp {
    pub path: Option<String>,
    pub whitelist: Option<Vec<String>>,
    pub ignores: Option<Vec<String>>,
    #[serde(flatten)]
    pub apps: HashMap<String, ItemConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ItemConfig {
    pub whitelist: Option<Vec<String>>,
    pub ignores: Option<Vec<String>>,
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
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_env(content: &str) -> (TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut f = fs::File::create(&path).unwrap();
        write!(f, "{}", content).unwrap();
        (dir, path)
    }

    #[test]
    fn test_load_valid_config() {
        let toml_str = r#"
ignores = ['.git']

[config]
whitelist = ['zsh', 'git']
"#;
        let (_dir, path) = setup_test_env(toml_str);
        let cfg = BackupConfig::load(path).unwrap();

        assert_eq!(cfg.ignores.unwrap(), vec![".git"]);
        let whitelists = cfg.items["config"].whitelist.as_ref().unwrap();
        assert_eq!(whitelists, &vec!["zsh", "git"]);
    }

    #[test]
    fn test_load_missing_file() {
        let result = BackupConfig::load("does_not_exist.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_malformed_toml() {
        let (_dir, path) = setup_test_env("invalid [ toml {{{");
        let result = BackupConfig::load(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_categories_filters_correctly() {
        let toml_str = r#"
[config]
whitelist = []

[dataHome]
whitelist = []

[myapp]
whitelist = []

[custom]
path = "~/custom"
whitelist = []
"#;
        let (_dir, path) = setup_test_env(toml_str);
        let cfg = BackupConfig::load(path).unwrap();
        let cats = cfg.categories();
        assert!(cats.contains_key(&"config".to_string()));
        assert!(cats.contains_key(&"dataHome".to_string()));
        assert!(cats.contains_key(&"custom".to_string())); // Because it has a path
        assert!(!cats.contains_key(&"myapp".to_string())); // No path, ignored as category
    }

    #[test]
    fn test_global_apps_excludes_categories() {
        let toml_str = r#"
[config]
whitelist = []

[dataHome]
whitelist = []

[myapp]
whitelist = []
"#;
        let (_dir, path) = setup_test_env(toml_str);
        let cfg = BackupConfig::load(path).unwrap();
        let apps = cfg.global_apps();
        assert!(apps.contains_key(&"myapp".to_string()));
        assert!(!apps.contains_key(&"config".to_string()));
        assert!(!apps.contains_key(&"dataHome".to_string()));
    }

    #[test]
    fn test_rulemap_applist_vs_categorymap() {
        let toml_str = r#"
[whitelists]
zsh = [".z*", "*.zsh"]
"#;
        let (_dir, path) = setup_test_env(toml_str);
        let cfg = BackupConfig::load(path).unwrap();
        let whitelists = cfg.whitelists.as_ref().unwrap();

        if let RuleMap::AppList(list) = &whitelists["zsh"] {
            assert_eq!(list, &vec![".z*", "*.zsh"]);
        } else {
            panic!("Expected AppList");
        }

        let toml_str2 = r#"
[whitelists.config]
zsh = [".z*", "*.zsh"]
"#;
        let (_dir2, path2) = setup_test_env(toml_str2);
        let cfg2 = BackupConfig::load(path2).unwrap();
        let whitelists2 = cfg2.whitelists.as_ref().unwrap();

        if let RuleMap::CategoryMap(cmap) = &whitelists2["config"] {
            assert_eq!(cmap["zsh"], vec![".z*", "*.zsh"]);
        } else {
            panic!("Expected CategoryMap");
        }
    }
}
