// SPDX-License-Identifier: AGPL-3.0-only

//! 辅助客户端数据模型 (P2-15)
//!
//! 小模型侧任务调度、温度契约和计费相关 DTO

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 温度契约
// ---------------------------------------------------------------------------

/// 温度契约
///
/// 定义主代理和辅助模型之间的温度/任务分配契约
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemperatureContract {
    /// 契约 ID
    pub id: String,
    /// 主模型 ID
    pub primary_model: String,
    /// 辅助模型 ID
    pub secondary_model: String,
    /// 温度范围（主模型）
    pub primary_temperature_range: (f32, f32),
    /// 温度范围（辅助模型）
    pub secondary_temperature_range: (f32, f32),
    /// 任务分配策略
    pub task_allocation: TaskAllocationStrategy,
    /// 成本限制
    pub cost_limit: CostLimit,
    /// 是否启用
    pub enabled: bool,
}

/// 任务分配策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAllocationStrategy {
    /// 基于任务复杂度
    ComplexityBased,
    /// 基于 token 预算
    TokenBudgetBased,
    /// 轮询分配
    RoundRobin,
    /// 主模型优先
    PrimaryFirst,
}

/// 成本限制
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostLimit {
    /// 每日最大成本（美元）
    pub daily_usd: f64,
    /// 单次最大成本（美元）
    pub per_request_usd: f64,
    /// 告警阈值（美元）
    pub alert_threshold_usd: f64,
    /// 硬限制（美元）
    pub hard_limit_usd: f64,
}

impl Default for CostLimit {
    fn default() -> Self {
        Self {
            daily_usd: 10.0,
            per_request_usd: 1.0,
            alert_threshold_usd: 8.0,
            hard_limit_usd: 15.0,
        }
    }
}

// ---------------------------------------------------------------------------
// 辅助任务
// ---------------------------------------------------------------------------

/// 辅助任务（分配给辅助模型的小任务）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuxiliaryTask {
    /// 任务 ID
    pub id: String,
    /// 任务类型
    pub task_type: AuxiliaryTaskType,
    /// 任务描述
    pub description: String,
    /// 输入
    pub input: serde_json::Value,
    /// 优先级
    pub priority: TaskPriority,
    /// 状态
    pub status: AuxiliaryTaskStatus,
    /// 分配的模型
    pub assigned_model: Option<String>,
    /// 温度
    pub temperature: f32,
    /// 创建时间
    pub created_at: String,
    /// 完成时间
    pub completed_at: Option<String>,
}

/// 辅助任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuxiliaryTaskType {
    /// 摘要生成
    Summarization,
    /// 分类
    Classification,
    /// 提取
    Extraction,
    /// 翻译
    Translation,
    /// 格式化
    Formatting,
    /// 扩展
    Expansion,
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// 辅助任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuxiliaryTaskStatus {
    Pending,
    Assigned,
    Processing,
    Completed,
    Failed,
    Cancelled,
}
