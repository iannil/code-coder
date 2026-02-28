//! Modular configuration loader for Zero services.
//!
//! Supports loading configuration from multiple files:
//! - `config.json` - Core configuration
//! - `secrets.json` - Credentials (API keys, tokens)
//! - `trading.json` - Trading module configuration
//! - `channels.json` - IM channels configuration
//! - `providers.json` - LLM provider configuration
//!
//! Files are loaded from `~/.codecoder/` with proper merging.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

use crate::config::config_dir;

/// Configuration file names
pub const CONFIG_FILES: &[&str] = &[
    "config.json",
    "secrets.json",
    "trading.json",
    "channels.json",
    "providers.json",
];

/// Load a JSON file and return its contents as a Value.
/// Returns None if file doesn't exist.
fn load_json_file(path: &PathBuf) -> Result<Option<Value>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let value: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    Ok(Some(value))
}

/// Deep merge two JSON values.
/// Source values override target values, with object merging at each level.
fn merge_json(target: &mut Value, source: Value) {
    match (target, source) {
        (Value::Object(target_map), Value::Object(source_map)) => {
            for (key, source_value) in source_map {
                match target_map.get_mut(&key) {
                    Some(target_value) => {
                        merge_json(target_value, source_value);
                    }
                    None => {
                        target_map.insert(key, source_value);
                    }
                }
            }
        }
        (target, source) => {
            *target = source;
        }
    }
}

/// Load modular configuration from the config directory.
///
/// Priority (lowest to highest):
/// 1. Default config.json
/// 2. Modular files (secrets.json, trading.json, etc.)
/// 3. Environment variables (applied separately)
pub fn load_modular_config(dir: Option<PathBuf>) -> Result<Value> {
    let cfg_dir = dir.unwrap_or_else(config_dir);

    // Start with base config
    let config_path = cfg_dir.join("config.json");
    let mut config = load_json_file(&config_path)?.unwrap_or(Value::Object(Default::default()));

    tracing::debug!("Loading modular config from {}", cfg_dir.display());

    // Load and merge secrets
    // secrets.json has mixed structure:
    // - auth, tunnel, workflow go to top-level config fields
    // - llm, channels, external go to config.secrets
    if let Some(secrets) = load_json_file(&cfg_dir.join("secrets.json"))? {
        if let Some(secrets_obj) = secrets.as_object() {
            if let Some(config_obj) = config.as_object_mut() {
                // Fields that go to config.secrets
                let secret_fields = ["llm", "channels", "external"];
                let mut secrets_value = serde_json::Map::new();

                for (key, value) in secrets_obj.iter() {
                    // Filter out meta-fields
                    if key.starts_with('$') {
                        continue;
                    }

                    match key.as_str() {
                        // Fields that merge into top-level config
                        "auth" => {
                            if let Some(auth_obj) = config_obj.get_mut("auth") {
                                merge_json(auth_obj, value.clone());
                            } else {
                                config_obj.insert(key.clone(), value.clone());
                            }
                        }
                        "workflow" => {
                            if let Some(workflow_obj) = config_obj.get_mut("workflow") {
                                merge_json(workflow_obj, value.clone());
                            } else {
                                config_obj.insert(key.clone(), value.clone());
                            }
                        }
                        "tunnel" => {
                            // Transform tunnel secrets: cloudflare_token -> cloudflare.token
                            if let Some(tunnel_obj) = value.as_object() {
                                if let Some(config_tunnel) = config_obj.get_mut("tunnel") {
                                    if let Some(tunnel_map) = config_tunnel.as_object_mut() {
                                        // cloudflare_token -> cloudflare.token
                                        if let Some(token) = tunnel_obj.get("cloudflare_token").and_then(|v| v.as_str()) {
                                            if !token.is_empty() {
                                                tunnel_map.entry("cloudflare")
                                                    .or_insert_with(|| Value::Object(Default::default()))
                                                    .as_object_mut()
                                                    .map(|cf| cf.insert("token".to_string(), Value::String(token.to_string())));
                                            }
                                        }
                                        // ngrok_auth -> ngrok.token
                                        if let Some(token) = tunnel_obj.get("ngrok_auth").and_then(|v| v.as_str()) {
                                            if !token.is_empty() {
                                                tunnel_map.entry("ngrok")
                                                    .or_insert_with(|| Value::Object(Default::default()))
                                                    .as_object_mut()
                                                    .map(|ng| ng.insert("token".to_string(), Value::String(token.to_string())));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Fields that go to config.secrets
                        "llm" | "channels" | "external" => {
                            secrets_value.insert(key.clone(), value.clone());
                        }
                        // Unknown fields - put in secrets for compatibility
                        _ => {
                            secrets_value.insert(key.clone(), value.clone());
                        }
                    }
                }

                // Update config.secrets with the filtered values
                if !secrets_value.is_empty() {
                    match config_obj.get_mut("secrets") {
                        Some(existing) => merge_json(existing, Value::Object(secrets_value)),
                        None => {
                            config_obj.insert("secrets".to_string(), Value::Object(secrets_value));
                        }
                    }
                }
            }
        }
        tracing::debug!("Loaded secrets.json");
    }

    // Load and merge trading config
    if let Some(trading) = load_json_file(&cfg_dir.join("trading.json"))? {
        if let Some(config_obj) = config.as_object_mut() {
            match config_obj.get_mut("trading") {
                Some(existing) => merge_json(existing, trading),
                None => {
                    config_obj.insert("trading".to_string(), trading);
                }
            }
        }
        tracing::debug!("Loaded trading.json");
    }

    // Load and merge channels config into top-level channels field
    if let Some(channels) = load_json_file(&cfg_dir.join("channels.json"))? {
        if let Some(config_obj) = config.as_object_mut() {
            // Filter out meta-fields ($schema, etc.)
            let filtered_channels = if let Some(obj) = channels.as_object() {
                let filtered: serde_json::Map<String, Value> = obj
                    .iter()
                    .filter(|(key, _)| !key.starts_with('$'))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Value::Object(filtered)
            } else {
                channels
            };

            match config_obj.get_mut("channels") {
                Some(existing) => merge_json(existing, filtered_channels),
                None => {
                    config_obj.insert("channels".to_string(), filtered_channels);
                }
            }
        }
        tracing::debug!("Loaded channels.json");
    }

    // Load and merge providers config
    if let Some(providers) = load_json_file(&cfg_dir.join("providers.json"))? {
        // Filter out meta-fields ($schema, _settings, etc.) that aren't actual provider configs
        let filtered_providers = if let Some(obj) = providers.as_object() {
            let filtered: serde_json::Map<String, Value> = obj
                .iter()
                .filter(|(key, _)| !key.starts_with('$') && !key.starts_with('_'))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Value::Object(filtered)
        } else {
            providers
        };

        if let Some(config_obj) = config.as_object_mut() {
            match config_obj.get_mut("provider") {
                Some(existing) => merge_json(existing, filtered_providers),
                None => {
                    config_obj.insert("provider".to_string(), filtered_providers);
                }
            }
        }
        tracing::debug!("Loaded providers.json");
    }

    Ok(config)
}

/// Check which modular config files exist.
pub fn check_modular_files(dir: Option<PathBuf>) -> Vec<(String, bool)> {
    let cfg_dir = dir.unwrap_or_else(config_dir);

    CONFIG_FILES
        .iter()
        .map(|file| {
            let path = cfg_dir.join(file);
            (file.to_string(), path.exists())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_merge_json_objects() {
        let mut target = json!({
            "a": 1,
            "b": {
                "x": 10,
                "y": 20
            }
        });

        let source = json!({
            "b": {
                "y": 25,
                "z": 30
            },
            "c": 3
        });

        merge_json(&mut target, source);

        assert_eq!(target["a"], 1);
        assert_eq!(target["b"]["x"], 10);
        assert_eq!(target["b"]["y"], 25);
        assert_eq!(target["b"]["z"], 30);
        assert_eq!(target["c"], 3);
    }

    #[test]
    fn test_merge_json_overwrite_non_object() {
        let mut target = json!({ "a": [1, 2, 3] });
        let source = json!({ "a": [4, 5] });

        merge_json(&mut target, source);

        assert_eq!(target["a"], json!([4, 5]));
    }
}
