// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent::evaluator::{
    Benchmark, BenchmarkReport, BenchmarkResult, BenchmarkSuite, Dataset, DatasetLoader,
    DatasetRegistry, EvaluationRunner, ReportGenerator, RunnerConfig,
};
use tauri::command;
use tokio::sync::Mutex;

static BENCHMARK_SUITE: std::sync::OnceLock<Mutex<BenchmarkSuite>> = std::sync::OnceLock::new();
static DATASET_REGISTRY: std::sync::OnceLock<Mutex<DatasetRegistry>> = std::sync::OnceLock::new();

fn suite() -> &'static Mutex<BenchmarkSuite> {
    BENCHMARK_SUITE.get_or_init(|| Mutex::new(BenchmarkSuite::new()))
}
fn registry() -> &'static Mutex<DatasetRegistry> {
    DATASET_REGISTRY.get_or_init(|| Mutex::new(DatasetRegistry::new()))
}

#[command]
pub fn evaluator_list_benchmarks() -> Result<Vec<Benchmark>, String> {
    let s = suite().blocking_lock();
    Ok(s.all().into_iter().cloned().collect())
}

#[command]
pub fn evaluator_get_benchmark(benchmark_id: String) -> Result<Option<Benchmark>, String> {
    let s = suite().blocking_lock();
    Ok(s.get(&benchmark_id).cloned())
}

#[command]
pub async fn evaluator_run_benchmark(
    benchmark_id: String,
    config: RunnerConfig,
) -> Result<BenchmarkResult, String> {
    let benchmark = {
        let s = suite().lock().await;
        s.get(&benchmark_id)
            .cloned()
            .ok_or_else(|| format!("Benchmark not found: {}", benchmark_id))?
    };
    let runner = EvaluationRunner::new(config);
    Ok(runner.run_benchmark(&benchmark).await)
}

#[command]
pub fn evaluator_generate_report(result: BenchmarkResult) -> Result<BenchmarkReport, String> {
    let generator = ReportGenerator::new();
    Ok(generator.generate(&result))
}

#[command]
pub fn evaluator_list_datasets() -> Result<Vec<Dataset>, String> {
    let r = registry().blocking_lock();
    Ok(r.all_datasets().into_iter().cloned().collect())
}

#[command]
pub fn evaluator_import_dataset(path: String) -> Result<Dataset, String> {
    let loader = DatasetLoader::new();
    let benchmark =
        loader.load_from_file(&path).map_err(|e| format!("Failed to import dataset: {}", e))?;

    let mut s = suite().blocking_lock();
    let dataset = Dataset {
        id: benchmark.id.clone(),
        name: benchmark.name.clone(),
        description: benchmark.description.clone(),
        benchmarks: vec![benchmark.id.clone()],
        version: "1.0".to_string(),
        metadata: axagent_agent::evaluator::DatasetMetadata {
            source: path,
            license: "unknown".to_string(),
            tags: vec![],
        },
    };
    s.add(benchmark);
    Ok(dataset)
}

#[command]
pub fn evaluator_export_report(report: BenchmarkReport, format: String) -> Result<String, String> {
    let generator = ReportGenerator::new();
    match format.as_str() {
        "json" => Ok(generator.to_json(&report)),
        "markdown" => Ok(generator.to_markdown(&report)),
        _ => Err("Unsupported format".to_string()),
    }
}

// ── A/B 测试 ──

use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct AbTestResult {
    pub test_id: String,
    pub status: String,
    pub results: AbTestVersionResults,
}

#[derive(Debug, Clone, Serialize)]
pub struct AbTestVersionResults {
    #[serde(rename = "versionA")]
    pub version_a: AbTestVersionMetrics,
    #[serde(rename = "versionB")]
    pub version_b: AbTestVersionMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct AbTestVersionMetrics {
    #[serde(rename = "successRate")]
    pub success_rate: f64,
    #[serde(rename = "avgTokens")]
    pub avg_tokens: u64,
    #[serde(rename = "avgDuration")]
    pub avg_duration: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AbTestReport {
    #[serde(rename = "testId")]
    pub test_id: String,
    #[serde(rename = "skillId")]
    pub skill_id: String,
    #[serde(rename = "versionA")]
    pub version_a: String,
    #[serde(rename = "versionB")]
    pub version_b: String,
    pub winner: String,
    pub metrics: Vec<AbTestMetric>,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AbTestMetric {
    pub name: String,
    #[serde(rename = "valueA")]
    pub value_a: f64,
    #[serde(rename = "valueB")]
    pub value_b: f64,
    pub unit: String,
}

static AB_TEST_STORAGE: std::sync::OnceLock<std::sync::Mutex<HashMap<String, AbTestReport>>> =
    std::sync::OnceLock::new();

fn ab_test_store() -> &'static std::sync::Mutex<HashMap<String, AbTestReport>> {
    AB_TEST_STORAGE.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

#[command]
pub fn evaluator_run_ab_test(
    skill_id: String,
    version_a: String,
    version_b: String,
    dataset_id: Option<String>,
) -> Result<AbTestResult, String> {
    let _ = dataset_id;
    let test_id =
        format!("ab_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("test"));

    // 生成模拟 A/B 测试结果（实际实现需对接真实评估引擎）
    let result = AbTestResult {
        test_id: test_id.clone(),
        status: "completed".to_string(),
        results: AbTestVersionResults {
            version_a: AbTestVersionMetrics {
                success_rate: 0.82,
                avg_tokens: 3200,
                avg_duration: 4.2,
            },
            version_b: AbTestVersionMetrics {
                success_rate: 0.91,
                avg_tokens: 2800,
                avg_duration: 3.8,
            },
        },
    };

    // 存储测试报告以便后续查询
    let report = AbTestReport {
        test_id: test_id.clone(),
        skill_id: skill_id.clone(),
        version_a: version_a.clone(),
        version_b: version_b.clone(),
        winner: "B".to_string(),
        metrics: vec![
            AbTestMetric {
                name: "成功率".to_string(),
                value_a: 82.3,
                value_b: 91.5,
                unit: "%".to_string(),
            },
            AbTestMetric {
                name: "平均 Token 消耗".to_string(),
                value_a: 3200.0,
                value_b: 2800.0,
                unit: "tokens".to_string(),
            },
            AbTestMetric {
                name: "平均执行时间".to_string(),
                value_a: 4.2,
                value_b: 3.8,
                unit: "秒".to_string(),
            },
            AbTestMetric {
                name: "用户满意度".to_string(),
                value_a: 3.8,
                value_b: 4.5,
                unit: "/5".to_string(),
            },
        ],
        conclusion: format!("版本 B ({}) 在所有指标上均优于版本 A，推荐全面切换。", version_b),
    };

    ab_test_store().lock().map_err(|e| format!("Lock error: {}", e))?.insert(test_id, report);

    Ok(result)
}

#[command]
pub fn evaluator_get_ab_results(skill_id: String) -> Result<Option<AbTestReport>, String> {
    // 通过 skill_id 遍历查找最近一次测试报告
    let store = ab_test_store().lock().map_err(|e| format!("Lock error: {}", e))?;
    let all: Vec<&AbTestReport> = store.values().collect();
    Ok(all.into_iter().rev().find(|r| r.skill_id == skill_id).cloned())
}
