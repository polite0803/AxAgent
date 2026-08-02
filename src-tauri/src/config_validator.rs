// SPDX-License-Identifier: AGPL-3.0-only

//! m7: agent_roles.yaml 基本结构校验。
//!
//! 在启动时对 config/agent_roles.yaml 执行基本语义校验，包括：
//! - 必需的顶层字段 (schema_version, roles)
//! - 每个 role 的必填字段 (name, system_prompt)
//! - 字段类型与合法值范围
//!
//! 校验失败时记录 warning 而非 panic，避免因配置错误导致启动中断。

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AgentRolesConfig {
    schema_version: Option<u32>,
    roles: Option<Vec<RoleConfig>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RoleConfig {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub system_prompt: Option<String>,
    pub allowed_tools: Option<Vec<String>>,
    pub max_concurrent: Option<u32>,
    pub timeout_seconds: Option<u64>,
}

/// 校验 agent_roles.yaml 的基本结构，返回是否通过校验。
pub fn validate_agent_roles(path: &str) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("[config-validator] Cannot read {}: {}", path, e);
            return false;
        },
    };

    let config: AgentRolesConfig = match serde_yaml::from_str(&content) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("[config-validator] {} is not valid YAML: {}", path, e);
            return false;
        },
    };

    let mut valid = true;

    // 顶层字段校验
    match config.schema_version {
        None => {
            tracing::warn!("[config-validator] {} missing 'schema_version'", path);
            valid = false;
        },
        Some(0) => {
            tracing::warn!("[config-validator] {} schema_version is 0 (reserved)", path);
            valid = false;
        },
        Some(v) => {
            tracing::info!("[config-validator] {} schema_version={}", path, v);
        },
    }

    let roles = match config.roles {
        Some(r) => r,
        None => {
            tracing::warn!("[config-validator] {} missing 'roles' array", path);
            return false;
        },
    };

    if roles.is_empty() {
        tracing::warn!("[config-validator] {} has empty 'roles' array", path);
        return false;
    }

    for (i, role) in roles.iter().enumerate() {
        let idx = i + 1;

        if role.enabled == Some(false) {
            tracing::info!(
                "[config-validator] {} role #{} is disabled, skip validation",
                path,
                idx
            );
            continue;
        }

        if role.name.as_ref().is_none_or(|n| n.trim().is_empty()) {
            tracing::warn!("[config-validator] {} role #{} missing or empty 'name'", path, idx);
            valid = false;
        }

        if role.system_prompt.as_ref().is_none_or(|p| p.trim().is_empty()) {
            tracing::warn!(
                "[config-validator] {} role #{} missing or empty 'system_prompt'",
                path,
                idx
            );
            valid = false;
        }

        if let Some(tools) = &role.allowed_tools {
            if tools.is_empty() {
                tracing::warn!(
                    "[config-validator] {} role #{} allowed_tools is empty (no tools permitted)",
                    path,
                    idx
                );
            }
        }

        if let Some(mc) = role.max_concurrent {
            if mc == 0 {
                tracing::warn!(
                    "[config-validator] {} role #{} max_concurrent is 0 (will never execute)",
                    path,
                    idx
                );
            }
        }

        if let Some(ts) = role.timeout_seconds {
            if ts == 0 {
                tracing::warn!("[config-validator] {} role #{} timeout_seconds is 0", path, idx);
            }
        }
    }

    if valid {
        tracing::info!("[config-validator] {} passed validation ({} roles)", path, roles.len());
    }

    valid
}

/// 解析 agent_roles.yaml 中启用的角色（供启动时 upsert 到 agent_roles 表）。
pub fn parse_enabled_roles(content: &str) -> Vec<RoleConfig> {
    match serde_yaml::from_str::<AgentRolesConfig>(content) {
        Ok(config) => config
            .roles
            .unwrap_or_default()
            .into_iter()
            .filter(|r| r.enabled != Some(false))
            .filter(|r| r.name.as_ref().is_some_and(|n| !n.trim().is_empty()))
            .collect(),
        Err(e) => {
            tracing::warn!("[config-validator] parse_enabled_roles failed: {e}");
            Vec::new()
        },
    }
}
