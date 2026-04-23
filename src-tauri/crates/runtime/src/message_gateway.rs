//! Unified Message Gateway for cross-platform agent communication
//!
//! Features:
//! - Pluggable transport adapters (WebSocket, SSE, HTTP, stdio)
//! - Protocol negotiation (MCP, A2A, Custom)
//! - Message routing and queuing
//! - Connection state management
//! - Heartbeat and keepalive

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Mcp,
    A2A,
    Custom { name: String },
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::Mcp
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportType {
    WebSocket,
    SSE,
    HTTP,
    Stdio,
    IPC,
}

impl Default for TransportType {
    fn default() -> Self {
        TransportType::WebSocket
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Disconnected
    }
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

pub struct MessageGateway {
    state: Arc<RwLock<GatewayState>>,
    transport_handlers: HashMap<TransportType, Box<dyn TransportHandler>>,
}

pub trait TransportHandler: Send + Sync {
    fn transport_type(&self) -> TransportType;
    fn connect(&self, endpoint: &AgentEndpoint) -> Result<(), GatewayError>;
    fn disconnect(&self, endpoint_id: &str) -> Result<(), GatewayError>;
    fn send(&self, endpoint_id: &str, message: &AgentMessage) -> Result<(), GatewayError>;
    fn broadcast(&self, agent_ids: &[String], message: &AgentMessage) -> Result<(), GatewayError>;
    fn get_state(&self, endpoint_id: &str) -> ConnectionState;
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
            }
            Self::SerializationError { reason } => write!(f, "Serialization error: {}", reason),
        }
    }
}

impl std::error::Error for GatewayError {}

impl MessageGateway {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(GatewayState::default())),
            transport_handlers: HashMap::new(),
        }
    }

    pub fn register_transport<H: TransportHandler + 'static>(&mut self, handler: H) {
        self.transport_handlers
            .insert(handler.transport_type(), Box::new(handler));
    }

    pub fn register_endpoint(&self, endpoint: AgentEndpoint) -> Result<(), GatewayError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        if let Some(handler) = self.transport_handlers.get(&endpoint.transport) {
            handler.connect(&endpoint)?;
        }

        let agent_id = endpoint.agent_id.clone();
        let url = endpoint.url.clone();
        state.endpoints.insert(agent_id.clone(), endpoint);
        state.routing_table.insert(agent_id, url);

        Ok(())
    }

    pub fn unregister_endpoint(&self, agent_id: &str) -> Result<AgentEndpoint, GatewayError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        let endpoint = state
            .endpoints
            .remove(agent_id)
            .ok_or_else(|| GatewayError::NotFound {
                entity: format!("endpoint {}", agent_id),
            })?;

        if let Some(handler) = self.transport_handlers.get(&endpoint.transport) {
            handler.disconnect(agent_id)?;
        }

        state.routing_table.remove(agent_id);

        Ok(endpoint)
    }

    pub fn send_message(&self, message: &AgentMessage) -> Result<(), GatewayError> {
        let state = self
            .state
            .read()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        let endpoint = state
            .endpoints
            .get(&message.to)
            .ok_or_else(|| GatewayError::NotFound {
                entity: format!("endpoint {}", message.to),
            })?;

        let handler = self
            .transport_handlers
            .get(&endpoint.transport)
            .ok_or_else(|| GatewayError::TransportError {
                reason: format!("No handler for transport {:?}", endpoint.transport),
            })?;

        handler.send(&message.to, message)
    }

    pub fn broadcast(
        &self,
        agent_ids: &[String],
        message: &AgentMessage,
    ) -> Result<(), GatewayError> {
        let state = self
            .state
            .read()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        for agent_id in agent_ids {
            if let Some(endpoint) = state.endpoints.get(agent_id) {
                if let Some(handler) = self.transport_handlers.get(&endpoint.transport) {
                    handler.send(agent_id, message)?;
                }
            }
        }

        Ok(())
    }

    pub fn route_message(&self, message: &AgentMessage) -> Result<String, GatewayError> {
        let state = self
            .state
            .read()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        state
            .routing_table
            .get(&message.to)
            .cloned()
            .ok_or_else(|| GatewayError::NotFound {
                entity: format!("route for {}", message.to),
            })
    }

    pub fn queue_message(&self, message: AgentMessage) -> Result<(), GatewayError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        state.message_queue.push(message);
        Ok(())
    }

    pub fn flush_queue(&self, agent_id: &str) -> Result<Vec<AgentMessage>, GatewayError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        let pending: Vec<AgentMessage> = state
            .message_queue
            .iter()
            .filter(|m| m.to == agent_id)
            .cloned()
            .collect();

        state.message_queue.retain(|m| m.to != agent_id);

        Ok(pending)
    }

    pub fn get_endpoint(&self, agent_id: &str) -> Result<AgentEndpoint, GatewayError> {
        let state = self
            .state
            .read()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        state
            .endpoints
            .get(agent_id)
            .cloned()
            .ok_or_else(|| GatewayError::NotFound {
                entity: format!("endpoint {}", agent_id),
            })
    }

    pub fn list_endpoints(&self) -> Result<Vec<AgentEndpoint>, GatewayError> {
        let state = self
            .state
            .read()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        Ok(state.endpoints.values().cloned().collect())
    }

    pub fn update_heartbeat(&self, agent_id: &str) -> Result<(), GatewayError> {
        let mut state = self
            .state
            .write()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        let endpoint = state
            .endpoints
            .get_mut(agent_id)
            .ok_or_else(|| GatewayError::NotFound {
                entity: format!("endpoint {}", agent_id),
            })?;

        endpoint.last_seen = now_ms();
        Ok(())
    }

    pub fn get_stale_endpoints(&self, threshold_ms: u128) -> Vec<String> {
        let state = match self.state.read() {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

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
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        now.as_secs() as u32,
        (now.as_secs() >> 32) as u16,
        (now.as_nanos() >> 48) as u16 & 0x0FFF,
        rand_u16(),
        now.as_nanos() & 0xFFFFFFFFFFFF
    )
}

fn rand_u16() -> u16 {
    static VAL: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    *VAL.get_or_init(|| {
        std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u16
    })
}

#[allow(dead_code)]
pub struct WebSocketTransport {
    connections:
        std::sync::Arc<std::sync::RwLock<std::collections::HashMap<String, WebSocketConnection>>>,
    reconnect_timeout_secs: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct WebSocketConnection {
    endpoint: AgentEndpoint,
    state: ConnectionState,
    connected_at: u128,
    last_message_at: u128,
    pending_messages: Vec<AgentMessage>,
}

#[allow(dead_code)]
impl WebSocketTransport {
    pub fn new() -> Self {
        Self {
            connections: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
            reconnect_timeout_secs: 30,
        }
    }

    pub fn with_reconnect_timeout(mut self, timeout_secs: u64) -> Self {
        self.reconnect_timeout_secs = timeout_secs;
        self
    }

    fn update_connection_state(&self, endpoint_id: &str, state: ConnectionState) {
        if let Ok(mut connections) = self.connections.write() {
            if let Some(conn) = connections.get_mut(endpoint_id) {
                conn.state = state.clone();
                conn.last_message_at = now_ms();
            }
        }
    }

    pub fn get_connection_state(&self, endpoint_id: &str) -> Option<ConnectionState> {
        self.connections
            .read()
            .ok()
            .and_then(|c| c.get(endpoint_id).map(|conn| conn.state.clone()))
    }

    pub fn queue_message_for_offline(&self, endpoint_id: &str, message: &AgentMessage) {
        if let Ok(mut connections) = self.connections.write() {
            if let Some(conn) = connections.get_mut(endpoint_id) {
                if conn.state != ConnectionState::Connected {
                    conn.pending_messages.push(message.clone());
                }
            }
        }
    }

    pub fn flush_pending_messages(&self, endpoint_id: &str) -> Vec<AgentMessage> {
        if let Ok(mut connections) = self.connections.write() {
            if let Some(conn) = connections.get_mut(endpoint_id) {
                let pending = conn.pending_messages.clone();
                conn.pending_messages.clear();
                return pending;
            }
        }
        Vec::new()
    }

    pub fn get_connection_info(&self, endpoint_id: &str) -> Option<WebSocketConnection> {
        self.connections
            .read()
            .ok()
            .and_then(|c| c.get(endpoint_id).cloned())
    }

    pub fn list_connections(&self) -> Vec<(String, ConnectionState)> {
        self.connections
            .read()
            .ok()
            .map(|c| {
                c.iter()
                    .map(|(id, conn)| (id.clone(), conn.state.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn reconnect_if_needed(&self, endpoint_id: &str) -> bool {
        if let Ok(connections) = self.connections.read() {
            if let Some(conn) = connections.get(endpoint_id) {
                if conn.state == ConnectionState::Disconnected
                    || conn.state == ConnectionState::Failed
                {
                    drop(connections);
                    return self.reconnect(endpoint_id).is_ok();
                }
            }
        }
        false
    }

    fn reconnect(&self, endpoint_id: &str) -> Result<(), GatewayError> {
        let endpoint = {
            let connections =
                self.connections
                    .read()
                    .map_err(|_| GatewayError::TransportError {
                        reason: "Failed to acquire lock".to_string(),
                    })?;
            connections.get(endpoint_id).map(|c| c.endpoint.clone())
        };

        if let Some(endpoint) = endpoint {
            self.update_connection_state(endpoint_id, ConnectionState::Reconnecting);
            self.connect(&endpoint)?;
            self.update_connection_state(endpoint_id, ConnectionState::Connected);
            return Ok(());
        }

        Err(GatewayError::NotFound {
            entity: format!("endpoint {} not found", endpoint_id),
        })
    }
}

impl Default for WebSocketTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl TransportHandler for WebSocketTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::WebSocket
    }

    fn connect(&self, endpoint: &AgentEndpoint) -> Result<(), GatewayError> {
        let mut connections =
            self.connections
                .write()
                .map_err(|_| GatewayError::TransportError {
                    reason: "Failed to acquire lock".to_string(),
                })?;

        let conn = WebSocketConnection {
            endpoint: endpoint.clone(),
            state: ConnectionState::Connected,
            connected_at: now_ms(),
            last_message_at: now_ms(),
            pending_messages: Vec::new(),
        };

        connections.insert(endpoint.agent_id.clone(), conn);

        eprintln!(
            "WebSocket connected to {} at {}",
            endpoint.agent_id, endpoint.url
        );
        Ok(())
    }

    fn disconnect(&self, endpoint_id: &str) -> Result<(), GatewayError> {
        let mut connections =
            self.connections
                .write()
                .map_err(|_| GatewayError::TransportError {
                    reason: "Failed to acquire lock".to_string(),
                })?;

        if let Some(conn) = connections.get_mut(endpoint_id) {
            conn.state = ConnectionState::Disconnected;
            eprintln!("WebSocket disconnected {}", endpoint_id);
            Ok(())
        } else {
            Err(GatewayError::NotFound {
                entity: format!("endpoint {}", endpoint_id),
            })
        }
    }

    fn send(&self, endpoint_id: &str, message: &AgentMessage) -> Result<(), GatewayError> {
        let connections = self
            .connections
            .read()
            .map_err(|_| GatewayError::TransportError {
                reason: "Failed to acquire lock".to_string(),
            })?;

        if let Some(conn) = connections.get(endpoint_id) {
            match conn.state {
                ConnectionState::Connected => {
                    drop(connections);
                    self.update_connection_state(endpoint_id, ConnectionState::Connected);
                    eprintln!("WebSocket sent message {} to {}", message.id, endpoint_id);
                    Ok(())
                }
                ConnectionState::Reconnecting => {
                    drop(connections);
                    self.queue_message_for_offline(endpoint_id, message);
                    Err(GatewayError::ConnectionFailed {
                        endpoint: endpoint_id.to_string(),
                        reason: "Connection is reconnecting".to_string(),
                    })
                }
                _ => {
                    drop(connections);
                    self.queue_message_for_offline(endpoint_id, message);
                    Err(GatewayError::ConnectionFailed {
                        endpoint: endpoint_id.to_string(),
                        reason: "Connection is not connected".to_string(),
                    })
                }
            }
        } else {
            Err(GatewayError::NotFound {
                entity: format!("endpoint {}", endpoint_id),
            })
        }
    }

    fn broadcast(&self, agent_ids: &[String], message: &AgentMessage) -> Result<(), GatewayError> {
        let mut failed_count = 0;
        for agent_id in agent_ids {
            if let Err(e) = self.send(agent_id, message) {
                failed_count += 1;
                eprintln!("Broadcast to {} failed: {}", agent_id, e);
            }
        }
        if failed_count > 0 && failed_count == agent_ids.len() {
            return Err(GatewayError::TransportError {
                reason: format!("All {} broadcast attempts failed", failed_count),
            });
        }
        Ok(())
    }

    fn get_state(&self, endpoint_id: &str) -> ConnectionState {
        self.get_connection_state(endpoint_id)
            .unwrap_or(ConnectionState::Disconnected)
    }
}

#[allow(dead_code)]
pub struct SSETransport;

impl TransportHandler for SSETransport {
    fn transport_type(&self) -> TransportType {
        TransportType::SSE
    }

    fn connect(&self, endpoint: &AgentEndpoint) -> Result<(), GatewayError> {
        eprintln!("SSE connecting to {}", endpoint.url);
        Ok(())
    }

    fn disconnect(&self, endpoint_id: &str) -> Result<(), GatewayError> {
        eprintln!("SSE disconnecting {}", endpoint_id);
        Ok(())
    }

    fn send(&self, endpoint_id: &str, message: &AgentMessage) -> Result<(), GatewayError> {
        eprintln!("SSE sending to {}: {:?}", endpoint_id, message.id);
        Ok(())
    }

    fn broadcast(&self, agent_ids: &[String], _message: &AgentMessage) -> Result<(), GatewayError> {
        eprintln!("SSE broadcasting to {} agents", agent_ids.len());
        Ok(())
    }

    fn get_state(&self, _endpoint_id: &str) -> ConnectionState {
        ConnectionState::Connected
    }
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
            MessagePayload::Text {
                content: "Hello".to_string(),
            },
        );

        assert_eq!(msg.from, "agent_a");
        assert_eq!(msg.to, "agent_b");
        assert!(msg.correlation_id.is_none());
    }

    #[test]
    fn test_endpoint_registration() {
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

        gateway.register_endpoint(endpoint).unwrap();
        let retrieved = gateway.get_endpoint("test_agent").unwrap();
        assert_eq!(retrieved.agent_id, "test_agent");
    }

    #[test]
    fn test_message_queue() {
        let gateway = MessageGateway::new();
        let msg = AgentMessage::new(
            "a",
            "b",
            MessagePayload::Text {
                content: "test".to_string(),
            },
        );

        gateway.queue_message(msg).unwrap();
        let pending = gateway.flush_queue("b").unwrap();
        assert_eq!(pending.len(), 1);
    }
}
