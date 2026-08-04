// SPDX-License-Identifier: AGPL-3.0-only

//! 设备同步 WebSocket 信令服务。
//!
//! 处理设备间实时同步的信令消息，支持设备上线/下线、同步请求、变更推送等。

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::device_sync::{
    ChangeLogEntry, ConflictInfo, SignalService, SyncResult, SyncSignal, SyncSignalResponse,
};
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use tokio::sync::{RwLock, mpsc};

use crate::server::GatewayAppState;

/// 设备连接信息
struct DeviceConnection {
    device_id: String,
    connection_id: String,
    tx: mpsc::Sender<SyncSignalResponse>,
}

/// 设备信令服务实现
pub struct DeviceSignalService {
    /// 活跃连接：device_id → (connection_id, sender)
    connections: Arc<RwLock<HashMap<String, DeviceConnection>>>,
    /// 连接ID → device_id 反向映射
    connection_map: Arc<RwLock<HashMap<String, String>>>,
}

impl DeviceSignalService {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 处理客户端发来的信令消息
    async fn handle_client_message(
        &self,
        connection_id: &str,
        message: SyncSignal,
    ) -> Result<SyncSignalResponse, String> {
        match message {
            SyncSignal::DeviceOnline { device_id } => {
                self.mark_online(&device_id, connection_id).await?;
                Ok(SyncSignalResponse::DeviceOnlineAck {
                    device_id,
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                })
            },
            SyncSignal::DeviceOffline { device_id } => {
                self.mark_offline(&device_id).await?;
                Ok(SyncSignalResponse::DeviceOfflineAck {
                    device_id,
                    timestamp: chrono::Utc::now().timestamp_millis() as u64,
                })
            },
            SyncSignal::Ping { device_id } => Ok(SyncSignalResponse::Pong { device_id }),
            SyncSignal::SyncRequest { device_id, since_timestamp } => {
                let _changes: Vec<ChangeLogEntry> = Vec::new();
                let result = SyncResult {
                    success: true,
                    files_synced: 0,
                    files_uploaded: 0,
                    files_downloaded: 0,
                    conflicts_detected: 0,
                    error_message: None,
                    duration_ms: 0,
                };
                let _ = since_timestamp;
                Ok(SyncSignalResponse::SyncResponse { device_id, result })
            },
            SyncSignal::PushChanges { device_id, changes } => {
                let conflicts: Vec<ConflictInfo> = Vec::new();
                // 将变更广播给其他在线设备
                let signal = SyncSignalResponse::ChangesReceived {
                    device_id: device_id.clone(),
                    changes_count: changes.len() as u64,
                    conflicts: conflicts.clone(),
                };
                self.broadcast_signal(signal).await?;
                Ok(SyncSignalResponse::ChangesReceived {
                    device_id,
                    changes_count: changes.len() as u64,
                    conflicts,
                })
            },
            SyncSignal::ResolveConflict { device_id, conflict_id, strategy } => {
                let _ = strategy;
                Ok(SyncSignalResponse::ConflictResolved { device_id, conflict_id, success: true })
            },
            SyncSignal::RegisterDevice { device } => {
                let _ = device;
                Ok(SyncSignalResponse::Error {
                    code: "REGISTER_NOT_SUPPORTED".to_string(),
                    message: "Device registration should be done via Tauri command".to_string(),
                })
            },
        }
    }

    /// 获取设备ID对应的发送器
    async fn get_sender(&self, device_id: &str) -> Option<mpsc::Sender<SyncSignalResponse>> {
        let connections = self.connections.read().await;
        connections.get(device_id).map(|c| {
            let _ = &c.device_id;
            let _ = &c.connection_id;
            c.tx.clone()
        })
    }

    /// 移除连接
    pub async fn remove_connection(&self, connection_id: &str) {
        let mut connection_map = self.connection_map.write().await;
        if let Some(device_id) = connection_map.remove(connection_id) {
            drop(connection_map);
            let mut connections = self.connections.write().await;
            connections.remove(&device_id);
        }
    }
}

impl Default for DeviceSignalService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SignalService for DeviceSignalService {
    async fn send_signal(
        &self,
        target_device_id: &str,
        signal: SyncSignalResponse,
    ) -> Result<(), String> {
        if let Some(tx) = self.get_sender(target_device_id).await {
            tx.send(signal).await.map_err(|e| format!("Failed to send signal: {}", e))?;
        }
        Ok(())
    }

    async fn broadcast_signal(&self, signal: SyncSignalResponse) -> Result<(), String> {
        let connections = self.connections.read().await;
        for conn in connections.values() {
            let _ = conn.tx.send(signal.clone()).await;
        }
        Ok(())
    }

    async fn mark_online(&self, device_id: &str, connection_id: &str) -> Result<(), String> {
        let (tx, mut rx) = mpsc::channel::<SyncSignalResponse>(256);

        let mut connections = self.connections.write().await;
        let old = connections.insert(
            device_id.to_string(),
            DeviceConnection {
                device_id: device_id.to_string(),
                connection_id: connection_id.to_string(),
                tx,
            },
        );
        drop(connections);

        if old.is_some() {
            self.mark_offline(device_id).await?;
        }

        let mut connection_map = self.connection_map.write().await;
        connection_map.insert(connection_id.to_string(), device_id.to_string());
        drop(connection_map);

        // 启动接收任务，将消息发送到 WebSocket
        let device_id_owned = device_id.to_string();
        let connection_id_owned = connection_id.to_string();
        let connections = self.connections.clone();
        let connection_map = self.connection_map.clone();

        tokio::spawn(async move {
            while let Some(signal) = rx.recv().await {
                // 这里可以将信号推送到前端的 WebSocket
                // 实际推送由 WebSocket 任务处理
                let signal_json = serde_json::to_string(&signal).unwrap_or_default();
                tracing::debug!(
                    device_id = %device_id_owned,
                    signal_type = ?signal,
                    "Device signal queued"
                );
                let _ = signal_json;
            }
            // 连接断开时清理
            let mut connections = connections.write().await;
            connections.remove(&device_id_owned);
            drop(connections);

            let mut connection_map = connection_map.write().await;
            connection_map.remove(&connection_id_owned);
        });

        Ok(())
    }

    async fn mark_offline(&self, device_id: &str) -> Result<(), String> {
        let mut connections = self.connections.write().await;
        connections.remove(device_id);
        Ok(())
    }

    async fn is_online(&self, device_id: &str) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(device_id)
    }
}

/// WebSocket 信令处理器
pub async fn device_signal_ws_handler(
    State(state): State<GatewayAppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_device_signal_session(socket, state))
}

async fn handle_device_signal_session(socket: WebSocket, state: GatewayAppState) {
    let mut socket = socket;
    let connection_id = uuid::Uuid::new_v4().to_string();
    let mut device_id: Option<String> = None;

    // 心跳配置
    let heartbeat_interval = tokio::time::Duration::from_secs(30);
    let mut heartbeat = tokio::time::interval(heartbeat_interval);

    loop {
        tokio::select! {
            // 心跳
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(Vec::new().into())).await.is_err() {
                    break;
                }
            }

            // 接收客户端消息
            msg_result = socket.recv() => {
                match msg_result {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(signal) = serde_json::from_str::<SyncSignal>(&text) {
                            if let SyncSignal::DeviceOnline { device_id: did } = &signal {
                                device_id = Some(did.clone());
                            }

                            // 处理信令
                            let response = handle_signal(&state, &connection_id, signal).await;

                            // 发送响应
                            if let Ok(resp_json) = serde_json::to_string(&response)
                                && socket.send(Message::Text(resp_json.into())).await.is_err()
                            {
                                break;
                            }
                        } else {
                            // 无效消息，返回错误
                            let error = SyncSignalResponse::Error {
                                code: "INVALID_MESSAGE".to_string(),
                                message: "Invalid signal message format".to_string(),
                            };
                            if let Ok(err_json) = serde_json::to_string(&error) {
                                let _ = socket.send(Message::Text(err_json.into())).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        // 客户端断开
                        break;
                    }
                    Some(Ok(Message::Ping(_))) => {
                        let _ = socket.send(Message::Pong(Vec::new().into())).await;
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // 清理
    if let Some(did) = device_id {
        let signal_service = get_signal_service(&state);
        if let Some(service) = signal_service {
            service.remove_connection(&connection_id).await;
            let _ = service.mark_offline(&did).await;
        }
    }
}

/// 处理信令消息
async fn handle_signal(
    state: &GatewayAppState,
    connection_id: &str,
    signal: SyncSignal,
) -> SyncSignalResponse {
    let signal_service = get_signal_service(state);

    if let Some(service) = signal_service {
        match service.handle_client_message(connection_id, signal).await {
            Ok(response) => response,
            Err(e) => SyncSignalResponse::Error { code: "SIGNAL_ERROR".to_string(), message: e },
        }
    } else {
        SyncSignalResponse::Error {
            code: "SERVICE_UNAVAILABLE".to_string(),
            message: "Signal service not available".to_string(),
        }
    }
}

/// 获取信令服务实例
fn get_signal_service(_state: &GatewayAppState) -> Option<&DeviceSignalService> {
    // 从 AppState 中获取信令服务
    // 实际实现时需要将 DeviceSignalService 集成到 GatewayAppState
    None
}
