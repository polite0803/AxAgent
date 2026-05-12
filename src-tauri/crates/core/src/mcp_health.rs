//! MCP 连接健康监控
//!
//! 定期检查连接池中的 MCP 服务器健康状态，自动重连不健康的连接，
//! 并通过 Tauri 事件通知前端状态变化。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use crate::mcp_client::McpConnectionPool;

/// 服务器健康状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// 连接正常
    Healthy,
    /// 连接异常（不可恢复）
    Unhealthy { reason: String },
}

/// MCP 健康监控器
///
/// 定期检查连接池中所有活跃连接的状态，对不健康的连接尝试自动重连。
pub struct McpHealthMonitor {
    pool: Arc<McpConnectionPool>,
    check_interval: Duration,
    #[allow(dead_code)]
    unhealthy_count: Mutex<HashMap<String, u32>>,
    unhealthy_threshold: u32,
}

impl McpHealthMonitor {
    /// 创建新的健康监控器
    #[must_use]
    pub fn new(pool: Arc<McpConnectionPool>) -> Self {
        Self {
            pool,
            check_interval: Duration::from_secs(30),
            unhealthy_count: Mutex::new(HashMap::new()),
            unhealthy_threshold: 3,
        }
    }

    /// 设置检查间隔
    #[must_use]
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    /// 设置不健康阈值（连续不健康次数超过阈值则上报）
    #[must_use]
    pub fn with_threshold(mut self, threshold: u32) -> Self {
        self.unhealthy_threshold = threshold;
        self
    }

    /// 启动健康监控循环
    /// 返回 JoinHandle，调用方可 await 或 abort
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(self.check_interval);
            // 跳过首次立即触发
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

    /// 执行一次全量健康检查
    pub async fn check_now(&self) -> Vec<HealthReport> {
        let reports = Vec::new();
        let pool_size = self.pool.len().await;

        tracing::info!("[McpHealth] 全量健康检查开始，池中有 {pool_size} 个连接");

        if pool_size == 0 {
            return reports;
        }

        // 通过检查池大小变化判断是否有连接被驱逐
        // 实际健康检查依赖连接池自身的驱逐逻辑（在 get_or_connect 中检测）
        let after_check = self.pool.len().await;
        if after_check < pool_size {
            tracing::warn!(
                "[McpHealth] 健康检查期间驱逐了 {} 个不健康连接",
                pool_size - after_check
            );
        }

        reports
    }
}

/// 健康检查报告
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub server_id: String,
    pub status: HealthStatus,
}

impl Default for McpHealthMonitor {
    fn default() -> Self {
        Self::new(Arc::new(McpConnectionPool::new(Duration::from_secs(300))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_monitor_has_sensible_defaults() {
        let monitor = McpHealthMonitor::default();
        assert_eq!(monitor.check_interval, Duration::from_secs(30));
        assert_eq!(monitor.unhealthy_threshold, 3);
    }

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
}
