use prometheus::{Histogram, HistogramOpts, IntCounterVec, IntGauge, Opts, Registry};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MetricsCollector {
    registry: Registry,
    workflow_executions_total: IntCounterVec,
    workflow_execution_duration: Histogram,
    workflow_node_executions_total: IntCounterVec,
    llm_requests_total: IntCounterVec,
    rag_retrieval_total: IntCounterVec,
    active_workflows: IntGauge,
    app_metrics: Arc<RwLock<AppMetricsSnapshot>>,
}

#[derive(Debug, Clone)]
pub struct AppMetricsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_tokens: u64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        let registry = Registry::new();

        let workflow_executions_total = IntCounterVec::new(
            Opts::new("workflow_executions_total", "Total workflow executions"),
            &["status"],
        )
        .expect("Failed to create workflow_executions_total counter");

        let workflow_execution_duration = Histogram::with_opts(
            HistogramOpts::new(
                "workflow_execution_duration_seconds",
                "Workflow execution duration in seconds",
            )
            .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0]),
        )
        .expect("Failed to create workflow_execution_duration histogram");

        let workflow_node_executions_total = IntCounterVec::new(
            Opts::new(
                "workflow_node_executions_total",
                "Total workflow node executions",
            ),
            &["node_type", "status"],
        )
        .expect("Failed to create workflow_node_executions_total counter");

        let llm_requests_total = IntCounterVec::new(
            Opts::new("llm_requests_total", "Total LLM requests"),
            &["model", "status"],
        )
        .expect("Failed to create llm_requests_total counter");

        let rag_retrieval_total = IntCounterVec::new(
            Opts::new("rag_retrieval_total", "Total RAG retrieval requests"),
            &["status"],
        )
        .expect("Failed to create rag_retrieval_total counter");

        let active_workflows = IntGauge::new("active_workflows", "Number of active workflows")
            .expect("Failed to create active_workflows gauge");

        registry
            .register(Box::new(workflow_executions_total.clone()))
            .expect("Failed to register workflow_executions_total");
        registry
            .register(Box::new(workflow_execution_duration.clone()))
            .expect("Failed to register workflow_execution_duration");
        registry
            .register(Box::new(workflow_node_executions_total.clone()))
            .expect("Failed to register workflow_node_executions_total");
        registry
            .register(Box::new(llm_requests_total.clone()))
            .expect("Failed to register llm_requests_total");
        registry
            .register(Box::new(rag_retrieval_total.clone()))
            .expect("Failed to register rag_retrieval_total");
        registry
            .register(Box::new(active_workflows.clone()))
            .expect("Failed to register active_workflows");

        Self {
            registry,
            workflow_executions_total,
            workflow_execution_duration,
            workflow_node_executions_total,
            llm_requests_total,
            rag_retrieval_total,
            active_workflows,
            app_metrics: Arc::new(RwLock::new(AppMetricsSnapshot {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                total_tokens: 0,
            })),
        }
    }

    pub fn inc_workflow_execution(&self, status: &str) {
        self.workflow_executions_total
            .with_label_values(&[status])
            .inc();
    }

    pub fn observe_workflow_duration(&self, duration_secs: f64) {
        self.workflow_execution_duration.observe(duration_secs);
    }

    pub fn inc_workflow_node_execution(&self, node_type: &str, status: &str) {
        self.workflow_node_executions_total
            .with_label_values(&[node_type, status])
            .inc();
    }

    pub fn inc_llm_request(&self, model: &str, status: &str) {
        self.llm_requests_total
            .with_label_values(&[model, status])
            .inc();
    }

    pub fn inc_rag_retrieval(&self, status: &str) {
        self.rag_retrieval_total.with_label_values(&[status]).inc();
    }

    pub fn set_active_workflows(&self, count: i64) {
        self.active_workflows.set(count);
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub async fn get_app_metrics_snapshot(&self) -> AppMetricsSnapshot {
        self.app_metrics.read().await.clone()
    }

    pub async fn record_request(&self, success: bool, tokens: u64) {
        let mut metrics = self.app_metrics.write().await;
        metrics.total_requests += 1;
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }
        metrics.total_tokens += tokens;
    }
}

impl Clone for MetricsCollector {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
            workflow_executions_total: self.workflow_executions_total.clone(),
            workflow_execution_duration: self.workflow_execution_duration.clone(),
            workflow_node_executions_total: self.workflow_node_executions_total.clone(),
            llm_requests_total: self.llm_requests_total.clone(),
            rag_retrieval_total: self.rag_retrieval_total.clone(),
            active_workflows: self.active_workflows.clone(),
            app_metrics: self.app_metrics.clone(),
        }
    }
}
