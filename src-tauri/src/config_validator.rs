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
struct AgentRolesConfig {
    schema_version: Option<u32>,
    roles: Option<Vec<RoleConfig>>,
}

#[derive(Debug, Deserialize)]
struct RoleConfig {
    name: Option<String>,
    enabled: Option<bool>,
    system_prompt: Option<String>,
    allowed_tools: Option<Vec<String>>,
    max_concurrent: Option<u32>,
    timeout_seconds: Option<u64>,
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
        Some(v) if v == 0 => {
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

        if role.name.as_ref().map_or(true, |n| n.trim().is_empty()) {
            tracing::warn!("[config-validator] {} role #{} missing or empty 'name'", path, idx);
            valid = false;
        }

        if role.system_prompt.as_ref().map_or(true, |p| p.trim().is_empty()) {
            tracing::warn!(
                "[config-validator] {} role #{} missing or empty 'system_prompt'",
                path,
                idx
            );
            valid = false;
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
