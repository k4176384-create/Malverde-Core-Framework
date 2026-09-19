//! Configuration model and management

use crate::error::{ConfigError, ConfigResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Configuration value that can be any JSON type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Null,
    Bool(bool),
    Number(i64),
    Float(f64),
    String(String),
    Array(Vec<ConfigValue>),
    Object(HashMap<String, ConfigValue>),
}

impl ConfigValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConfigValue::Number(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(v) => Some(*v),
            ConfigValue::Number(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        self.as_str().map(|s| s.to_string())
    }

    pub fn as_array(&self) -> Option<&Vec<ConfigValue>> {
        match self {
            ConfigValue::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            ConfigValue::Object(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, ConfigValue::Null)
    }

    pub fn to_serde_value(&self) -> serde_json::Value {
        match self {
            ConfigValue::Null => serde_json::Value::Null,
            ConfigValue::Bool(v) => serde_json::Value::Bool(*v),
            ConfigValue::Number(v) => serde_json::Value::Number((*v).into()),
            ConfigValue::Float(v) => {
                if v.is_finite() {
                    serde_json::Value::Number(serde_json::Number::from_f64(*v).unwrap())
                } else {
                    serde_json::Value::Null
                }
            }
            ConfigValue::String(v) => serde_json::Value::String(v.clone()),
            ConfigValue::Array(v) => {
                serde_json::Value::Array(v.iter().map(|cv| cv.to_serde_value()).collect())
            }
            ConfigValue::Object(v) => {
                let mut map = serde_json::Map::new();
                for (k, v) in v {
                    map.insert(k.clone(), v.to_serde_value());
                }
                serde_json::Value::Object(map)
            }
        }
    }

    pub fn from_serde_value(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => ConfigValue::Null,
            serde_json::Value::Bool(v) => ConfigValue::Bool(v),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ConfigValue::Number(i)
                } else if let Some(f) = n.as_f64() {
                    ConfigValue::Float(f)
                } else {
                    ConfigValue::Number(0)
                }
            }
            serde_json::Value::String(s) => ConfigValue::String(s),
            serde_json::Value::Array(a) => {
                ConfigValue::Array(a.into_iter().map(ConfigValue::from_serde_value).collect())
            }
            serde_json::Value::Object(o) => {
                ConfigValue::Object(
                    o.into_iter()
                        .map(|(k, v)| (k, ConfigValue::from_serde_value(v)))
                        .collect(),
                )
            }
        }
    }
}

impl std::fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_serde_value())
    }
}

impl From<serde_json::Value> for ConfigValue {
    fn from(value: serde_json::Value) -> Self {
        ConfigValue::from_serde_value(value)
    }
}

impl From<ConfigValue> for serde_json::Value {
    fn from(value: ConfigValue) -> Self {
        value.to_serde_value()
    }
}

/// Configuration entry with metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: ConfigValue,
    pub description: Option<String>,
    pub source: Option<String>,
    pub is_default: bool,
    pub is_required: bool,
    pub is_secret: bool,
}

impl ConfigEntry {
    pub fn new(key: impl Into<String>, value: impl Into<ConfigValue>) -> Self {
        ConfigEntry {
            key: key.into(),
            value: value.into(),
            description: None,
            source: None,
            is_default: false,
            is_required: false,
            is_secret: false,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    pub fn with_required(mut self, is_required: bool) -> Self {
        self.is_required = is_required;
        self
    }

    pub fn with_secret(mut self, is_secret: bool) -> Self {
        self.is_secret = is_secret;
        self
    }
}

/// Configuration source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigSource {
    Default,
    File,
    Environment,
    CommandLine,
    Database,
    Remote,
}

impl std::fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Configuration schema for validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigSchema {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub entries: HashMap<String, SchemaEntry>,
}

impl ConfigSchema {
    pub fn new(name: impl Into<String>) -> Self {
        ConfigSchema {
            name: name.into(),
            description: None,
            version: "1.0.0".to_string(),
            entries: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn add_entry(mut self, key: impl Into<String>, entry: SchemaEntry) -> Self {
        self.entries.insert(key.into(), entry);
        self
    }
}

/// Schema entry for configuration validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaEntry {
    pub data_type: DataType,
    pub description: Option<String>,
    pub default: Option<ConfigValue>,
    pub required: bool,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub enum_values: Option<Vec<ConfigValue>>,
}

impl SchemaEntry {
    pub fn new(data_type: DataType) -> Self {
        SchemaEntry {
            data_type,
            description: None,
            default: None,
            required: false,
            min: None,
            max: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_default(mut self, default: impl Into<ConfigValue>) -> Self {
        self.default = Some(default.into());
        self
    }

    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn with_range(mut self, min: i64, max: i64) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub fn with_length_range(mut self, min: usize, max: usize) -> Self {
        self.min_length = Some(min);
        self.max_length = Some(max);
        self
    }

    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    pub fn with_enum_values(mut self, values: Vec<ConfigValue>) -> Self {
        self.enum_values = Some(values);
        self
    }

    /// Validate a value against this schema
    pub fn validate(&self, value: &ConfigValue) -> ConfigResult<()> {
        // Check required
        if self.required && value.is_null() {
            return Err(ConfigError::validation_failed(format!(
                "Required field is null"
            )));
        }

        // Check data type
        match self.data_type {
            DataType::Bool => {
                if value.as_bool().is_none() {
                    return Err(ConfigError::validation_failed("Expected boolean".to_string()));
                }
            }
            DataType::Integer => {
                if value.as_i64().is_none() {
                    return Err(ConfigError::validation_failed("Expected integer".to_string()));
                }
            }
            DataType::Float => {
                if value.as_f64().is_none() {
                    return Err(ConfigError::validation_failed("Expected float".to_string()));
                }
            }
            DataType::String => {
                if value.as_str().is_none() {
                    return Err(ConfigError::validation_failed("Expected string".to_string()));
                }
            }
            DataType::Array => {
                if value.as_array().is_none() {
                    return Err(ConfigError::validation_failed("Expected array".to_string()));
                }
            }
            DataType::Object => {
                if value.as_object().is_none() {
                    return Err(ConfigError::validation_failed("Expected object".to_string()));
                }
            }
            _ => {}
        }

        // Check min/max for numbers
        if let Some(min) = self.min {
            if let Some(v) = value.as_i64() {
                if v < min {
                    return Err(ConfigError::validation_failed(format!(
                        "Value {} is less than minimum {}", v, min
                    )));
                }
            } else if let Some(v) = value.as_f64() {
                if v < min as f64 {
                    return Err(ConfigError::validation_failed(format!(
                        "Value {} is less than minimum {}", v, min
                    )));
                }
            }
        }

        if let Some(max) = self.max {
            if let Some(v) = value.as_i64() {
                if v > max {
                    return Err(ConfigError::validation_failed(format!(
                        "Value {} is greater than maximum {}", v, max
                    )));
                }
            } else if let Some(v) = value.as_f64() {
                if v > max as f64 {
                    return Err(ConfigError::validation_failed(format!(
                        "Value {} is greater than maximum {}", v, max
                    )));
                }
            }
        }

        // Check string length
        if let Some(min_len) = self.min_length {
            if let Some(s) = value.as_str() {
                if s.len() < min_len {
                    return Err(ConfigError::validation_failed(format!(
                        "String length {} is less than minimum {}", s.len(), min_len
                    )));
                }
            }
        }

        if let Some(max_len) = self.max_length {
            if let Some(s) = value.as_str() {
                if s.len() > max_len {
                    return Err(ConfigError::validation_failed(format!(
                        "String length {} is greater than maximum {}", s.len(), max_len
                    )));
                }
            }
        }

        // Check enum values
        if let Some(ref enum_values) = self.enum_values {
            if !enum_values.contains(value) {
                return Err(ConfigError::validation_failed(format!(
                    "Value is not one of the allowed values"
                )));
            }
        }

        Ok(())
    }
}

/// Data type for configuration values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    Any,
    Bool,
    Integer,
    Float,
    String,
    Array,
    Object,
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Configuration store
#[derive(Debug, Clone)]
pub struct ConfigStore {
    data: Arc<RwLock<HashMap<String, ConfigEntry>>>,
    schema: Option<ConfigSchema>,
    source: ConfigSource,
    path: Option<PathBuf>,
}

impl ConfigStore {
    pub fn new() -> Self {
        ConfigStore {
            data: Arc::new(RwLock::new(HashMap::new())),
            schema: None,
            source: ConfigSource::Default,
            path: None,
        }
    }

    pub fn with_source(mut self, source: ConfigSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_schema(mut self, schema: ConfigSchema) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Get a configuration value
    pub fn get(&self, key: &str) -> Option<ConfigValue> {
        let data = self.data.read().unwrap();
        data.get(key).map(|e| e.value.clone())
    }

    /// Get a configuration entry
    pub fn get_entry(&self, key: &str) -> Option<ConfigEntry> {
        let data = self.data.read().unwrap();
        data.get(key).cloned()
    }

    /// Set a configuration value
    pub fn set(&self, key: impl Into<String>, value: impl Into<ConfigValue>) -> ConfigResult<()> {
        let mut data = self.data.write().unwrap();
        let key = key.into();
        
        // Validate if schema exists
        if let Some(ref schema) = self.schema {
            if let Some(entry) = schema.entries.get(&key) {
                let value = value.into();
                entry.validate(&value)?;
                data.insert(key, ConfigEntry::new(key, value));
            } else {
                data.insert(key, ConfigEntry::new(key, value.into()));
            }
        } else {
            data.insert(key, ConfigEntry::new(key, value.into()));
        }
        
        Ok(())
    }

    /// Remove a configuration value
    pub fn remove(&self, key: &str) -> Option<ConfigValue> {
        let mut data = self.data.write().unwrap();
        data.remove(key).map(|e| e.value)
    }

    /// Check if a key exists
    pub fn contains(&self, key: &str) -> bool {
        let data = self.data.read().unwrap();
        data.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        let data = self.data.read().unwrap();
        data.keys().cloned().collect()
    }

    /// Get all entries
    pub fn all(&self) -> HashMap<String, ConfigValue> {
        let data = self.data.read().unwrap();
        data.iter().map(|(k, v)| (k.clone(), v.value.clone())).collect()
    }

    /// Clear all configuration
    pub fn clear(&self) {
        let mut data = self.data.write().unwrap();
        data.clear();
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        let data = self.data.read().unwrap();
        data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get nested configuration
    pub fn get_nested(&self, path: &str) -> Option<ConfigValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current: Option<ConfigValue> = self.get(parts[0]);
        
        for part in &parts[1..] {
            if let Some(obj) = current.and_then(|v| v.as_object()) {
                current = obj.get(*part).cloned();
            } else {
                return None;
            }
        }
        
        current
    }

    /// Set nested configuration
    pub fn set_nested(&self, path: &str, value: impl Into<ConfigValue>) -> ConfigResult<()> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut data = self.data.write().unwrap();
        
        if parts.len() == 1 {
            return self.set(parts[0], value);
        }

        let key = parts[0].to_string();
        let last_part = parts[parts.len() - 1];
        
        // Get or create the parent object
        let parent_entry = data.entry(key.clone())
            .or_insert_with(|| ConfigEntry::new(key.clone(), ConfigValue::Object(HashMap::new())));
        
        if let ConfigValue::Object(ref mut parent_obj) = parent_entry.value {
            // Navigate through the path
            let mut current_obj = parent_obj;
            
            for (i, part) in parts[1..parts.len() - 1].iter().enumerate() {
                let next_key = part.to_string();
                let entry = current_obj.entry(next_key.clone())
                    .or_insert_with(|| ConfigValue::Object(HashMap::new()));
                
                if let ConfigValue::Object(ref mut next_obj) = entry {
                    current_obj = next_obj;
                } else {
                    // Path conflict - existing value is not an object
                    return Err(ConfigError::validation_failed(format!(
                        "Cannot set nested value at {}, path conflict", path
                    )));
                }
            }
            
            // Set the final value
            current_obj.insert(last_part.to_string(), value.into());
        }
        
        Ok(())
    }

    /// Merge another configuration into this one
    pub fn merge(&self, other: &ConfigStore) -> ConfigResult<()> {
        let other_data = other.all();
        
        for (key, value) in other_data {
            // If the value is an object, merge recursively
            if let (Some(ConfigValue::Object(existing)), ConfigValue::Object(new)) = 
                (self.get(&key), value) {
                    let mut merged = existing.clone();
                    for (k, v) in new {
                        merged.insert(k, v);
                    }
                    self.set(key, ConfigValue::Object(merged))?;
                } else {
                    // Overwrite
                    self.set(key, value)?;
                }
        }
        
        Ok(())
    }

    /// Validate the entire configuration
    pub fn validate(&self) -> ConfigResult<Vec<String>> {
        let mut errors = Vec::new();
        
        if let Some(ref schema) = self.schema {
            let data = self.data.read().unwrap();
            
            for (key, entry) in &schema.entries {
                if entry.required {
                    if !data.contains_key(key) {
                        errors.push(format!("Missing required configuration: {}", key));
                    } else if let Some(config_entry) = data.get(key) {
                        if config_entry.value.is_null() {
                            errors.push(format!("Required configuration is null: {}", key));
                        }
                    }
                }
                
                if let Some(config_entry) = data.get(key) {
                    if let Err(e) = entry.validate(&config_entry.value) {
                        errors.push(format!("Validation failed for {}: {}", key, e));
                    }
                }
            }
        }
        
        Ok(errors)
    }
}

impl Default for ConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Hierarchical configuration with multiple sources
#[derive(Debug, Clone)]
pub struct ConfigManager {
    sources: Vec<ConfigStore>,
}

impl ConfigManager {
    pub fn new() -> Self {
        ConfigManager {
            sources: Vec::new(),
        }
    }

    /// Add a configuration source (lower index = higher priority)
    pub fn add_source(&mut self, store: ConfigStore) {
        self.sources.push(store);
    }

    /// Get a configuration value (checks sources in order)
    pub fn get(&self, key: &str) -> Option<ConfigValue> {
        for source in &self.sources {
            if let Some(value) = source.get(key) {
                return Some(value);
            }
        }
        None
    }

    /// Get a configuration value with source information
    pub fn get_with_source(&self, key: &str) -> Option<(ConfigValue, ConfigSource)> {
        for source in &self.sources {
            if let Some(value) = source.get(key) {
                return Some((value, source.source));
            }
        }
        None
    }

    /// Set a configuration value in the first writable source
    pub fn set(&self, key: impl Into<String>, value: impl Into<ConfigValue>) -> ConfigResult<()> {
        for source in &self.sources {
            // Only allow setting in file or database sources
            match source.source {
                ConfigSource::File | ConfigSource::Database => {
                    return source.set(key, value);
                }
                _ => {}
            }
        }
        
        // If no writable source, create a new default one
        let mut store = ConfigStore::new().with_source(ConfigSource::Default);
        store.set(key, value)?;
        self.sources.push(store);
        
        Ok(())
    }

    /// Reload all sources
    pub fn reload(&mut self) -> ConfigResult<()> {
        // In a real implementation, this would reload from files/databases
        Ok(())
    }

    /// Get all keys from all sources
    pub fn all_keys(&self) -> HashSet<String> {
        let mut keys = HashSet::new();
        for source in &self.sources {
            for key in source.keys() {
                keys.insert(key);
            }
        }
        keys
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_value_types() {
        let bool_val = ConfigValue::Bool(true);
        assert_eq!(bool_val.as_bool(), Some(true));
        
        let num_val = ConfigValue::Number(42);
        assert_eq!(num_val.as_i64(), Some(42));
        
        let str_val = ConfigValue::String("test".to_string());
        assert_eq!(str_val.as_str(), Some("test"));
    }

    #[test]
    fn test_config_store() {
        let store = ConfigStore::new();
        
        store.set("test.bool", true).unwrap();
        store.set("test.number", 42).unwrap();
        store.set("test.string", "hello").unwrap();
        
        assert_eq!(store.get("test.bool"), Some(ConfigValue::Bool(true)));
        assert_eq!(store.get("test.number"), Some(ConfigValue::Number(42)));
        assert_eq!(store.get("test.string"), Some(ConfigValue::String("hello".to_string())));
        
        assert!(store.contains("test.bool"));
        assert!(!store.contains("nonexistent"));
    }

    #[test]
    fn test_nested_config() {
        let store = ConfigStore::new();
        
        store.set_nested("db.host", "localhost").unwrap();
        store.set_nested("db.port", 5432).unwrap();
        store.set_nested("db.credentials.user", "admin").unwrap();
        
        assert_eq!(
            store.get_nested("db.host"),
            Some(ConfigValue::String("localhost".to_string()))
        );
        assert_eq!(
            store.get_nested("db.port"),
            Some(ConfigValue::Number(5432))
        );
        assert_eq!(
            store.get_nested("db.credentials.user"),
            Some(ConfigValue::String("admin".to_string()))
        );
    }

    #[test]
    fn test_config_schema() {
        let mut schema = ConfigSchema::new("test");
        
        schema = schema.add_entry("port", SchemaEntry::new(DataType::Integer)
            .with_description("Server port")
            .with_default(ConfigValue::Number(8080))
            .with_range(1, 65535));
        
        schema = schema.add_entry("host", SchemaEntry::new(DataType::String)
            .with_description("Server host")
            .with_default(ConfigValue::String("localhost".to_string()))
            .with_required(true));
        
        assert_eq!(schema.entries.len(), 2);
    }

    #[test]
    fn test_schema_validation() {
        let schema = ConfigSchema::new("test")
            .add_entry("port", SchemaEntry::new(DataType::Integer)
                .with_range(1, 65535));
        
        let store = ConfigStore::new().with_schema(schema);
        
        // Valid value
        store.set("port", 8080).unwrap();
        
        // Invalid value - out of range
        let result = store.set("port", 70000);
        assert!(result.is_err());
        
        // Invalid type
        let result = store.set("port", "not a number");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_manager() {
        let mut manager = ConfigManager::new();
        
        let mut store1 = ConfigStore::new();
        store1.set("key1", "value1").unwrap();
        
        let mut store2 = ConfigStore::new();
        store2.set("key2", "value2").unwrap();
        store2.set("key1", "overridden").unwrap();
        
        manager.add_source(store1);
        manager.add_source(store2);
        
        // Should get from the second source (higher priority)
        assert_eq!(
            manager.get("key1"),
            Some(ConfigValue::String("overridden".to_string()))
        );
        assert_eq!(
            manager.get("key2"),
            Some(ConfigValue::String("value2".to_string()))
        );
    }
}
