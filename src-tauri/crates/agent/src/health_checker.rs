use crate::event_bus::{AgentEventBus, AgentEventType, UnifiedAgentEvent};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
            HealthStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthMetric {
    pub name: String,
    pub value: f64,
    pub timestamp: Instant,
    pub is_healthy: bool,
}

impl HealthMetric {
    pub fn new(name: impl Into<String>, value: f64, is_healthy: bool) -> Self {
        Self {
            name: name.into(),
            value,
            timestamp: Instant::now(),
            is_healthy,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub status: HealthStatus,
    pub message: String,
    pub metrics: Vec<HealthMetric>,
    pub timestamp: Instant,
    pub duration_ms: u64,
}

impl HealthCheckResult {
    pub fn healthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: message.into(),
            metrics: Vec::new(),
            timestamp: Instant::now(),
            duration_ms: 0,
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            message: message.into(),
            metrics: Vec::new(),
            timestamp: Instant::now(),
            duration_ms: 0,
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            message: message.into(),
            metrics: Vec::new(),
            timestamp: Instant::now(),
            duration_ms: 0,
        }
    }

    pub fn with_metrics(mut self, metrics: Vec<HealthMetric>) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

#[derive(Debug, Clone)]
pub struct HealthThresholds {
    pub max_iteration_time_ms: u64,
    pub max_memory_mb: u64,
    pub max_error_rate: f64,
    pub max_queue_depth: usize,
    pub min_throughput_per_sec: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_iteration_time_ms: 10000,
            max_memory_mb: 512,
            max_error_rate: 0.3,
            max_queue_depth: 100,
            min_throughput_per_sec: 0.1,
        }
    }
}

#[derive(Debug)]
struct OperationRecord {
    timestamp: Instant,
    duration_ms: u64,
    success: bool,
    operation_type: String,
}

pub struct HealthChecker {
    event_bus: Arc<AgentEventBus>,
    thresholds: HealthThresholds,
    recent_operations: RwLock<VecDeque<OperationRecord>>,
    max_operation_history: usize,
    status: RwLock<HealthStatus>,
    last_check: RwLock<Instant>,
    check_count: RwLock<u64>,
    unhealthy_count: RwLock<u64>,
}

impl HealthChecker {
    pub fn new(event_bus: Arc<AgentEventBus>) -> Self {
        Self {
            event_bus,
            thresholds: HealthThresholds::default(),
            recent_operations: RwLock::new(VecDeque::with_capacity(1000)),
            max_operation_history: 1000,
            status: RwLock::new(HealthStatus::Unknown),
            last_check: RwLock::new(Instant::now()),
            check_count: RwLock::new(0),
            unhealthy_count: RwLock::new(0),
        }
    }

    pub fn with_thresholds(mut self, thresholds: HealthThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    pub async fn check(&self) -> HealthCheckResult {
        let start = Instant::now();

        {
            let mut last = self.last_check.write().await;
            *last = Instant::now();
        }

        {
            let mut count = self.check_count.write().await;
            *count += 1;
        }

        let mut metrics = Vec::new();
        let mut issues = Vec::new();

        let throughput = self.calculate_throughput().await;
        let throughput_healthy = throughput >= self.thresholds.min_throughput_per_sec;
        metrics.push(HealthMetric::new("throughput_per_sec", throughput, throughput_healthy));
        if !throughput_healthy {
            issues.push(format!(
                "Low throughput: {:.2} ops/sec (min: {:.2})",
                throughput, self.thresholds.min_throughput_per_sec
            ));
        }

        let error_rate = self.calculate_error_rate().await;
        let error_rate_healthy = error_rate <= self.thresholds.max_error_rate;
        metrics.push(HealthMetric::new("error_rate", error_rate, error_rate_healthy));
        if !error_rate_healthy {
            issues.push(format!(
                "High error rate: {:.1}% (max: {:.1}%)",
                error_rate * 100.0,
                self.thresholds.max_error_rate * 100.0
            ));
        }

        let avg_latency = self.calculate_avg_latency().await;
        let latency_healthy = avg_latency <= self.thresholds.max_iteration_time_ms as f64;
        metrics.push(HealthMetric::new(
            "avg_latency_ms",
            avg_latency,
            latency_healthy,
        ));
        if !latency_healthy {
            issues.push(format!(
                "High latency: {:.0}ms (max: {}ms)",
                avg_latency, self.thresholds.max_iteration_time_ms
            ));
        }

        let status = if issues.is_empty() {
            {
                let mut u = self.unhealthy_count.write().await;
                *u = 0;
            }
            HealthStatus::Healthy
        } else if issues.len() == 1 {
            {
                let mut u = self.unhealthy_count.write().await;
                *u = 0;
            }
            HealthStatus::Degraded
        } else {
            {
                let mut u = self.unhealthy_count.write().await;
                *u += 1;
            }
            HealthStatus::Unhealthy
        };

        {
            let mut s = self.status.write().await;
            *s = status;
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        let message = if issues.is_empty() {
            "All health checks passed".to_string()
        } else {
            issues.join("; ")
        };

        let result = HealthCheckResult {
            status,
            message,
            metrics,
            timestamp: Instant::now(),
            duration_ms,
        };

        if let Err(e) = self.event_bus.emit(UnifiedAgentEvent::new(
            "health_checker",
            AgentEventType::Debug,
            serde_json::json!({
                "status": result.status.to_string(),
                "message": result.message,
                "duration_ms": result.duration_ms,
                "metrics_count": result.metrics.len(),
            }),
        )) {
            tracing::warn!("Failed to emit health check event: {:?}", e);
        }

        result
    }

    pub async fn record_operation(
        &self,
        duration_ms: u64,
        success: bool,
        operation_type: impl Into<String>,
    ) {
        let mut ops = self.recent_operations.write().await;
        ops.push_back(OperationRecord {
            timestamp: Instant::now(),
            duration_ms,
            success,
            operation_type: operation_type.into(),
        });

        if ops.len() > self.max_operation_history {
            ops.pop_front();
        }
    }

    async fn calculate_throughput(&self) -> f64 {
        let ops = self.recent_operations.read().await;
        let recent_window = ops
            .iter()
            .filter(|op| op.timestamp.elapsed() < Duration::from_secs(60))
            .count();

        recent_window as f64 / 60.0
    }

    async fn calculate_error_rate(&self) -> f64 {
        let ops = self.recent_operations.read().await;
        let recent_ops: Vec<_> = ops
            .iter()
            .filter(|op| op.timestamp.elapsed() < Duration::from_secs(300))
            .collect();

        if recent_ops.is_empty() {
            return 0.0;
        }

        let errors = recent_ops.iter().filter(|op| !op.success).count();
        errors as f64 / recent_ops.len() as f64
    }

    async fn calculate_avg_latency(&self) -> f64 {
        let ops = self.recent_operations.read().await;
        let recent_ops: Vec<_> = ops
            .iter()
            .filter(|op| op.timestamp.elapsed() < Duration::from_secs(60))
            .collect();

        if recent_ops.is_empty() {
            return 0.0;
        }

        let total: u64 = recent_ops.iter().map(|op| op.duration_ms).sum();
        total as f64 / recent_ops.len() as f64
    }

    pub async fn get_status(&self) -> HealthStatus {
        self.status.read().await.clone()
    }

    pub async fn get_check_count(&self) -> u64 {
        *self.check_count.read().await
    }

    pub async fn get_unhealthy_count(&self) -> u64 {
        *self.unhealthy_count.read().await
    }

    pub async fn get_operation_stats(&self) -> std::collections::HashMap<String, OperationStats> {
        let ops = self.recent_operations.read().await;
        let mut stats_map: std::collections::HashMap<String, OperationStats> = std::collections::HashMap::new();

        for op in ops.iter() {
            let stats = stats_map.entry(op.operation_type.clone()).or_insert_with(|| OperationStats {
                count: 0,
                success_count: 0,
                failure_count: 0,
                total_duration_ms: 0,
            });
            stats.count += 1;
            stats.total_duration_ms += op.duration_ms;
            if op.success {
                stats.success_count += 1;
            } else {
                stats.failure_count += 1;
            }
        }

        stats_map
    }

    pub fn thresholds(&self) -> &HealthThresholds {
        &self.thresholds
    }
}

#[derive(Debug, Clone)]
pub struct OperationStats {
    pub count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub total_duration_ms: u64,
}

pub struct HealthCheckRunner {
    checker: Arc<HealthChecker>,
    interval_secs: u64,
}

impl HealthCheckRunner {
    pub fn new(checker: Arc<HealthChecker>, interval_secs: u64) -> Self {
        Self {
            checker,
            interval_secs,
        }
    }

    pub async fn run<F>(self, on_unhealthy: F)
    where
        F: Fn(HealthCheckResult) + Send + Clone + Sync + 'static,
    {
        let on_unhealthy = Arc::new(on_unhealthy);
        let mut ticker = interval(Duration::from_secs(self.interval_secs));

        loop {
            ticker.tick().await;

            let result = self.checker.check().await;

            if matches!(result.status, HealthStatus::Unhealthy) {
                tracing::warn!(
                    "Health check failed: {} (check #{})",
                    result.message,
                    self.checker.get_check_count().await
                );

                let on_unhealthy = on_unhealthy.clone();
                tokio::spawn({
                    let result = result.clone();
                    async move {
                        on_unhealthy(result);
                    }
                });
            } else if matches!(result.status, HealthStatus::Degraded) {
                tracing::debug!(
                    "Health check degraded: {}",
                    result.message
                );
            }
        }
    }
}

impl std::fmt::Debug for HealthChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HealthChecker")
            .field("thresholds", &self.thresholds)
            .field("max_operation_history", &self.max_operation_history)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_event_bus() -> Arc<AgentEventBus> {
        Arc::new(AgentEventBus::new("test"))
    }

    #[tokio::test]
    async fn test_healthy_status() {
        let bus = create_test_event_bus();
        let checker = HealthChecker::new(bus);

        checker
            .record_operation(100, true, "test_op")
            .await;

        let result = checker.check().await;
        assert!(matches!(result.status, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_error_rate_detection() {
        let bus = create_test_event_bus();
        let checker = HealthChecker::new(bus);

        for _ in 0..10 {
            checker.record_operation(100, false, "failing_op").await;
        }

        let result = checker.check().await;
        assert!(matches!(result.status, HealthStatus::Unhealthy | HealthStatus::Degraded));
    }

    #[tokio::test]
    async fn test_throughput_calculation() {
        let bus = create_test_event_bus();
        let checker = HealthChecker::new(bus);

        for _ in 0..10 {
            checker.record_operation(100, true, "test_op").await;
        }

        let throughput = checker.calculate_throughput().await;
        assert!(throughput > 0.0);
    }

    #[tokio::test]
    async fn test_unhealthy_count() {
        let bus = create_test_event_bus();
        let checker = HealthChecker::new(bus);

        for _ in 0..5 {
            checker.record_operation(100, false, "op").await;
            checker.check().await;
        }

        let unhealthy_count = checker.get_unhealthy_count().await;
        assert!(unhealthy_count > 0);
    }
}
