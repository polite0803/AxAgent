// SPDX-License-Identifier: AGPL-3.0-only

//! MoA 降级策略 (P2-14)
//!
//! Mixture-of-Agents 降级策略：当主代理失败时，
//! 降级到更小、更快的模型或更简单的策略

use serde::{Deserialize, Serialize};

/// MoA 降级配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoADegradationConfig {
    /// 是否启用降级
    pub enabled: bool,
    /// 降级策略
    pub strategies: Vec<DegradationStrategy>,
    /// 最大降级次数
    pub max_degradation_depth: u32,
    /// 降级触发条件
    pub trigger_conditions: Vec<DegradationTrigger>,
    /// 冷却时间（秒）
    pub cooldown_seconds: u64,
}

impl Default for MoADegradationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strategies: vec![
                DegradationStrategy::SmallerModel,
                DegradationStrategy::SimplifiedPrompt,
                DegradationStrategy::LimitedTools,
            ],
            max_degradation_depth: 3,
            trigger_conditions: vec![
                DegradationTrigger::ErrorRate(0.5),
                DegradationTrigger::Timeout(30),
            ],
            cooldown_seconds: 60,
        }
    }
}

/// 降级策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationStrategy {
    /// 使用更小的模型
    SmallerModel,
    /// 简化 prompt
    SimplifiedPrompt,
    /// 限制可用工具
    LimitedTools,
    /// 单轮对话（不使用多轮）
    SingleTurn,
    /// 仅使用 RAG（不使用工具）
    RAGOnly,
    /// 最终回退到模板
    TemplateFallback,
}

/// 降级触发条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DegradationTrigger {
    /// 错误率超过阈值
    ErrorRate(f64),
    /// 响应时间超过阈值（秒）
    Timeout(u64),
    /// 连续失败次数
    ConsecutiveFailures(u32),
    /// token 限制警告
    TokenLimitWarning,
}

/// 降级状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradationState {
    /// 当前降级深度
    pub current_depth: u32,
    /// 当前使用的策略
    pub current_strategy: Option<DegradationStrategy>,
    /// 最近降级时间
    pub last_degradation_at: Option<String>,
    /// 恢复时间
    pub recovery_at: Option<String>,
    /// 连续失败次数
    pub consecutive_failures: u32,
}

impl Default for DegradationState {
    fn default() -> Self {
        Self {
            current_depth: 0,
            current_strategy: None,
            last_degradation_at: None,
            recovery_at: None,
            consecutive_failures: 0,
        }
    }
}

impl DegradationState {
    /// 是否处于降级状态
    pub fn is_degraded(&self) -> bool {
        self.current_depth > 0
    }

    /// 执行降级
    pub fn degrade(&mut self, strategy: DegradationStrategy) {
        self.current_depth += 1;
        self.current_strategy = Some(strategy);
        self.last_degradation_at = Some(chrono::Utc::now().to_rfc3339());
        self.recovery_at = None;
    }

    /// 恢复正常
    pub fn recover(&mut self) {
        self.current_depth = 0;
        self.current_strategy = None;
        self.recovery_at = Some(chrono::Utc::now().to_rfc3339());
        self.consecutive_failures = 0;
    }

    /// 记录失败
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
    }
}
