use axagent_agent::evaluator::{
    Benchmark, BenchmarkReport, BenchmarkResult, Dataset, RunnerConfig,
};
use chrono::Utc;
use tauri::command;

#[command]
pub fn evaluator_list_benchmarks() -> Result<Vec<Benchmark>, String> {
    Ok(vec![])
}

#[command]
pub fn evaluator_get_benchmark(benchmark_id: String) -> Result<Option<Benchmark>, String> {
    let _ = benchmark_id;
    Ok(None)
}

#[command]
pub fn evaluator_run_benchmark(
    benchmark_id: String,
    config: RunnerConfig,
) -> Result<BenchmarkResult, String> {
    Ok(BenchmarkResult {
        benchmark_id,
        benchmark_name: "".to_string(),
        run_at: Utc::now(),
        config,
        task_results: vec![],
        aggregate: axagent_agent::evaluator::AggregateMetrics {
            total_tasks: 0,
            passed_tasks: 0,
            failed_tasks: 0,
            pass_rate: 0.0,
            avg_duration_ms: 0.0,
            avg_score: 0.0,
            score_breakdown: std::collections::HashMap::new(),
            difficulty_distribution: std::collections::HashMap::new(),
        },
        duration_ms: 0,
    })
}

#[command]
pub fn evaluator_generate_report(result: BenchmarkResult) -> Result<BenchmarkReport, String> {
    Ok(BenchmarkReport {
        benchmark_id: result.benchmark_id,
        benchmark_name: result.benchmark_name,
        generated_at: Utc::now(),
        summary: axagent_agent::evaluator::ReportSummary {
            total_tasks: result.aggregate.total_tasks,
            passed_tasks: result.aggregate.passed_tasks,
            failed_tasks: result.aggregate.failed_tasks,
            pass_rate: result.aggregate.pass_rate,
            overall_score: result.aggregate.avg_score,
            total_duration_ms: result.duration_ms,
            avg_task_duration_ms: result.aggregate.avg_duration_ms,
        },
        task_breakdown: vec![],
        category_scores: std::collections::HashMap::new(),
        recommendations: vec![],
    })
}

#[command]
pub fn evaluator_list_datasets() -> Result<Vec<Dataset>, String> {
    Ok(vec![])
}

#[command]
pub fn evaluator_import_dataset(path: String) -> Result<Dataset, String> {
    let _ = path;
    Ok(Dataset {
        id: "imported".to_string(),
        name: "Imported Dataset".to_string(),
        description: "".to_string(),
        benchmarks: vec![],
        version: "1.0".to_string(),
        metadata: axagent_agent::evaluator::DatasetMetadata {
            source: "imported".to_string(),
            license: "unknown".to_string(),
            tags: vec![],
        },
    })
}

#[command]
pub fn evaluator_export_report(report: BenchmarkReport, format: String) -> Result<String, String> {
    match format.as_str() {
        "json" => serde_json::to_string_pretty(&report).map_err(|e| e.to_string()),
        "markdown" => Ok(format_report_markdown(&report)),
        _ => Err("Unsupported format".to_string()),
    }
}

fn format_report_markdown(report: &BenchmarkReport) -> String {
    let mut md = format!("# Benchmark Report: {}\n\n", report.benchmark_name);
    md.push_str(&format!("Generated: {}\n\n", report.generated_at));
    md.push_str("## Summary\n\n");
    md.push_str(&format!("- Total Tasks: {}\n", report.summary.total_tasks));
    md.push_str(&format!(
        "- Passed: {} ({:.1}%)\n",
        report.summary.passed_tasks,
        report.summary.pass_rate * 100.0
    ));
    md.push_str(&format!(
        "- Overall Score: {:.1}%\n",
        report.summary.overall_score * 100.0
    ));
    md
}
