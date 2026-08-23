// SPDX-License-Identifier: AGPL-3.0-only
//! 插件组合 Profile 命令集（缺陷 #9：agent 预设上升为可 dump/patch 的组合机制）
//!
//! 把「agent 预设 + 插件组合」抽象为可命名的 Profile，支持：
//! - `plugin_profile_create`：创建组合（选 agent 预设 + 插件列表）
//! - `plugin_profile_list`：列出全部组合
//! - `plugin_profile_dump`：dump-config——导出某组合的完整配置 + 能力接缝快照
//! - `plugin_profile_patch`：patch 覆盖——增删插件、改字段、切换启用状态
//! - `plugin_profile_delete`：删除组合
//!
//! 持久化为 JSON 到 `app_data_dir/plugin_profiles.json`（无需 DB migration），
//! 与既有 Profile 概念正交：运行时 Profile 管数据目录、UserProfile 是用户画像、
//! AgentProfile 是能力集，本模块是「插件组合」的命名视图。

use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use axagent_agent_macro::agent_command;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::app_state::AppState;
use crate::commands::capability::CapabilityRegistrationDetailDto;
use crate::commands::error::{CommandError, ErrorCategory};
use crate::commands::error_code;

/// 插件组合中的单条插件选择。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSelectionDto {
    pub plugin_id: String,
    pub enabled: bool,
}

/// 插件组合 Profile（同时作为存储模型与 DTO）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginBundleProfileDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub agent_profile_id: Option<String>,
    pub plugins: Vec<PluginSelectionDto>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 创建插件组合 Profile 的请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePluginProfileRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub agent_profile_id: Option<String>,
    #[serde(default)]
    pub plugin_ids: Vec<String>,
}

/// patch 覆盖插件组合 Profile 的请求（仅提供需变更的字段）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPluginProfileRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// 显式置 `null` 可清空 agent 预设关联。
    #[serde(default)]
    pub agent_profile_id: Option<Option<String>>,
    #[serde(default)]
    pub add_plugins: Vec<String>,
    #[serde(default)]
    pub remove_plugins: Vec<String>,
    /// plugin_id → 是否启用。
    #[serde(default)]
    pub plugin_enabled: HashMap<String, bool>,
}

/// dump-config 检视结果：Profile 完整配置 + 其插件声明的能力接缝快照。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginProfileDumpResponse {
    pub profile: PluginBundleProfileDto,
    /// 该组合内插件在能力注册表中声明的能力接缝。
    pub capabilities: Vec<CapabilityRegistrationDetailDto>,
}

// ── 存储 ──────────────────────────────────────────

/// JSON 文件持久化的插件组合 Profile 存储（惰性全局单例，按 app_data_dir 初始化）。
struct PluginProfileStore {
    path: PathBuf,
    inner: std::sync::Mutex<BTreeMap<String, PluginBundleProfileDto>>,
}

impl PluginProfileStore {
    fn new(path: PathBuf) -> Self {
        let inner = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str::<BTreeMap<String, PluginBundleProfileDto>>(&s).ok())
            .unwrap_or_default();
        Self { path, inner: std::sync::Mutex::new(inner) }
    }

    fn persist(&self) -> io::Result<()> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let json = serde_json::to_string_pretty(&*guard)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        std::fs::write(&self.path, json)
    }

    fn list(&self) -> Vec<PluginBundleProfileDto> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.values().cloned().collect()
    }

    fn get(&self, id: &str) -> Option<PluginBundleProfileDto> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.get(id).cloned()
    }

    fn insert(&self, profile: PluginBundleProfileDto) -> io::Result<()> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).insert(profile.id.clone(), profile);
        self.persist()
    }

    /// 返回是否真删除了某 id。
    fn remove(&self, id: &str) -> io::Result<bool> {
        let removed = self.inner.lock().unwrap_or_else(|e| e.into_inner()).remove(id).is_some();
        if removed {
            self.persist()?;
        }
        Ok(removed)
    }
}

static STORE: OnceLock<Arc<PluginProfileStore>> = OnceLock::new();

fn store_for(app_data_dir: &Path) -> Arc<PluginProfileStore> {
    STORE
        .get_or_init(|| {
            Arc::new(PluginProfileStore::new(app_data_dir.join("plugin_profiles.json")))
        })
        .clone()
}

fn not_found(id: &str) -> CommandError {
    CommandError::new(error_code::plugin_profile::NOT_FOUND)
        .with_category(ErrorCategory::Validation)
        .with_detail(format!("插件组合 Profile `{id}` 不存在"))
}

fn io_error(e: io::Error) -> CommandError {
    CommandError::new(error_code::plugin_profile::IO_FAILED)
        .with_category(ErrorCategory::Unrecoverable)
        .with_detail(e.to_string())
}

// ── 命令 ──────────────────────────────────────────

/// 列出全部插件组合 Profile。
#[agent_command(domain = plugin, safety = Safe, call_mode = StateOnly, description = "列出全部插件组合 Profile")]
#[tauri::command]
pub async fn plugin_profile_list(
    state: State<'_, AppState>,
) -> Result<Vec<PluginBundleProfileDto>, String> {
    let store = store_for(&state.app_data_dir);
    Ok(store.list())
}

/// 创建插件组合 Profile。
#[agent_command(domain = plugin, safety = Safe, call_mode = StateInput, description = "创建插件组合 Profile")]
#[tauri::command]
pub async fn plugin_profile_create(
    state: State<'_, AppState>,
    request: CreatePluginProfileRequest,
) -> Result<PluginBundleProfileDto, String> {
    if request.name.trim().is_empty() {
        return Err(String::from(
            CommandError::new(error_code::plugin_profile::DUPLICATE_NAME)
                .with_category(ErrorCategory::Validation)
                .with_detail("Profile 名称不能为空"),
        ));
    }
    let store = store_for(&state.app_data_dir);
    if store.list().iter().any(|p| p.name == request.name) {
        return Err(String::from(
            CommandError::new(error_code::plugin_profile::DUPLICATE_NAME)
                .with_category(ErrorCategory::Validation)
                .with_detail(format!("插件组合 Profile 名称 `{}` 已存在", request.name)),
        ));
    }
    let now = chrono::Utc::now().timestamp_millis();
    let profile = PluginBundleProfileDto {
        id: uuid::Uuid::new_v4().to_string(),
        name: request.name,
        description: request.description,
        agent_profile_id: request.agent_profile_id,
        plugins: request
            .plugin_ids
            .into_iter()
            .map(|plugin_id| PluginSelectionDto { plugin_id, enabled: true })
            .collect(),
        created_at: now,
        updated_at: now,
    };
    store.insert(profile.clone()).map_err(|e| String::from(io_error(e)))?;
    Ok(profile)
}

/// 删除插件组合 Profile。
#[agent_command(domain = plugin, safety = Safe, call_mode = StateInput, description = "删除插件组合 Profile")]
#[tauri::command]
pub async fn plugin_profile_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = store_for(&state.app_data_dir);
    let removed = store.remove(&id).map_err(|e| String::from(io_error(e)))?;
    if !removed {
        return Err(String::from(not_found(&id)));
    }
    Ok(())
}

/// dump-config：导出插件组合 Profile 的完整配置与其能力接缝快照。
#[agent_command(domain = plugin, safety = Safe, call_mode = StateOnly, description = "dump 插件组合 Profile 配置与能力快照")]
#[tauri::command]
pub async fn plugin_profile_dump(
    state: State<'_, AppState>,
    id: String,
) -> Result<PluginProfileDumpResponse, String> {
    let store = store_for(&state.app_data_dir);
    let profile = store.get(&id).ok_or_else(|| String::from(not_found(&id)))?;
    let capabilities = axagent_harness::get_capability_registry()
        .list_with_details()
        .into_iter()
        .filter(|d| {
            profile.plugins.iter().any(|p| d.plugin_id.as_deref() == Some(p.plugin_id.as_str()))
        })
        .map(|d| CapabilityRegistrationDetailDto {
            id: d.definition.id,
            version: d.definition.version,
            contract: d.definition.contract,
            description: d.definition.description,
            origin: d.origin.as_str().to_string(),
            plugin_id: d.plugin_id,
        })
        .collect();
    Ok(PluginProfileDumpResponse { profile, capabilities })
}

/// patch 覆盖插件组合 Profile（增删插件、改字段、切换启用状态）。
#[agent_command(domain = plugin, safety = Safe, call_mode = StateInput, description = "patch 覆盖插件组合 Profile")]
#[tauri::command]
pub async fn plugin_profile_patch(
    state: State<'_, AppState>,
    id: String,
    request: PatchPluginProfileRequest,
) -> Result<PluginBundleProfileDto, String> {
    let store = store_for(&state.app_data_dir);
    let mut profile = store.get(&id).ok_or_else(|| String::from(not_found(&id)))?;

    if let Some(name) = request.name {
        if name.trim().is_empty() {
            return Err(String::from(
                CommandError::new(error_code::plugin_profile::DUPLICATE_NAME)
                    .with_category(ErrorCategory::Validation)
                    .with_detail("Profile 名称不能为空"),
            ));
        }
        if profile.name != name && store.list().iter().any(|p| p.name == name) {
            return Err(String::from(
                CommandError::new(error_code::plugin_profile::DUPLICATE_NAME)
                    .with_category(ErrorCategory::Validation)
                    .with_detail(format!("插件组合 Profile 名称 `{name}` 已存在")),
            ));
        }
        profile.name = name;
    }
    if let Some(description) = request.description {
        profile.description = description;
    }
    if let Some(agent_profile_id) = request.agent_profile_id {
        profile.agent_profile_id = agent_profile_id;
    }
    for plugin_id in request.add_plugins {
        if !profile.plugins.iter().any(|p| p.plugin_id == plugin_id) {
            profile.plugins.push(PluginSelectionDto { plugin_id, enabled: true });
        }
    }
    if !request.remove_plugins.is_empty() {
        profile.plugins.retain(|p| !request.remove_plugins.contains(&p.plugin_id));
    }
    for (plugin_id, enabled) in request.plugin_enabled {
        if let Some(sel) = profile.plugins.iter_mut().find(|p| p.plugin_id == plugin_id) {
            sel.enabled = enabled;
        }
    }
    profile.updated_at = chrono::Utc::now().timestamp_millis();
    store.insert(profile.clone()).map_err(|e| String::from(io_error(e)))?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(tag: &str) -> (std::path::PathBuf, PluginProfileStore) {
        let dir = std::env::temp_dir()
            .join(format!("axagent_plugin_profile_{tag}_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("plugin_profiles.json");
        let store = PluginProfileStore::new(path.clone());
        (dir, store)
    }

    fn sample(id: &str, name: &str) -> PluginBundleProfileDto {
        PluginBundleProfileDto {
            id: id.to_string(),
            name: name.to_string(),
            description: "测试".into(),
            agent_profile_id: Some("ap-1".into()),
            plugins: vec![
                PluginSelectionDto { plugin_id: "p-a".into(), enabled: true },
                PluginSelectionDto { plugin_id: "p-b".into(), enabled: false },
            ],
            created_at: 1,
            updated_at: 2,
        }
    }

    #[test]
    fn store_crud_and_persists_across_reopen() {
        let (dir, store) = temp_store("crud");
        store.insert(sample("1", "工作台")).unwrap();
        store.insert(sample("2", "评测")).unwrap();

        assert_eq!(store.list().len(), 2);
        assert_eq!(store.get("1").unwrap().name, "工作台");
        assert_eq!(store.get("1").unwrap().plugins.len(), 2);

        // 重建实例（模拟进程重启）验证持久化
        let reopened = PluginProfileStore::new(dir.join("plugin_profiles.json"));
        assert_eq!(reopened.list().len(), 2);
        assert_eq!(reopened.get("2").unwrap().name, "评测");

        // 删除
        assert!(reopened.remove("1").unwrap());
        assert!(!reopened.remove("1").unwrap());
        assert_eq!(reopened.list().len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn store_new_loads_empty_when_no_file() {
        let (dir, store) = temp_store("empty");
        assert!(store.list().is_empty());
        assert!(store.get("nope").is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
