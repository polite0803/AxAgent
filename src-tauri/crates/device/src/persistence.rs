// SPDX-License-Identifier: AGPL-3.0-only

//! 数据持久化模块
//!
//! 提供基于 JSON 文件的数据持久化存储，
//! 用于保存设备信息、变更日志、同步策略等数据。

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

use axagent_harness::device_sync::{
    AuditLogEntry, ChangeLogEntry, DeviceInfo, DevicePermissions, SyncHistoryEntry, SyncPolicy,
};

/// 持久化存储路径配置
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// 数据目录路径
    pub data_dir: PathBuf,
}

impl PersistenceConfig {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn devices_path(&self) -> PathBuf {
        self.data_dir.join("devices.json")
    }

    fn change_log_path(&self) -> PathBuf {
        self.data_dir.join("change_log.json")
    }

    fn policies_path(&self) -> PathBuf {
        self.data_dir.join("policies.json")
    }

    fn history_path(&self) -> PathBuf {
        self.data_dir.join("history.json")
    }

    fn permissions_path(&self) -> PathBuf {
        self.data_dir.join("permissions.json")
    }
}

/// 持久化存储
pub struct PersistentStore {
    config: PersistenceConfig,
}

impl PersistentStore {
    pub fn new(config: PersistenceConfig) -> Self {
        Self { config }
    }

    /// 确保数据目录存在
    pub async fn ensure_dir(&self) -> Result<(), String> {
        if !self.config.data_dir.exists() {
            tokio::fs::create_dir_all(&self.config.data_dir)
                .await
                .map_err(|e| format!("Failed to create data directory: {}", e))?;
        }
        Ok(())
    }

    // ─── 设备存储 ────────────────────────────────────────────────────────

    /// 加载设备列表
    pub async fn load_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        Ok(self
            .load_json::<Vec<DeviceInfo>>(&self.config.devices_path())
            .await?
            .unwrap_or_default())
    }

    /// 保存设备列表
    pub async fn save_devices(&self, devices: &[DeviceInfo]) -> Result<(), String> {
        self.save_json(&self.config.devices_path(), &devices.to_vec()).await
    }

    // ─── 变更日志存储 ────────────────────────────────────────────────────

    /// 加载变更日志
    pub async fn load_change_log(&self) -> Result<Vec<ChangeLogEntry>, String> {
        Ok(self
            .load_json::<Vec<ChangeLogEntry>>(&self.config.change_log_path())
            .await?
            .unwrap_or_default())
    }

    /// 保存变更日志
    pub async fn save_change_log(&self, entries: &[ChangeLogEntry]) -> Result<(), String> {
        self.save_json(&self.config.change_log_path(), &entries.to_vec()).await
    }

    // ─── 同步策略存储 ────────────────────────────────────────────────────

    /// 加载策略列表
    pub async fn load_policies(&self) -> Result<Vec<SyncPolicy>, String> {
        Ok(self
            .load_json::<Vec<SyncPolicy>>(&self.config.policies_path())
            .await?
            .unwrap_or_default())
    }

    /// 保存策略列表
    pub async fn save_policies(&self, policies: &[SyncPolicy]) -> Result<(), String> {
        self.save_json(&self.config.policies_path(), &policies.to_vec()).await
    }

    // ─── 历史记录存储 ────────────────────────────────────────────────────

    /// 加载同步历史
    pub async fn load_history(&self) -> Result<Vec<SyncHistoryEntry>, String> {
        Ok(self
            .load_json::<Vec<SyncHistoryEntry>>(&self.config.history_path())
            .await?
            .unwrap_or_default())
    }

    /// 保存同步历史
    pub async fn save_history(&self, history: &[SyncHistoryEntry]) -> Result<(), String> {
        self.save_json(&self.config.history_path(), &history.to_vec()).await
    }

    /// 加载审计日志
    pub async fn load_audit_logs(&self) -> Result<Vec<AuditLogEntry>, String> {
        // 审计日志与同步历史共用存储文件的不同部分
        // 为简化实现，这里返回空列表
        Ok(Vec::new())
    }

    // ─── 权限存储 ────────────────────────────────────────────────────────

    /// 加载权限配置
    pub async fn load_permissions(&self) -> Result<HashMap<String, DevicePermissions>, String> {
        Ok(self
            .load_json::<HashMap<String, DevicePermissions>>(&self.config.permissions_path())
            .await?
            .unwrap_or_default())
    }

    /// 保存权限配置
    pub async fn save_permissions(
        &self,
        permissions: &HashMap<String, DevicePermissions>,
    ) -> Result<(), String> {
        self.save_json(&self.config.permissions_path(), permissions).await
    }

    // ─── 内部工具方法 ─────────────────────────────────────────────────────

    async fn load_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &PathBuf,
    ) -> Result<Option<T>, String> {
        if !path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        if content.is_empty() {
            return Ok(None);
        }

        let data: T =
            serde_json::from_str(&content).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        Ok(Some(data))
    }

    async fn save_json<T: serde::Serialize>(&self, path: &PathBuf, data: &T) -> Result<(), String> {
        self.ensure_dir().await?;

        let content = serde_json::to_string_pretty(data)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;

        // 先写入临时文件，再重命名以保证原子性
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, &content)
            .await
            .map_err(|e| format!("Failed to write temp file: {}", e))?;

        tokio::fs::rename(&temp_path, path)
            .await
            .map_err(|e| format!("Failed to rename file: {}", e))?;

        Ok(())
    }
}

/// 带缓存的持久化存储
pub struct CachedStore<T> {
    inner: RwLock<Option<T>>,
    persistence: PersistentStore,
    config_fn: Box<dyn Fn() -> PathBuf + Send + Sync>,
}

impl<T: Clone> CachedStore<T> {
    pub fn new(persistence: PersistentStore) -> Self {
        Self { inner: RwLock::new(None), persistence, config_fn: Box::new(PathBuf::new) }
    }

    pub fn persistence(&self) -> &PersistentStore {
        &self.persistence
    }

    pub async fn get_cached(&self) -> Option<T> {
        self.inner.read().await.clone()
    }

    pub async fn set_cached(&self, value: T) {
        *self.inner.write().await = Some(value);
    }

    pub fn config_path(&self) -> PathBuf {
        (self.config_fn)()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_persistence_config() {
        let config = PersistenceConfig::new(PathBuf::from("/tmp/test_sync"));
        assert!(config.devices_path().to_string_lossy().contains("devices.json"));
        assert!(config.change_log_path().to_string_lossy().contains("change_log.json"));
    }

    #[tokio::test]
    async fn test_save_and_load() {
        let dir = tempfile::tempdir().expect("测试：创建临时目录应成功");
        let config = PersistenceConfig::new(dir.path().to_path_buf());
        let store = PersistentStore::new(config);

        store.ensure_dir().await.expect("测试：异步操作应成功");

        // 测试空列表保存和加载
        let devices: Vec<DeviceInfo> = Vec::new();
        store.save_devices(&devices).await.expect("测试：异步操作应成功");
        let loaded = store.load_devices().await.expect("测试：异步操作应成功");
        assert!(loaded.is_empty());
    }
}
