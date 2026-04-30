pub mod swe_bench;
pub mod terminal_bench;

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub name: String,
    pub benchmarks: Vec<Benchmark>,
    pub metadata: BenchMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmark {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: BenchCategory,
    pub tasks: Vec<BenchTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BenchCategory {
    CodeRepair,
    CodeGeneration,
    TerminalOperations,
    WebNavigation,
    CodeReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchTask {
    pub id: String,
    pub input: String,
    pub expected_output: Option<String>,
    pub context: Option<serde_json::Value>,
    pub max_steps: usize,
    pub time_limit_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchMetadata {
    pub version: String,
    pub total_tasks: usize,
    pub created_at: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchResult {
    pub run_id: String,
    pub benchmark_id: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration: Option<Duration>,
    pub task_results: Vec<TaskResult>,
    pub summary: ResultSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub score: f64,
    pub steps_taken: usize,
    pub output: Option<String>,
    pub error: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Success,
    Failed,
    Timeout,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultSummary {
    pub total_tasks: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub timed_out: usize,
    pub pass_rate: f64,
    pub avg_score: f64,
    pub avg_steps: f64,
    pub total_duration: Option<Duration>,
}

pub trait BenchEvaluator: Send + Sync {
    fn evaluate(
        &self,
        output: &str,
        expected: Option<&str>,
        context: Option<&serde_json::Value>,
    ) -> BenchScore;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchScore {
    pub score: f64,
    pub passed: bool,
    pub details: Option<String>,
}

pub struct BenchmarkRunner {
    suites: Vec<BenchmarkSuite>,
    #[allow(dead_code)]
    evaluator: Box<dyn BenchEvaluator>,
    history: Vec<BenchResult>,
}

impl BenchmarkRunner {
    pub fn new(evaluator: Box<dyn BenchEvaluator>) -> Self {
        Self {
            suites: Vec::new(),
            evaluator,
            history: Vec::new(),
        }
    }

    pub fn register_suite(&mut self, suite: BenchmarkSuite) {
        self.suites.push(suite);
    }

    pub fn get_suite(&self, name: &str) -> Option<&BenchmarkSuite> {
        self.suites.iter().find(|s| s.name == name)
    }

    pub fn list_suites(&self) -> Vec<&BenchmarkSuite> {
        self.suites.iter().collect()
    }

    pub fn get_run_history(&self) -> &[BenchResult] {
        &self.history
    }
}
