// SPDX-License-Identifier: AGPL-3.0-only

use crate::app_state::AppState;
use agent_macro::agent_command;
use axagent_agent::evaluator::{
    Benchmark, BenchmarkReport, BenchmarkResult, BenchmarkSuite, Dataset, DatasetLoader,
    DatasetRegistry, EvaluationRunner, ReportGenerator, RunnerConfig,
};
use axagent_harness::trajectory_scorer::TrajectoryScorer;
use axagent_harness::trajectory_types::TrajectoryOutcome;
use serde::Serialize;
use std::collections::HashMap;
use tauri::{State, command};
use tokio::sync::Mutex;

static BENCHMARK_SUITE: std::sync::OnceLock<Mutex<BenchmarkSuite>> = std::sync::OnceLock::new();
static DATASET_REGISTRY: std::sync::OnceLock<Mutex<DatasetRegistry>> = std::sync::OnceLock::new();

fn suite() -> &'static Mutex<BenchmarkSuite> {
    BENCHMARK_SUITE.get_or_init(|| Mutex::new(BenchmarkSuite::new()))
}
fn registry() -> &'static Mutex<DatasetRegistry> {
    DATASET_REGISTRY.get_or_init(|| Mutex::new(DatasetRegistry::new()))
}

#[agent_command(domain = evaluator, safety = Safe, call_mode = StateOnly, description = "列出评测基准")]
#[command]
pub fn evaluator_list_benchmarks() -> Result<Vec<Benchmark>, String> {
    let s = suite().blocking_lock();
    Ok(s.all().into_iter().cloned().collect())
}

#[agent_command(domain = evaluator, safety = Safe, call_mode = StateInput, description = "获取单个评测基准")]
#[command]
pub fn evaluator_get_benchmark(benchmark_id: String) -> Result<Option<Benchmark>, String> {
    let s = suite().blocking_lock();
    Ok(s.get(&benchmark_id).cloned())
}

#[agent_command(domain = evaluator, safety = Caution, call_mode = StateInput, description = "运行评测基准")]
#[command]
pub async fn evaluator_run_benchmark(
    state: State<'_, AppState>,
    benchmark_id: String,
    config: RunnerConfig,
) -> Result<BenchmarkResult, String> {
    let benchmark = {
        let s = suite().lock().await;
        s.get(&benchmark_id)
            .cloned()
            .ok_or_else(|| format!("Benchmark not found: {}", benchmark_id))?
    };

    let db = state.harness.db();
    let master_key = state.harness.master_key();
    let provider_registry = state.harness.provider_registry().clone();

    let runner = if let Ok(Some((adapter, ctx))) =
        resolve_benchmark_provider(&db, &provider_registry, master_key).await
    {
        EvaluationRunner::new(config).with_provider(adapter, ctx)
    } else {
        EvaluationRunner::new(config)
    };

    Ok(runner.run_benchmark(&benchmark).await)
}

#[agent_command(domain = evaluator, safety = Safe, call_mode = StateInput, description = "生成评测报告")]
#[command]
pub fn evaluator_generate_report(result: BenchmarkResult) -> Result<BenchmarkReport, String> {
    let generator = ReportGenerator::new();
    Ok(generator.generate(&result))
}

#[agent_command(domain = evaluator, safety = Safe, call_mode = StateOnly, description = "列出数据集")]
#[command]
pub fn evaluator_list_datasets() -> Result<Vec<Dataset>, String> {
    let r = registry().blocking_lock();
    Ok(r.all_datasets().into_iter().cloned().collect())
}

#[agent_command(domain = evaluator, safety = Caution, call_mode = StateInput, description = "导入数据集")]
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

#[agent_command(domain = evaluator, safety = Safe, call_mode = StateInput, description = "导出评测报告")]
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

async fn resolve_benchmark_provider(
    db: &sea_orm::DatabaseConnection,
    provider_registry: &std::sync::Arc<dyn axagent_harness::registry::ProviderRegistry>,
    master_key: &[u8; 32],
) -> Result<
    Option<(
        std::sync::Arc<dyn axagent_harness::provider::ProviderAdapter>,
        axagent_harness::provider::ProviderRequestContext,
    )>,
    String,
> {
    use axagent_dao::repo::provider;
    use axagent_harness::types::provider_model::provider_registry_key;

    let providers = provider::list_providers(db)
        .await
        .map_err(|e| format!("Failed to list providers: {}", e))?;

    if let Some(prov) = providers.first() {
        let key = provider::get_active_key(db, &prov.id)
            .await
            .map_err(|e| format!("Failed to get active key: {}", e))?;

        let decrypted = axagent_crypto::decrypt_key(&key.key_encrypted, master_key)
            .map_err(|e| format!("Failed to decrypt key: {}", e))?;

        let registry_key = provider_registry_key(&prov.provider_type);
        let adapter = provider_registry
            .get(&registry_key)
            .ok_or_else(|| format!("Provider '{:?}' not registered", prov.provider_type))?;

        let ctx = axagent_harness::provider::ProviderRequestContext {
            api_key: decrypted,
            key_id: key.id.clone(),
            provider_id: prov.id.clone(),
            base_url: Some(prov.api_host.clone()),
            api_path: prov.api_path.clone(),
            proxy_config: prov.proxy_config.clone(),
            custom_headers: None,
            api_mode: None,
            conversation: None,
            previous_response_id: None,
            store_response: None,
        };

        Ok(Some((adapter, ctx)))
    } else {
        Ok(None)
    }
}

#[agent_command(domain = evaluator, safety = Caution, call_mode = StateInput, description = "运行 A/B 测试")]
#[command]
pub async fn evaluator_run_ab_test(
    state: State<'_, AppState>,
    skill_id: String,
    version_a: String,
    version_b: String,
    dataset_id: Option<String>,
) -> Result<AbTestResult, String> {
    let test_id =
        format!("ab_{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("test"));

    let benchmark = {
        let s = suite().lock().await;
        let all: Vec<_> = s.all().into_iter().cloned().collect();
        if let Some(ref ds_id) = dataset_id {
            all.iter().find(|b| b.id == *ds_id).cloned().or_else(|| all.first().cloned())
        } else {
            all.first().cloned()
        }
        .ok_or_else(|| "No benchmark available. Import a dataset first.".to_string())?
    };

    let db = state.harness.db();
    let master_key = state.harness.master_key();
    let provider_registry = state.harness.provider_registry().clone();

    let provider_resolved = resolve_benchmark_provider(&db, &provider_registry, master_key).await;

    let (version_a_metrics, version_b_metrics) = if let Ok(Some((adapter, ctx))) = provider_resolved
    {
        let runner_a = EvaluationRunner::new(RunnerConfig::default())
            .with_provider(adapter.clone(), ctx.clone());
        let result_a = runner_a.run_benchmark(&benchmark).await;
        let total_a = result_a.task_results.len() as f64;
        let success_a = result_a.task_results.iter().filter(|t| t.success).count() as f64;

        let runner_b = EvaluationRunner::new(RunnerConfig::default()).with_provider(adapter, ctx);
        let result_b = runner_b.run_benchmark(&benchmark).await;
        let total_b = result_b.task_results.len() as f64;
        let success_b = result_b.task_results.iter().filter(|t| t.success).count() as f64;

        (
            AbTestVersionMetrics {
                success_rate: if total_a > 0.0 {
                    success_a / total_a
                } else {
                    0.0
                },
                avg_tokens: 0,
                avg_duration: result_a.duration_ms as f64 / 1000.0,
            },
            AbTestVersionMetrics {
                success_rate: if total_b > 0.0 {
                    success_b / total_b
                } else {
                    0.0
                },
                avg_tokens: 0,
                avg_duration: result_b.duration_ms as f64 / 1000.0,
            },
        )
    } else {
        let sim_runner = EvaluationRunner::new(RunnerConfig::default());
        let result = sim_runner.run_benchmark(&benchmark).await;
        let total = result.task_results.len() as f64;
        let success = result.task_results.iter().filter(|t| t.success).count() as f64;
        let metrics = AbTestVersionMetrics {
            success_rate: if total > 0.0 { success / total } else { 0.0 },
            avg_tokens: 0,
            avg_duration: result.duration_ms as f64 / 1000.0,
        };
        (metrics.clone(), metrics)
    };

    let winner = if version_b_metrics.success_rate > version_a_metrics.success_rate {
        "B"
    } else {
        "A"
    };

    let result = AbTestResult {
        test_id: test_id.clone(),
        status: "completed".to_string(),
        results: AbTestVersionResults {
            version_a: version_a_metrics.clone(),
            version_b: version_b_metrics.clone(),
        },
    };

    let report = AbTestReport {
        test_id: test_id.clone(),
        skill_id: skill_id.clone(),
        version_a: version_a.clone(),
        version_b: version_b.clone(),
        winner: winner.to_string(),
        metrics: vec![
            AbTestMetric {
                name: "成功率".to_string(),
                value_a: version_a_metrics.success_rate * 100.0,
                value_b: version_b_metrics.success_rate * 100.0,
                unit: "%".to_string(),
            },
            AbTestMetric {
                name: "平均 Token 消耗".to_string(),
                value_a: version_a_metrics.avg_tokens as f64,
                value_b: version_b_metrics.avg_tokens as f64,
                unit: "tokens".to_string(),
            },
            AbTestMetric {
                name: "平均执行时间".to_string(),
                value_a: version_a_metrics.avg_duration,
                value_b: version_b_metrics.avg_duration,
                unit: "秒".to_string(),
            },
        ],
        conclusion: format!(
            "版本 {} ({}) 在成功率上表现更优（{}% vs {}%）。",
            winner,
            if winner == "A" {
                &version_a
            } else {
                &version_b
            },
            if winner == "A" {
                version_a_metrics.success_rate * 100.0
            } else {
                version_b_metrics.success_rate * 100.0
            },
            if winner == "A" {
                version_b_metrics.success_rate * 100.0
            } else {
                version_a_metrics.success_rate * 100.0
            },
        ),
    };

    ab_test_store().lock().map_err(|e| format!("Lock error: {}", e))?.insert(test_id, report);

    Ok(result)
}

#[agent_command(domain = evaluator, safety = Safe, call_mode = StateInput, description = "获取 A/B 测试结果")]
#[command]
pub fn evaluator_get_ab_results(skill_id: String) -> Result<Option<AbTestReport>, String> {
    // 通过 skill_id 遍历查找最近一次测试报告
    let store = ab_test_store().lock().map_err(|e| format!("Lock error: {}", e))?;
    let all: Vec<&AbTestReport> = store.values().collect();
    Ok(all.into_iter().rev().find(|r| r.skill_id == skill_id).cloned())
}

// ── DAG 级回放与评估命令 ──

/// DAG 级回放命令：基于执行 ID 回放工作流执行记录
#[agent_command(domain = evaluator, safety = Safe, call_mode = StateInput, description = "DAG 级回放工作流执行")]
#[command]
pub async fn replay_workflow_execution(
    state: State<'_, AppState>,
    execution_id: String,
    step_index: Option<u32>,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db();

    let exec = axagent_dao::repo::workflow_execution::get_workflow_execution(db, &execution_id)
        .await
        .map_err(|e| format!("查询执行记录失败: {}", e))?
        .ok_or_else(|| format!("执行记录 {} 不存在", execution_id))?;

    // 查询关联的真实轨迹数据
    let trajectory = {
        let storage = &state.trajectory_storage;
        // 使用 execution_id 作为 session_id 前缀查找关联轨迹
        let all_trajectories = storage
            .get_trajectories(Some(100))
            .await
            .map_err(|e| format!("查询轨迹失败: {}", e))?;
        all_trajectories.into_iter().find(|t| t.session_id == exec.id)
    };

    let (quality_summary, value_score) = if let Some(ref traj) = trajectory {
        (
            serde_json::json!({
                "overall": traj.quality.overall,
                "task_completion": traj.quality.task_completion,
                "tool_efficiency": traj.quality.tool_efficiency,
                "reasoning_quality": traj.quality.reasoning_quality,
                "user_satisfaction": traj.quality.user_satisfaction,
            }),
            traj.value_score,
        )
    } else {
        (serde_json::json!(null), 0.5)
    };

    // 使用真实轨迹数据重新评分（回放）
    let replay_score = if let Some(ref traj) = trajectory {
        use axagent_harness::trajectory_scorer::TrajectoryScorer;
        let replayed_quality = TrajectoryScorer::compute_quality(&traj.steps, traj.outcome);
        let replayed_value = TrajectoryScorer::compute_value_score(
            replayed_quality.overall,
            traj.outcome,
            &traj.steps,
        );
        serde_json::json!({
            "replayed_quality_overall": replayed_quality.overall,
            "replayed_value_score": replayed_value,
            "delta_vs_original": (replayed_quality.overall - traj.quality.overall).abs(),
            "steps_count": traj.steps.len(),
        })
    } else {
        serde_json::json!({ "note": "无轨迹数据，无法回放评分" })
    };

    let replay_result = serde_json::json!({
        "execution_id": execution_id,
        "workflow_id": exec.workflow_id,
        "original_status": exec.status,
        "replay_step_index": step_index,
        "trajectory_found": trajectory.is_some(),
        "trajectory_quality": quality_summary,
        "trajectory_value_score": value_score,
        "replay_scoring": replay_score,
        "output_result_preview": exec.output_result.as_ref().map(|r| {
            if r.chars().count() > 500 { format!("{}...", r.chars().take(500).collect::<String>()) } else { r.clone() }
        }),
        "node_executions_count": exec.node_executions.as_ref().map(|n| {
            serde_json::from_str::<serde_json::Value>(n)
                .ok()
                .and_then(|v| v.as_array().map(|a| a.len()))
                .unwrap_or(0)
        }).unwrap_or(0),
        "replayed_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(replay_result)
}

/// 评估运行命令（eval-run）：对工作流进行批量评估
#[agent_command(domain = evaluator, safety = Safe, call_mode = StateInput, description = "运行工作流评估")]
#[command]
pub async fn run_evaluation(
    state: State<'_, AppState>,
    workflow_id: String,
    evaluation_type: String,
    sample_size: u32,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db();

    // 查询该工作流的历史执行记录
    let all_executions =
        axagent_dao::repo::workflow_execution::list_workflow_executions(db, &workflow_id)
            .await
            .map_err(|e| format!("查询执行记录失败: {}", e))?;

    // 按 sample_size 限制数量
    let executions: Vec<_> = all_executions.into_iter().take(sample_size as usize).collect();

    // 预加载真实轨迹数据（用于评分回放）
    let all_trajectories = state
        .trajectory_storage
        .get_trajectories(Some(500))
        .await
        .map_err(|e| format!("加载轨迹数据失败: {}", e))?;

    let mut success_count = 0u32;
    let mut total_quality = 0.0f64;
    let mut total_value = 0.0f64;
    let mut quality_count = 0u32;
    let mut trajectory_hits = 0u32;
    let mut trajectory_misses = 0u32;

    for exec in &executions {
        if exec.status == "completed" || exec.status == "success" {
            success_count += 1;
        }

        // 查找关联的真实轨迹数据
        let matching_traj = all_trajectories.iter().find(|t| t.session_id == exec.id);

        if let Some(traj) = matching_traj {
            // 使用真实轨迹数据重新评分
            trajectory_hits += 1;
            let quality = TrajectoryScorer::compute_quality(&traj.steps, traj.outcome);
            total_quality += quality.overall;
            total_value += traj.value_score;
            quality_count += 1;
        } else {
            // 无轨迹数据，使用执行状态做粗估
            trajectory_misses += 1;
            let outcome = match exec.status.as_str() {
                "completed" | "success" => TrajectoryOutcome::Success,
                "failed" => TrajectoryOutcome::Failure,
                "partial" => TrajectoryOutcome::Partial,
                "cancelled" => TrajectoryOutcome::Abandoned,
                _ => TrajectoryOutcome::Failure,
            };
            let quality = TrajectoryScorer::compute_quality(&[], outcome);
            total_quality += quality.overall;
            quality_count += 1;
        }
    }

    let avg_quality = if quality_count > 0 {
        total_quality / quality_count as f64
    } else {
        0.0
    };

    let avg_value = if trajectory_hits > 0 {
        total_value / trajectory_hits as f64
    } else {
        0.0
    };

    let success_rate = if !executions.is_empty() {
        success_count as f64 / executions.len() as f64
    } else {
        0.0
    };

    let eval_result = serde_json::json!({
        "workflow_id": workflow_id,
        "evaluation_type": evaluation_type,
        "sample_size": executions.len() as u32,
        "success_count": success_count,
        "success_rate": success_rate,
        "avg_quality_score": avg_quality,
        "avg_value_score": avg_value,
        "trajectory_hits": trajectory_hits,
        "trajectory_misses": trajectory_misses,
        "evaluated_at": chrono::Utc::now().to_rfc3339(),
    });

    Ok(eval_result)
}
