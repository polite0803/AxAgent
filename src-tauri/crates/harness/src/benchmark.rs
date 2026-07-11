// SPDX-License-Identifier: AGPL-3.0-only
//! 基准测试契约
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub difficulty: Difficulty,
    pub category: String,
    pub input: serde_json::Value,
    pub expected_output: Option<serde_json::Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub token_count: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub suite_name: String,
    pub total_tasks: usize,
    pub passed: usize,
    pub failed: usize,
    pub avg_duration_ms: f64,
    pub tasks: Vec<TaskResult>,
    pub started_at: i64,
    pub finished_at: i64,
}

#[async_trait]
pub trait BenchmarkRunner: Send + Sync {
    async fn run_task(&self, task: &BenchmarkTask) -> Result<TaskResult, String>;
    async fn run_suite(&self, tasks: &[BenchmarkTask]) -> Result<BenchmarkReport, String>;
}
