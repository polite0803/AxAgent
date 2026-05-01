use axagent_agent::evaluator::{
    Benchmark, BenchmarkReport, BenchmarkResult, BenchmarkSuite, Dataset,
    DatasetLoader, DatasetRegistry, EvaluationRunner, ReportGenerator, RunnerConfig,
};
use std::sync::Mutex;
use tauri::command;

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
    let s = suite().lock().map_err(|e| e.to_string())?;
    Ok(s.all().into_iter().cloned().collect())
}

#[command]
pub fn evaluator_get_benchmark(benchmark_id: String) -> Result<Option<Benchmark>, String> {
    let s = suite().lock().map_err(|e| e.to_string())?;
    Ok(s.get(&benchmark_id).cloned())
}

#[command]
pub async fn evaluator_run_benchmark(
    benchmark_id: String,
    config: RunnerConfig,
) -> Result<BenchmarkResult, String> {
    let benchmark = {
        let s = suite().lock().map_err(|e| e.to_string())?;
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
    let r = registry().lock().map_err(|e| e.to_string())?;
    Ok(r.all_datasets().into_iter().cloned().collect())
}

#[command]
pub fn evaluator_import_dataset(path: String) -> Result<Dataset, String> {
    let loader = DatasetLoader::new();
    let benchmark = loader
        .load_from_file(&path)
        .map_err(|e| format!("Failed to import dataset: {}", e))?;

    let mut s = suite().lock().map_err(|e| e.to_string())?;
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
