use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::message_gateway::{AgentEndpoint, AgentMessage, ConnectionState, GatewayError, TransportHandler, TransportType};

const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_WS_PING_INTERVAL: Duration = Duration::from_secs(25);
const DEFAULT_SSE_RECONNECT_DELAY: Duration = Duration::from_secs(2);

#[derive(Clone)]
pub struct WebSocketTransportConfig {
    pub ping_interval: Duration,
    pub pong_timeout: Duration,
    pub connect_timeout: Duration,
    pub max_reconnect_attempts: u32,
}

impl Default for WebSocketTransportConfig {
    fn default() -> Self {
        Self {
            ping_interval: DEFAULT_WS_PING_INTERVAL,
            pong_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(10),
            max_reconnect_attempts: 5,
        }
    }
}

pub struct WebSocketTransportHandler {
    config: WebSocketTransportConfig,
    connections: Arc<RwLock<HashMap<String, mpsc::Sender<AgentMessage>>>>,
    connection_states: Arc<RwLock<HashMap<String, ConnectionState>>>,
}

impl WebSocketTransportHandler {
    pub fn new() -> Self {
        Self {
            config: WebSocketTransportConfig::default(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_config(mut self, config: WebSocketTransportConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn connect_internal(
        &self,
        endpoint: &AgentEndpoint,
    ) -> Result<(), GatewayError> {
        let url = if endpoint.url.starts_with("ws://") || endpoint.url.starts_with("wss://") {
            endpoint.url.clone()
        } else {
            format!("wss://{}", endpoint.url)
        };

        let (ws_stream, _) = tokio::time::timeout(
            self.config.connect_timeout,
            connect_async(&url),
        )
        .await
        .map_err(|_| GatewayError::ConnectionFailed {
            endpoint: endpoint.agent_id.clone(),
            reason: "Connection timeout".to_string(),
        })?
        .map_err(|e| GatewayError::ConnectionFailed {
            endpoint: endpoint.agent_id.clone(),
            reason: e.to_string(),
        })?;

        let (mut write, read) = ws_stream.split();
        let agent_id = endpoint.agent_id.clone();
        let connection_states = self.connection_states.clone();
        let connections = self.connections.clone();
        let ping_interval = self.config.ping_interval;
        let pong_timeout = self.config.pong_timeout;

        let (tx, mut rx) = mpsc::channel::<AgentMessage>(100);
        {
            let mut conns = connections.write().await;
            conns.insert(agent_id.clone(), tx);
        }
        {
            let mut states = connection_states.write().await;
            states.insert(agent_id.clone(), ConnectionState::Connected);
        }

        tokio::spawn(async move {
            let mut last_pong = Instant::now();
            let mut ping_interval = interval(ping_interval);

            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if let Err(e) = write.send(Message::Text(json.into())).await {
                                tracing::error!("WebSocket send error for {}: {}", agent_id, e);
                                break;
                            }
                        }
                    }
                    _ = ping_interval.tick() => {
                        if let Err(e) = write.send(Message::Ping(Vec::new().into())).await {
                            tracing::warn!("WebSocket ping failed for {}: {}", agent_id, e);
                            break;
                        }
                        last_pong = Instant::now();
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Pong(_))) => {
                                last_pong = Instant::now();
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                tracing::info!("WebSocket closed for {}", agent_id);
                                break;
                            }
                            Some(Err(e)) => {
                                tracing::error!("WebSocket read error for {}: {}", agent_id, e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }

                if last_pong.elapsed() > pong_timeout {
                    tracing::warn!("WebSocket pong timeout for {}", agent_id);
                    break;
                }
            }

            let mut states = connection_states.write().await;
            states.insert(agent_id.clone(), ConnectionState::Disconnected);
            let mut conns = connections.write().await;
            conns.remove(&agent_id);
            tracing::debug!("WebSocket connection cleaned up for {}", agent_id);
        });

        Ok(())
    }

    pub async fn start_heartbeat(&self, agent_id: String) {
        let connection_states = self.connection_states.clone();
        let ping_interval = self.config.ping_interval;
        let connections = self.connections.clone();

        tokio::spawn(async move {
            let mut ping_timer = interval(ping_interval);
            loop {
                ping_timer.tick().await;
                let states = connection_states.read().await;
                if let Some(state) = states.get(&agent_id) {
                    if *state == ConnectionState::Connected {
                        if let Some(tx) = connections.read().await.get(&agent_id) {
                            let ping_msg = AgentMessage::new("heartbeat", &agent_id, crate::message_gateway::MessagePayload::Text {
                                content: "ping".to_string(),
                            });
                            let _ = tx.try_send(ping_msg);
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }
}

impl Default for WebSocketTransportHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportHandler for WebSocketTransportHandler {
    fn transport_type(&self) -> TransportType {
        TransportType::WebSocket
    }

    async fn connect(&self, endpoint: &AgentEndpoint) -> Result<(), GatewayError> {
        self.connect_internal(endpoint).await
    }

    async fn disconnect(&self, endpoint_id: &str) -> Result<(), GatewayError> {
        let mut connections = self.connections.write().await;
        let tx = connections.remove(endpoint_id).ok_or_else(|| GatewayError::NotFound {
            entity: format!("connection {}", endpoint_id),
        })?;
        drop(tx);
        let mut states = self.connection_states.write().await;
        states.remove(endpoint_id);
        Ok(())
    }

    async fn send(&self, endpoint_id: &str, message: &AgentMessage) -> Result<(), GatewayError> {
        let connections = self.connections.read().await;
        let tx = connections.get(endpoint_id).ok_or_else(|| GatewayError::NotFound {
            entity: format!("connection {}", endpoint_id),
        })?;

        let states = self.connection_states.read().await;
        let state = states.get(endpoint_id).ok_or_else(|| GatewayError::NotFound {
            entity: format!("connection state {}", endpoint_id),
        })?;

        if *state != ConnectionState::Connected {
            return Err(GatewayError::ConnectionFailed {
                endpoint: endpoint_id.to_string(),
                reason: "Not connected".to_string(),
            });
        }

        tx.send(message.clone()).await.map_err(|e| GatewayError::TransportError {
            reason: format!("Failed to send message: {}", e),
        })
    }

    async fn broadcast(
        &self,
        agent_ids: &[String],
        message: &AgentMessage,
    ) -> Result<(), GatewayError> {
        for agent_id in agent_ids {
            self.send(agent_id, message).await?;
        }
        Ok(())
    }

    fn get_state(&self, endpoint_id: &str) -> ConnectionState {
        let states = self.connection_states.blocking_read();
        states
            .get(endpoint_id)
            .cloned()
            .unwrap_or(ConnectionState::Disconnected)
    }
}

pub struct HTTPTransportConfig {
    pub timeout: Duration,
    pub max_retries: u32,
    pub retry_delay: Duration,
}

impl Default for HTTPTransportConfig {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_HTTP_TIMEOUT,
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

pub struct HTTPTransportHandler {
    config: HTTPTransportConfig,
    connections: Arc<RwLock<HashMap<String, AgentEndpoint>>>,
    client: reqwest::Client,
}

impl HTTPTransportHandler {
    pub fn new() -> Self {
        Self {
            config: HTTPTransportConfig::default(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_config(mut self, config: HTTPTransportConfig) -> Self {
        self.config = config;
        self
    }

    async fn send_http_request(
        &self,
        endpoint: &AgentEndpoint,
        message: &AgentMessage,
    ) -> Result<(), GatewayError> {
        let url = if endpoint.url.starts_with("http://") || endpoint.url.starts_with("https://") {
            endpoint.url.clone()
        } else {
            format!("https://{}", endpoint.url)
        };

        let body = serde_json::to_string(message)
            .map_err(|e| GatewayError::SerializationError { reason: e.to_string() })?;

        let mut last_error = None;
        for attempt in 0..self.config.max_retries {
            let result = self
                .client
                .post(&url)
                .timeout(self.config.timeout)
                .header("Content-Type", "application/json")
                .body(body.clone())
                .send()
                .await;

            match result {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                Ok(resp) => {
                    last_error = Some(GatewayError::ConnectionFailed {
                        endpoint: endpoint.agent_id.clone(),
                        reason: format!("HTTP error: {}", resp.status()),
                    });
                }
                Err(e) => {
                    last_error = Some(GatewayError::TransportError {
                        reason: e.to_string(),
                    });
                }
            }

            if attempt < self.config.max_retries - 1 {
                tokio::time::sleep(self.config.retry_delay * (attempt + 1)).await;
            }
        }

        Err(last_error.unwrap_or_else(|| GatewayError::TransportError {
            reason: "Max retries exceeded".to_string(),
        }))
    }
}

impl Default for HTTPTransportHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportHandler for HTTPTransportHandler {
    fn transport_type(&self) -> TransportType {
        TransportType::HTTP
    }

    async fn connect(&self, endpoint: &AgentEndpoint) -> Result<(), GatewayError> {
        let connections = self.connections.clone();
        let endpoint = endpoint.clone();

        tokio::spawn(async move {
            let mut conns = connections.write().await;
            conns.insert(endpoint.agent_id.clone(), endpoint);
        });

        Ok(())
    }

    async fn disconnect(&self, endpoint_id: &str) -> Result<(), GatewayError> {
        let mut connections = self.connections.write().await;
        connections
            .remove(endpoint_id)
            .ok_or_else(|| GatewayError::NotFound {
                entity: format!("connection {}", endpoint_id),
            })?;
        Ok(())
    }

    async fn send(&self, endpoint_id: &str, message: &AgentMessage) -> Result<(), GatewayError> {
        let connections = self.connections.read().await;
        let endpoint = connections.get(endpoint_id).ok_or_else(|| GatewayError::NotFound {
            entity: format!("connection {}", endpoint_id),
        })?;

        self.send_http_request(endpoint, message).await
    }

    async fn broadcast(
        &self,
        agent_ids: &[String],
        message: &AgentMessage,
    ) -> Result<(), GatewayError> {
        for agent_id in agent_ids {
            self.send(agent_id, message).await?;
        }
        Ok(())
    }

    fn get_state(&self, endpoint_id: &str) -> ConnectionState {
        let connections = self.connections.blocking_read();
        if connections.contains_key(endpoint_id) {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        }
    }
}

pub struct SSETransportConfig {
    pub reconnect_delay: Duration,
    pub max_reconnect_attempts: u32,
    pub heartbeat_interval: Duration,
}

impl Default for SSETransportConfig {
    fn default() -> Self {
        Self {
            reconnect_delay: DEFAULT_SSE_RECONNECT_DELAY,
            max_reconnect_attempts: 10,
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

pub struct SSETransportHandler {
    config: SSETransportConfig,
    connections: Arc<RwLock<HashMap<String, AgentEndpoint>>>,
    streams: Arc<RwLock<HashMap<String, mpsc::Sender<AgentMessage>>>>,
}

#[allow(dead_code)]
impl SSETransportHandler {
    pub fn new() -> Self {
        Self {
            config: SSETransportConfig::default(),
            connections: Arc::new(RwLock::new(HashMap::new())),
            streams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_config(mut self, config: SSETransportConfig) -> Self {
        self.config = config;
        self
    }

    async fn establish_sse_connection(&self, _endpoint: &AgentEndpoint) -> Result<(), GatewayError> {
        Ok(())
    }

    pub async fn start_sse_client(&self, endpoint: AgentEndpoint) -> Result<(), GatewayError> {
        let agent_id = endpoint.agent_id.clone();
        let streams = self.streams.clone();

        let (tx, mut rx) = mpsc::channel::<AgentMessage>(100);

        {
            let mut stream_guard = streams.write().await;
            stream_guard.insert(agent_id.clone(), tx);
        }

        tokio::spawn(async move {
            while let Some(_message) = rx.recv().await {}
        });

        Ok(())
    }
}

impl Default for SSETransportHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportHandler for SSETransportHandler {
    fn transport_type(&self) -> TransportType {
        TransportType::SSE
    }

    async fn connect(&self, endpoint: &AgentEndpoint) -> Result<(), GatewayError> {
        let connections = self.connections.clone();
        let endpoint = endpoint.clone();

        tokio::spawn(async move {
            let mut conns = connections.write().await;
            conns.insert(endpoint.agent_id.clone(), endpoint);
        });

        Ok(())
    }

    async fn disconnect(&self, endpoint_id: &str) -> Result<(), GatewayError> {
        let mut connections = self.connections.write().await;
        connections
            .remove(endpoint_id)
            .ok_or_else(|| GatewayError::NotFound {
                entity: format!("connection {}", endpoint_id),
            })?;
        Ok(())
    }

    async fn send(&self, endpoint_id: &str, _message: &AgentMessage) -> Result<(), GatewayError> {
        let connections = self.connections.read().await;
        let _endpoint = connections.get(endpoint_id).ok_or_else(|| GatewayError::NotFound {
            entity: format!("connection {}", endpoint_id),
        })?;

        Ok(())
    }

    async fn broadcast(
        &self,
        agent_ids: &[String],
        message: &AgentMessage,
    ) -> Result<(), GatewayError> {
        for agent_id in agent_ids {
            self.send(agent_id, message).await?;
        }
        Ok(())
    }

    fn get_state(&self, endpoint_id: &str) -> ConnectionState {
        let connections = self.connections.blocking_read();
        if connections.contains_key(endpoint_id) {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        }
    }
}

#[derive(Clone, Default)]
pub struct StdioTransportConfig {
    pub buffer_size: usize,
}

#[derive(Clone)]
pub struct StdioTransportHandler {
    connections: Arc<RwLock<HashMap<String, AgentEndpoint>>>,
}

impl StdioTransportHandler {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for StdioTransportHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportHandler for StdioTransportHandler {
    fn transport_type(&self) -> TransportType {
        TransportType::Stdio
    }

    async fn connect(&self, endpoint: &AgentEndpoint) -> Result<(), GatewayError> {
        let connections = self.connections.clone();
        let endpoint = endpoint.clone();

        tokio::spawn(async move {
            let mut conns = connections.write().await;
            conns.insert(endpoint.agent_id.clone(), endpoint);
        });

        Ok(())
    }

    async fn disconnect(&self, endpoint_id: &str) -> Result<(), GatewayError> {
        let mut connections = self.connections.write().await;
        connections
            .remove(endpoint_id)
            .ok_or_else(|| GatewayError::NotFound {
                entity: format!("connection {}", endpoint_id),
            })?;
        Ok(())
    }

    async fn send(&self, endpoint_id: &str, message: &AgentMessage) -> Result<(), GatewayError> {
        let connections = self.connections.read().await;
        let _endpoint = connections.get(endpoint_id).ok_or_else(|| GatewayError::NotFound {
            entity: format!("connection {}", endpoint_id),
        })?;

        let json = serde_json::to_string(message)
            .map_err(|e| GatewayError::SerializationError { reason: e.to_string() })?;

        println!("{}", json);

        Ok(())
    }

    async fn broadcast(
        &self,
        agent_ids: &[String],
        message: &AgentMessage,
    ) -> Result<(), GatewayError> {
        for agent_id in agent_ids {
            self.send(agent_id, message).await?;
        }
        Ok(())
    }

    fn get_state(&self, endpoint_id: &str) -> ConnectionState {
        let connections = self.connections.blocking_read();
        if connections.contains_key(endpoint_id) {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        }
    }
}

pub fn create_default_transport_handlers(
) -> Vec<Arc<dyn TransportHandler>> {
    vec![
        Arc::new(WebSocketTransportHandler::new()) as Arc<dyn TransportHandler>,
        Arc::new(HTTPTransportHandler::new()) as Arc<dyn TransportHandler>,
        Arc::new(SSETransportHandler::new()) as Arc<dyn TransportHandler>,
        Arc::new(StdioTransportHandler::new()) as Arc<dyn TransportHandler>,
    ]
}
