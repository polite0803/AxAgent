use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use futures::stream::SplitStream;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub const DEFAULT_REALTIME_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(25);
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RealtimeClientMessage {
    #[serde(rename = "session.create")]
    SessionCreate { model: String },
    #[serde(rename = "input_audio_buffer.append")]
    AudioAppend { audio: String },
    #[serde(rename = "input_audio_buffer.commit")]
    AudioCommit,
    #[serde(rename = "session.close")]
    SessionClose,
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate { item: serde_json::Value },
    #[serde(rename = "response.create")]
    ResponseCreate { response: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RealtimeServerMessage {
    #[serde(rename = "session.created")]
    SessionCreated { session_id: String },
    #[serde(rename = "session.updated")]
    SessionUpdated { session: serde_json::Value },
    #[serde(rename = "response.audio.delta")]
    AudioDelta { delta: String },
    #[serde(rename = "response.audio.done")]
    AudioDone,
    #[serde(rename = "response.text.delta")]
    TextDelta { delta: String },
    #[serde(rename = "response.text.done")]
    TextDone,
    #[serde(rename = "response.done")]
    ResponseDone,
    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated { item: serde_json::Value },
    #[serde(rename = "error")]
    Error { error: RealtimeError },
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeError {
    pub message: String,
    pub code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RealtimeClientConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub heartbeat_interval: Duration,
    pub max_reconnect_attempts: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for RealtimeClientConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            api_key: None,
            model: None,
            timeout: DEFAULT_REALTIME_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            max_reconnect_attempts: 5,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum RealtimeConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed(String),
}


pub struct RealtimeClient {
    config: RealtimeClientConfig,
    state: Arc<RwLock<RealtimeConnectionState>>,
    session_id: Arc<RwLock<Option<String>>>,
    sender: Arc<RwLock<Option<mpsc::Sender<RealtimeClientMessage>>>>,
    reconnect_attempts: Arc<RwLock<u32>>,
}

impl RealtimeClient {
    pub fn new(config: RealtimeClientConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(RealtimeConnectionState::Disconnected)),
            session_id: Arc::new(RwLock::new(None)),
            sender: Arc::new(RwLock::new(None)),
            reconnect_attempts: Arc::new(RwLock::new(0)),
        }
    }

    fn calculate_backoff_delay(&self, attempts: u32) -> Duration {
        let delay_ms = std::cmp::min(
            self.config.initial_backoff_ms * 2u64.pow(attempts.min(10)),
            self.config.max_backoff_ms,
        );
        Duration::from_millis(delay_ms)
    }

    async fn increment_reconnect_attempts(&self) -> u32 {
        let mut attempts = self.reconnect_attempts.write().await;
        *attempts += 1;
        *attempts
    }

    async fn reset_reconnect_attempts(&self) {
        let mut attempts = self.reconnect_attempts.write().await;
        *attempts = 0;
    }

    pub async fn connect(&self) -> Result<(), RealtimeClientError> {
        {
            let mut state = self.state.write().await;
            *state = RealtimeConnectionState::Connecting;
        }

        let url = if let Some(ref api_key) = self.config.api_key {
            format!("{}?api_key={}", self.config.url, api_key)
        } else {
            self.config.url.clone()
        };

        let (ws_stream, _) = timeout(self.config.connect_timeout, connect_async(&url))
            .await
            .map_err(|_| RealtimeClientError::ConnectTimeout)?
            .map_err(|e| RealtimeClientError::ConnectionFailed(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();

        let session_msg = RealtimeClientMessage::SessionCreate {
            model: self.config.model.clone().unwrap_or_else(|| "gpt-4o-realtime".to_string()),
        };
        let msg_json = serde_json::to_string(&session_msg)
            .map_err(|e| RealtimeClientError::SerializationError(e.to_string()))?;
        write
            .send(Message::Text(msg_json.into()))
            .await
            .map_err(|e| RealtimeClientError::SendError(e.to_string()))?;

        let (session_id, read_result) = Self::wait_for_session_created(&mut read).await?;
        read_result?;

        {
            let mut sid = self.session_id.write().await;
            *sid = Some(session_id);
        }

        {
            let mut state = self.state.write().await;
            *state = RealtimeConnectionState::Connected;
        }

        let (tx, mut rx) = mpsc::channel::<RealtimeClientMessage>(100);
        {
            let mut sender = self.sender.write().await;
            *sender = Some(tx);
        }

        let state = self.state.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if let Err(e) = write.send(Message::Text(json.into())).await {
                                tracing::error!("Failed to send message: {}", e);
                                break;
                            }
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(server_msg) = serde_json::from_str::<RealtimeServerMessage>(&text) {
                                    tracing::debug!("Received server message: {:?}", server_msg);
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                let mut s = state.write().await;
                                *s = RealtimeConnectionState::Disconnected;
                                break;
                            }
                            Some(Err(e)) => {
                                tracing::error!("WebSocket read error: {}", e);
                                let mut s = state.write().await;
                                *s = RealtimeConnectionState::Failed(e.to_string());
                                break;
                            }
                            _ => continue,
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn wait_for_session_created(
        read: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    ) -> Result<(String, Result<(), RealtimeClientError>), RealtimeClientError> {
        loop {
            match timeout(Duration::from_secs(30), read.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Ok(server_msg) = serde_json::from_str::<RealtimeServerMessage>(&text) {
                        match server_msg {
                            RealtimeServerMessage::SessionCreated { session_id } => {
                                return Ok((session_id, Ok(())));
                            }
                            RealtimeServerMessage::Error { error } => {
                                return Ok((String::new(), Err(RealtimeClientError::ProviderError(error.message))));
                            }
                            _ => continue,
                        }
                    }
                }
                Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {
                    return Err(RealtimeClientError::ConnectionClosed);
                }
                Ok(Some(Ok(Message::Ping(_)))) => {
                    continue;
                }
                Ok(Some(Ok(Message::Pong(_)))) => {
                    continue;
                }
                Ok(Some(Ok(Message::Binary(_)))) => {
                    continue;
                }
                Ok(Some(Ok(Message::Frame(_)))) => {
                    continue;
                }
                Ok(Some(Err(e))) => {
                    return Err(RealtimeClientError::ReadError(e.to_string()));
                }
                Err(_) => {
                    return Err(RealtimeClientError::Timeout);
                }
            }
        }
    }

    pub async fn send_audio(&self, audio_data: &str) -> Result<(), RealtimeClientError> {
        let msg = RealtimeClientMessage::AudioAppend {
            audio: audio_data.to_string(),
        };
        self.send_message(msg).await
    }

    pub async fn commit_audio(&self) -> Result<(), RealtimeClientError> {
        self.send_message(RealtimeClientMessage::AudioCommit).await
    }

    pub async fn close(&self) -> Result<(), RealtimeClientError> {
        self.send_message(RealtimeClientMessage::SessionClose).await
    }

    async fn send_message(&self, msg: RealtimeClientMessage) -> Result<(), RealtimeClientError> {
        let state = self.state.read().await;
        if !matches!(*state, RealtimeConnectionState::Connected) {
            return Err(RealtimeClientError::NotConnected);
        }
        drop(state);

        let sender = self.sender.read().await;
        if let Some(tx) = sender.as_ref() {
            tx.send(msg).await.map_err(|_| RealtimeClientError::SendError("Channel closed".to_string()))?;
        }

        Ok(())
    }

    pub async fn get_state(&self) -> RealtimeConnectionState {
        self.state.read().await.clone()
    }

    pub async fn get_session_id(&self) -> Option<String> {
        self.session_id.read().await.clone()
    }
}

impl Clone for RealtimeClient {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            state: self.state.clone(),
            session_id: self.session_id.clone(),
            sender: self.sender.clone(),
            reconnect_attempts: self.reconnect_attempts.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RealtimeClientError {
    #[error("Connection timeout")]
    ConnectTimeout,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Not connected")]
    NotConnected,

    #[error("Send error: {0}")]
    SendError(String),

    #[error("Read error: {0}")]
    ReadError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Timeout")]
    Timeout,

    #[error("Provider error: {0}")]
    ProviderError(String),
}

pub struct RealtimeStreamHandler {
    client: RealtimeClient,
}

impl RealtimeStreamHandler {
    pub fn new(client: RealtimeClient) -> Self {
        Self { client }
    }

    pub async fn run(&self) -> Result<(), RealtimeClientError> {
        loop {
            let state = self.client.get_state().await;
            match state {
                RealtimeConnectionState::Connected => {
                    self.client.reset_reconnect_attempts().await;
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                RealtimeConnectionState::Connecting => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                RealtimeConnectionState::Reconnecting => {
                    let attempts = *self.client.reconnect_attempts.read().await;
                    if attempts >= self.client.config.max_reconnect_attempts {
                        tracing::error!("Max reconnect attempts reached for RealtimeClient");
                        break;
                    }
                    let delay = self.client.calculate_backoff_delay(attempts);
                    tracing::info!("Reconnecting in {:?} (attempt {}/{})",
                        delay, attempts + 1, self.client.config.max_reconnect_attempts);
                    tokio::time::sleep(delay).await;
                    if let Err(e) = self.client.connect().await {
                        tracing::warn!("Reconnect failed: {}", e);
                        let new_attempts = self.client.increment_reconnect_attempts().await;
                        if new_attempts >= self.client.config.max_reconnect_attempts {
                            tracing::error!("Max reconnect attempts reached for RealtimeClient");
                            break;
                        }
                    }
                }
                RealtimeConnectionState::Disconnected | RealtimeConnectionState::Failed(_) => {
                    let attempts = self.client.increment_reconnect_attempts().await;
                    if attempts > self.client.config.max_reconnect_attempts {
                        tracing::error!("Max reconnect attempts reached for RealtimeClient");
                        break;
                    }
                    let delay = self.client.calculate_backoff_delay(attempts.saturating_sub(1));
                    tracing::info!("Connection lost, attempting reconnect in {:?} (attempt {}/{})",
                        delay, attempts, self.client.config.max_reconnect_attempts);
                    {
                        let mut state = self.client.state.write().await;
                        *state = RealtimeConnectionState::Reconnecting;
                    }
                    tokio::time::sleep(delay).await;
                    if let Err(e) = self.client.connect().await {
                        tracing::warn!("Reconnect failed: {}", e);
                        let new_attempts = *self.client.reconnect_attempts.read().await;
                        if new_attempts >= self.client.config.max_reconnect_attempts {
                            tracing::error!("Max reconnect attempts reached for RealtimeClient");
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}