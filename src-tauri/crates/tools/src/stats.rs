//! ToolUsageStats - 工具使用统计
//!
//! 追踪每个工具的执行次数、成功率、平均耗时。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatCategory {
    ReadOnly,
    Write,
    Execute,
    Network,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub category: StatCategory,
    pub avg_execution_time_ms: Option<f64>,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_used: Option<DateTime<Utc>>,
}

impl ToolMetadata {
    pub fn new(name: String, category: StatCategory) -> Self {
        Self {
            name,
            category,
            avg_execution_time_ms: None,
            success_count: 0,
            failure_count: 0,
            last_used: None,
        }
    }

    pub fn record_success(&mut self, execution_time_ms: f64) {
        self.success_count += 1;
        self.last_used = Some(Utc::now());
        let n = self.success_count as f64;
        let current_avg = self.avg_execution_time_ms.unwrap_or(execution_time_ms);
        self.avg_execution_time_ms = Some(current_avg + (execution_time_ms - current_avg) / n);
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_used = Some(Utc::now());
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            1.0
        } else {
            self.success_count as f64 / total as f64
        }
    }
}

#[derive(Clone)]
pub struct ToolUsageStats {
    metrics: Arc<parking_lot::RwLock<HashMap<String, ToolMetadata>>>,
}

impl Default for ToolUsageStats {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolUsageStats {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    pub fn record_execution(
        &self,
        tool_name: &str,
        category: StatCategory,
        time_ms: f64,
        success: bool,
    ) {
        let mut m = self.metrics.write();
        let meta = m
            .entry(tool_name.to_string())
            .or_insert_with(|| ToolMetadata::new(tool_name.to_string(), category));
        if success {
            meta.record_success(time_ms);
        } else {
            meta.record_failure();
        }
    }

    pub fn get_stats(&self, tool_name: &str) -> Option<ToolMetadata> {
        self.metrics.read().get(tool_name).cloned()
    }

    pub fn all_stats(&self) -> HashMap<String, ToolMetadata> {
        self.metrics.read().clone()
    }

    pub fn top_tools(&self, limit: usize) -> Vec<(String, ToolMetadata)> {
        let metrics = self.metrics.read();
        let mut sorted: Vec<_> = metrics
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        sorted.sort_by_key(|(_, v)| std::cmp::Reverse(v.success_count));
        sorted.truncate(limit);
        sorted
    }
}
