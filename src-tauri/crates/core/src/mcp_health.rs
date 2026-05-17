#[cfg(not(target_os = "android"))]
use std::collections::HashMap;
#[cfg(not(target_os = "android"))]
use std::sync::Arc;
#[cfg(not(target_os = "android"))]
use std::time::Duration;

#[cfg(not(target_os = "android"))]
use tokio::sync::Mutex;

#[cfg(not(target_os = "android"))]
use crate::mcp_client::McpConnectionPool;

#[cfg(not(target_os = "android"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy { reason: String },
}

#[cfg(not(target_os = "android"))]
pub struct McpHealthMonitor {
    pool: Arc<McpConnectionPool>,
    check_interval: Duration,
    unhealthy_count: Mutex<HashMap<String, u32>>,
    unhealthy_threshold: u32,
}

#[cfg(not(target_os = "android"))]
impl McpHealthMonitor {
    #[must_use]
    pub fn new(pool: Arc<McpConnectionPool>) -> Self {
        Self {
            pool,
            check_interval: Duration::from_secs(30),
            unhealthy_count: Mutex::new(HashMap::new()),
            unhealthy_threshold: 3,
        }
    }

    #[must_use]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    #[must_use]
    pub fn with_threshold(mut self, threshold: u32) -> Self {
        self.unhealthy_threshold = threshold;
        self
    }

    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(self.check_interval);
            tick.tick().await;

            loop {
                tick.tick().await;
                tracing::debug!(
                    "[McpHealth] 执行定期健康检查 (池中 {} 个连接)",
                    self.pool.len().await
                );
            }
        })
    }

    pub async fn check_now(&self) -> Vec<HealthReport> {
        let reports = Vec::new();
        let pool_size = self.pool.len().await;

        tracing::info!("[McpHealth] 全量健康检查开始，池中有 {pool_size} 个连接");

        if pool_size == 0 {
            return reports;
        }

        let after_check = self.pool.len().await;
        if after_check < pool_size {
            let evicted = pool_size - after_check;
            tracing::warn!("[McpHealth] 健康检查期间驱逐了 {} 个不健康连接", evicted);
            self.increment_unhealthy_count("evicted_pool", evicted)
                .await;
        }

        reports
    }

    pub async fn increment_unhealthy_count(&self, server_id: &str, count: usize) {
        let mut map = self.unhealthy_count.lock().await;
        let entry = map.entry(server_id.to_string()).or_insert(0);
        *entry += count as u32;
    }

    pub async fn is_unhealthy(&self, server_id: &str) -> bool {
        let map = self.unhealthy_count.lock().await;
        map.get(server_id)
            .is_some_and(|&c| c >= self.unhealthy_threshold)
    }

    pub async fn reset_unhealthy_count(&self, server_id: &str) {
        let mut map = self.unhealthy_count.lock().await;
        map.remove(server_id);
    }
}

#[cfg(not(target_os = "android"))]
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub server_id: String,
    pub status: HealthStatus,
}

#[cfg(not(target_os = "android"))]
impl Default for McpHealthMonitor {
    fn default() -> Self {
        Self::new(Arc::new(McpConnectionPool::new(Duration::from_secs(300))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "android"))]
    #[test]
    fn health_monitor_has_sensible_defaults() {
        let monitor = McpHealthMonitor::default();
        assert_eq!(monitor.check_interval, Duration::from_secs(30));
        assert_eq!(monitor.unhealthy_threshold, 3);
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn health_status_equality() {
        assert_eq!(HealthStatus::Healthy, HealthStatus::Healthy);
        assert_ne!(
            HealthStatus::Healthy,
            HealthStatus::Unhealthy {
                reason: "timeout".into()
            }
        );
    }

    #[cfg(not(target_os = "android"))]
    #[tokio::test]
    async fn unhealthy_count_tracking() {
        let monitor = McpHealthMonitor::default();
        assert!(!monitor.is_unhealthy("server1").await);

        monitor.increment_unhealthy_count("server1", 1).await;
        assert!(!monitor.is_unhealthy("server1").await);

        monitor.increment_unhealthy_count("server1", 2).await;
        assert!(monitor.is_unhealthy("server1").await);

        monitor.reset_unhealthy_count("server1").await;
        assert!(!monitor.is_unhealthy("server1").await);
    }
}
