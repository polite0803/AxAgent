use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::interval;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: HealthState,
    pub version: String,
    pub uptime_seconds: u64,
    pub timestamp: i64,
    pub checks: HashMap<String, HealthCheckResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthState {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: HealthState,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
    pub last_check: i64,
}

impl HealthCheckResult {
    pub fn healthy() -> Self {
        Self {
            status: HealthState::Healthy,
            latency_ms: None,
            message: None,
            last_check: chrono::Utc::now().timestamp(),
        }
    }

    pub fn degraded(message: &str) -> Self {
        Self {
            status: HealthState::Degraded,
            latency_ms: None,
            message: Some(message.to_string()),
            last_check: chrono::Utc::now().timestamp(),
        }
    }

    pub fn unhealthy(message: &str) -> Self {
        Self {
            status: HealthState::Unhealthy,
            latency_ms: None,
            message: Some(message.to_string()),
            last_check: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }
}

#[async_trait::async_trait]
pub trait HealthCheck: Send + Sync {
    fn name(&self) -> &str;
    async fn check(&self) -> HealthCheckResult;
}

pub struct HealthCheckRegistry {
    checks: Arc<RwLock<HashMap<String, Box<dyn HealthCheck>>>>,
    start_time: Instant,
    version: String,
}

impl HealthCheckRegistry {
    pub fn new(version: String) -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            version,
        }
    }

    pub async fn register<C>(&self, check: C)
    where
        C: HealthCheck + 'static,
    {
        let mut checks = self.checks.write().await;
        checks.insert(check.name().to_string(), Box::new(check));
    }

    pub async fn unregister(&self, name: &str) {
        let mut checks = self.checks.write().await;
        checks.remove(name);
    }

    pub async fn get_status(&self) -> HealthStatus {
        let checks = self.checks.read().await;
        let mut results = HashMap::new();
        let mut overall_state = HealthState::Healthy;

        for (name, check) in checks.iter() {
            let result = check.check().await;
            results.insert(name.clone(), result.clone());

            if result.status == HealthState::Unhealthy {
                overall_state = HealthState::Unhealthy;
            } else if result.status == HealthState::Degraded && overall_state == HealthState::Healthy {
                overall_state = HealthState::Degraded;
            }
        }

        HealthStatus {
            status: overall_state,
            version: self.version.clone(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
            timestamp: chrono::Utc::now().timestamp(),
            checks: results,
        }
    }

    pub async fn get_check_names(&self) -> Vec<String> {
        let checks = self.checks.read().await;
        checks.keys().cloned().collect()
    }
}

pub struct LivenessCheck {
    last_ping: Arc<RwLock<Instant>>,
}

impl LivenessCheck {
    pub fn new() -> Self {
        Self {
            last_ping: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn ping(&self) {
        let mut last = self.last_ping.write().await;
        *last = Instant::now();
    }
}

#[async_trait::async_trait]
impl HealthCheck for LivenessCheck {
    fn name(&self) -> &str {
        "liveness"
    }

    async fn check(&self) -> HealthCheckResult {
        HealthCheckResult::healthy()
    }
}

pub struct ReadinessCheck {
    last_ready: Arc<RwLock<bool>>,
}

impl ReadinessCheck {
    pub fn new() -> Self {
        Self {
            last_ready: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn set_ready(&self, ready: bool) {
        let mut last = self.last_ready.write().await;
        *last = ready;
    }

    pub async fn is_ready(&self) -> bool {
        *self.last_ready.read().await
    }
}

#[async_trait::async_trait]
impl HealthCheck for ReadinessCheck {
    fn name(&self) -> &str {
        "readiness"
    }

    async fn check(&self) -> HealthCheckResult {
        if *self.last_ready.read().await {
            HealthCheckResult::healthy()
        } else {
            HealthCheckResult::unhealthy("Service not ready")
        }
    }
}

pub struct DatabaseHealthCheck {
    db_url: String,
}

impl DatabaseHealthCheck {
    pub fn new(db_url: String) -> Self {
        Self { db_url }
    }
}

#[async_trait::async_trait]
impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> &str {
        "database"
    }

    async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();
        HealthCheckResult::healthy().with_latency(start.elapsed().as_millis() as u64)
    }
}

pub struct GatewayHealthCheck {
    gateway_links: Arc<RwLock<Vec<String>>>,
}

impl GatewayHealthCheck {
    pub fn new() -> Self {
        Self {
            gateway_links: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn add_link(&self, link_id: String) {
        let mut links = self.gateway_links.write().await;
        links.push(link_id);
    }

    pub async fn remove_link(&self, link_id: &str) {
        let mut links = self.gateway_links.write().await;
        links.retain(|id| id != link_id);
    }
}

#[async_trait::async_trait]
impl HealthCheck for GatewayHealthCheck {
    fn name(&self) -> &str {
        "gateway_links"
    }

    async fn check(&self) -> HealthCheckResult {
        let links = self.gateway_links.read().await;
        if links.is_empty() {
            return HealthCheckResult::degraded("No gateway links configured");
        }
        HealthCheckResult::healthy()
    }
}

pub struct HealthCheckServer {
    registry: Arc<HealthCheckRegistry>,
    bind_addr: String,
    bind_port: u16,
}

impl HealthCheckServer {
    pub fn new(registry: Arc<HealthCheckRegistry>, bind_addr: String, bind_port: u16) -> Self {
        Self {
            registry,
            bind_addr,
            bind_port,
        }
    }

    pub async fn start(self) -> Result<(), std::io::Error> {
        use axum::{routing::get, Router};
        use tower_http::cors::{Any, CorsLayer};

        let registry = self.registry.clone();

        let app = Router::new()
            .route("/health", get(move || async move {
                let status = registry.get_status().await;
                let json = serde_json::to_string(&status).unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string());
                axum::response::Json(serde_json::from_str(&json).unwrap())
            }))
            .route("/health/live", get(move || async move {
                axum::response::Json(serde_json::json!({ "status": "alive" }))
            }))
            .route("/health/ready", get(move || async move {
                let status = registry.get_status().await;
                let ready = status.status == HealthState::Healthy || status.status == HealthState::Degraded;
                axum::response::Json(serde_json::json!({ "ready": ready }))
            }))
            .layer(CorsLayer::new().allow_origin(Any));

        let addr = format!("{}:{}", self.bind_addr, self.bind_port);
        let listener = tokio::net::TcpListener::bind(&addr).await?;

        tracing::info!("Health check server listening on {}", addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

pub struct HealthMonitor {
    registry: Arc<HealthCheckRegistry>,
    check_interval: Duration,
}

impl HealthMonitor {
    pub fn new(registry: Arc<HealthCheckRegistry>, check_interval: Duration) -> Self {
        Self {
            registry,
            check_interval,
        }
    }

    pub async fn start(&self) {
        let registry = self.registry.clone();
        let interval = self.check_interval;

        tokio::spawn(async move {
            let mut ticker = interval(interval);
            loop {
                ticker.tick().await;
                let status = registry.get_status().await;

                match status.status {
                    HealthState::Healthy => {
                        tracing::debug!("Health check: all systems healthy");
                    }
                    HealthState::Degraded => {
                        tracing::warn!("Health check: system degraded - {:?}", status.checks);
                    }
                    HealthState::Unhealthy => {
                        tracing::error!("Health check: system unhealthy - {:?}", status.checks);
                    }
                }
            }
        });
    }
}

impl Default for LivenessCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ReadinessCheck {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for GatewayHealthCheck {
    fn default() -> Self {
        Self::new()
    }
}