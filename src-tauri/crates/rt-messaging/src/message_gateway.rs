// SPDX-License-Identifier: AGPL-3.0-only

//! Unified Message Gateway for cross-platform agent communication
//!
//! Features:
//! - Pluggable transport adapters (WebSocket, SSE, HTTP, stdio)
//! - Protocol negotiation (MCP, A2A, Custom)
//! - Message routing and queuing
//! - Connection state management
//! - Heartbeat and keepalive

pub mod media_types;
pub mod platform_bridge;
pub mod platform_config;
pub mod platform_manager;
pub mod platforms;
pub mod session_router;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

/// P1-10: 消息队列上限（背压控制）。超出后入队时丢弃最老消息。
pub const MAX_QUEUE_SIZE: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub payload: MessagePayload,
    pub timestamp: u128,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
}

impl AgentMessage {
    pub fn new(from: &str, to: &str, payload: MessagePayload) -> Self {
        Self {
            id: uuid_v4(),
            from: from.to_string(),
            to: to.to_string(),
            payload,
            timestamp: now_ms(),
            correlation_id: None,
            reply_to: None,
        }
    }

    pub fn with_correlation(mut self, correlation_id: &str) -> Self {
        self.correlation_id = Some(correlation_id.to_string());
        self
    }

    pub fn with_reply_to(mut self, reply_to: &str) -> Self {
        self.reply_to = Some(reply_to.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum MessagePayload {
    Text {
        content: String,
    },
    Json {
        schema: String,
        body: serde_json::Value,
    },
    Binary {
        mime: String,
        data: Vec<u8>,
    },
    Command {
        name: String,
        args: HashMap<String, String>,
    },
    Event {
        name: String,
        params: serde_json::Value,
    },
    Response {
        status: u16,
        body: String,
    },
    Error {
        code: String,
        message: String,
    },
    /// Blackboard 状态同步消息
    BlackboardSync {
        task_id: String,
        shared_state: std::collections::HashMap<String, String>,
        from_agent: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum Protocol {
    #[default]
    Mcp,
    A2A,
    Custom {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum TransportType {
    #[default]
    WebSocket,
    SSE,
    HTTP,
    Stdio,
    IPC,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

#[derive(Debug, Clone, Default)]
pub struct AgentEndpoint {
    pub agent_id: String,
    pub url: String,
    pub transport: TransportType,
    pub protocol: Protocol,
    pub capabilities: Vec<String>,
    pub state: ConnectionState,
    pub last_seen: u128,
}

#[derive(Debug, Clone, Default)]
pub struct GatewayState {
    pub endpoints: HashMap<String, AgentEndpoint>,
    pub message_queue: Vec<AgentMessage>,
    pub routing_table: HashMap<String, String>,
}

#[derive(Clone)]
pub struct MessageGateway {
    state: Arc<tokio::sync::RwLock<GatewayState>>,
    transport_handlers: HashMap<TransportType, Arc<dyn TransportHandler>>,
}

#[async_trait]
pub trait TransportHandler: Send + Sync {
    fn transport_type(&self) -> TransportType;
    async fn connect(&self, endpoint: &AgentEndpoint) -> Result<(), GatewayError>;
    async fn disconnect(&self, endpoint_id: &str) -> Result<(), GatewayError>;
    async fn send(&self, endpoint_id: &str, message: &AgentMessage) -> Result<(), GatewayError>;
    async fn broadcast(
        &self,
        agent_ids: &[String],
        message: &AgentMessage,
    ) -> Result<(), GatewayError>;
    fn get_state(&self, endpoint_id: &str) -> ConnectionState;

    async fn send_media(
        &self,
        _endpoint_id: &str,
        _attachment: &media_types::MediaAttachment,
    ) -> Result<(), GatewayError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum GatewayError {
    TransportError { reason: String },
    ProtocolError { reason: String },
    NotFound { entity: String },
    ConnectionFailed { endpoint: String, reason: String },
    SerializationError { reason: String },
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransportError { reason } => write!(f, "Transport error: {}", reason),
            Self::ProtocolError { reason } => write!(f, "Protocol error: {}", reason),
            Self::NotFound { entity } => write!(f, "Not found: {}", entity),
            Self::ConnectionFailed { endpoint, reason } => {
                write!(f, "Connection failed to {}: {}", endpoint, reason)
            },
            Self::SerializationError { reason } => write!(f, "Serialization error: {}", reason),
        }
    }
}

impl std::error::Error for GatewayError {}

impl MessageGateway {
    pub fn new() -> Self {
        Self {
            state: Arc::new(tokio::sync::RwLock::new(GatewayState::default())),
            transport_handlers: HashMap::new(),
        }
    }

    pub fn register_transport<H: TransportHandler + 'static>(&mut self, handler: H) {
        self.transport_handlers.insert(handler.transport_type(), Arc::new(handler));
    }

    /// 注册 endpoint：先调 transport 的 connect（async），拿到结果后再持写锁写 state。
    /// 这样写锁不会跨 await，符合 async 锁的预期用法。
    pub async fn register_endpoint(&self, endpoint: AgentEndpoint) -> Result<(), GatewayError> {
        if let Some(handler) = self.transport_handlers.get(&endpoint.transport) {
            handler.connect(&endpoint).await?;
        }

        let mut state = self.state.write().await;
        let agent_id = endpoint.agent_id.clone();
        let url = endpoint.url.clone();
        state.endpoints.insert(agent_id.clone(), endpoint);
        state.routing_table.insert(agent_id, url);
        Ok(())
    }

    /// 注销 endpoint：先在持写锁前取出 endpoint（仅读锁），调用 disconnect 后再写。
    pub async fn unregister_endpoint(&self, agent_id: &str) -> Result<AgentEndpoint, GatewayError> {
        let endpoint = {
            let state = self.state.read().await;
            state.endpoints.get(agent_id).cloned().ok_or_else(|| GatewayError::NotFound {
                entity: format!("endpoint {}", agent_id),
            })?
        };

        if let Some(handler) = self.transport_handlers.get(&endpoint.transport) {
            handler.disconnect(agent_id).await?;
        }

        let mut state = self.state.write().await;
        state.endpoints.remove(agent_id);
        state.routing_table.remove(agent_id);
        Ok(endpoint)
    }

    /// 发送：先 read 拿到 endpoint/handler 引用（read 锁内不 await），释放后调 handler.send。
    pub async fn send_message(&self, message: &AgentMessage) -> Result<(), GatewayError> {
        let (handler, target) = {
            let state = self.state.read().await;
            let endpoint = state
                .endpoints
                .get(&message.to)
                .ok_or_else(|| GatewayError::NotFound {
                    entity: format!("endpoint {}", message.to),
                })?
                .clone();
            let handler = self
                .transport_handlers
                .get(&endpoint.transport)
                .ok_or_else(|| GatewayError::TransportError {
                    reason: format!("No handler for transport {:?}", endpoint.transport),
                })?
                .clone();
            (handler, endpoint)
        };

        handler.send(&target.agent_id, message).await
    }

    /// 广播：先 read 收集所有 (agent_id, handler_clone) 然后释放锁，循环 await。
    pub async fn broadcast(
        &self,
        agent_ids: &[String],
        message: &AgentMessage,
    ) -> Result<(), GatewayError> {
        // 把每次 send 需要的 handler 与 agent_id 提前取出
        let targets: Vec<(String, Arc<dyn TransportHandler>)> = {
            let state = self.state.read().await;
            agent_ids
                .iter()
                .filter_map(|aid| {
                    let endpoint = state.endpoints.get(aid)?;
                    let handler = self.transport_handlers.get(&endpoint.transport)?.clone();
                    Some((aid.clone(), handler))
                })
                .collect()
        };

        for (agent_id, handler) in &targets {
            handler.send(agent_id, message).await?;
        }
        Ok(())
    }

    pub fn route_message(&self, message: &AgentMessage) -> Result<String, GatewayError> {
        // 同步接口不能 await：先尝试 read 锁（非阻塞），失败则返回错误。
        // 调用方应使用 `try_route_message` 的 async 替代品。
        let state = self.state.try_read().map_err(|_| GatewayError::TransportError {
            reason: "state lock contended; use async try_route_message".to_string(),
        })?;
        state
            .routing_table
            .get(&message.to)
            .cloned()
            .ok_or_else(|| GatewayError::NotFound { entity: format!("route for {}", message.to) })
    }

    pub async fn queue_message(&self, message: AgentMessage) -> Result<(), GatewayError> {
        // 入队时若超过 MAX_QUEUE_SIZE 则丢弃最老元素（背压控制）
        let mut state = self.state.write().await;
        if state.message_queue.len() >= MAX_QUEUE_SIZE {
            state.message_queue.remove(0);
            tracing::warn!(
                queue_size = state.message_queue.len(),
                "message_queue 已达上限，丢弃最老元素"
            );
        }
        state.message_queue.push(message);
        Ok(())
    }

    pub async fn flush_queue(&self, agent_id: &str) -> Result<Vec<AgentMessage>, GatewayError> {
        let mut state = self.state.write().await;
        let pending: Vec<AgentMessage> =
            state.message_queue.iter().filter(|m| m.to == agent_id).cloned().collect();
        state.message_queue.retain(|m| m.to != agent_id);
        Ok(pending)
    }

    pub async fn get_endpoint(&self, agent_id: &str) -> Result<AgentEndpoint, GatewayError> {
        let state = self.state.read().await;
        state
            .endpoints
            .get(agent_id)
            .cloned()
            .ok_or_else(|| GatewayError::NotFound { entity: format!("endpoint {}", agent_id) })
    }

    pub async fn list_endpoints(&self) -> Result<Vec<AgentEndpoint>, GatewayError> {
        let state = self.state.read().await;
        Ok(state.endpoints.values().cloned().collect())
    }

    pub async fn update_heartbeat(&self, agent_id: &str) -> Result<(), GatewayError> {
        let mut state = self.state.write().await;
        let endpoint = state
            .endpoints
            .get_mut(agent_id)
            .ok_or_else(|| GatewayError::NotFound { entity: format!("endpoint {}", agent_id) })?;
        endpoint.last_seen = now_ms();
        Ok(())
    }

    pub async fn get_stale_endpoints(&self, threshold_ms: u128) -> Vec<String> {
        let state = self.state.read().await;
        let now = now_ms();
        state
            .endpoints
            .iter()
            .filter(|(_, e)| now - e.last_seen > threshold_ms)
            .map(|(id, _)| id.clone())
            .collect()
    }
}

fn uuid_v4() -> String {
    // P1-10: 使用真正的 uuid v4（密码学随机），不再用时间戳拼凑
    uuid::Uuid::new_v4().to_string()
}

impl Default for MessageGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = AgentMessage::new(
            "agent_a",
            "agent_b",
            MessagePayload::Text { content: "Hello".to_string() },
        );

        assert_eq!(msg.from, "agent_a");
        assert_eq!(msg.to, "agent_b");
        assert!(msg.correlation_id.is_none());
    }

    #[tokio::test]
    async fn test_endpoint_registration() {
        let gateway = MessageGateway::new();
        let endpoint = AgentEndpoint {
            agent_id: "test_agent".to_string(),
            url: "ws://localhost:8080".to_string(),
            transport: TransportType::WebSocket,
            protocol: Protocol::A2A,
            capabilities: vec!["chat".to_string()],
            state: ConnectionState::Disconnected,
            last_seen: now_ms(),
        };

        gateway.register_endpoint(endpoint).await.unwrap();
        let retrieved = gateway.get_endpoint("test_agent").await.unwrap();
        assert_eq!(retrieved.agent_id, "test_agent");
    }

    #[tokio::test]
    async fn test_message_queue() {
        let gateway = MessageGateway::new();
        let msg = AgentMessage::new("a", "b", MessagePayload::Text { content: "test".to_string() });

        gateway.queue_message(msg).await.unwrap();
        let pending = gateway.flush_queue("b").await.unwrap();
        assert_eq!(pending.len(), 1);
    }
}
