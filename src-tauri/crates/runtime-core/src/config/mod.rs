// SPDX-License-Identifier: AGPL-3.0-only

pub mod loader;
pub mod runtime;
pub mod types;

#[cfg(test)]
mod tests;

pub use loader::ConfigLoader;
pub use types::{
    CLAW_SETTINGS_SCHEMA_NAME, ConfigEntry, ConfigError, ConfigSource, McpConfigCollection,
    McpManagedProxyServerConfig, McpOAuthConfig, McpRemoteServerConfig, McpSdkServerConfig,
    McpServerConfig, McpStdioServerConfig, McpTransport, McpWebSocketServerConfig, OAuthConfig,
    ProviderFallbackConfig, ResolvedPermissionMode, RuntimeConfig, RuntimeFeatureConfig,
    RuntimeHookConfig, RuntimePermissionRuleConfig, RuntimePluginConfig, ScopedMcpServerConfig,
};
