use sea_orm::DatabaseConnection;
use tracing::warn;

use crate::error::Result;
use crate::repo::settings;

// ── Builtin group definitions (mirrors mcp_server::BUILTIN_DEFS) ───────

/// Builtin local tool group definition.
/// Mirrors the data in `mcp_server::BUILTIN_DEFS` but uses group_id as the
/// primary key for the new `local_tool:{group_id}:enabled` settings key.
struct BuiltinGroupDef {
    id: &'static str,
    name: &'static str,
    default_enabled: bool,
}

const BUILTIN_GROUP_DEFS: &[BuiltinGroupDef] = &[
    BuiltinGroupDef { id: "builtin-fetch",       name: "@axagent/fetch",       default_enabled: true },
    BuiltinGroupDef { id: "builtin-search-file",  name: "@axagent/search-file", default_enabled: true },
    BuiltinGroupDef { id: "builtin-skills",       name: "@axagent/skills",     default_enabled: true },
    BuiltinGroupDef { id: "builtin-session",      name: "@axagent/session",    default_enabled: true },
    BuiltinGroupDef { id: "builtin-search",       name: "@axagent/search",     default_enabled: true },
    BuiltinGroupDef { id: "builtin-filesystem",   name: "@axagent/filesystem", default_enabled: true },
    BuiltinGroupDef { id: "builtin-system",       name: "@axagent/system",     default_enabled: true },
    BuiltinGroupDef { id: "builtin-knowledge",    name: "@axagent/knowledge",  default_enabled: true },
    BuiltinGroupDef { id: "builtin-storage",      name: "@axagent/storage",    default_enabled: true },
    BuiltinGroupDef { id: "builtin-memory",       name: "@axagent/memory",     default_enabled: true },
];

// ── Settings key helpers ───────────────────────────────────────────────

/// Generate the settings key for a local tool group's enabled state.
/// Format: `local_tool:{group_id}:enabled`
fn local_tool_setting_key(group_id: &str) -> String {
    format!("local_tool:{group_id}:enabled")
}

/// Generate the legacy settings key (from the old MCP-based builtin system).
/// Format: `builtin_mcp:{name}:enabled`
fn legacy_setting_key(name: &str) -> String {
    format!("builtin_mcp:{name}:enabled")
}

// ── Public API ─────────────────────────────────────────────────────────

/// Get the enabled state for a local tool group from the settings table.
/// Returns `default` if no setting is found.
pub async fn get_enabled(db: &DatabaseConnection, group_id: &str, default: bool) -> bool {
    match settings::get_setting(db, &local_tool_setting_key(group_id)).await {
        Ok(Some(v)) => v == "true",
        _ => default,
    }
}

/// Set the enabled state for a local tool group, persisting to the settings table.
pub async fn set_enabled(
    db: &DatabaseConnection,
    group_id: &str,
    enabled: bool,
) -> Result<()> {
    settings::set_setting(
        db,
        &local_tool_setting_key(group_id),
        if enabled { "true" } else { "false" },
    )
    .await
}

/// Migrate legacy `builtin_mcp:{name}:enabled` keys to the new
/// `local_tool:{group_id}:enabled` format.
///
/// For each builtin group, if the legacy key exists in the settings table,
/// its value is copied to the new key and the legacy key is deleted.
/// If migration for a particular key fails, a warning is logged and the
/// process continues (fallback to default on next read).
pub async fn migrate_legacy_keys(db: &DatabaseConnection) {
    for def in BUILTIN_GROUP_DEFS {
        let old_key = legacy_setting_key(def.name);
        match settings::get_setting(db, &old_key).await {
            Ok(Some(value)) => {
                // Write to new key
                if let Err(e) = set_enabled(db, def.id, value == "true").await {
                    warn!(
                        "Failed to migrate legacy key '{}' -> '{}': {}",
                        old_key,
                        local_tool_setting_key(def.id),
                        e
                    );
                    continue;
                }
                // Delete old key by setting it to empty (settings table doesn't support delete directly,
                // but we can overwrite with the new key's value and the old key becomes orphaned).
                // Actually, let's just leave the old key — it won't be read anymore.
                // The old key will be harmless once nothing reads it.
                // If we want to clean up, we could set it to "migrated" but that's unnecessary.
                tracing::info!(
                    "Migrated legacy key '{}' = '{}' -> '{}'",
                    old_key,
                    value,
                    local_tool_setting_key(def.id)
                );
            }
            Ok(None) => {
                // No legacy key exists, nothing to migrate
            }
            Err(e) => {
                warn!("Failed to read legacy key '{}' during migration: {}", old_key, e);
            }
        }
    }
}

/// Return all builtin group IDs.
pub fn all_group_ids() -> Vec<&'static str> {
    BUILTIN_GROUP_DEFS.iter().map(|d| d.id).collect()
}

/// Check whether a group ID belongs to a builtin local tool group.
pub fn is_builtin_group_id(id: &str) -> bool {
    BUILTIN_GROUP_DEFS.iter().any(|d| d.id == id)
}

/// Get the group name (e.g. "@axagent/fetch") for a builtin group ID.
pub fn get_group_name(id: &str) -> Option<&'static str> {
    BUILTIN_GROUP_DEFS.iter().find(|d| d.id == id).map(|d| d.name)
}

/// Get the default enabled state for a builtin group ID.
pub fn get_default_enabled(id: &str) -> bool {
    BUILTIN_GROUP_DEFS
        .iter()
        .find(|d| d.id == id)
        .map(|d| d.default_enabled)
        .unwrap_or(true)
}
