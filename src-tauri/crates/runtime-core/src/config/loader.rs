// SPDX-License-Identifier: AGPL-3.0-only
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::json::JsonValue;

use super::runtime::{
    default_config_home, merge_mcp_servers, parse_optional_aliases, parse_optional_features,
    parse_optional_hooks_config, parse_optional_model, parse_optional_oauth_config,
    parse_optional_permission_mode, parse_optional_permission_rules, parse_optional_plugin_config,
    parse_optional_provider_fallbacks, parse_optional_sandbox_config, parse_optional_trusted_roots,
    read_optional_json_object, validate_optional_hooks_config,
};
use super::types::{
    CURRENT_SCHEMA_VERSION, ConfigEntry, ConfigError, ConfigSource, McpConfigCollection,
    RuntimeConfig, RuntimeFeatureConfig, apply_migrations, deep_merge_objects,
};

/// Discovers config files and merges them into a [`RuntimeConfig`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigLoader {
    cwd: PathBuf,
    config_home: PathBuf,
}

impl ConfigLoader {
    #[must_use]
    pub fn new(cwd: impl Into<PathBuf>, config_home: impl Into<PathBuf>) -> Self {
        Self {
            cwd: cwd.into(),
            config_home: config_home.into(),
        }
    }

    #[must_use]
    pub fn default_for(cwd: impl Into<PathBuf>) -> Self {
        let cwd = cwd.into();
        let config_home = default_config_home();
        Self { cwd, config_home }
    }

    #[must_use]
    pub fn config_home(&self) -> &Path {
        &self.config_home
    }

    #[must_use]
    pub fn discover(&self) -> Vec<ConfigEntry> {
        let user_legacy_path = self
            .config_home
            .parent()
            .map_or_else(|| PathBuf::from(".claw.json"), |parent| parent.join(".claw.json"));
        vec![
            ConfigEntry {
                source: ConfigSource::User,
                path: user_legacy_path,
            },
            ConfigEntry {
                source: ConfigSource::User,
                path: self.config_home.join("settings.json"),
            },
            ConfigEntry {
                source: ConfigSource::Project,
                path: self.cwd.join(".claw.json"),
            },
            ConfigEntry {
                source: ConfigSource::Project,
                path: self.cwd.join(".claw").join("settings.json"),
            },
            ConfigEntry {
                source: ConfigSource::Local,
                path: self.cwd.join(".claw").join("settings.local.json"),
            },
        ]
    }

    pub fn load(&self) -> Result<RuntimeConfig, ConfigError> {
        let mut merged = BTreeMap::new();
        let mut loaded_entries = Vec::new();
        let mut mcp_servers = BTreeMap::new();
        let mut all_warnings = Vec::new();

        for entry in self.discover() {
            crate::config_validate::check_unsupported_format(&entry.path)?;
            let Some(parsed) = read_optional_json_object(&entry.path)? else {
                continue;
            };
            let validation = crate::config_validate::validate_config_file(
                &parsed.object,
                &parsed.source,
                &entry.path,
            );
            if !validation.is_ok() {
                let first_error = &validation.errors[0];
                return Err(ConfigError::Parse(first_error.to_string()));
            }
            all_warnings.extend(validation.warnings);
            validate_optional_hooks_config(&parsed.object, &entry.path)?;
            merge_mcp_servers(&mut mcp_servers, entry.source, &parsed.object, &entry.path)?;
            deep_merge_objects(&mut merged, &parsed.object);
            loaded_entries.push(entry);
        }

        for warning in &all_warnings {
            eprintln!("warning: {warning}");
        }

        let merged_value = JsonValue::Object(merged.clone());

        // ── Schema version migration ──
        let schema_version: u32 = merged_value
            .as_object()
            .and_then(|obj| obj.get("schema_version"))
            .and_then(|v| v.as_i64())
            .map(|n| n as u32)
            .unwrap_or(1);
        if schema_version < CURRENT_SCHEMA_VERSION {
            apply_migrations(&mut merged, schema_version);
        }

        let merged_value = JsonValue::Object(merged.clone());

        let feature_config = RuntimeFeatureConfig {
            hooks: parse_optional_hooks_config(&merged_value)?,
            plugins: parse_optional_plugin_config(&merged_value)?,
            mcp: McpConfigCollection {
                servers: mcp_servers,
            },
            oauth: parse_optional_oauth_config(&merged_value, "merged settings.oauth")?,
            model: parse_optional_model(&merged_value),
            aliases: parse_optional_aliases(&merged_value)?,
            permission_mode: parse_optional_permission_mode(&merged_value)?,
            permission_rules: parse_optional_permission_rules(&merged_value)?,
            sandbox: parse_optional_sandbox_config(&merged_value)?,
            provider_fallbacks: parse_optional_provider_fallbacks(&merged_value)?,
            trusted_roots: parse_optional_trusted_roots(&merged_value)?,
            features: parse_optional_features(&merged_value)?,
        };

        Ok(RuntimeConfig {
            schema_version: CURRENT_SCHEMA_VERSION,
            merged,
            loaded_entries,
            feature_config,
        })
    }
}
