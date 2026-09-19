//! Configuration file loading and saving

use crate::config::{ConfigStore, ConfigValue, ConfigSource};
use crate::error::{ConfigError, ConfigResult};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Configuration file format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigFormat {
    Json,
    Toml,
    Yaml,
    Ron,
}

impl std::fmt::Display for ConfigFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigFormat::Json => write!(f, "json"),
            ConfigFormat::Toml => write!(f, "toml"),
            ConfigFormat::Yaml => write!(f, "yaml"),
            ConfigFormat::Ron => write!(f, "ron"),
        }
    }
}

impl ConfigFormat {
    pub fn from_path(path: &Path) -> Self {
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());
        
        match ext.as_deref() {
            Some("json") => ConfigFormat::Json,
            Some("toml") => ConfigFormat::Toml,
            Some("yaml") | Some("yml") => ConfigFormat::Yaml,
            Some("ron") => ConfigFormat::Ron,
            _ => ConfigFormat::Json,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => ConfigFormat::Json,
            "toml" => ConfigFormat::Toml,
            "yaml" | "yml" => ConfigFormat::Yaml,
            "ron" => ConfigFormat::Ron,
            _ => ConfigFormat::Json,
        }
    }
}

impl Default for ConfigFormat {
    fn default() -> Self {
        ConfigFormat::Json
    }
}

/// Configuration file loader
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from a file
    pub fn load_from_file(path: impl AsRef<Path>) -> ConfigResult<ConfigStore> {
        let path = path.as_ref();
        let format = ConfigFormat::from_path(path);
        
        if !path.exists() {
            return Err(ConfigError::file_not_found(path.display().to_string()));
        }
        
        let content = fs::read_to_string(path)?;
        let store = Self::load_from_str(&content, format)?;
        
        Ok(store.with_source(ConfigSource::File).with_path(path.to_path_buf()))
    }

    /// Load configuration from a string
    pub fn load_from_str(content: &str, format: ConfigFormat) -> ConfigResult<ConfigStore> {
        let value: serde_json::Value = match format {
            ConfigFormat::Json => serde_json::from_str(content)?,
            ConfigFormat::Toml => {
                // For TOML, we need the toml crate, but we'll use a simple approach
                // In a real implementation, we'd use the toml crate
                serde_json::from_str(content)?
            }
            ConfigFormat::Yaml => {
                // For YAML, we need the serde_yaml crate
                serde_json::from_str(content)?
            }
            ConfigFormat::Ron => {
                // For RON, we need the ron crate
                serde_json::from_str(content)?
            }
        };
        
        let mut store = ConfigStore::new();
        Self::load_value_into_store(&value, &mut store)?;
        
        Ok(store)
    }

    /// Save configuration to a file
    pub fn save_to_file(store: &ConfigStore, path: impl AsRef<Path>) -> ConfigResult<()> {
        let path = path.as_ref();
        let format = ConfigFormat::from_path(path);
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = Self::store_to_str(store, format)?;
        fs::write(path, content)?;
        
        Ok(())
    }

    /// Save configuration to a string
    pub fn store_to_str(store: &ConfigStore, format: ConfigFormat) -> ConfigResult<String> {
        let value = Self::store_to_value(store);
        
        match format {
            ConfigFormat::Json => {
                serde_json::to_string_pretty(&value)
                    .map_err(|e| ConfigError::invalid_format(e.to_string()))
            }
            _ => {
                // For other formats, we'd use the appropriate serializer
                serde_json::to_string_pretty(&value)
                    .map_err(|e| ConfigError::invalid_format(e.to_string()))
            }
        }
    }

    /// Load a serde_json::Value into a ConfigStore
    fn load_value_into_store(value: &serde_json::Value, store: &mut ConfigStore) -> ConfigResult<()> {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    if let serde_json::Value::Object(_) = val {
                        // Nested object - create a nested store entry
                        let nested_value = ConfigValue::from_serde_value(val.clone());
                        store.set(key, nested_value)?;
                    } else {
                        let config_value = ConfigValue::from_serde_value(val.clone());
                        store.set(key, config_value)?;
                    }
                }
            }
            _ => {
                return Err(ConfigError::invalid_format("Expected JSON object at root".to_string()));
            }
        }
        
        Ok(())
    }

    /// Convert a ConfigStore to a serde_json::Value
    fn store_to_value(store: &ConfigStore) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        
        for (key, entry) in store.all() {
            map.insert(key, entry.into());
        }
        
        serde_json::Value::Object(map)
    }

    /// Load environment variables into a ConfigStore
    pub fn load_from_env(prefix: Option<&str>) -> ConfigResult<ConfigStore> {
        let mut store = ConfigStore::new().with_source(ConfigSource::Environment);
        
        for (key, value) in std::env::vars() {
            // Filter by prefix if specified
            if let Some(prefix) = prefix {
                if !key.starts_with(prefix) {
                    continue;
                }
                // Remove prefix from key
                let config_key = key.strip_prefix(prefix)
                    .and_then(|s| if s.starts_with('_') { s.strip_prefix('_') } else { Some(s) })
                    .unwrap_or(&key)
                    .to_string();
                
                // Convert underscore to dot for nested keys
                let config_key = config_key.replace('_', ".");
                store.set_nested(&config_key, ConfigValue::String(value))?;
            } else {
                store.set(key, ConfigValue::String(value))?;
            }
        }
        
        Ok(store)
    }

    /// Load command-line arguments into a ConfigStore
    pub fn load_from_args(args: Vec<String>) -> ConfigResult<ConfigStore> {
        let mut store = ConfigStore::new().with_source(ConfigSource::CommandLine);
        
        // Simple argument parsing (in a real implementation, use clap or similar)
        let mut iter = args.into_iter().skip(1); // Skip program name
        
        while let Some(arg) = iter.next() {
            if arg.starts_with("--") {
                let key = arg.trim_start_matches("--").to_string();
                let value = iter.next().unwrap_or_default();
                store.set(key, ConfigValue::String(value))?;
            }
        }
        
        Ok(store)
    }

    /// Load default configuration
    pub fn load_defaults(defaults: serde_json::Value) -> ConfigResult<ConfigStore> {
        let mut store = ConfigStore::new().with_source(ConfigSource::Default);
        Self::load_value_into_store(&defaults, &mut store)?;
        Ok(store)
    }
}

/// Configuration file watcher (for hot reload)
pub struct ConfigWatcher {
    path: PathBuf,
    format: ConfigFormat,
    last_modified: Option<std::time::SystemTime>,
}

impl ConfigWatcher {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        ConfigWatcher {
            path: path.clone(),
            format: ConfigFormat::from_path(&path),
            last_modified: None,
        }
    }

    /// Check if the file has been modified
    pub fn is_modified(&mut self) -> ConfigResult<bool> {
        let metadata = fs::metadata(&self.path)?;
        let modified = metadata.modified()?;
        
        if let Some(last) = self.last_modified {
            if modified > last {
                self.last_modified = Some(modified);
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            self.last_modified = Some(modified);
            Ok(false)
        }
    }

    /// Reload the configuration
    pub fn reload(&self) -> ConfigResult<ConfigStore> {
        ConfigLoader::load_from_file(&self.path)
    }
}

/// Configuration builder for creating configuration from multiple sources
pub struct ConfigBuilder {
    stores: Vec<ConfigStore>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            stores: Vec::new(),
        }
    }

    /// Add a file source
    pub fn with_file(mut self, path: impl AsRef<Path>) -> ConfigResult<Self> {
        let store = ConfigLoader::load_from_file(path)?;
        self.stores.push(store);
        Ok(self)
    }

    /// Add environment variables
    pub fn with_env(mut self, prefix: Option<&str>) -> ConfigResult<Self> {
        let store = ConfigLoader::load_from_env(prefix)?;
        self.stores.push(store);
        Ok(self)
    }

    /// Add command-line arguments
    pub fn with_args(mut self, args: Vec<String>) -> ConfigResult<Self> {
        let store = ConfigLoader::load_from_args(args)?;
        self.stores.push(store);
        Ok(self)
    }

    /// Add default configuration
    pub fn with_defaults(mut self, defaults: serde_json::Value) -> ConfigResult<Self> {
        let store = ConfigLoader::load_defaults(defaults)?;
        self.stores.push(store);
        Ok(self)
    }

    /// Build the configuration manager
    pub fn build(self) -> ConfigManager {
        let mut manager = ConfigManager::new();
        for store in self.stores {
            manager.add_source(store);
        }
        manager
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// ConfigManager import
use super::config::ConfigManager;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_from_str() {
        let json = r#"{"key": "value", "number": 42, "bool": true}"#;
        let store = ConfigLoader::load_from_str(json, ConfigFormat::Json).unwrap();
        
        assert_eq!(
            store.get("key"),
            Some(ConfigValue::String("value".to_string()))
        );
        assert_eq!(
            store.get("number"),
            Some(ConfigValue::Number(42))
        );
        assert_eq!(
            store.get("bool"),
            Some(ConfigValue::Bool(true))
        );
    }

    #[test]
    fn test_save_to_file() {
        let mut store = ConfigStore::new();
        store.set("test", "value").unwrap();
        store.set("number", 42).unwrap();
        
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        ConfigLoader::save_to_file(&store, path).unwrap();
        
        // Read back and verify
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("test"));
        assert!(content.contains("value"));
    }

    #[test]
    fn test_load_from_env() {
        std::env::set_var("TEST_CONFIG_VAR", "test_value");
        
        let store = ConfigLoader::load_from_env(Some("TEST_")).unwrap();
        
        assert_eq!(
            store.get("config.var"),
            Some(ConfigValue::String("test_value".to_string()))
        );
        
        std::env::remove_var("TEST_CONFIG_VAR");
    }

    #[test]
    fn test_config_format() {
        assert_eq!(ConfigFormat::from_path("test.json"), ConfigFormat::Json);
        assert_eq!(ConfigFormat::from_path("test.toml"), ConfigFormat::Toml);
        assert_eq!(ConfigFormat::from_path("test.yaml"), ConfigFormat::Yaml);
        assert_eq!(ConfigFormat::from_path("test.ron"), ConfigFormat::Ron);
        assert_eq!(ConfigFormat::from_path("test"), ConfigFormat::Json);
    }

    #[test]
    fn test_config_builder() {
        let builder = ConfigBuilder::new()
            .with_defaults(serde_json::json!({"default": "value"}))
            .unwrap();
        
        let manager = builder.build();
        assert_eq!(
            manager.get("default"),
            Some(ConfigValue::String("value".to_string()))
        );
    }
}
