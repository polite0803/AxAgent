// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use crate::json::JsonValue;
use crate::sandbox::SandboxConfig;

/// Schema name advertised by generated settings files.
pub const CLAW_SETTINGS_SCHEMA_NAME: &str = "SettingsSchema";

/// Origin of a loaded settings file in the configuration precedence chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigSource {
    User,
    Project,
    Local,
}

/// Effective permission mode after decoding config values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedPermissionMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

/// A discovered config file and the scope it contributes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigEntry {
    pub source: ConfigSource,
    pub path: PathBuf,
}

/// Fully merged runtime configuration plus parsed feature-specific views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub schema_version: u32,
    pub(crate) merged: BTreeMap<String, JsonValue>,
    pub(crate) loaded_entries: Vec<ConfigEntry>,
    pub(crate) feature_config: RuntimeFeatureConfig,
}

/// Parsed plugin-related settings extracted from runtime config.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimePluginConfig {
    pub(crate) enabled_plugins: BTreeMap<String, bool>,
    pub(crate) external_directories: Vec<String>,
    pub(crate) install_root: Option<String>,
    pub(crate) registry_path: Option<String>,
    pub(crate) bundled_root: Option<String>,
    pub(crate) max_output_tokens: Option<u32>,
}

/// Structured feature configuration consumed by runtime subsystems.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeFeatureConfig {
    pub(crate) hooks: RuntimeHookConfig,
    pub(crate) plugins: RuntimePluginConfig,
    pub(crate) mcp: McpConfigCollection,
    pub(crate) oauth: Option<OAuthConfig>,
    pub(crate) model: Option<String>,
    pub(crate) aliases: BTreeMap<String, String>,
    pub(crate) permission_mode: Option<ResolvedPermissionMode>,
    pub(crate) permission_rules: RuntimePermissionRuleConfig,
    pub(crate) sandbox: SandboxConfig,
    pub(crate) provider_fallbacks: ProviderFallbackConfig,
    pub(crate) trusted_roots: Vec<String>,
    pub(crate) features: BTreeMap<String, bool>,
}

/// Ordered chain of fallback model identifiers used when the primary
/// provider returns a retryable failure (429/500/503/etc.). The chain is
/// strict: each entry is tried in order until one succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderFallbackConfig {
    pub(crate) primary: Option<String>,
    pub(crate) fallbacks: Vec<String>,
}

/// Hook command lists grouped by lifecycle stage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeHookConfig {
    pub(crate) pre_tool_use: Vec<String>,
    pub(crate) post_tool_use: Vec<String>,
    pub(crate) post_tool_use_failure: Vec<String>,
    // 新增事件
    pub(crate) subagent_start: Vec<String>,
    pub(crate) subagent_stop: Vec<String>,
    pub(crate) pre_compact: Vec<String>,
    pub(crate) post_compact: Vec<String>,
    pub(crate) session_start: Vec<String>,
    pub(crate) session_end: Vec<String>,
    pub(crate) stop: Vec<String>,
    pub(crate) stop_failure: Vec<String>,
    pub(crate) task_created: Vec<String>,
    pub(crate) task_completed: Vec<String>,
    pub(crate) config_change: Vec<String>,
    pub(crate) instructions_loaded: Vec<String>,
    pub(crate) file_changed: Vec<String>,
    pub(crate) cwd_changed: Vec<String>,
    pub(crate) permission_request: Vec<String>,
    pub(crate) permission_denied: Vec<String>,
    pub(crate) worktree_create: Vec<String>,
    pub(crate) worktree_remove: Vec<String>,
}

/// Raw permission rule lists grouped by allow, deny, and ask behavior.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimePermissionRuleConfig {
    pub(crate) allow: Vec<String>,
    pub(crate) deny: Vec<String>,
    pub(crate) ask: Vec<String>,
}

/// Collection of configured MCP servers after scope-aware merging.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct McpConfigCollection {
    pub(crate) servers: BTreeMap<String, ScopedMcpServerConfig>,
}

/// MCP server config paired with the scope that defined it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedMcpServerConfig {
    pub scope: ConfigSource,
    pub config: McpServerConfig,
}

pub use axagent_harness::tool_service::McpTransport;

/// Scope-normalized MCP server configuration variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpRemoteServerConfig),
    Http(McpRemoteServerConfig),
    Ws(McpWebSocketServerConfig),
    Sdk(McpSdkServerConfig),
    ManagedProxy(McpManagedProxyServerConfig),
}

/// Configuration for an MCP server launched as a local stdio process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpStdioServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub tool_call_timeout_ms: Option<u64>,
}

/// Configuration for an MCP server reached over HTTP or SSE.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRemoteServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
    pub oauth: Option<McpOAuthConfig>,
}

/// Configuration for an MCP server reached over WebSocket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpWebSocketServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
}

/// Configuration for an MCP server addressed through an SDK name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSdkServerConfig {
    pub name: String,
}

/// Configuration for an MCP managed-proxy endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpManagedProxyServerConfig {
    pub url: String,
    pub id: String,
}

/// OAuth overrides associated with a remote MCP server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpOAuthConfig {
    pub client_id: Option<String>,
    pub callback_port: Option<u16>,
    pub auth_server_metadata_url: Option<String>,
    pub xaa: Option<bool>,
}

/// OAuth client configuration used by the main Claw runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthConfig {
    pub client_id: String,
    pub authorize_url: String,
    pub token_url: String,
    pub callback_port: Option<u16>,
    pub manual_redirect_url: Option<String>,
    pub scopes: Vec<String>,
}

/// Errors raised while reading or parsing runtime configuration files.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(String),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Parse(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Discovers config files and merges them into a [`RuntimeConfig`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigLoader {
    pub(crate) cwd: PathBuf,
    pub(crate) config_home: PathBuf,
}

// ── Config merge helpers ──

pub(crate) fn deep_merge_objects(
    target: &mut BTreeMap<String, JsonValue>,
    source: &BTreeMap<String, JsonValue>,
) {
    for (key, value) in source {
        match (target.get_mut(key), value) {
            (Some(JsonValue::Object(existing)), JsonValue::Object(incoming)) => {
                deep_merge_objects(existing, incoming);
            },
            _ => {
                target.insert(key.clone(), value.clone());
            },
        }
    }
}

pub(crate) fn extend_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        push_unique(target, value.clone());
    }
}

pub(crate) fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}

/// Current schema version — bump when adding new required fields.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

/// Migrate from v1 to v2: adds `features` map with `auto_install_plugins: true` default.
pub fn migrate_v1_to_v2(_merged: &mut BTreeMap<String, JsonValue>) {
    // v1→v2: ensure `features.auto_install_plugins` exists
    _merged.entry("features".to_string()).or_insert_with(|| {
        let mut f = BTreeMap::new();
        f.insert("auto_install_plugins".to_string(), JsonValue::Bool(true));
        JsonValue::Object(f)
    });
}

/// Apply migration chain from `source_version` to current.
pub fn apply_migrations(merged: &mut BTreeMap<String, JsonValue>, source_version: u32) {
    if source_version < 2 {
        migrate_v1_to_v2(merged);
    }
    // Future: if source_version < 3 { migrate_v2_to_v3(merged); }
}

// ── 测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::JsonValue;

    fn kv(key: &str, val: &str) -> (String, JsonValue) {
        (key.to_string(), JsonValue::String(val.to_string()))
    }

    #[test]
    fn deep_merge_scalar_overrides() {
        let mut target = BTreeMap::new();
        target.insert("a".into(), JsonValue::String("old".into()));
        let mut source = BTreeMap::new();
        source.insert("a".into(), JsonValue::String("new".into()));
        deep_merge_objects(&mut target, &source);
        assert_eq!(target.get("a"), Some(&JsonValue::String("new".into())));
    }

    #[test]
    fn deep_merge_adds_new_keys() {
        let mut target = BTreeMap::new();
        target.insert("a".into(), JsonValue::String("a_val".into()));
        let mut source = BTreeMap::new();
        source.insert("b".into(), JsonValue::String("b_val".into()));
        deep_merge_objects(&mut target, &source);
        assert_eq!(target.get("a"), Some(&JsonValue::String("a_val".into())));
        assert_eq!(target.get("b"), Some(&JsonValue::String("b_val".into())));
    }

    #[test]
    fn deep_merge_nested_objects() {
        let inner_target = BTreeMap::from([kv("x", "t_x")]);
        let inner_source = BTreeMap::from([kv("y", "s_y")]);
        let mut target = BTreeMap::from([("nested".into(), JsonValue::Object(inner_target))]);
        let source = BTreeMap::from([("nested".into(), JsonValue::Object(inner_source))]);
        deep_merge_objects(&mut target, &source);
        if let Some(JsonValue::Object(m)) = target.get("nested") {
            assert_eq!(m.get("x"), Some(&JsonValue::String("t_x".into())));
            assert_eq!(m.get("y"), Some(&JsonValue::String("s_y".into())));
        } else {
            panic!("expected nested object");
        }
    }

    #[test]
    fn push_unique_skips_duplicates() {
        let mut v = vec!["a".to_string(), "b".to_string()];
        push_unique(&mut v, "a".to_string());
        push_unique(&mut v, "c".to_string());
        assert_eq!(v, vec!["a", "b", "c"]);
    }

    #[test]
    fn extend_unique_works() {
        let mut v = vec!["x".to_string()];
        extend_unique(&mut v, &["y".to_string(), "x".to_string(), "z".to_string()]);
        assert_eq!(v, vec!["x", "y", "z"]);
    }
}
