// SPDX-License-Identifier: AGPL-3.0-only

//! 训练数据管线 (P1-9)
//!
//! 借鉴 Hermes Agent 的轨迹处理：
//! - 批量轨迹生成的配置 DTO
//! - 压缩管线配置
//! - 质量评估指标

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 批量生成配置
// ---------------------------------------------------------------------------

/// 批量轨迹生成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGenerationConfig {
    /// 会话 ID 列表
    pub session_ids: Vec<String>,
    /// 质量阈值（0-1）
    pub quality_threshold: f64,
    /// 采样策略
    pub sampling_strategy: SamplingStrategy,
    /// 采样数量
    pub sample_size: usize,
    /// 最大并发处理数
    pub max_concurrent: usize,
    /// 是否包含部分成功的轨迹
    pub include_partial: bool,
    /// 是否包含失败的轨迹
    pub include_failure: bool,
    /// 最小轨迹长度（步数）
    pub min_steps: usize,
    /// 最大轨迹长度（步数）
    pub max_steps: Option<usize>,
    /// 时间范围过滤
    pub time_range: Option<TimeRangeFilter>,
}

impl Default for BatchGenerationConfig {
    fn default() -> Self {
        Self {
            session_ids: Vec::new(),
            quality_threshold: 0.5,
            sampling_strategy: SamplingStrategy::Random,
            sample_size: 100,
            max_concurrent: 10,
            include_partial: true,
            include_failure: false,
            min_steps: 3,
            max_steps: Some(100),
            time_range: None,
        }
    }
}

/// 时间范围过滤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRangeFilter {
    pub from_timestamp_ms: Option<i64>,
    pub to_timestamp_ms: Option<i64>,
}

/// 采样策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplingStrategy {
    /// 随机采样
    Random,
    /// Top-K 采样（按质量排序）
    TopK,
    /// 阈值采样（按质量过滤）
    Threshold,
    /// 分层采样（按结果类型分层）
    Stratified,
}

// ---------------------------------------------------------------------------
// 压缩管线配置
// ---------------------------------------------------------------------------

/// 训练数据压缩管线
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionPipeline {
    /// 管线 ID
    pub id: String,
    /// 管线名称
    pub name: String,
    /// 管线描述
    pub description: String,
    /// 压缩步骤列表
    pub steps: Vec<CompressionStep>,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 压缩步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤类型
    pub step_type: CompressionStepType,
    /// 步骤配置
    pub config: serde_json::Value,
    /// 是否启用
    pub enabled: bool,
}

/// 压缩步骤类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionStepType {
    /// 长度过滤
    LengthFilter,
    /// 质量过滤
    QualityFilter,
    /// 去重
    Deduplication,
    /// 轨迹截断
    Truncation,
    /// 摘要压缩
    SummaryCompression,
    /// 工具调用简化
    ToolCallSimplification,
    /// 噪声移除
    NoiseRemoval,
    /// 格式化
    Formatting,
}

impl CompressionStepType {
    pub fn default_config(&self) -> serde_json::Value {
        match self {
            CompressionStepType::LengthFilter => {
                serde_json::json!({
                    "min_steps": 3,
                    "max_steps": 50,
                    "truncate_strategy": "tail"
                })
            },
            CompressionStepType::QualityFilter => {
                serde_json::json!({
                    "min_quality": 0.5,
                    "min_value_score": 0.3,
                    "outcome_filter": ["success", "partial"]
                })
            },
            CompressionStepType::Deduplication => {
                serde_json::json!({
                    "method": "exact_match",
                    "similarity_threshold": 0.95,
                    "compare_fields": ["topic", "tool_sequence"]
                })
            },
            CompressionStepType::Truncation => {
                serde_json::json!({
                    "max_total_tokens": 8000,
                    "preserve_system_prompt": true,
                    "preserve_key_steps": true
                })
            },
            CompressionStepType::SummaryCompression => {
                serde_json::json!({
                    "max_summary_length": 200,
                    "preserve_key_decisions": true
                })
            },
            CompressionStepType::ToolCallSimplification => {
                serde_json::json!({
                    "remove_redundant_calls": true,
                    "merge_sequential_calls": false,
                    "minimize_tool_args": true
                })
            },
            CompressionStepType::NoiseRemoval => {
                serde_json::json!({
                    "remove_error_messages": false,
                    "remove_debug_output": true,
                    "minimize_reasoning": "summarize"
                })
            },
            CompressionStepType::Formatting => {
                serde_json::json!({
                    "format": "jsonl",
                    "include_metadata": true,
                    "normalize_roles": true
                })
            },
        }
    }
}

impl CompressionPipeline {
    /// 创建默认管线
    pub fn default_pipeline() -> Self {
        let now = chrono::Utc::now().to_rfc3339();

        Self {
            id: "pipeline-default".to_string(),
            name: "默认训练数据管线".to_string(),
            description: "标准训练数据压缩管线，包括质量过滤、去重和格式化".to_string(),
            steps: vec![
                CompressionStep {
                    id: "step-1".to_string(),
                    step_type: CompressionStepType::QualityFilter,
                    config: CompressionStepType::QualityFilter.default_config(),
                    enabled: true,
                },
                CompressionStep {
                    id: "step-2".to_string(),
                    step_type: CompressionStepType::LengthFilter,
                    config: CompressionStepType::LengthFilter.default_config(),
                    enabled: true,
                },
                CompressionStep {
                    id: "step-3".to_string(),
                    step_type: CompressionStepType::Deduplication,
                    config: CompressionStepType::Deduplication.default_config(),
                    enabled: true,
                },
                CompressionStep {
                    id: "step-4".to_string(),
                    step_type: CompressionStepType::NoiseRemoval,
                    config: CompressionStepType::NoiseRemoval.default_config(),
                    enabled: true,
                },
                CompressionStep {
                    id: "step-5".to_string(),
                    step_type: CompressionStepType::Formatting,
                    config: CompressionStepType::Formatting.default_config(),
                    enabled: true,
                },
            ],
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 创建轻量管线（用于快速迭代）
    pub fn lightweight_pipeline() -> Self {
        let now = chrono::Utc::now().to_rfc3339();

        Self {
            id: "pipeline-lightweight".to_string(),
            name: "轻量训练数据管线".to_string(),
            description: "快速训练数据管线，仅包含核心过滤步骤".to_string(),
            steps: vec![
                CompressionStep {
                    id: "step-1".to_string(),
                    step_type: CompressionStepType::QualityFilter,
                    config: serde_json::json!({
                        "min_quality": 0.7,
                        "outcome_filter": ["success"]
                    }),
                    enabled: true,
                },
                CompressionStep {
                    id: "step-2".to_string(),
                    step_type: CompressionStepType::Formatting,
                    config: CompressionStepType::Formatting.default_config(),
                    enabled: true,
                },
            ],
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 获取启用的步骤
    pub fn enabled_steps(&self) -> Vec<&CompressionStep> {
        self.steps.iter().filter(|s| s.enabled).collect()
    }

    /// 添加步骤
    pub fn add_step(&mut self, step_type: CompressionStepType) {
        let step_id = format!("step-{}", self.steps.len() + 1);
        self.steps.push(CompressionStep {
            id: step_id,
            step_type,
            config: step_type.default_config(),
            enabled: true,
        });
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// 移除步骤
    pub fn remove_step(&mut self, step_id: &str) -> bool {
        let before = self.steps.len();
        self.steps.retain(|s| s.id != step_id);
        if self.steps.len() < before {
            self.updated_at = chrono::Utc::now().to_rfc3339();
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// 质量评估指标
// ---------------------------------------------------------------------------

/// 训练数据质量报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataReport {
    /// 报告 ID
    pub id: String,
    /// 生成时间
    pub generated_at: String,
    /// 数据源
    pub data_source: String,
    /// 管线 ID
    pub pipeline_id: String,
    /// 原始数据统计
    pub raw_stats: DataStats,
    /// 压缩后数据统计
    pub compressed_stats: DataStats,
    /// 压缩比
    pub compression_ratio: f64,
    /// 质量指标
    pub quality_metrics: TrainingQualityMetrics,
    /// 建议
    pub recommendations: Vec<String>,
}

/// 数据统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataStats {
    /// 轨迹数量
    pub trajectory_count: usize,
    /// 总步数
    pub total_steps: usize,
    /// 总 token 数
    pub total_tokens: usize,
    /// 平均每轨迹步数
    pub avg_steps_per_trajectory: f64,
    /// 成功率
    pub success_rate: f64,
}

/// 质量指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingQualityMetrics {
    /// 平均质量分数
    pub avg_quality: f64,
    /// 平均价值分数
    pub avg_value_score: f64,
    /// 结果分布
    pub outcome_distribution: HashMap<String, usize>,
    /// 工具使用多样性
    pub tool_diversity: f64,
    /// 推理深度
    pub reasoning_depth: f64,
}

impl TrainingDataReport {
    /// 生成报告
    pub fn generate(
        raw_stats: DataStats,
        compressed_stats: DataStats,
        quality_metrics: TrainingQualityMetrics,
        pipeline_id: &str,
        data_source: &str,
    ) -> Self {
        let compression_ratio = if raw_stats.total_tokens > 0 {
            1.0 - (compressed_stats.total_tokens as f64 / raw_stats.total_tokens as f64)
        } else {
            0.0
        };

        let mut recommendations = Vec::new();

        if quality_metrics.avg_quality < 0.6 {
            recommendations.push("平均质量分数偏低，建议提高质量阈值".to_string());
        }

        if quality_metrics.success_rate() < 0.7 {
            recommendations.push("成功率偏低，建议检查任务难度".to_string());
        }

        if compression_ratio > 0.8 {
            recommendations.push("压缩比过高，可能丢失重要信息".to_string());
        }

        Self {
            id: format!("report-{}", chrono::Utc::now().timestamp()),
            generated_at: chrono::Utc::now().to_rfc3339(),
            data_source: data_source.to_string(),
            pipeline_id: pipeline_id.to_string(),
            raw_stats,
            compressed_stats,
            compression_ratio,
            quality_metrics,
            recommendations,
        }
    }
}

impl TrainingQualityMetrics {
    fn success_rate(&self) -> f64 {
        let total: usize = self.outcome_distribution.values().sum();
        let success = self.outcome_distribution.get("success").copied().unwrap_or(0);
        if total > 0 {
            success as f64 / total as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pipeline() {
        let pipeline = CompressionPipeline::default_pipeline();
        assert_eq!(pipeline.steps.len(), 5);
        assert!(pipeline.enabled);

        let enabled = pipeline.enabled_steps();
        assert_eq!(enabled.len(), 5);
    }

    #[test]
    fn test_lightweight_pipeline() {
        let pipeline = CompressionPipeline::lightweight_pipeline();
        assert_eq!(pipeline.steps.len(), 2);
    }

    #[test]
    fn test_pipeline_add_remove_step() {
        let mut pipeline = CompressionPipeline::default_pipeline();
        let initial_count = pipeline.steps.len();

        pipeline.add_step(CompressionStepType::SummaryCompression);
        assert_eq!(pipeline.steps.len(), initial_count + 1);

        let step_id = pipeline.steps.last().expect("测试：列表应非空").id.clone();
        assert!(pipeline.remove_step(&step_id));
        assert_eq!(pipeline.steps.len(), initial_count);
    }

    #[test]
    fn test_step_type_default_config() {
        let config = CompressionStepType::QualityFilter.default_config();
        assert!(config.is_object());
        assert!(config.get("min_quality").is_some());
    }

    #[test]
    fn test_batch_config_default() {
        let config = BatchGenerationConfig::default();
        assert_eq!(config.quality_threshold, 0.5);
        assert_eq!(config.sample_size, 100);
        assert!(config.include_partial);
        assert!(!config.include_failure);
    }

    #[test]
    fn test_training_report() {
        let raw = DataStats {
            trajectory_count: 100,
            total_steps: 5000,
            total_tokens: 500000,
            avg_steps_per_trajectory: 50.0,
            success_rate: 0.85,
        };

        let compressed = DataStats {
            trajectory_count: 80,
            total_steps: 3000,
            total_tokens: 200000,
            avg_steps_per_trajectory: 37.5,
            success_rate: 0.9,
        };

        let mut outcome_distribution = HashMap::new();
        outcome_distribution.insert("success".to_string(), 70);
        outcome_distribution.insert("partial".to_string(), 15);
        outcome_distribution.insert("failure".to_string(), 5);

        let metrics = TrainingQualityMetrics {
            avg_quality: 0.55,
            avg_value_score: 0.65,
            outcome_distribution,
            tool_diversity: 0.8,
            reasoning_depth: 0.7,
        };

        let report =
            TrainingDataReport::generate(raw, compressed, metrics, "pipeline-1", "session-1");
        assert!(report.compression_ratio > 0.0);
        assert!(!report.recommendations.is_empty());
    }
}
