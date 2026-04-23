use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, RwLock};
use tokio::time::interval;

use crate::error::Result;
use crate::repo::gateway_link as link_repo;
use crate::repo::gateway_link::ExponentialBackoff;

const DEFAULT_HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);
const DEFAULT_RECONNECT_INTERVAL: Duration = Duration::from_secs(5);
const DEFAULT_MAX_RECONNECT_ATTEMPTS: u32 = 5;
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 1000;
const DEFAULT_MAX_BACKOFF_MS: u64 = 60000;
const DEFAULT_REQUIRED_HEALTH_SUCCESSES: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

impl Default for LinkConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

#[derive(Debug, Clone)]
pub struct GatewayLinkHandle {
    pub link_id: String,
    pub state: LinkConnectionState,
    pub last_health_check: Option<Instant>,
    pub last_error: Option<String>,
    pub reconnect_attempts: u32,
    pub consecutive_health_successes: u32,
    pub required_health_successes: u32,
}

pub struct GatewayLinkManager {
    links: Arc<RwLock<HashMap<String, GatewayLinkHandle>>>,
    health_check_interval: Duration,
    reconnect_interval: Duration,
    max_reconnect_attempts: u32,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
}

impl Default for GatewayLinkManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayLinkManager {
    pub fn new() -> Self {
        Self {
            links: Arc::new(RwLock::new(HashMap::new())),
            health_check_interval: DEFAULT_HEALTH_CHECK_INTERVAL,
            reconnect_interval: DEFAULT_RECONNECT_INTERVAL,
            max_reconnect_attempts: DEFAULT_MAX_RECONNECT_ATTEMPTS,
            initial_backoff_ms: DEFAULT_INITIAL_BACKOFF_MS,
            max_backoff_ms: DEFAULT_MAX_BACKOFF_MS,
        }
    }

    pub fn with_config(
        mut self,
        health_check_interval: Duration,
        reconnect_interval: Duration,
        max_reconnect_attempts: u32,
    ) -> Self {
        self.health_check_interval = health_check_interval;
        self.reconnect_interval = reconnect_interval;
        self.max_reconnect_attempts = max_reconnect_attempts;
        self
    }

    pub async fn register_link(&self, link_id: String) {
        let mut links = self.links.write().await;
        links.insert(
            link_id.clone(),
            GatewayLinkHandle {
                link_id,
                state: LinkConnectionState::Disconnected,
                last_health_check: None,
                last_error: None,
                reconnect_attempts: 0,
                consecutive_health_successes: 0,
                required_health_successes: DEFAULT_REQUIRED_HEALTH_SUCCESSES,
            },
        );
    }

    pub async fn unregister_link(&self, link_id: &str) {
        let mut links = self.links.write().await;
        links.remove(link_id);
    }

    pub async fn get_link_state(&self, link_id: &str) -> Option<LinkConnectionState> {
        let links = self.links.read().await;
        links.get(link_id).map(|h| h.state)
    }

    pub async fn update_link_state(&self, link_id: &str, state: LinkConnectionState, error: Option<String>) -> bool {
        let mut links = self.links.write().await;
        if let Some(handle) = links.get_mut(link_id) {
            handle.state = state;
            handle.last_error = error;
            if state == LinkConnectionState::Connected {
                handle.reconnect_attempts = 0;
            }
            true
        } else {
            false
        }
    }

    pub async fn record_health_check(&self, link_id: &str, success: bool) {
        let mut links = self.links.write().await;
        if let Some(handle) = links.get_mut(link_id) {
            handle.last_health_check = Some(Instant::now());
            if success {
                handle.consecutive_health_successes += 1;
            } else {
                handle.consecutive_health_successes = 0;
                handle.state = LinkConnectionState::Reconnecting;
            }
        }
    }

    pub async fn increment_reconnect_attempts(&self, link_id: &str) -> u32 {
        let mut links = self.links.write().await;
        if let Some(handle) = links.get_mut(link_id) {
            handle.reconnect_attempts += 1;
            handle.state = LinkConnectionState::Reconnecting;
            handle.reconnect_attempts
        } else {
            0
        }
    }

    pub async fn get_all_links(&self) -> Vec<GatewayLinkHandle> {
        let links = self.links.read().await;
        links.values().cloned().collect()
    }

    pub async fn get_stale_links(&self, threshold: Duration) -> Vec<String> {
        let links = self.links.read().await;
        let now = Instant::now();
        links
            .iter()
            .filter(|(_, handle)| {
                handle.state == LinkConnectionState::Connected
                    && handle.last_health_check.map_or(true, |t| now.duration_since(t) > threshold)
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    fn calculate_backoff_delay(&self, attempts: u32) -> Duration {
        let delay_ms = std::cmp::min(
            self.initial_backoff_ms * 2u64.pow(attempts.min(10)),
            self.max_backoff_ms,
        );
        Duration::from_millis(delay_ms)
    }
}

pub struct GatewayLinkConnectionHandle {
    manager: Arc<GatewayLinkManager>,
    db: sea_orm::DatabaseConnection,
    link_id: String,
    api_key: Option<String>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl GatewayLinkConnectionHandle {
    pub fn new(
        manager: Arc<GatewayLinkManager>,
        db: sea_orm::DatabaseConnection,
        link_id: String,
        api_key: Option<String>,
    ) -> Self {
        Self {
            manager,
            db,
            link_id,
            api_key,
            shutdown_tx: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let link_id = self.link_id.clone();
        let manager = self.manager.clone();
        let db = self.db.clone();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            Self::run_connection_loop(link_id, manager, db, api_key, shutdown_rx).await;
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        if let Some(tx) = &self.shutdown_tx {
            let _ = tx.send(()).await;
        }
        Ok(())
    }

    async fn run_connection_loop(
        link_id: String,
        manager: Arc<GatewayLinkManager>,
        db: sea_orm::DatabaseConnection,
        api_key: Option<String>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) {
        let mut health_check_interval = interval(manager.health_check_interval);
        let mut reconnect_attempts: u32 = 0;

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("Gateway link {} shutting down", link_id);
                    break;
                }
                _ = health_check_interval.tick() => {
                    let current_state = manager.get_link_state(&link_id).await;

                    match current_state {
                        Some(LinkConnectionState::Connected) | Some(LinkConnectionState::Disconnected) => {
                            match link_repo::check_gateway_health(&db, &link_id, api_key.as_deref()).await {
                                Ok(latency_ms) => {
                                    manager.record_health_check(&link_id, true).await;
                                    let links = manager.links.read().await;
                                    let consecutive = links.get(&link_id).map(|h| h.consecutive_health_successes).unwrap_or(0);
                                    let required = links.get(&link_id).map(|h| h.required_health_successes).unwrap_or(DEFAULT_REQUIRED_HEALTH_SUCCESSES);
                                    drop(links);

                                    let _ = link_repo::update_gateway_link_status(
                                        &db,
                                        &link_id,
                                        "connected",
                                        None,
                                        Some(latency_ms as i64),
                                        None,
                                    ).await;

                                    if consecutive >= required {
                                        manager.update_link_state(&link_id, LinkConnectionState::Connected, None).await;
                                        reconnect_attempts = 0;
                                        tracing::info!("Gateway link {} is now fully connected after {} consecutive health checks", link_id, consecutive);
                                    } else {
                                        tracing::debug!("Gateway link {} health check passed ({}/{} consecutive)", link_id, consecutive, required);
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Gateway link {} health check failed: {}", link_id, e);
                                    manager.record_health_check(&link_id, false).await;

                                    let _ = link_repo::update_gateway_link_status(
                                        &db,
                                        &link_id,
                                        "error",
                                        Some(&e.to_string()),
                                        None,
                                        None,
                                    ).await;

                                    if e.is_transient() {
                                        let new_attempts = manager.increment_reconnect_attempts(&link_id).await;
                                        if new_attempts <= manager.max_reconnect_attempts {
                                            let delay = manager.calculate_backoff_delay(new_attempts);
                                            tracing::info!(
                                                "Gateway link {} scheduling reconnect in {:?} (attempt {}/{})",
                                                link_id, delay, new_attempts, manager.max_reconnect_attempts
                                            );

                                            tokio::time::sleep(delay).await;

                                            if let Err(e) = link_repo::connect_gateway_link(&db, &link_id, api_key.as_deref()).await {
                                                tracing::error!("Gateway link {} reconnect failed: {}", link_id, e);
                                            }
                                        } else {
                                            tracing::error!("Gateway link {} exceeded max reconnect attempts", link_id);
                                            manager.update_link_state(&link_id, LinkConnectionState::Failed, Some("Max reconnect attempts exceeded".to_string())).await;
                                        }
                                    } else {
                                        tracing::error!(
                                            "Gateway link {} health check returned permanent error, not retrying: {}",
                                            link_id, e
                                        );
                                        manager.update_link_state(&link_id, LinkConnectionState::Failed, Some(e.to_string())).await;
                                    }
                                }
                            }
                        }
                        Some(LinkConnectionState::Reconnecting) => {
                            if reconnect_attempts < manager.max_reconnect_attempts {
                                let delay = manager.calculate_backoff_delay(reconnect_attempts);
                                tokio::time::sleep(delay).await;

                                match link_repo::connect_gateway_link(&db, &link_id, api_key.as_deref()).await {
                                    Ok(_) => {
                                        reconnect_attempts = 0;
                                    }
                                    Err(e) => {
                                        tracing::warn!("Gateway link {} reconnect failed: {}", link_id, e);
                                        reconnect_attempts += 1;
                                    }
                                }
                            } else {
                                tracing::error!("Gateway link {} exceeded max reconnect attempts", link_id);
                                manager.update_link_state(&link_id, LinkConnectionState::Failed, Some("Max reconnect attempts exceeded".to_string())).await;
                                break;
                            }
                        }
                        Some(LinkConnectionState::Failed) => {
                            break;
                        }
                        None => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        manager.update_link_state(&link_id, LinkConnectionState::Disconnected, None).await;
    }
}

impl Drop for GatewayLinkConnectionHandle {
    fn drop(&mut self) {
        if self.shutdown_tx.is_some() {
            let link_id = self.link_id.clone();
            let manager = self.manager.clone();
            tokio::spawn(async move {
                manager.unregister_link(&link_id).await;
            });
        }
    }
}

pub async fn retry_with_backoff<F, Fut, T>(
    operation: &str,
    mut f: F,
    max_attempts: u32,
    mut backoff: ExponentialBackoff,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..max_attempts {
        match f().await {
            Ok(result) => {
                backoff.reset();
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < max_attempts - 1 {
                    let delay = backoff.next_delay();
                    tracing::warn!(
                        "{} failed (attempt {}/{}), retrying in {:?}: {}",
                        operation,
                        attempt + 1,
                        max_attempts,
                        delay,
                        last_error.as_ref().expect("last_error was just set")
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.expect("last_error must be set if loop completed without returning"))
}
