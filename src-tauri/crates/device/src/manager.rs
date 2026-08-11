// SPDX-License-Identifier: AGPL-3.0-only

//! 设备管理器实现
//!
//! 提供设备注册、配对、列表管理等核心功能。
//! 设备持久化通过 Sea-ORM 抽象层支持 PostgreSQL/SQLite 双数据库，
//! 通过注入 SyncStorage trait 实现，同时保留内存存储用于测试场景。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axagent_harness::device_sync::{
    DeviceInfo, DeviceManager, DeviceType, PairingCode, PairingRequest, PairingResponse,
    SyncStorage, TrustLevel,
};
use axagent_harness::util_fns::current_rfc3339;
use rand::Rng;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 设备存储（线程安全）
///
/// 支持两种存储模式：
/// - 数据库模式：通过注入 SyncStorage trait，设备数据持久化到 PostgreSQL/SQLite
/// - 内存模式：用于测试或无数据库场景
///
/// 配对码始终使用内存存储（临时数据，不需要持久化）
pub struct DeviceStore {
    /// 数据库存储实现（可选）
    sync_storage: Option<Arc<dyn SyncStorage>>,
    /// 内存设备存储（作为回退或缓存）
    devices: RwLock<HashMap<String, DeviceInfo>>,
    /// 配对码存储（始终使用内存）
    pending_codes: RwLock<HashMap<String, PairingCode>>,
}

impl DeviceStore {
    /// 创建内存模式的设备存储（用于测试或无数据库场景）
    pub fn new() -> Self {
        Self {
            sync_storage: None,
            devices: RwLock::new(HashMap::new()),
            pending_codes: RwLock::new(HashMap::new()),
        }
    }

    /// 创建数据库模式的设备存储
    pub fn with_storage(sync_storage: Arc<dyn SyncStorage>) -> Self {
        Self {
            sync_storage: Some(sync_storage),
            devices: RwLock::new(HashMap::new()),
            pending_codes: RwLock::new(HashMap::new()),
        }
    }

    /// 检查是否使用数据库存储
    pub fn uses_database(&self) -> bool {
        self.sync_storage.is_some()
    }

    /// 从数据库加载所有设备到内存缓存
    pub async fn load_devices_from_db(&self) -> Result<(), String> {
        if let Some(storage) = &self.sync_storage {
            let devices = storage.get_all_devices().await?;
            let mut cache = self.devices.write().await;
            for device in devices {
                cache.insert(device.device_id.clone(), device);
            }
        }
        Ok(())
    }

    /// 获取单个设备
    pub async fn get_device(&self, device_id: &str) -> Option<DeviceInfo> {
        // 先检查内存缓存
        if let Some(device) = self.devices.read().await.get(device_id).cloned() {
            return Some(device);
        }

        // 如果使用数据库，尝试从数据库获取
        if let Some(storage) = &self.sync_storage
            && let Ok(Some(device)) = storage.get_device_by_id(device_id).await
        {
            // 回填到缓存
            self.devices.write().await.insert(device_id.to_string(), device.clone());
            return Some(device);
        }

        None
    }

    /// 获取所有设备
    pub async fn get_all_devices(&self) -> Vec<DeviceInfo> {
        if let Some(storage) = &self.sync_storage {
            // 使用数据库存储时，从数据库获取最新数据
            if let Ok(devices) = storage.get_all_devices().await {
                return devices;
            }
        }

        // 回退到内存存储
        let devices = self.devices.read().await;
        let mut list: Vec<_> = devices.values().cloned().collect();
        list.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
        list
    }

    /// 保存或更新设备
    pub async fn upsert_device(&self, device: DeviceInfo) {
        // 先更新内存缓存
        self.devices.write().await.insert(device.device_id.clone(), device.clone());

        // 如果使用数据库，同步到数据库
        if let Some(storage) = &self.sync_storage
            && let Err(e) = storage.save_device(&device).await
        {
            tracing::error!("Failed to save device to database: {}", e);
        }
    }

    /// 删除设备
    pub async fn remove_device(&self, device_id: &str) {
        self.devices.write().await.remove(device_id);

        if let Some(storage) = &self.sync_storage
            && let Err(e) = storage.delete_device(device_id).await
        {
            tracing::error!("Failed to delete device from database: {}", e);
        }
    }

    /// 添加配对码（仅内存）
    pub async fn add_pending_code(&self, code: PairingCode) {
        self.pending_codes.write().await.insert(code.code.clone(), code);
    }

    /// 获取并移除配对码
    pub async fn get_and_remove_pending_code(&self, code: &str) -> Option<PairingCode> {
        self.pending_codes.write().await.remove(code)
    }

    /// 清理过期配对码
    pub async fn cleanup_expired_codes(&self) {
        let now = current_rfc3339();
        let mut codes = self.pending_codes.write().await;
        codes.retain(|_, v| v.expires_at > now);
    }
}

impl Default for DeviceStore {
    fn default() -> Self {
        Self::new()
    }
}

/// 设备管理器实现
pub struct DeviceManagerImpl {
    store: Arc<DeviceStore>,
}

impl DeviceManagerImpl {
    pub fn new(store: Arc<DeviceStore>) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &Arc<DeviceStore> {
        &self.store
    }

    /// 生成本地设备信息
    pub fn create_local_device(
        name: String,
        hostname: String,
        os: String,
        app_version: String,
    ) -> DeviceInfo {
        let now = current_rfc3339();
        DeviceInfo {
            device_id: Uuid::new_v4().to_string(),
            name,
            hostname,
            os,
            device_type: DeviceType::Desktop,
            app_version,
            registered_at: now.clone(),
            last_active_at: now,
            is_paired: false,
            trust_level: TrustLevel::Standard,
        }
    }
}

#[async_trait]
impl DeviceManager for DeviceManagerImpl {
    async fn register_device(&self, device: DeviceInfo) -> Result<DeviceInfo, String> {
        let now = current_rfc3339();
        let mut device = device;
        device.registered_at = now.clone();
        device.last_active_at = now;
        self.store.upsert_device(device.clone()).await;
        Ok(device)
    }

    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, String> {
        Ok(self.store.get_all_devices().await)
    }

    async fn accept_pairing(
        &self,
        request: PairingRequest,
        trust_level: TrustLevel,
    ) -> Result<PairingResponse, String> {
        let mut device = request.device;
        device.is_paired = true;
        device.trust_level = trust_level;
        device.last_active_at = current_rfc3339();

        self.store.upsert_device(device).await;

        Ok(PairingResponse {
            success: true,
            message: "配对成功".to_string(),
            assigned_trust_level: trust_level,
            session_token: Some(Uuid::new_v4().to_string()),
            peer_public_key: request.public_key,
        })
    }

    async fn reject_pairing(&self, _device_id: &str) -> Result<(), String> {
        Ok(())
    }

    async fn unpair_device(&self, device_id: &str) -> Result<(), String> {
        self.store.remove_device(device_id).await;
        Ok(())
    }

    async fn generate_pairing_code(&self) -> Result<PairingCode, String> {
        self.store.cleanup_expired_codes().await;

        let code = tokio::task::spawn_blocking(|| {
            let mut rng = rand::thread_rng();
            (0..6).map(|_| rng.gen_range('0'..='9')).collect::<String>()
        })
        .await
        .map_err(|e| format!("Failed to generate pairing code: {}", e))?;

        let now = chrono::Utc::now();
        let expires_at = now + chrono::Duration::minutes(5);

        let pairing_code = PairingCode {
            code,
            created_at: now.to_rfc3339(),
            expires_at: expires_at.to_rfc3339(),
            pending_device_id: String::new(),
        };

        self.store.add_pending_code(pairing_code.clone()).await;
        Ok(pairing_code)
    }

    async fn verify_pairing_code(&self, code: &str) -> Result<PairingRequest, String> {
        let pairing_code = self
            .store
            .get_and_remove_pending_code(code)
            .await
            .ok_or_else(|| "配对码不存在或已过期".to_string())?;

        let now = current_rfc3339();
        if pairing_code.expires_at < now {
            return Err("配对码已过期".to_string());
        }

        Ok(PairingRequest {
            device: DeviceInfo {
                device_id: pairing_code.pending_device_id,
                name: String::new(),
                hostname: String::new(),
                os: String::new(),
                device_type: DeviceType::Desktop,
                app_version: String::new(),
                registered_at: now.clone(),
                last_active_at: now,
                is_paired: false,
                trust_level: TrustLevel::Standard,
            },
            pairing_code: code.to_string(),
            public_key: None,
        })
    }

    async fn update_device_activity(&self, device_id: &str) -> Result<(), String> {
        if let Some(mut device) = self.store.get_device(device_id).await {
            device.last_active_at = current_rfc3339();
            self.store.upsert_device(device).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_device() {
        let store = Arc::new(DeviceStore::new());
        let manager = DeviceManagerImpl::new(store);

        let device = DeviceManagerImpl::create_local_device(
            "Test Device".to_string(),
            "localhost".to_string(),
            "Windows".to_string(),
            "1.0.0".to_string(),
        );

        let registered = manager.register_device(device).await.expect("测试：异步操作应成功");
        assert!(!registered.device_id.is_empty());
        assert_eq!(registered.name, "Test Device");
    }

    #[tokio::test]
    async fn test_pairing_code() {
        let store = Arc::new(DeviceStore::new());
        let manager = DeviceManagerImpl::new(store);

        let code = manager.generate_pairing_code().await.expect("测试：异步操作应成功");
        assert_eq!(code.code.len(), 6);
        assert!(code.code.chars().all(|c| c.is_ascii_digit()));
    }

    #[tokio::test]
    async fn test_list_devices() {
        let store = Arc::new(DeviceStore::new());
        let manager = DeviceManagerImpl::new(store);

        let device1 = DeviceManagerImpl::create_local_device(
            "Device 1".to_string(),
            "host1".to_string(),
            "Windows".to_string(),
            "1.0.0".to_string(),
        );
        manager.register_device(device1).await.expect("测试：异步操作应成功");

        let device2 = DeviceManagerImpl::create_local_device(
            "Device 2".to_string(),
            "host2".to_string(),
            "macOS".to_string(),
            "1.0.0".to_string(),
        );
        manager.register_device(device2).await.expect("测试：异步操作应成功");

        let devices = manager.list_devices().await.expect("测试：异步操作应成功");
        assert_eq!(devices.len(), 2);
    }

    #[tokio::test]
    async fn test_device_store_without_database() {
        let store = DeviceStore::new();
        assert!(!store.uses_database());
    }

    #[tokio::test]
    async fn test_device_store_default() {
        let store = DeviceStore::default();
        assert!(!store.uses_database());
    }
}
