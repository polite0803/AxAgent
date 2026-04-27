use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::evaluator::benchmark::{Benchmark, BenchmarkTask};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub benchmarks: Vec<String>,
    pub version: String,
    pub metadata: DatasetMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub source: String,
    pub license: String,
    pub tags: Vec<String>,
}

pub struct DatasetRegistry {
    datasets: HashMap<String, Dataset>,
    custom_tasks: HashMap<String, BenchmarkTask>,
}

impl DatasetRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            datasets: HashMap::new(),
            custom_tasks: HashMap::new(),
        };
        registry.register_builtin_datasets();
        registry
    }

    fn register_builtin_datasets(&mut self) {
        self.datasets.insert(
            "builtin".to_string(),
            Dataset {
                id: "builtin".to_string(),
                name: "内置基准测试".to_string(),
                description: "系统内置的标准基准测试集".to_string(),
                benchmarks: vec![
                    "reasoning".to_string(),
                    "tool_usage".to_string(),
                    "code_generation".to_string(),
                    "error_recovery".to_string(),
                ],
                version: "1.0.0".to_string(),
                metadata: DatasetMetadata {
                    source: "system".to_string(),
                    license: "MIT".to_string(),
                    tags: vec!["内置".to_string(), "标准".to_string()],
                },
            },
        );

        self.datasets.insert(
            "extended".to_string(),
            Dataset {
                id: "extended".to_string(),
                name: "扩展基准测试".to_string(),
                description: "包含更多高级任务的扩展测试集".to_string(),
                benchmarks: vec![
                    "reasoning".to_string(),
                    "tool_usage".to_string(),
                    "code_generation".to_string(),
                    "error_recovery".to_string(),
                ],
                version: "1.1.0".to_string(),
                metadata: DatasetMetadata {
                    source: "community".to_string(),
                    license: "CC BY-SA 4.0".to_string(),
                    tags: vec!["扩展".to_string(), "高级".to_string()],
                },
            },
        );
    }

    pub fn get_dataset(&self, id: &str) -> Option<&Dataset> {
        self.datasets.get(id)
    }

    pub fn all_datasets(&self) -> Vec<&Dataset> {
        self.datasets.values().collect()
    }

    pub fn register_custom_task(&mut self, task: BenchmarkTask) {
        self.custom_tasks.insert(task.id.clone(), task);
    }

    pub fn get_custom_task(&self, id: &str) -> Option<&BenchmarkTask> {
        self.custom_tasks.get(id)
    }

    pub fn list_custom_tasks(&self) -> Vec<&BenchmarkTask> {
        self.custom_tasks.values().collect()
    }

    pub fn remove_custom_task(&mut self, id: &str) -> Option<BenchmarkTask> {
        self.custom_tasks.remove(id)
    }
}

impl Default for DatasetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DatasetLoader {
    base_path: Option<PathBuf>,
}

impl DatasetLoader {
    pub fn new() -> Self {
        Self { base_path: None }
    }

    pub fn with_base_path(path: PathBuf) -> Self {
        Self {
            base_path: Some(path),
        }
    }

    pub fn load_from_file(&self, path: &str) -> Result<Benchmark, DatasetError> {
        let file_path = if let Some(ref base) = self.base_path {
            base.join(path)
        } else {
            PathBuf::from(path)
        };

        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| DatasetError::IoError(e.to_string()))?;

        serde_json::from_str(&content).map_err(|e| DatasetError::ParseError(e.to_string()))
    }

    pub fn save_to_file(&self, benchmark: &Benchmark, path: &str) -> Result<(), DatasetError> {
        let file_path = if let Some(ref base) = self.base_path {
            base.join(path)
        } else {
            PathBuf::from(path)
        };

        let content = serde_json::to_string_pretty(benchmark)
            .map_err(|e| DatasetError::SerializeError(e.to_string()))?;

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DatasetError::IoError(e.to_string()))?;
        }

        std::fs::write(&file_path, content).map_err(|e| DatasetError::IoError(e.to_string()))?;

        Ok(())
    }
}

impl Default for DatasetLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Serialize error: {0}")]
    SerializeError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl Clone for DatasetError {
    fn clone(&self) -> Self {
        match self {
            DatasetError::IoError(msg) => DatasetError::IoError(msg.clone()),
            DatasetError::ParseError(msg) => DatasetError::ParseError(msg.clone()),
            DatasetError::SerializeError(msg) => DatasetError::SerializeError(msg.clone()),
            DatasetError::ValidationError(msg) => DatasetError::ValidationError(msg.clone()),
        }
    }
}

pub fn validate_task(task: &BenchmarkTask) -> Result<(), DatasetError> {
    if task.id.is_empty() {
        return Err(DatasetError::ValidationError(
            "Task ID cannot be empty".to_string(),
        ));
    }
    if task.name.is_empty() {
        return Err(DatasetError::ValidationError(
            "Task name cannot be empty".to_string(),
        ));
    }
    if task.input.query.is_empty() {
        return Err(DatasetError::ValidationError(
            "Task query cannot be empty".to_string(),
        ));
    }
    if task.evaluation_criteria.is_empty() {
        return Err(DatasetError::ValidationError(
            "Task must have at least one evaluation criteria".to_string(),
        ));
    }
    let total_weight: f32 = task.evaluation_criteria.iter().map(|c| c.weight).sum();
    if (total_weight - 1.0).abs() > 0.01 {
        return Err(DatasetError::ValidationError(format!(
            "Evaluation criteria weights must sum to 1.0, got {}",
            total_weight
        )));
    }
    Ok(())
}

pub fn validate_benchmark(benchmark: &Benchmark) -> Result<(), DatasetError> {
    if benchmark.id.is_empty() {
        return Err(DatasetError::ValidationError(
            "Benchmark ID cannot be empty".to_string(),
        ));
    }
    if benchmark.tasks.is_empty() {
        return Err(DatasetError::ValidationError(
            "Benchmark must have at least one task".to_string(),
        ));
    }
    for task in &benchmark.tasks {
        validate_task(task)?;
    }
    Ok(())
}
