// SPDX-License-Identifier: AGPL-3.0-only

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
            // 首次即时检查（L2 修复：不跳过第一拍，立即探测）
            let reports = self.check_now().await;
            if !reports.is_empty() {
                tracing::info!(
                    "[McpHealth] 启动时即时健康检查完成，{} 个连接已探测",
                    reports.len()
                );
            }

            loop {
                tick.tick().await;
                let reports = self.check_now().await;
                for r in &reports {
                    match &r.status {
                        HealthStatus::Healthy => {
                            tracing::debug!("[McpHealth] {} 健康", r.server_id);
                        },
                        HealthStatus::Unhealthy { reason } => {
                            tracing::warn!("[McpHealth] {} 不健康: {reason}", r.server_id);
                        },
                    }
                }
                if reports.is_empty() {
                    tracing::debug!(
                        "[McpHealth] 健康检查完成，池中无活动连接 ({} 个)",
                        self.pool.len().await
                    );
                }
            }
        })
    }

    pub async fn check_now(&self) -> Vec<HealthReport> {
        let pool_size = self.pool.len().await;

        tracing::info!("[McpHealth] 全量健康检查开始，池中有 {pool_size} 个连接");

        if pool_size == 0 {
            return Vec::new();
        }

        // 实际探测每个连接的存活状态，并驱逐死亡连接
        let probe_results = self.pool.probe_and_evict().await;

        let mut reports = Vec::with_capacity(probe_results.len());
        for (key, alive) in probe_results {
            let server_id = key.server_id.clone().unwrap_or_else(|| key.command.clone());
            if alive {
                self.reset_unhealthy_count(&server_id).await;
                reports.push(HealthReport { server_id, status: HealthStatus::Healthy });
            } else {
                self.increment_unhealthy_count(&server_id, 1).await;
                reports.push(HealthReport {
                    server_id: server_id.clone(),
                    status: HealthStatus::Unhealthy {
                        reason: "连接探测失败（list_all_tools 超时/错误）".into(),
                    },
                });
            }
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
        map.get(server_id).is_some_and(|&c| c >= self.unhealthy_threshold)
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
        assert_ne!(HealthStatus::Healthy, HealthStatus::Unhealthy { reason: "timeout".into() });
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
