// SPDX-License-Identifier: AGPL-3.0-only

//! 多模型协同路由契约 — Cascade / Chain 模式的统一抽象
//!
//! 本模块定义多模型协同路由的契约层，支持两种主要模式：
//! - **Cascade（级联）**：先尝试低成本模型，失败/低置信度时升级到高成本模型
//! - **Chain（链式）**：按优先级顺序依次尝试，前一个失败才用下一个
//!
//! 设计原则：
//! - 仅定义 trait + DTO + 纯函数，零业务逻辑
//! - 与具体 Provider 解耦：通过 `ModelCascadeExecutor` trait 抽象执行
//! - 升级决策可测试：`should_escalate` 是纯函数
//!
//! 与既有路由系统的关系：
//! - `ProviderFallbackManager`（agent crate）：provider 级降级链（4 档），关注 provider 健康
//! - `CostAwareRouter`（smart_router）：model 级成本感知路由，关注成本/延迟
//! - 本模块：model 级协同路由契约，关注"先 cheap 尝试 → 失败/低置信度时升级"

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

// ── DTO：级联链节点 ──────────────────────────────────────────────────

/// 级联模型条目 — 定义级联链中的一个模型节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CascadeModel {
    /// 模型标识（如 "gpt-4o-mini"）
    pub model_id: String,
    /// Provider 标识（如 "openai"）
    pub provider_id: String,
    /// 优先级（0 = 最高优先级，数值越大优先级越低）
    /// 升级时按 priority 升序遍历，从当前模型升级到下一个 priority 更大的模型
    pub priority: u32,
    /// 单模型最大尝试次数（在该模型上重试多少次后升级）
    pub max_attempts: u32,
}

impl CascadeModel {
    pub fn new(model_id: impl Into<String>, provider_id: impl Into<String>, priority: u32) -> Self {
        Self {
            model_id: model_id.into(),
            provider_id: provider_id.into(),
            priority,
            max_attempts: 1,
        }
    }

    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts.max(1);
        self
    }
}

// ── DTO：升级规则 ────────────────────────────────────────────────────

/// 升级触发规则 — 定义何时从当前模型升级到下一个模型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscalationRule {
    /// 置信度阈值（0.0-1.0），低于此值触发升级
    pub min_confidence: Option<f64>,
    /// 错误模式（子串匹配，大小写不敏感），匹配则触发升级
    pub error_patterns: Vec<String>,
    /// 是否在超时后升级
    pub escalate_on_timeout: bool,
    /// 是否在限流（429）后升级
    pub escalate_on_rate_limit: bool,
}

impl Default for EscalationRule {
    fn default() -> Self {
        Self {
            min_confidence: None,
            error_patterns: Vec::new(),
            escalate_on_timeout: true,
            escalate_on_rate_limit: true,
        }
    }
}

impl EscalationRule {
    pub fn builder() -> EscalationRuleBuilder {
        EscalationRuleBuilder::default()
    }
}

/// 升级规则构建器
#[derive(Default)]
pub struct EscalationRuleBuilder {
    min_confidence: Option<f64>,
    error_patterns: Vec<String>,
    escalate_on_timeout: bool,
    escalate_on_rate_limit: bool,
}

impl EscalationRuleBuilder {
    pub fn min_confidence(mut self, threshold: f64) -> Self {
        self.min_confidence = Some(threshold.clamp(0.0, 1.0));
        self
    }

    pub fn error_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.error_patterns.push(pattern.into());
        self
    }

    pub fn escalate_on_timeout(mut self, enabled: bool) -> Self {
        self.escalate_on_timeout = enabled;
        self
    }

    pub fn escalate_on_rate_limit(mut self, enabled: bool) -> Self {
        self.escalate_on_rate_limit = enabled;
        self
    }

    pub fn build(self) -> EscalationRule {
        EscalationRule {
            min_confidence: self.min_confidence,
            error_patterns: self.error_patterns,
            escalate_on_timeout: self.escalate_on_timeout,
            escalate_on_rate_limit: self.escalate_on_rate_limit,
        }
    }
}

// ── DTO：级联策略 ────────────────────────────────────────────────────

/// 级联策略 — 定义多模型协同的整体规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelCascadeStrategy {
    /// 单模型模式（禁用级联，仅用第一个模型）
    Single { model: CascadeModel },
    /// 级联模式：按 priority 顺序尝试，满足升级规则时切换到下一个
    Cascade {
        models: Vec<CascadeModel>,
        escalation: EscalationRule,
        /// 最大升级次数（防止无限升级）
        max_escalations: u32,
    },
}

impl ModelCascadeStrategy {
    /// 创建简单的级联策略（按优先级顺序，默认升级规则）
    pub fn cascade(models: Vec<CascadeModel>) -> Self {
        Self::Cascade { models, escalation: EscalationRule::default(), max_escalations: 3 }
    }

    /// 创建单模型策略
    pub fn single(model: CascadeModel) -> Self {
        Self::Single { model }
    }

    /// 获取第一个（首选）模型
    pub fn first_model(&self) -> Option<&CascadeModel> {
        match self {
            Self::Single { model } => Some(model),
            Self::Cascade { models, .. } => models.first(),
        }
    }

    /// 按 priority 排序后的模型列表
    pub fn ordered_models(&self) -> Vec<&CascadeModel> {
        match self {
            Self::Single { model } => vec![model],
            Self::Cascade { models, .. } => {
                let mut indexed: Vec<&CascadeModel> = models.iter().collect();
                indexed.sort_by_key(|m| m.priority);
                indexed
            },
        }
    }

    /// 最大升级次数
    pub fn max_escalations(&self) -> u32 {
        match self {
            Self::Single { .. } => 0,
            Self::Cascade { max_escalations, .. } => *max_escalations,
        }
    }
}

// ── DTO：调用结果摘要 ────────────────────────────────────────────────

/// 单次模型调用结果摘要 — 用于升级决策
#[derive(Debug, Clone)]
pub struct ModelCallSummary {
    /// 实际使用的模型
    pub model_id: String,
    /// 实际使用的 provider
    pub provider_id: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息（失败时）
    pub error: Option<String>,
    /// 模型自报置信度（如响应中包含）
    pub confidence: Option<f64>,
    /// 调用耗时（毫秒）
    pub duration_ms: u64,
    /// 是否超时
    pub timed_out: bool,
    /// 是否被限流
    pub rate_limited: bool,
}

impl ModelCallSummary {
    pub fn success(model_id: impl Into<String>, provider_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            provider_id: provider_id.into(),
            success: true,
            error: None,
            confidence: None,
            duration_ms: 0,
            timed_out: false,
            rate_limited: false,
        }
    }

    pub fn failure(
        model_id: impl Into<String>,
        provider_id: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            provider_id: provider_id.into(),
            success: false,
            error: Some(error.into()),
            confidence: None,
            duration_ms: 0,
            timed_out: false,
            rate_limited: false,
        }
    }

    pub fn with_confidence(mut self, conf: f64) -> Self {
        self.confidence = Some(conf);
        self
    }

    pub fn with_timeout(mut self, timed_out: bool) -> Self {
        self.timed_out = timed_out;
        self
    }

    pub fn with_rate_limited(mut self, rate_limited: bool) -> Self {
        self.rate_limited = rate_limited;
        self
    }
}

// ── DTO：升级决策 ────────────────────────────────────────────────────

/// 升级决策 — 纯函数判断是否应该升级到下一个模型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscalationDecision {
    /// 继续使用当前模型
    Continue,
    /// 升级到下一个模型
    Escalate { reason: EscalationReason },
    /// 所有模型已耗尽或达到最大升级次数，不再升级
    Exhausted,
}

/// 升级原因
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EscalationReason {
    LowConfidence,
    Timeout,
    RateLimited,
    ErrorPatternMatched,
    MaxAttemptsExceeded,
    CallFailed,
}

impl fmt::Display for EscalationReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LowConfidence => write!(f, "置信度低于阈值"),
            Self::Timeout => write!(f, "调用超时"),
            Self::RateLimited => write!(f, "被限流"),
            Self::ErrorPatternMatched => write!(f, "错误匹配升级模式"),
            Self::MaxAttemptsExceeded => write!(f, "当前模型尝试次数耗尽"),
            Self::CallFailed => write!(f, "调用失败"),
        }
    }
}

// ── 纯函数：升级决策 ─────────────────────────────────────────────────

/// 判断是否应该升级到下一个模型
///
/// 决策逻辑：
/// 1. 成功且置信度达标 → `Continue`
/// 2. 成功但置信度低于阈值 → 升级（低置信度）
/// 3. 失败 + 超时 + 规则允许 → 升级（超时）
/// 4. 失败 + 限流 + 规则允许 → 升级（限流）
/// 5. 失败 + 错误匹配模式 → 升级（错误匹配）
/// 6. 失败 + `current_attempt < model_max_attempts` → `Continue`（继续在当前模型重试）
/// 7. 失败 + `current_attempt >= model_max_attempts` → 升级（尝试耗尽）
/// 8. 无下一个模型或达到最大升级次数 → `Exhausted`
///
/// # 参数
/// - `summary`: 当前模型的调用结果摘要
/// - `rule`: 升级规则
/// - `current_attempt`: 在当前模型上的尝试次数（1-based）
/// - `model_max_attempts`: 当前模型的最大尝试次数（来自 `CascadeModel.max_attempts`）
/// - `escalations_done`: 已升级次数
/// - `max_escalations`: 最大允许升级次数
/// - `has_next_model`: 是否还有下一个模型可升级
pub fn should_escalate(
    summary: &ModelCallSummary,
    rule: &EscalationRule,
    current_attempt: u32,
    model_max_attempts: u32,
    escalations_done: u32,
    max_escalations: u32,
    has_next_model: bool,
) -> EscalationDecision {
    // 无法升级的两种情况：无下一个模型 或 已达最大升级次数
    let can_escalate = has_next_model && escalations_done < max_escalations;

    // 成功路径：检查置信度
    if summary.success {
        if let Some(threshold) = rule.min_confidence
            && let Some(conf) = summary.confidence
            && conf < threshold
        {
            return decide_escalation(EscalationReason::LowConfidence, can_escalate);
        }
        return EscalationDecision::Continue;
    }

    // 失败路径：按具体原因判断（这些原因立即升级，不重试）
    if summary.timed_out && rule.escalate_on_timeout {
        return decide_escalation(EscalationReason::Timeout, can_escalate);
    }

    if summary.rate_limited && rule.escalate_on_rate_limit {
        return decide_escalation(EscalationReason::RateLimited, can_escalate);
    }

    // 错误模式匹配（大小写不敏感的子串匹配）
    if let Some(ref err) = summary.error
        && !rule.error_patterns.is_empty()
    {
        let err_lower = err.to_lowercase();
        for pattern in &rule.error_patterns {
            if err_lower.contains(&pattern.to_lowercase()) {
                return decide_escalation(EscalationReason::ErrorPatternMatched, can_escalate);
            }
        }
    }

    // 非特定原因失败：检查是否还有重试机会
    if current_attempt < model_max_attempts {
        // 还有重试机会，继续在当前模型上尝试
        return EscalationDecision::Continue;
    }

    // 重试次数耗尽，升级
    decide_escalation(EscalationReason::MaxAttemptsExceeded, can_escalate)
}

/// 根据是否可升级返回 `Escalate` 或 `Exhausted`
fn decide_escalation(reason: EscalationReason, can_escalate: bool) -> EscalationDecision {
    if can_escalate {
        EscalationDecision::Escalate { reason }
    } else {
        EscalationDecision::Exhausted
    }
}

// ── DTO：级联执行结果 ────────────────────────────────────────────────

/// 级联执行结果 — 整个级联流程的最终结果
#[derive(Debug, Clone)]
pub struct CascadeOutcome {
    /// 最终使用的模型
    pub final_model_id: String,
    /// 最终使用的 provider
    pub final_provider_id: String,
    /// 是否发生了升级
    pub escalated: bool,
    /// 升级来源模型（如果发生了升级）
    pub escalated_from: Option<String>,
    /// 总尝试次数（所有模型累计）
    pub total_attempts: u32,
    /// 升级次数
    pub escalation_count: u32,
    /// 最终是否成功
    pub success: bool,
    /// 最终错误（失败时）
    pub final_error: Option<String>,
    /// 升级历史
    pub escalation_history: Vec<EscalationRecord>,
}

impl CascadeOutcome {
    /// 构建成功结果
    pub fn success(
        final_model_id: impl Into<String>,
        final_provider_id: impl Into<String>,
        total_attempts: u32,
        escalation_count: u32,
        escalation_history: Vec<EscalationRecord>,
    ) -> Self {
        let escalated = !escalation_history.is_empty();
        let escalated_from = escalation_history.first().map(|r| r.from_model.clone());
        Self {
            final_model_id: final_model_id.into(),
            final_provider_id: final_provider_id.into(),
            escalated,
            escalated_from,
            total_attempts,
            escalation_count,
            success: true,
            final_error: None,
            escalation_history,
        }
    }

    /// 构建失败结果
    pub fn failure(
        final_model_id: impl Into<String>,
        final_provider_id: impl Into<String>,
        error: impl Into<String>,
        total_attempts: u32,
        escalation_count: u32,
        escalation_history: Vec<EscalationRecord>,
    ) -> Self {
        let escalated = !escalation_history.is_empty();
        let escalated_from = escalation_history.first().map(|r| r.from_model.clone());
        Self {
            final_model_id: final_model_id.into(),
            final_provider_id: final_provider_id.into(),
            escalated,
            escalated_from,
            total_attempts,
            escalation_count,
            success: false,
            final_error: Some(error.into()),
            escalation_history,
        }
    }
}

/// 单次升级记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EscalationRecord {
    pub from_model: String,
    pub to_model: String,
    pub reason: EscalationReason,
    pub timestamp_ms: u64,
}

// ── Trait：级联执行器 ────────────────────────────────────────────────

/// 多模型协同路由执行器 trait
///
/// 实现方（如 `agent` crate 或 `runtime` crate）提供具体的执行逻辑，
/// 调用方（如 orchestrator）通过此 trait 解耦具体实现。
///
/// 与 `ProviderFallbackManager` 的区别：
/// - `ProviderFallbackManager`：provider 级降级，关注 provider 健康状态
/// - `ModelCascadeExecutor`：model 级协同，关注"先 cheap 尝试 → 失败时升级"
#[async_trait]
pub trait ModelCascadeExecutor: Send + Sync {
    /// 执行单个模型调用
    ///
    /// 实现方负责：
    /// 1. 根据 `model.provider_id` 找到对应的 `ProviderAdapter`
    /// 2. 构造 `ProviderRequestContext` 和 `ChatRequest`
    /// 3. 调用 `adapter.chat()` 或 `adapter.chat_stream()`
    /// 4. 返回调用结果摘要（包含置信度、是否限流、是否超时等）
    async fn call_single_model(
        &self,
        model: &CascadeModel,
        request_payload: &serde_json::Value,
    ) -> Result<ModelCallSummary, String>;

    /// 执行多模型协同路由
    ///
    /// 默认实现基于 `call_single_model` + `should_escalate` 纯函数，
    /// 按 `strategy.ordered_models()` 顺序尝试，满足升级规则时切换到下一个模型。
    ///
    /// 实现方可覆盖此方法以提供自定义的级联逻辑（如并发尝试、流式级联等）。
    async fn execute_cascade(
        &self,
        strategy: &ModelCascadeStrategy,
        request_payload: &serde_json::Value,
    ) -> Result<CascadeOutcome, String> {
        let ordered = strategy.ordered_models();
        if ordered.is_empty() {
            return Err("级联策略模型列表为空".to_string());
        }

        let max_escalations = strategy.max_escalations();
        let rule = match strategy {
            ModelCascadeStrategy::Single { .. } => EscalationRule::default(),
            ModelCascadeStrategy::Cascade { escalation, .. } => escalation.clone(),
        };

        let mut escalation_history: Vec<EscalationRecord> = Vec::new();
        let mut total_attempts: u32 = 0;
        let mut escalations_done: u32 = 0;
        let mut last_error: Option<String> = None;

        for (idx, model) in ordered.iter().enumerate() {
            let has_next = idx + 1 < ordered.len();
            let mut current_attempt: u32 = 0;

            // 在当前模型上尝试 max_attempts 次
            for _ in 0..model.max_attempts {
                current_attempt += 1;
                total_attempts += 1;

                let summary = self.call_single_model(model, request_payload).await?;

                if summary.success
                    && !(matches!(rule.min_confidence, Some(th) if matches!(summary.confidence, Some(c) if c < th)))
                {
                    // 成功且置信度达标
                    return Ok(CascadeOutcome::success(
                        &model.model_id,
                        &model.provider_id,
                        total_attempts,
                        escalations_done,
                        escalation_history,
                    ));
                }

                // 判断是否升级
                let decision = should_escalate(
                    &summary,
                    &rule,
                    current_attempt,
                    model.max_attempts,
                    escalations_done,
                    max_escalations,
                    has_next,
                );

                match decision {
                    EscalationDecision::Continue => {
                        // 继续在当前模型上尝试（理论上不会到这里，因为成功已处理）
                        continue;
                    },
                    EscalationDecision::Escalate { reason } => {
                        // 记录升级
                        if let Some(next_model) = ordered.get(idx + 1) {
                            escalation_history.push(EscalationRecord {
                                from_model: model.model_id.clone(),
                                to_model: next_model.model_id.clone(),
                                reason: reason.clone(),
                                timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
                            });
                            escalations_done += 1;
                            last_error = summary.error.clone();
                        }
                        break; // 跳出当前模型的重试循环，进入下一个模型
                    },
                    EscalationDecision::Exhausted => {
                        // 无法升级，记录错误并返回失败
                        last_error = Some(
                            summary.error.clone().unwrap_or_else(|| "所有模型已耗尽".to_string()),
                        );
                        return Ok(CascadeOutcome::failure(
                            &model.model_id,
                            &model.provider_id,
                            last_error.unwrap_or_else(|| "未知错误".to_string()),
                            total_attempts,
                            escalations_done,
                            escalation_history,
                        ));
                    },
                }
            }
        }

        // 所有模型都尝试完毕
        let final_model = ordered.last().expect("集合为空"); // 非空，前面已检查
        Ok(CascadeOutcome::failure(
            &final_model.model_id,
            &final_model.provider_id,
            last_error.unwrap_or_else(|| "所有模型均已耗尽".to_string()),
            total_attempts,
            escalations_done,
            escalation_history,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CascadeModel 测试 ──

    #[test]
    fn test_cascade_model_new() {
        let m = CascadeModel::new("gpt-4o-mini", "openai", 0);
        assert_eq!(m.model_id, "gpt-4o-mini");
        assert_eq!(m.provider_id, "openai");
        assert_eq!(m.priority, 0);
        assert_eq!(m.max_attempts, 1);
    }

    #[test]
    fn test_cascade_model_with_max_attempts() {
        let m = CascadeModel::new("gpt-4o", "openai", 1).with_max_attempts(3);
        assert_eq!(m.max_attempts, 3);
    }

    #[test]
    fn test_cascade_model_max_attempts_clamped() {
        let m = CascadeModel::new("gpt-4o", "openai", 1).with_max_attempts(0);
        assert_eq!(m.max_attempts, 1); // 最小为 1
    }

    // ── EscalationRule 构建器测试 ──

    #[test]
    fn test_escalation_rule_builder() {
        let rule = EscalationRule::builder()
            .min_confidence(0.8)
            .error_pattern("overloaded")
            .error_pattern("capacity")
            .escalate_on_timeout(false)
            .escalate_on_rate_limit(true)
            .build();

        assert_eq!(rule.min_confidence, Some(0.8));
        assert_eq!(rule.error_patterns.len(), 2);
        assert!(!rule.escalate_on_timeout);
        assert!(rule.escalate_on_rate_limit);
    }

    #[test]
    fn test_escalation_rule_min_confidence_clamped() {
        let rule = EscalationRule::builder().min_confidence(1.5).build();
        assert_eq!(rule.min_confidence, Some(1.0));

        let rule = EscalationRule::builder().min_confidence(-0.5).build();
        assert_eq!(rule.min_confidence, Some(0.0));
    }

    #[test]
    fn test_escalation_rule_default() {
        let rule = EscalationRule::default();
        assert!(rule.escalate_on_timeout);
        assert!(rule.escalate_on_rate_limit);
        assert!(rule.error_patterns.is_empty());
        assert_eq!(rule.min_confidence, None);
    }

    // ── ModelCascadeStrategy 测试 ──

    #[test]
    fn test_strategy_single() {
        let m = CascadeModel::new("gpt-4o", "openai", 0);
        let s = ModelCascadeStrategy::single(m);
        assert_eq!(s.first_model().expect("测试应成功").model_id, "gpt-4o");
        assert_eq!(s.ordered_models().len(), 1);
        assert_eq!(s.max_escalations(), 0);
    }

    #[test]
    fn test_strategy_cascade() {
        let m1 = CascadeModel::new("gpt-4o-mini", "openai", 0);
        let m2 = CascadeModel::new("gpt-4o", "openai", 1);
        let s = ModelCascadeStrategy::cascade(vec![m1, m2]);
        assert_eq!(s.first_model().expect("测试应成功").model_id, "gpt-4o-mini");
        assert_eq!(s.ordered_models().len(), 2);
        assert_eq!(s.max_escalations(), 3);
    }

    #[test]
    fn test_strategy_ordered_models_sorted_by_priority() {
        let m1 = CascadeModel::new("gpt-4o", "openai", 2); // priority=2
        let m2 = CascadeModel::new("gpt-4o-mini", "openai", 0); // priority=0
        let m3 = CascadeModel::new("claude-3-haiku", "anthropic", 1); // priority=1
        let s = ModelCascadeStrategy::cascade(vec![m1, m2, m3]);
        let ordered = s.ordered_models();
        assert_eq!(ordered[0].model_id, "gpt-4o-mini"); // priority=0
        assert_eq!(ordered[1].model_id, "claude-3-haiku"); // priority=1
        assert_eq!(ordered[2].model_id, "gpt-4o"); // priority=2
    }

    // ── should_escalate 纯函数测试 ──

    #[test]
    fn test_should_escalate_success_continue() {
        let summary = ModelCallSummary::success("gpt-4o", "openai");
        let rule = EscalationRule::default();
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        assert_eq!(decision, EscalationDecision::Continue);
    }

    #[test]
    fn test_should_escalate_low_confidence() {
        let summary = ModelCallSummary::success("gpt-4o", "openai").with_confidence(0.5);
        let rule = EscalationRule::builder().min_confidence(0.8).build();
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        assert_eq!(
            decision,
            EscalationDecision::Escalate { reason: EscalationReason::LowConfidence }
        );
    }

    #[test]
    fn test_should_escalate_low_confidence_no_threshold() {
        let summary = ModelCallSummary::success("gpt-4o", "openai").with_confidence(0.3);
        let rule = EscalationRule::default(); // 无 min_confidence
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        assert_eq!(decision, EscalationDecision::Continue);
    }

    #[test]
    fn test_should_escalate_timeout() {
        let summary =
            ModelCallSummary::failure("gpt-4o", "openai", "request timed out").with_timeout(true);
        let rule = EscalationRule::default();
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        assert_eq!(decision, EscalationDecision::Escalate { reason: EscalationReason::Timeout });
    }

    #[test]
    fn test_should_escalate_timeout_disabled() {
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "timed out").with_timeout(true);
        let rule = EscalationRule::builder().escalate_on_timeout(false).build();
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        // 超时不升级 → 检查尝试次数耗尽 → 升级
        assert_eq!(
            decision,
            EscalationDecision::Escalate { reason: EscalationReason::MaxAttemptsExceeded }
        );
    }

    #[test]
    fn test_should_escalate_rate_limited() {
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "429 too many requests")
            .with_rate_limited(true);
        let rule = EscalationRule::default();
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        assert_eq!(
            decision,
            EscalationDecision::Escalate { reason: EscalationReason::RateLimited }
        );
    }

    #[test]
    fn test_should_escalate_error_pattern_matched() {
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "model is overloaded");
        let rule = EscalationRule::builder().error_pattern("overloaded").build();
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        assert_eq!(
            decision,
            EscalationDecision::Escalate { reason: EscalationReason::ErrorPatternMatched }
        );
    }

    #[test]
    fn test_should_escalate_error_pattern_case_insensitive() {
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "SERVICE UNAVAILABLE");
        let rule = EscalationRule::builder().error_pattern("service unavailable").build();
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        assert_eq!(
            decision,
            EscalationDecision::Escalate { reason: EscalationReason::ErrorPatternMatched }
        );
    }

    #[test]
    fn test_should_escalate_no_next_model() {
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "failed");
        let rule = EscalationRule::default();
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, false); // 无下一个模型
        assert_eq!(decision, EscalationDecision::Exhausted);
    }

    #[test]
    fn test_should_escalate_max_escalations_reached() {
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "failed");
        let rule = EscalationRule::default();
        let decision = should_escalate(&summary, &rule, 1, 1, 3, 3, true); // 已升级 3 次，上限 3
        assert_eq!(decision, EscalationDecision::Exhausted);
    }

    #[test]
    fn test_should_escalate_call_failed_generic() {
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "unknown error");
        let rule = EscalationRule::default();
        // 无超时、无限流、无错误模式匹配 → 尝试次数耗尽 → 升级
        let decision = should_escalate(&summary, &rule, 1, 1, 0, 3, true);
        assert_eq!(
            decision,
            EscalationDecision::Escalate { reason: EscalationReason::MaxAttemptsExceeded }
        );
    }

    #[test]
    fn test_should_escalate_retry_when_attempts_remaining() {
        // 失败 + 非特定原因 + current_attempt(1) < model_max_attempts(3) → Continue（继续重试）
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "unknown error");
        let rule = EscalationRule::default();
        let decision = should_escalate(&summary, &rule, 1, 3, 0, 3, true);
        assert_eq!(decision, EscalationDecision::Continue);
    }

    #[test]
    fn test_should_escalate_retry_exhausted_on_last_attempt() {
        // 失败 + 非特定原因 + current_attempt(3) >= model_max_attempts(3) → MaxAttemptsExceeded
        let summary = ModelCallSummary::failure("gpt-4o", "openai", "unknown error");
        let rule = EscalationRule::default();
        let decision = should_escalate(&summary, &rule, 3, 3, 0, 3, true);
        assert_eq!(
            decision,
            EscalationDecision::Escalate { reason: EscalationReason::MaxAttemptsExceeded }
        );
    }

    // ── CascadeOutcome 测试 ──

    #[test]
    fn test_cascade_outcome_success_no_escalation() {
        let outcome = CascadeOutcome::success("gpt-4o", "openai", 1, 0, vec![]);
        assert!(outcome.success);
        assert!(!outcome.escalated);
        assert_eq!(outcome.final_model_id, "gpt-4o");
        assert_eq!(outcome.escalated_from, None);
    }

    #[test]
    fn test_cascade_outcome_success_with_escalation() {
        let history = vec![EscalationRecord {
            from_model: "gpt-4o-mini".into(),
            to_model: "gpt-4o".into(),
            reason: EscalationReason::LowConfidence,
            timestamp_ms: 1000,
        }];
        let outcome = CascadeOutcome::success("gpt-4o", "openai", 2, 1, history);
        assert!(outcome.success);
        assert!(outcome.escalated);
        assert_eq!(outcome.escalated_from, Some("gpt-4o-mini".to_string()));
        assert_eq!(outcome.escalation_count, 1);
    }

    #[test]
    fn test_cascade_outcome_failure() {
        let outcome =
            CascadeOutcome::failure("gpt-4o", "openai", "all models exhausted", 3, 2, vec![]);
        assert!(!outcome.success);
        assert_eq!(outcome.final_error, Some("all models exhausted".to_string()));
        assert_eq!(outcome.total_attempts, 3);
    }

    // ── EscalationReason Display 测试 ──

    #[test]
    fn test_escalation_reason_display() {
        assert_eq!(EscalationReason::LowConfidence.to_string(), "置信度低于阈值");
        assert_eq!(EscalationReason::Timeout.to_string(), "调用超时");
        assert_eq!(EscalationReason::RateLimited.to_string(), "被限流");
        assert_eq!(EscalationReason::ErrorPatternMatched.to_string(), "错误匹配升级模式");
        assert_eq!(EscalationReason::MaxAttemptsExceeded.to_string(), "当前模型尝试次数耗尽");
        assert_eq!(EscalationReason::CallFailed.to_string(), "调用失败");
    }

    // ── ModelCascadeExecutor 默认实现测试 ──

    /// 用于测试的 mock 执行器
    struct MockCascadeExecutor {
        /// 模拟每个模型的调用结果（model_id -> 结果）
        results: parking_lot::Mutex<Vec<ModelCallSummary>>,
        /// 调用计数
        call_count: std::sync::atomic::AtomicU32,
    }

    impl MockCascadeExecutor {
        fn new(results: Vec<ModelCallSummary>) -> Self {
            Self {
                results: parking_lot::Mutex::new(results),
                call_count: std::sync::atomic::AtomicU32::new(0),
            }
        }
    }

    #[async_trait]
    impl ModelCascadeExecutor for MockCascadeExecutor {
        async fn call_single_model(
            &self,
            model: &CascadeModel,
            _request_payload: &serde_json::Value,
        ) -> Result<ModelCallSummary, String> {
            let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let results = self.results.lock();
            if (count as usize) < results.len() {
                let mut summary = results[count as usize].clone();
                // 覆盖 model_id/provider_id 以匹配实际调用的模型
                summary.model_id = model.model_id.clone();
                summary.provider_id = model.provider_id.clone();
                Ok(summary)
            } else {
                Ok(ModelCallSummary::failure(
                    &model.model_id,
                    &model.provider_id,
                    "no more mock results",
                ))
            }
        }
    }

    #[tokio::test]
    async fn test_execute_cascade_success_first_model() {
        let summary = ModelCallSummary::success("gpt-4o-mini", "openai");
        let executor = MockCascadeExecutor::new(vec![summary]);
        let strategy = ModelCascadeStrategy::cascade(vec![
            CascadeModel::new("gpt-4o-mini", "openai", 0),
            CascadeModel::new("gpt-4o", "openai", 1),
        ]);

        let outcome = executor
            .execute_cascade(&strategy, &serde_json::Value::Null)
            .await
            .expect("cascade should succeed");

        assert!(outcome.success);
        assert!(!outcome.escalated);
        assert_eq!(outcome.final_model_id, "gpt-4o-mini");
        assert_eq!(outcome.total_attempts, 1);
    }

    #[tokio::test]
    async fn test_execute_cascade_escalate_on_failure() {
        // 第一个模型失败，第二个成功
        let results = vec![
            ModelCallSummary::failure("gpt-4o-mini", "openai", "overloaded"),
            ModelCallSummary::success("gpt-4o", "openai"),
        ];
        let executor = MockCascadeExecutor::new(results);
        let strategy = ModelCascadeStrategy::cascade(vec![
            CascadeModel::new("gpt-4o-mini", "openai", 0),
            CascadeModel::new("gpt-4o", "openai", 1),
        ]);

        let outcome = executor
            .execute_cascade(&strategy, &serde_json::Value::Null)
            .await
            .expect("cascade should succeed");

        assert!(outcome.success);
        assert!(outcome.escalated);
        assert_eq!(outcome.final_model_id, "gpt-4o");
        assert_eq!(outcome.escalated_from, Some("gpt-4o-mini".to_string()));
        assert_eq!(outcome.escalation_count, 1);
        assert_eq!(outcome.escalation_history.len(), 1);
        assert_eq!(outcome.escalation_history[0].from_model, "gpt-4o-mini");
        assert_eq!(outcome.escalation_history[0].to_model, "gpt-4o");
    }

    #[tokio::test]
    async fn test_execute_cascade_all_fail() {
        let results = vec![
            ModelCallSummary::failure("gpt-4o-mini", "openai", "overloaded"),
            ModelCallSummary::failure("gpt-4o", "openai", "also overloaded"),
        ];
        let executor = MockCascadeExecutor::new(results);
        let strategy = ModelCascadeStrategy::cascade(vec![
            CascadeModel::new("gpt-4o-mini", "openai", 0),
            CascadeModel::new("gpt-4o", "openai", 1),
        ]);

        let outcome = executor
            .execute_cascade(&strategy, &serde_json::Value::Null)
            .await
            .expect("cascade should return failure outcome");

        assert!(!outcome.success);
        assert_eq!(outcome.final_model_id, "gpt-4o");
        assert_eq!(outcome.escalation_count, 1);
    }

    #[tokio::test]
    async fn test_execute_cascade_single_model() {
        let summary = ModelCallSummary::success("gpt-4o", "openai");
        let executor = MockCascadeExecutor::new(vec![summary]);
        let strategy = ModelCascadeStrategy::single(CascadeModel::new("gpt-4o", "openai", 0));

        let outcome = executor
            .execute_cascade(&strategy, &serde_json::Value::Null)
            .await
            .expect("single model should succeed");

        assert!(outcome.success);
        assert!(!outcome.escalated);
        assert_eq!(outcome.escalation_count, 0);
    }

    #[tokio::test]
    async fn test_execute_cascade_empty_models_error() {
        let executor = MockCascadeExecutor::new(vec![]);
        let strategy = ModelCascadeStrategy::cascade(vec![]);

        let result = executor.execute_cascade(&strategy, &serde_json::Value::Null).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("为空"));
    }

    #[tokio::test]
    async fn test_execute_cascade_low_confidence_escalation() {
        // 第一个模型成功但低置信度，第二个模型成功且高置信度
        let results = vec![
            ModelCallSummary::success("gpt-4o-mini", "openai").with_confidence(0.5),
            ModelCallSummary::success("gpt-4o", "openai").with_confidence(0.95),
        ];
        let executor = MockCascadeExecutor::new(results);
        let strategy = ModelCascadeStrategy::Cascade {
            models: vec![
                CascadeModel::new("gpt-4o-mini", "openai", 0),
                CascadeModel::new("gpt-4o", "openai", 1),
            ],
            escalation: EscalationRule::builder().min_confidence(0.8).build(),
            max_escalations: 3,
        };

        let outcome = executor
            .execute_cascade(&strategy, &serde_json::Value::Null)
            .await
            .expect("cascade should succeed via escalation");

        assert!(outcome.success);
        assert!(outcome.escalated);
        assert_eq!(outcome.final_model_id, "gpt-4o");
        assert_eq!(outcome.escalation_history.len(), 1);
        assert_eq!(outcome.escalation_history[0].reason, EscalationReason::LowConfidence);
    }
}
