// SPDX-License-Identifier: AGPL-3.0-only

//! 贝叶斯后验证据模块（自我进化通道二：能力偏弱进化改进）。
//!
//! 对每个技能/工具维护 Beta 分布后验，以「决策置信度 + 执行结果」作为双重证据，
//! 计算 `P(success)` 并驱动进化触发决策（低于阈值触发进化）。
//!
//! 参数化约定：`alpha = 成功加权计数 + 1`，`beta = 失败加权计数 + 1`，
//! 即 `Beta(1,1)` 均匀先验 + 贝叶斯累加更新，无证据时 `P(success)=0.5`。
//!
//! T3.3：决策标签作为证据源。已持久化的决策 JSON（每条 assistant 消息的 decision 字段）
//! 可解析为 `DecisionEvidence`，经 `EvolutionDecider::consume_decision_label` 接入
//! 贝叶斯后验，实现「决策标签流 → 贝叶斯后验 → 进化触发」闭环。

use axagent_harness::workflow_evolution::ToolExecutionStats;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════
// T3.3：决策标签作为证据源
// ═══════════════════════════════════════════════════════════════════

/// 从认知编排器决策标签 JSON 解析的证据源。
///
/// 每条 assistant 消息的 `decision` 字段格式为：
/// ```json
/// {
///   "executionMode": "workflow|direct|agent|plan|ask|clarify|rejected|gap_proposal",
///   "routePath": "domain/cluster/capability",
///   "confidence": 0.95,
///   "selectedWorkflowName": "股票分析",
///   "selectedAgentProfile": { "id": "...", "name": "...", "role": "...", "expert": "..." }
/// }
/// ```
///
/// T3.3 映射规则：
/// - `executionMode` 决定成败：`workflow`/`direct`/`parameter_extract`/`agent`/`plan` → 成功；
///   `rejected`/`gap_proposal` → 失败；`clarify`/`ask` → 中立（不贡献证据，避免模糊决策污染后验）。
/// - `confidence` 直接作为贝叶斯证据权重。
/// - `routePath` 记录路由路径，供后续分析。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEvidence {
    /// 执行模式（小写），如 "workflow"、"direct"、"rejected"。
    pub execution_mode: String,
    /// 路由路径，如 "invest/stock_analysis/tech"。
    pub route_path: String,
    /// 编排器置信度（0.0 ~ 1.0）。
    pub confidence: f64,
    /// 选中工作流名称（可选）。
    pub selected_workflow_name: Option<String>,
    /// 选中专家 profile（可选）。
    pub selected_agent_profile: Option<serde_json::Value>,
}

/// 决策标签中 `executionMode` 到成败的映射。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceOutcome {
    /// 成功（workflow/direct/parameter_extract/agent/plan 等执行路径）。
    Success,
    /// 失败（rejected/gap_proposal 等拒绝或错误路径）。
    Failure,
    /// 中立（clarify/ask 等模糊决策，不贡献证据避免噪声）。
    Neutral,
}

impl DecisionEvidence {
    /// 从认知编排器持久化的决策 JSON 解析。
    ///
    /// 期望顶层字段 `executionMode` / `routePath` / `confidence` / `selectedWorkflowName` /
    /// `selectedAgentProfile`。缺失字段用默认值，不报错。
    pub fn from_json(value: &serde_json::Value) -> Self {
        let execution_mode =
            value.get("executionMode").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let route_path = value.get("routePath").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let confidence = value.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let selected_workflow_name =
            value.get("selectedWorkflowName").and_then(|v| v.as_str()).map(|s| s.to_string());
        let selected_agent_profile = value.get("selectedAgentProfile").cloned();
        Self {
            execution_mode,
            route_path,
            confidence,
            selected_workflow_name,
            selected_agent_profile,
        }
    }

    /// 判断该决策标签的证据方向（Success / Failure / Neutral）。
    ///
    /// 映射规则（对齐实施计划 T3.3 证据源汇总）：
    /// - `workflow` / `direct` / `parameter_extract` / `agent` / `plan` → Success
    /// - `rejected` / `gap_proposal` → Failure
    /// - `clarify` / `ask` / 空 → Neutral（不贡献证据，避免模糊决策污染后验）
    pub fn outcome(&self) -> EvidenceOutcome {
        match self.execution_mode.as_str() {
            "workflow" | "direct" | "parameter_extract" | "agent" | "plan" => {
                EvidenceOutcome::Success
            },
            "rejected" | "gap_proposal" => EvidenceOutcome::Failure,
            _ => EvidenceOutcome::Neutral,
        }
    }

    /// 是否可作为贝叶斯证据（outcome 不为 Neutral）。
    pub fn is_evidential(&self) -> bool {
        !matches!(self.outcome(), EvidenceOutcome::Neutral)
    }

    /// 转换为 `(weighted_confidence, success_bool)` 元组，供 `EvolutionDecider::consume_decision` 消费。
    /// 若 `outcome` 为 Neutral 则返回 None。
    pub fn to_evidence_tuple(&self) -> Option<(f64, bool)> {
        match self.outcome() {
            EvidenceOutcome::Success => Some((self.confidence, true)),
            EvidenceOutcome::Failure => Some((self.confidence, false)),
            EvidenceOutcome::Neutral => None,
        }
    }
}

/// Beta 分布后验（技能/工具成功概率）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkillPosterior {
    /// 后验成功参数（成功加权计数 + 1）
    alpha: f64,
    /// 后验失败参数（失败加权计数 + 1）
    beta: f64,
    /// 已累积的（置信度加权）成功次数（不含先验 +1）
    weighted_successes: f64,
    /// 已累积的（置信度加权）失败次数（不含先验 +1）
    weighted_failures: f64,
}

impl SkillPosterior {
    /// 均匀先验 `Beta(1,1)`：无任何证据时 `P(success)=0.5`。
    pub fn new() -> Self {
        Self { alpha: 1.0, beta: 1.0, weighted_successes: 0.0, weighted_failures: 0.0 }
    }

    /// 从既有计数初始化（对齐 `Skill` 结构字段，见 `skill.rs`）。
    ///
    /// 成功/失败计数各 +1 构成 `Beta(α+1, β+1)` 后验，保留历史证据。
    pub fn from_counts(successes: f64, failures: f64) -> Self {
        Self {
            alpha: successes + 1.0,
            beta: failures + 1.0,
            weighted_successes: successes,
            weighted_failures: failures,
        }
    }

    /// 双证据更新：决策置信度（认知编排器，0~1）+ 执行结果。
    ///
    /// 置信度越高，本次证据对后验影响越大：
    /// - `success=true` → `alpha += confidence`
    /// - `success=false` → `beta += confidence`
    pub fn update(&mut self, confidence: f64, success: bool) {
        let confidence = confidence.clamp(0.0, 1.0);
        if confidence <= f64::EPSILON {
            return; // 零置信度证据不更新，避免引入噪声
        }
        if success {
            self.alpha += confidence;
            self.weighted_successes += confidence;
        } else {
            self.beta += confidence;
            self.weighted_failures += confidence;
        }
    }

    /// `P(success) = alpha / (alpha + beta)`（后验均值）。
    pub fn p_success(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// 后验均值（与 `p_success` 等价，语义别名）。
    pub fn mean(&self) -> f64 {
        self.p_success()
    }

    /// 后验标准差（不确定性度量）。
    pub fn std_dev(&self) -> f64 {
        let a = self.alpha;
        let b = self.beta;
        let n = a + b;
        if n <= 0.0 {
            return 0.0;
        }
        (a * b / (n * n * (n + 1.0))).sqrt()
    }

    /// 95% 后验置信下限（`P(success)` 的保守下界，用于小样本防误触发）。
    pub fn lower_ci95(&self) -> f64 {
        let a = self.alpha;
        let b = self.beta;
        let n = a + b;
        let p_hat = a / n;
        let margin = 1.96 * ((p_hat * (1.0 - p_hat)) / n).sqrt();
        (p_hat - margin).clamp(0.0, 1.0)
    }

    /// 后验均值是否低于进化触发阈值。
    pub fn should_evolve(&self, threshold: f64) -> bool {
        self.p_success() < threshold
    }

    /// 已收集的（置信度加权）证据量，用于小样本场景下避免误触发。
    pub fn evidence_volume(&self) -> f64 {
        self.weighted_successes + self.weighted_failures
    }

    /// 是否已积累（置信度加权）失败证据（用于小样本下区分「无样本」与「有失败」）。
    pub fn has_failures(&self) -> bool {
        self.weighted_failures > 0.0
    }

    /// 是否已积累（置信度加权）成功证据。
    pub fn has_successes(&self) -> bool {
        self.weighted_successes > 0.0
    }
}

// ═══════════════════════════════════════════════════════════════════
// T3.2：贝叶斯后验接入进化决策
// ═══════════════════════════════════════════════════════════════════

/// 进化决策结果（证据驱动，替代旧的失败率/成功率 if-else 启发式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvolutionDecision {
    /// `P(success)` 低于触发阈值 → 触发进化（进入用户同意通道）。
    Evolve,
    /// `P(success)` 高于稳定阈值且证据足够 → 标记稳定，不触发。
    Stable,
    /// 证据不足 / 介于两阈值之间 → 继续观察，不打扰用户。
    Observe,
}

impl EvolutionDecision {
    /// 简短中文原因，供决策标签 / 日志展示。
    pub fn reason(&self, p_success: f64, evidence: f64) -> String {
        match self {
            EvolutionDecision::Evolve => format!(
                "贝叶斯后验 P(success)={:.3} 低于触发阈值，已积累证据 {:.1} → 触发进化",
                p_success, evidence
            ),
            EvolutionDecision::Stable => format!(
                "贝叶斯后验 P(success)={:.3} 高于稳定阈值，已积累证据 {:.1} → 标记稳定",
                p_success, evidence
            ),
            EvolutionDecision::Observe => format!(
                "证据不足或 P(success)={:.3} 介于两阈值间，已积累证据 {:.1} → 继续观察",
                p_success, evidence
            ),
        }
    }
}

/// 进化决策器：以「决策置信度 + 执行反馈」双重证据维护贝叶斯后验，
/// 按阈值输出 `EvolutionDecision`，替代 `should_auto_evolve` 的 if-else 启发式。
///
/// 决策规则（对齐实施计划 T0.10）：
/// - `P(success) < evolve_threshold`（且证据量足够）→ `Evolve`
/// - `P(success) > stable_threshold`（且证据量足够）→ `Stable`
/// - 介于两阈值之间 → `Observe`
/// - 样本量不足（`evidence_volume < min_evidence`）→ `Observe`（标记待观察，
///   避免小样本误触发；连续失败等强信号通过 `from_skill` 的加权已计入证据量）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionDecider {
    /// Beta 分布后验（技能/工具成功概率）。
    posterior: SkillPosterior,
    /// 进化触发低阈值：`P(success) <` 此值触发进化。
    pub evolve_threshold: f64,
    /// 稳定标记高阈值：`P(success) >` 此值且证据足够 → 稳定。
    pub stable_threshold: f64,
    /// 最小（加权）证据量，低于此值视为小样本，走保守分支。
    pub min_evidence: f64,
}

impl Default for EvolutionDecider {
    fn default() -> Self {
        Self::new()
    }
}

impl EvolutionDecider {
    /// 默认阈值：触发 0.4 / 稳定 0.7 / 最小证据 3.0（对齐原 auto_trigger_min_usages=3）。
    pub fn new() -> Self {
        Self {
            posterior: SkillPosterior::new(),
            evolve_threshold: 0.4,
            stable_threshold: 0.7,
            min_evidence: 3.0,
        }
    }

    /// 覆盖阈值（builder 风格，保留当前后验证据；阈值需满足 `0 < evolve <= stable < 1`）。
    pub fn with_thresholds(
        mut self,
        evolve_threshold: f64,
        stable_threshold: f64,
        min_evidence: f64,
    ) -> Self {
        self.evolve_threshold = evolve_threshold.clamp(0.0, 1.0);
        self.stable_threshold = stable_threshold.clamp(0.0, 1.0);
        self.min_evidence = min_evidence.max(0.0);
        self
    }

    /// 从 `Skill` 累计统计构建后验：
    /// - 成功计数 = `successful_usages`，失败计数 = `total_usages - successful_usages`
    /// - 连续失败（`consecutive_failures`）作为高置信失败信号额外加权（0.5），
    ///   使小样本下连续失败也能快速压低 `P(success)`。
    pub fn from_skill(skill: &crate::skill::Skill) -> Self {
        let successes = skill.successful_usages as f64;
        let total = skill.total_usages as f64;
        let failures = (total - successes).max(0.0);
        let mut posterior = SkillPosterior::from_counts(successes, failures);
        if skill.consecutive_failures > 0 {
            // 连续失败是强负信号：每次额外贡献 0.5 权重失败证据
            posterior.update(0.5, false);
        }
        Self { posterior, evolve_threshold: 0.4, stable_threshold: 0.7, min_evidence: 3.0 }
    }

    /// T5A.4：从进化产物真实执行统计构建后验。
    ///
    /// 与 `from_skill` 对称，但证据源为 `GeneratedToolAdapter::call` 上报的
    /// 真实执行结果（`ToolExecutionStats`）。真实执行是确定证据（无置信度），
    /// 成功/失败各计 1.0 权重，比「按模式推断成败」的决策标签更可信。
    pub fn from_execution_stats(stats: &ToolExecutionStats) -> Self {
        let posterior = SkillPosterior::from_counts(stats.successes as f64, stats.failures as f64);
        Self { posterior, evolve_threshold: 0.4, stable_threshold: 0.7, min_evidence: 3.0 }
    }

    /// 消费单条证据：决策置信度（认知编排器，0~1）+ 执行结果。
    /// 置信度越高该条证据对后验影响越大；零置信度证据被忽略。
    pub fn consume_decision(&mut self, confidence: f64, success: bool) {
        self.posterior.update(confidence, success);
    }

    /// T5A.4：将真实执行反馈并入当前后验（与决策标签流融合）。
    ///
    /// 决策标签流是「按 `executionMode` 推断成败」，真实执行反馈是「实际成败」，
    /// 二者融合可校正推断偏差（真实结果优先）。真实执行为确定证据，每次权重 1.0。
    pub fn consume_execution_stats(&mut self, stats: &ToolExecutionStats) {
        for _ in 0..stats.successes {
            self.consume_decision(1.0, true);
        }
        for _ in 0..stats.failures {
            self.consume_decision(1.0, false);
        }
    }

    /// 消费决策标签流中的单条记录（`consume_decision` 的语义别名，
    /// 便于 T3.3 从 `execution_mode / confidence / route_path` 解析后接入）。
    pub fn consume_evidence(&mut self, confidence: f64, success: bool) {
        self.consume_decision(confidence, success);
    }

    /// T3.3：消费一条决策标签（`DecisionEvidence`）作为贝叶斯证据。
    ///
    /// 仅 `outcome` 为 Success / Failure 的决策标签贡献证据（`clarify`/`ask` 中立，
    /// 不污染后验）；`confidence` 作为证据权重。返回是否真正消费了证据。
    pub fn consume_decision_label(&mut self, evidence: &DecisionEvidence) -> bool {
        match evidence.to_evidence_tuple() {
            Some((confidence, success)) => {
                self.consume_decision(confidence, success);
                true
            },
            None => false,
        }
    }

    /// T3.3：批量消费决策标签流（会话内已持久化的多条 decision JSON）。
    ///
    /// 逐条调用 `consume_decision_label`，返回实际消费的证据条数。
    /// 解析失败或非证据标签自动跳过。
    pub fn consume_decision_labels(&mut self, labels: &[serde_json::Value]) -> usize {
        let mut consumed = 0;
        for label in labels {
            let evidence = DecisionEvidence::from_json(label);
            if self.consume_decision_label(&evidence) {
                consumed += 1;
            }
        }
        consumed
    }

    /// 依据当前后验输出进化决策。
    pub fn decide(&self) -> EvolutionDecision {
        let p = self.posterior.p_success();
        let evidence = self.posterior.evidence_volume();

        // 样本量不足 → 待观察（避免小样本误触发进化）
        if evidence < self.min_evidence {
            return EvolutionDecision::Observe;
        }
        if p < self.evolve_threshold {
            EvolutionDecision::Evolve
        } else if p > self.stable_threshold {
            EvolutionDecision::Stable
        } else {
            EvolutionDecision::Observe
        }
    }

    /// `decide()` 是否触发进化（`EvolutionDecision::Evolve` 的便捷谓词）。
    pub fn should_evolve(&self) -> bool {
        matches!(self.decide(), EvolutionDecision::Evolve)
    }

    /// 当前后验 `P(success)`。
    pub fn p_success(&self) -> f64 {
        self.posterior.p_success()
    }

    /// 当前已积累的（加权）证据量。
    pub fn evidence_volume(&self) -> f64 {
        self.posterior.evidence_volume()
    }

    /// 决策结果 + 原因（用于决策标签 / 日志）。
    pub fn describe(&self) -> (EvolutionDecision, String) {
        let decision = self.decide();
        (decision, decision.reason(self.p_success(), self.evidence_volume()))
    }
}

impl Default for SkillPosterior {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_prior_gives_p50() {
        let posterior = SkillPosterior::new();
        assert!((posterior.p_success() - 0.5).abs() < 1e-9);
        assert_eq!(posterior.evidence_volume(), 0.0);
    }

    #[test]
    fn all_successes_drive_p_success_toward_one() {
        let mut posterior = SkillPosterior::new();
        for _ in 0..100 {
            posterior.update(1.0, true);
        }
        assert!(posterior.p_success() > 0.99);
        assert!((posterior.evidence_volume() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn all_failures_drive_p_success_toward_zero() {
        let mut posterior = SkillPosterior::new();
        for _ in 0..100 {
            posterior.update(1.0, false);
        }
        assert!(posterior.p_success() < 0.01);
    }

    #[test]
    fn high_confidence_evidence_has_larger_influence() {
        // 相同的成败序列，置信度越高后验偏离先验越多
        let mut low = SkillPosterior::new();
        let mut high = SkillPosterior::new();
        for _ in 0..5 {
            low.update(0.1, true);
            high.update(1.0, true);
        }
        // 后验均值随证据加权增大而升高
        assert!(high.p_success() > low.p_success());
        // 证据量按置信度加权累计
        assert!((low.evidence_volume() - 0.5).abs() < 1e-9);
        assert!((high.evidence_volume() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn zero_confidence_evidence_is_ignored() {
        let mut posterior = SkillPosterior::new();
        posterior.update(0.0, false);
        assert!((posterior.p_success() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn should_evolve_respects_threshold() {
        let mut posterior = SkillPosterior::new();
        // 30 次执行 25 次失败 → 后验均值显著低于 0.5
        for _ in 0..30 {
            posterior.update(1.0, false);
        }
        assert!(posterior.should_evolve(0.5));
        assert!((posterior.mean() - posterior.p_success()).abs() < 1e-9);
    }

    #[test]
    fn from_counts_seeds_historical_evidence() {
        // 80 成功 / 20 失败 → Beta(81,21)，P(success) = 81/102 ≈ 0.794
        let posterior = SkillPosterior::from_counts(80.0, 20.0);
        assert!((posterior.p_success() - 0.794).abs() < 1e-3);
        assert!((posterior.evidence_volume() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn std_dev_and_ci_are_well_formed() {
        let mut posterior = SkillPosterior::new();
        for _ in 0..50 {
            posterior.update(1.0, true);
        }
        let std_dev = posterior.std_dev();
        let ci = posterior.lower_ci95();
        assert!(std_dev >= 0.0);
        assert!(ci >= 0.0 && ci <= posterior.p_success());
    }

    // ── EvolutionDecider 测试 ──

    #[test]
    fn decider_new_returns_observe() {
        let decider = EvolutionDecider::new();
        let (decision, reason) = decider.describe();
        assert_eq!(decision, EvolutionDecision::Observe);
        assert!(reason.contains("继续观察"));
    }

    #[test]
    fn decider_evolve_when_below_threshold() {
        let mut decider = EvolutionDecider::new().with_thresholds(0.5, 0.8, 3.0);
        // 5 次全失败 → P(success) 显著低于 0.5
        for _ in 0..5 {
            decider.consume_decision(1.0, false);
        }
        assert_eq!(decider.decide(), EvolutionDecision::Evolve);
        assert!(decider.should_evolve());
    }

    #[test]
    fn decider_stable_when_above_threshold() {
        let mut decider = EvolutionDecider::new().with_thresholds(0.3, 0.6, 3.0);
        // 5 次全成功 → P(success) 显著高于 0.6
        for _ in 0..5 {
            decider.consume_decision(1.0, true);
        }
        assert_eq!(decider.decide(), EvolutionDecision::Stable);
        assert!(!decider.should_evolve());
    }

    #[test]
    fn decider_observe_between_thresholds() {
        let mut decider = EvolutionDecider::new().with_thresholds(0.3, 0.7, 3.0);
        // 3 成功 2 失败 → P(success) ≈ 0.57 介于 0.3~0.7
        for _ in 0..3 {
            decider.consume_decision(1.0, true);
        }
        for _ in 0..2 {
            decider.consume_decision(1.0, false);
        }
        assert_eq!(decider.decide(), EvolutionDecision::Observe);
    }

    #[test]
    fn decider_small_sample_observes_even_on_failures() {
        let mut decider = EvolutionDecider::new().with_thresholds(0.4, 0.7, 5.0);
        // 小样本（2 条证据 < min_evidence=5）即使全失败 → 观察，等待更多证据避免误触发
        decider.consume_decision(1.0, false);
        decider.consume_decision(1.0, false);
        assert_eq!(decider.decide(), EvolutionDecision::Observe);
    }

    #[test]
    fn decider_from_skill_uses_historical_counts() {
        let skill = test_skill(10, 2, 3);
        let decider = EvolutionDecider::from_skill(&skill);
        // 8 失败 / 2 成功 + 3 次连续失败额外加权(0.5) → P(success) 很低
        assert!(decider.p_success() < 0.4);
        assert!(decider.should_evolve());
    }

    // ── T5A.4：真实执行反馈作为证据源 ──

    #[test]
    fn decider_from_execution_stats_uses_real_feedback() {
        // 真实执行 8 次成功 / 2 次失败 → Beta(9,3)，P(success)=9/12=0.75
        let stats = ToolExecutionStats { usage_count: 10, successes: 8, failures: 2 };
        let decider = EvolutionDecider::from_execution_stats(&stats);
        assert!((decider.p_success() - 0.75).abs() < 1e-9);
        assert!((decider.evidence_volume() - 10.0).abs() < 1e-9);
        assert_eq!(decider.decide(), EvolutionDecision::Stable);
    }

    #[test]
    fn decider_consume_execution_stats_fuses_with_labels() {
        // 决策标签流推断 5 次成功 → P(success)≈0.857 → stable；
        // 再融合 3 次真实失败 → 后验降至 0.6 → observe（真实反馈校正推断偏差）
        let mut decider = EvolutionDecider::new().with_thresholds(0.4, 0.7, 3.0);
        for _ in 0..5 {
            decider.consume_decision(1.0, true);
        }
        assert_eq!(decider.decide(), EvolutionDecision::Stable);
        decider.consume_execution_stats(&ToolExecutionStats {
            usage_count: 3,
            successes: 0,
            failures: 3,
        });
        assert_eq!(decider.decide(), EvolutionDecision::Observe);
    }

    #[test]
    fn decider_consume_evidence_works() {
        let mut decider = EvolutionDecider::new().with_thresholds(0.4, 0.7, 2.0);
        decider.consume_evidence(0.8, true);
        decider.consume_evidence(0.9, true);
        decider.consume_evidence(0.7, true);
        // 证据量 2.4 >= min_evidence=2.0，P(success) ≈ 0.77 > 0.7 → Stable
        assert_eq!(decider.decide(), EvolutionDecision::Stable);
    }

    #[test]
    fn decider_low_confidence_evidence_has_less_impact() {
        // 同样 10 条失败：高置信度证据量 10.0（>= min_evidence）→ Evolve；
        // 低置信度证据量 1.0（< min_evidence）→ 观察（低置信证据不触发进化）
        let mut low = EvolutionDecider::new().with_thresholds(0.4, 0.7, 3.0);
        let mut high = EvolutionDecider::new().with_thresholds(0.4, 0.7, 3.0);
        for _ in 0..10 {
            low.consume_decision(0.1, false);
            high.consume_decision(1.0, false);
        }
        assert_eq!(low.decide(), EvolutionDecision::Observe);
        assert_eq!(high.decide(), EvolutionDecision::Evolve);
    }

    // ── T3.3：决策标签作为证据源 ──

    #[test]
    fn decision_evidence_parses_from_json() {
        let json = serde_json::json!({
            "executionMode": "workflow",
            "routePath": "invest/stock_analysis/tech",
            "confidence": 0.92,
            "selectedWorkflowName": "股票技术面分析",
            "selectedAgentProfile": { "id": "p1", "name": "分析师", "role": "analyst", "expert": "ex1" }
        });
        let evidence = DecisionEvidence::from_json(&json);
        assert_eq!(evidence.execution_mode, "workflow");
        assert_eq!(evidence.route_path, "invest/stock_analysis/tech");
        assert!((evidence.confidence - 0.92).abs() < 1e-9);
        assert_eq!(evidence.selected_workflow_name.as_deref(), Some("股票技术面分析"));
        assert!(evidence.selected_agent_profile.is_some());
    }

    #[test]
    fn decision_evidence_missing_fields_use_defaults() {
        let json = serde_json::json!({ "executionMode": "rejected" });
        let evidence = DecisionEvidence::from_json(&json);
        assert_eq!(evidence.route_path, "");
        assert!((evidence.confidence - 0.0).abs() < 1e-9);
        assert!(evidence.selected_workflow_name.is_none());
        assert!(evidence.selected_agent_profile.is_none());
    }

    #[test]
    fn decision_evidence_outcome_mapping() {
        // 执行路径 → 成功
        for mode in ["workflow", "direct", "parameter_extract", "agent", "plan"] {
            let ev = DecisionEvidence::from_json(&serde_json::json!({ "executionMode": mode }));
            assert_eq!(ev.outcome(), EvidenceOutcome::Success, "模式 {mode} 应为成功");
        }
        // 拒绝/补齐 → 失败
        for mode in ["rejected", "gap_proposal"] {
            let ev = DecisionEvidence::from_json(&serde_json::json!({ "executionMode": mode }));
            assert_eq!(ev.outcome(), EvidenceOutcome::Failure, "模式 {mode} 应为失败");
        }
        // 模糊决策 → 中立
        for mode in ["clarify", "ask", ""] {
            let ev = DecisionEvidence::from_json(&serde_json::json!({ "executionMode": mode }));
            assert_eq!(ev.outcome(), EvidenceOutcome::Neutral, "模式 {mode} 应为中立");
        }
    }

    #[test]
    fn decision_evidence_to_tuple_respects_outcome() {
        let success = DecisionEvidence::from_json(
            &serde_json::json!({ "executionMode": "workflow", "confidence": 0.9 }),
        );
        assert_eq!(success.to_evidence_tuple(), Some((0.9, true)));

        let failure = DecisionEvidence::from_json(
            &serde_json::json!({ "executionMode": "rejected", "confidence": 1.0 }),
        );
        assert_eq!(failure.to_evidence_tuple(), Some((1.0, false)));

        let neutral = DecisionEvidence::from_json(
            &serde_json::json!({ "executionMode": "clarify", "confidence": 0.8 }),
        );
        assert_eq!(neutral.to_evidence_tuple(), None);
    }

    #[test]
    fn decider_consumes_decision_label_stream() {
        // 决策标签流：5 次高置信执行成功 + 1 次拒绝 → P(success)≈0.85 > 0.7 → Stable
        let mut decider = EvolutionDecider::new().with_thresholds(0.4, 0.7, 3.0);
        let labels = vec![
            serde_json::json!({ "executionMode": "workflow", "confidence": 0.9 }),
            serde_json::json!({ "executionMode": "direct", "confidence": 0.95 }),
            serde_json::json!({ "executionMode": "parameter_extract", "confidence": 0.8 }),
            serde_json::json!({ "executionMode": "workflow", "confidence": 0.9 }),
            serde_json::json!({ "executionMode": "plan", "confidence": 0.85 }),
            serde_json::json!({ "executionMode": "rejected", "confidence": 1.0 }),
        ];
        let consumed = decider.consume_decision_labels(&labels);
        assert_eq!(consumed, 6);
        assert_eq!(decider.decide(), EvolutionDecision::Stable);
    }

    #[test]
    fn decider_skips_neutral_labels() {
        let mut decider = EvolutionDecider::new().with_thresholds(0.4, 0.7, 3.0);
        let labels = vec![
            serde_json::json!({ "executionMode": "clarify", "confidence": 0.8 }),
            serde_json::json!({ "executionMode": "ask", "confidence": 0.6 }),
            serde_json::json!({ "executionMode": "workflow", "confidence": 0.9 }),
        ];
        let consumed = decider.consume_decision_labels(&labels);
        assert_eq!(consumed, 1); // 仅 workflow 消费，两条中立跳过
        assert_eq!(decider.evidence_volume(), 0.9);
    }

    #[test]
    fn decider_rejection_stream_drives_evolve() {
        // 连续拒绝（安全拦截/补齐失败）→ 后验压低 → 触发进化
        let mut decider = EvolutionDecider::new().with_thresholds(0.4, 0.7, 3.0);
        for _ in 0..5 {
            decider.consume_decision_label(&DecisionEvidence::from_json(
                &serde_json::json!({ "executionMode": "rejected", "confidence": 1.0 }),
            ));
        }
        assert_eq!(decider.decide(), EvolutionDecision::Evolve);
    }

    /// 构造最小 `Skill` 测试实例（Skill 未实现 Default，需手动构造）。
    fn test_skill(
        total_usages: u32,
        successful_usages: u32,
        consecutive_failures: u32,
    ) -> crate::skill::Skill {
        use chrono::Utc;
        crate::skill::Skill {
            id: "skill-test".to_string(),
            name: "测试技能".to_string(),
            description: "test skill".to_string(),
            version: "1.0.0".to_string(),
            content: "1. Use write_file\n2. Verify output".to_string(),
            category: "test".to_string(),
            tags: vec![],
            platforms: vec![],
            scenarios: vec![],
            quality_score: 0.5,
            success_rate: if total_usages > 0 {
                successful_usages as f64 / total_usages as f64
            } else {
                0.0
            },
            avg_execution_time_ms: 100,
            total_usages,
            successful_usages,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_used_at: None,
            consecutive_failures,
            last_failure_at: None,
            metadata: crate::skill::SkillMetadata::default(),
        }
    }
}
