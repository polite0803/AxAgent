use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkCategory {
    Reasoning,
    CodeGeneration,
    ToolUsage,
    Research,
    Conversation,
    ErrorRecovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Expert,
}

impl std::cmp::PartialOrd for Difficulty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let to_int = |d: &Difficulty| match d {
            Difficulty::Easy => 0,
            Difficulty::Medium => 1,
            Difficulty::Hard => 2,
            Difficulty::Expert => 3,
        };
        to_int(self).partial_cmp(&to_int(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationMetric {
    ExactMatch,
    Contains,
    LevenshteinSimilarity,
    SemanticSimilarity,
    ToolCorrectness,
    OutputFormat,
    Performance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetadata {
    pub version: String,
    pub author: String,
    pub created_at: String,
    pub tags: Vec<String>,
}

impl Default for BenchmarkMetadata {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            author: "system".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            tags: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    #[serde(default)]
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub content: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationCriteria {
    pub name: String,
    pub metric: EvaluationMetric,
    pub weight: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub input: TaskInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_output: Option<TaskOutput>,
    pub evaluation_criteria: Vec<EvaluationCriteria>,
    pub difficulty: Difficulty,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmark {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: BenchmarkCategory,
    pub tasks: Vec<BenchmarkTask>,
    pub metadata: BenchmarkMetadata,
}

pub struct BenchmarkSuite {
    benchmarks: HashMap<String, Benchmark>,
}

impl BenchmarkSuite {
    pub fn new() -> Self {
        let mut suite = Self {
            benchmarks: HashMap::new(),
        };
        suite.register_default_benchmarks();
        suite
    }

    fn register_default_benchmarks(&mut self) {
        self.benchmarks.insert(
            "reasoning".to_string(),
            Benchmark {
                id: "reasoning".to_string(),
                name: "推理能力测试".to_string(),
                description: "评估 Agent 的逻辑推理和问题分解能力".to_string(),
                category: BenchmarkCategory::Reasoning,
                tasks: vec![
                    BenchmarkTask {
                        id: "reasoning_001".to_string(),
                        name: "逻辑推理".to_string(),
                        description: "给定一组逻辑陈述，推断正确结论".to_string(),
                        input: TaskInput {
                            query: "如果 A > B 且 B > C，则 A > C。请判断：如果 x > 5 且 5 > y，则 x 和 y 的大小关系？".to_string(),
                            context: None,
                            constraints: vec!["必须给出推理过程".to_string()],
                        },
                        expected_output: Some(TaskOutput {
                            content: "x > y".to_string(),
                            format: "text".to_string(),
                        }),
                        evaluation_criteria: vec![
                            EvaluationCriteria {
                                name: "答案正确性".to_string(),
                                metric: EvaluationMetric::ExactMatch,
                                weight: 0.6,
                                threshold: Some(1.0),
                            },
                            EvaluationCriteria {
                                name: "推理过程".to_string(),
                                metric: EvaluationMetric::Contains,
                                weight: 0.4,
                                threshold: Some(0.5),
                            },
                        ],
                        difficulty: Difficulty::Medium,
                        tags: vec!["逻辑".to_string(), "推理".to_string()],
                    },
                    BenchmarkTask {
                        id: "reasoning_002".to_string(),
                        name: "问题分解".to_string(),
                        description: "将复杂问题分解为可处理的子问题".to_string(),
                        input: TaskInput {
                            query: "如何设计一个分布式缓存系统？".to_string(),
                            context: None,
                            constraints: vec!["需要考虑一致性、可用性、分区容错性".to_string()],
                        },
                        expected_output: None,
                        evaluation_criteria: vec![
                            EvaluationCriteria {
                                name: "分解完整性".to_string(),
                                metric: EvaluationMetric::Contains,
                                weight: 0.5,
                                threshold: Some(0.6),
                            },
                            EvaluationCriteria {
                                name: "技术深度".to_string(),
                                metric: EvaluationMetric::SemanticSimilarity,
                                weight: 0.5,
                                threshold: Some(0.7),
                            },
                        ],
                        difficulty: Difficulty::Hard,
                        tags: vec!["分解".to_string(), "系统设计".to_string()],
                    },
                ],
                metadata: BenchmarkMetadata::default(),
            },
        );

        self.benchmarks.insert(
            "tool_usage".to_string(),
            Benchmark {
                id: "tool_usage".to_string(),
                name: "工具使用测试".to_string(),
                description: "评估 Agent 调用和使用工具的能力".to_string(),
                category: BenchmarkCategory::ToolUsage,
                tasks: vec![BenchmarkTask {
                    id: "tool_001".to_string(),
                    name: "文件操作".to_string(),
                    description: "使用文件读写工具完成指定任务".to_string(),
                    input: TaskInput {
                        query: "读取 src/main.rs 文件，统计其中的函数数量".to_string(),
                        context: Some(serde_json::json!({
                            "workspace": "/test/project"
                        })),
                        constraints: vec![],
                    },
                    expected_output: Some(TaskOutput {
                        content: "3".to_string(),
                        format: "number".to_string(),
                    }),
                    evaluation_criteria: vec![
                        EvaluationCriteria {
                            name: "工具调用正确性".to_string(),
                            metric: EvaluationMetric::ToolCorrectness,
                            weight: 0.7,
                            threshold: Some(1.0),
                        },
                        EvaluationCriteria {
                            name: "答案准确性".to_string(),
                            metric: EvaluationMetric::ExactMatch,
                            weight: 0.3,
                            threshold: Some(1.0),
                        },
                    ],
                    difficulty: Difficulty::Easy,
                    tags: vec!["文件".to_string(), "工具".to_string()],
                }],
                metadata: BenchmarkMetadata::default(),
            },
        );

        self.benchmarks.insert(
            "code_generation".to_string(),
            Benchmark {
                id: "code_generation".to_string(),
                name: "代码生成测试".to_string(),
                description: "评估 Agent 生成代码的能力".to_string(),
                category: BenchmarkCategory::CodeGeneration,
                tasks: vec![BenchmarkTask {
                    id: "code_001".to_string(),
                    name: "简单函数生成".to_string(),
                    description: "根据描述生成正确的函数实现".to_string(),
                    input: TaskInput {
                        query: "编写一个函数，计算斐波那契数列第 n 项".to_string(),
                        context: None,
                        constraints: vec!["使用递归实现".to_string()],
                    },
                    expected_output: None,
                    evaluation_criteria: vec![
                        EvaluationCriteria {
                            name: "语法正确性".to_string(),
                            metric: EvaluationMetric::OutputFormat,
                            weight: 0.3,
                            threshold: Some(1.0),
                        },
                        EvaluationCriteria {
                            name: "逻辑正确性".to_string(),
                            metric: EvaluationMetric::Contains,
                            weight: 0.7,
                            threshold: Some(0.6),
                        },
                    ],
                    difficulty: Difficulty::Medium,
                    tags: vec!["代码".to_string(), "递归".to_string()],
                }],
                metadata: BenchmarkMetadata::default(),
            },
        );

        self.benchmarks.insert(
            "error_recovery".to_string(),
            Benchmark {
                id: "error_recovery".to_string(),
                name: "错误恢复测试".to_string(),
                description: "评估 Agent 从错误中恢复的能力".to_string(),
                category: BenchmarkCategory::ErrorRecovery,
                tasks: vec![BenchmarkTask {
                    id: "error_001".to_string(),
                    name: "错误识别与修正".to_string(),
                    description: "识别代码中的错误并提供修正方案".to_string(),
                    input: TaskInput {
                        query: "以下代码有什么问题？\nfn main() {\n    let x = 5;\n    println!(\"{}\", x);\n}".to_string(),
                        context: None,
                        constraints: vec!["需要指出具体错误位置".to_string()],
                    },
                    expected_output: None,
                    evaluation_criteria: vec![
                        EvaluationCriteria {
                            name: "错误识别".to_string(),
                            metric: EvaluationMetric::Contains,
                            weight: 0.5,
                            threshold: Some(0.5),
                        },
                        EvaluationCriteria {
                            name: "修正建议".to_string(),
                            metric: EvaluationMetric::Contains,
                            weight: 0.5,
                            threshold: Some(0.5),
                        },
                    ],
                    difficulty: Difficulty::Easy,
                    tags: vec!["错误".to_string(), "调试".to_string()],
                }],
                metadata: BenchmarkMetadata::default(),
            },
        );
    }

    pub fn get(&self, id: &str) -> Option<&Benchmark> {
        self.benchmarks.get(id)
    }

    pub fn all(&self) -> Vec<&Benchmark> {
        self.benchmarks.values().collect()
    }

    pub fn by_category(&self, category: BenchmarkCategory) -> Vec<&Benchmark> {
        self.benchmarks
            .values()
            .filter(|b| b.category == category)
            .collect()
    }

    pub fn add(&mut self, benchmark: Benchmark) {
        self.benchmarks.insert(benchmark.id.clone(), benchmark);
    }

    pub fn remove(&mut self, id: &str) -> Option<Benchmark> {
        self.benchmarks.remove(id)
    }

    pub fn len(&self) -> usize {
        self.benchmarks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.benchmarks.is_empty()
    }
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}
