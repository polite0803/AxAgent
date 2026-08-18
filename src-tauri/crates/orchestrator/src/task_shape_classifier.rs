// SPDX-License-Identifier: AGPL-3.0-only

//! 默认任务形态分类器实现 — 原则三标尺的规则化落地。
//!
//! 基于「上下文保留成本 × 安全隔离需求」两条标尺的关键词匹配 + 启发式评分，
//! 输出 [`TaskShapeDecision`] 供下游编排消费。无 LLM 调用，确定性、零成本。
//!
//! ## 设计权衡
//!
//! - 规则化而非 LLM 化：P1 阶段优先保证可观测性与可回退性，LLM 化分类留待 P2。
//! - 关键词分层：高危动作词（转账/发布/删除）优先级最高，直接锁定 `ApprovalAndAirGap`；
//!   专业领域词（审计/合同/密钥）次之，触发 `SandboxRequired`；长周期词（项目/迭代/跟踪）
//!   影响 `ContextRetentionCost`；其余回退到 `HandleLocally`。
//! - 评分公式：`merge_score = context_cost / 3.0`（高连贯性 → 倾向合并）；
//!   `split_score = isolation_need / 3.0`（高隔离需求 → 倾向拆分）。二者独立计算，
//!   下游按 `should_merge()` 判断主导方向。

use axagent_harness::runtime_types::permissions::PermissionMode;
use axagent_harness::runtime_types::session::Session;
use axagent_harness::task_shape::{
    ContextRetentionCost, ExecutionStrategy, SecurityIsolationNeed, TaskShapeClassifier,
    TaskShapeDecision, TaskShapeLlmClassifier,
};

/// 默认任务形态分类器（规则化实现）。
///
/// 见模块文档的设计权衡。无外部依赖，线程安全。
pub struct DefaultTaskShapeClassifier;

impl DefaultTaskShapeClassifier {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultTaskShapeClassifier {
    fn default() -> Self {
        Self::new()
    }
}

// ── 关键词表（按优先级从高到低）──────────────────────────────

/// 高危动作词 → `ApprovalAndAirGap`（强制审批 + 气隔）
const HIGH_RISK_KEYWORDS: &[&str] = &[
    "转账",
    "付款",
    "支付",
    "transfer",
    "payment",
    "pay",
    "发布",
    "上线",
    "部署到生产",
    "deploy to prod",
    "publish",
    "release to prod",
    "删除生产",
    "清空数据库",
    "drop table",
    "delete from",
    "rm -rf",
    "撤销",
    "回滚生产",
    "rollback prod",
    "授权",
    " granting",
    "permission grant",
];

/// 专业领域词 → `SandboxRequired`（强制沙箱）
const SANDBOX_KEYWORDS: &[&str] = &[
    "审计",
    "audit",
    "财报",
    "财务报表",
    "financial statement",
    "合同",
    "contract",
    "法务",
    "legal",
    "密钥",
    "私钥",
    "private key",
    "secret",
    "credential",
    "合规",
    "compliance",
    "尽调",
    "due diligence",
    "税务",
    "tax",
    "发票",
    "invoice",
];

/// 工作区写入词 → `WorkspaceWrite`（常规代码/文件操作）
const WORKSPACE_WRITE_KEYWORDS: &[&str] = &[
    "写",
    "修改",
    "创建",
    "实现",
    "重构",
    "refactor",
    "implement",
    "write",
    "create",
    "update",
    "fix",
    "修复",
    "build",
    "构建",
];

/// 长周期任务词 → 提升 `ContextRetentionCost`
const LONG_RUNNING_KEYWORDS: &[&str] = &[
    "项目",
    "project",
    "迭代",
    "iteration",
    "跟踪",
    "track",
    "持续",
    "continuous",
    "长期",
    "long-term",
    "里程碑",
    "milestone",
    "阶段",
    "phase",
    "演进",
    "evolve",
];

/// 跨天持续词 → `PersistentJourney`
const PERSISTENT_KEYWORDS: &[&str] = &[
    "跨天",
    "持续演进",
    "长期跟进",
    "ongoing journey",
    "persistent",
    "陪伴",
    "全程",
    "end-to-end over time",
];

#[async_trait::async_trait]
impl TaskShapeClassifier for DefaultTaskShapeClassifier {
    async fn classify(
        &self,
        user_input: &str,
        _session: &Session,
        active_permission: PermissionMode,
    ) -> Result<TaskShapeDecision, String> {
        Ok(classify_input(user_input, active_permission))
    }
}

/// 任务形态分类核心函数（同步，便于测试）。
///
/// 输入用户原始文本 + 当前会话权限模式，输出 [`TaskShapeDecision`]。
/// 规则匹配顺序：高危动作 → 专业领域 → 工作区写入 → 长周期/持久化。
#[must_use]
pub fn classify_input(user_input: &str, active_permission: PermissionMode) -> TaskShapeDecision {
    let lower = user_input.to_lowercase();
    let mut evidence: Vec<String> = Vec::new();

    // ── 标尺 1：安全隔离需求（优先级从高到低）──
    let (isolation_need, _matched_risk) = if contains_any(&lower, HIGH_RISK_KEYWORDS) {
        evidence.push(format!("keyword(high-risk): {}", matched_first(&lower, HIGH_RISK_KEYWORDS)));
        (SecurityIsolationNeed::ApprovalAndAirGap, 3)
    } else if contains_any(&lower, SANDBOX_KEYWORDS) {
        evidence.push(format!("keyword(sandbox): {}", matched_first(&lower, SANDBOX_KEYWORDS)));
        (SecurityIsolationNeed::SandboxRequired, 2)
    } else if contains_any(&lower, WORKSPACE_WRITE_KEYWORDS) {
        evidence.push(format!(
            "keyword(workspace-write): {}",
            matched_first(&lower, WORKSPACE_WRITE_KEYWORDS)
        ));
        (SecurityIsolationNeed::WorkspaceWrite, 1)
    } else {
        (SecurityIsolationNeed::ReadOnlyThought, 0)
    };

    // ── 标尺 2：上下文保留成本（独立判断）──
    let (context_cost, _matched_ctx) = if contains_any(&lower, PERSISTENT_KEYWORDS) {
        evidence
            .push(format!("keyword(persistent): {}", matched_first(&lower, PERSISTENT_KEYWORDS)));
        (ContextRetentionCost::PersistentJourney, 3)
    } else if contains_any(&lower, LONG_RUNNING_KEYWORDS) {
        evidence.push(format!(
            "keyword(long-running): {}",
            matched_first(&lower, LONG_RUNNING_KEYWORDS)
        ));
        (ContextRetentionCost::LongRunningProject, 2)
    } else if has_follow_up_indicator(&lower) || _session_has_history_hint(user_input) {
        // 用户输入暗示后续会继续（如"然后"、"接下来"、"继续上次的"）
        (ContextRetentionCost::Resumable, 1)
    } else {
        (ContextRetentionCost::Ephemeral, 0)
    };

    // ── 评分：两条标尺独立打分，0.0~1.0 ──
    // merge_score：上下文成本越高，越倾向合并（减少转述损耗）
    // split_score：隔离需求越高，越倾向拆分（保证精度 + 物理权限墙）
    // 历史会话存在时，merge_score 额外加成（已有上下文不应丢弃）
    let mut merge_score = (context_cost as u8) as f32 / 3.0;
    let split_score = (isolation_need as u8) as f32 / 3.0;

    // 权限模式影响：当前已是 Prompt 模式 → 默认偏向拆分（用户已设定严格权限）
    if active_permission == PermissionMode::Prompt {
        merge_score *= 0.7;
    }
    // 高危动作无论上下文成本多高，都强制拆分（安全优先于连贯性）
    if isolation_need == SecurityIsolationNeed::ApprovalAndAirGap {
        merge_score = merge_score.min(0.3);
    }
    merge_score = merge_score.clamp(0.0, 1.0);

    // ── 执行策略推导：两条标尺交叉决策 ──
    let recommended_strategy = decide_strategy(context_cost, isolation_need);

    TaskShapeDecision {
        context_cost,
        isolation_need,
        recommended_strategy,
        merge_score,
        split_score,
        evidence,
    }
}

/// 根据两条标尺交叉推导执行策略。
///
/// 决策矩阵（行=上下文成本，列=隔离需求）：
/// ```text
///                 ReadOnly  WorkspaceWrite  Sandbox  ApprovalAndAirGap
/// Ephemeral       Local     Local           Single   ApprovalGate
/// Resumable       Local     Local           Single   ApprovalGate
/// LongRunning     Local     Dag             Single   ApprovalGate
/// Persistent      Local     Dag             Parallel ApprovalGate
/// ```
fn decide_strategy(
    context_cost: ContextRetentionCost,
    isolation_need: SecurityIsolationNeed,
) -> ExecutionStrategy {
    use ContextRetentionCost as C;
    use SecurityIsolationNeed as S;

    match (context_cost, isolation_need) {
        // 高危动作 → 强制审批门，无论上下文成本
        (_, S::ApprovalAndAirGap) => ExecutionStrategy::ApprovalGate,

        // 沙箱级专业领域 → 委派单专家（保证专业精度 + 隔离上下文）
        // 高上下文成本 + 沙箱 → 仍委派单专家（专家内部保持上下文，主控不污染）
        (_, S::SandboxRequired) => ExecutionStrategy::DelegateSingleExpert {
            expert_id: infer_expert_id(context_cost, isolation_need),
        },

        // 工作区写入 + 长周期 → Orchestrator DAG（可断点续传 + 监控重规划）
        (C::LongRunningProject | C::PersistentJourney, S::WorkspaceWrite) => {
            ExecutionStrategy::OrchestratorDag
        },

        // 持久化旅程 + 只读思考 → 并行专家（多视角长期跟踪，如研究/调研）
        (C::PersistentJourney, S::ReadOnlyThought) => {
            ExecutionStrategy::ParallelExperts { expert_ids: infer_parallel_experts() }
        },

        // 其余组合 → 主控自行处理（高上下文 + 低风险，合并优先）
        _ => ExecutionStrategy::HandleLocally,
    }
}

/// 根据上下文成本推断专家 ID（简化版，P2 可接入专家注册表）。
fn infer_expert_id(
    _context_cost: ContextRetentionCost,
    isolation_need: SecurityIsolationNeed,
) -> String {
    match isolation_need {
        SecurityIsolationNeed::SandboxRequired => "sandbox-specialist".to_string(),
        SecurityIsolationNeed::ApprovalAndAirGap => "approval-gate".to_string(),
        _ => "default".to_string(),
    }
}

/// 持久化旅程 + 只读思考场景的并行专家列表（简化版）。
fn infer_parallel_experts() -> Vec<String> {
    vec!["researcher".to_string(), "synthesizer".to_string()]
}

/// 检测后续跟进指示词（"然后"、"接下来"、"继续"）。
fn has_follow_up_indicator(lower: &str) -> bool {
    lower.contains("然后")
        || lower.contains("接下来")
        || lower.contains("继续")
        || lower.contains("接着")
        || lower.contains("之后")
        || lower.contains("then")
        || lower.contains("next")
        || lower.contains("continue")
}

/// 简化版会话历史检测：用户输入引用了之前的对话（如"上次说的"、"刚才那个"）。
///
/// 注：完整 Session 在调用方持有，此处仅做文本启发式判断，
/// 避免在分类器内重复扫描 session.messages（性能 + 耦合）。
fn _session_has_history_hint(user_input: &str) -> bool {
    user_input.contains("上次")
        || user_input.contains("刚才")
        || user_input.contains("之前")
        || user_input.contains("前面")
        || user_input.contains("previous")
}

/// 检查 `text` 是否包含关键词表中的任一词（大小写不敏感，调用方需先 lowercase）。
fn contains_any(lower_text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|k| lower_text.contains(k))
}

/// 返回第一个命中的关键词（用于 evidence 记录）。
fn matched_first<'a>(lower_text: &str, keywords: &[&'a str]) -> &'a str {
    keywords.iter().find(|k| lower_text.contains(*k)).copied().unwrap_or("")
}

// ── Hybrid 分类入口（规则优先 + LLM 兜底，P2） ──────────────────

/// 判断规则结果是否「模糊」（需要 LLM 兜底）。
///
/// 触发条件（任一命中）：
/// - 合并/拆分倾向差值 < 0.2（两轴打平，规则无法自信判断）
/// - 关键词证据 < 2 条（证据过于稀疏，可能漏判）
fn is_ambiguous(rule_result: &TaskShapeDecision) -> bool {
    let diff = (rule_result.merge_score - rule_result.split_score).abs();
    diff < 0.2 || rule_result.evidence.len() < 2
}

/// 混合分类入口：规则优先 + 规则置信度低时调用 LLM 分类器（若提供）。
///
/// ## 流程
///
/// 1. 先走规则分类（`classify_input`，零成本确定性判断）
/// 2. 若结果明确（非模糊）→ 直接返回规则结论
/// 3. 若模糊：
///    - `llm_classifier = None` → 回退规则结论（调用方未提供 LLM 分类器）
///    - `llm_classifier = Some(...)` → 异步调用 `refine()`，三种返回：
///      - `Ok(Some(refined))` → 返回 LLM 精炼结论
///      - `Ok(None)` → LLM 仍无法判断 → 回退规则结论
///      - `Err(e)` → LLM 失败，写 warning，回退规则（绝不阻断主流程）
///
/// ## 设计约束
///
/// - **永远不阻断**：LLM 分类失败必须静默回退规则，不可向上层报错
/// - **只在模糊时调用**：明确场景不浪费 LLM token，90%+ 场景零成本
pub async fn classify_hybrid(
    user_input: &str,
    active_permission: PermissionMode,
    llm_classifier: Option<&dyn TaskShapeLlmClassifier>,
) -> Result<TaskShapeDecision, String> {
    let rule_result = classify_input(user_input, active_permission);
    if !is_ambiguous(&rule_result) {
        return Ok(rule_result);
    }
    let Some(llm) = llm_classifier else { return Ok(rule_result) };
    match llm.refine(user_input, &rule_result).await {
        Ok(Some(refined)) => {
            tracing::debug!(
                rule_context = ?rule_result.context_cost,
                llm_context = ?refined.context_cost,
                rule_isolation = ?rule_result.isolation_need,
                llm_isolation = ?refined.isolation_need,
                "🧭 LLM 分类修正规则结论"
            );
            Ok(refined)
        },
        Ok(None) => Ok(rule_result),
        Err(e) => {
            tracing::warn!(error = %e, "LLM 分类失败，回退规则结论");
            Ok(rule_result)
        },
    }
}

// ── 测试 ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn classify_simple(input: &str) -> TaskShapeDecision {
        classify_input(input, PermissionMode::WorkspaceWrite)
    }

    #[test]
    fn high_risk_keyword_triggers_approval_gate() {
        let d = classify_simple("帮我转账 1000 元到张三账户");
        assert_eq!(d.isolation_need, SecurityIsolationNeed::ApprovalAndAirGap);
        assert!(matches!(d.recommended_strategy, ExecutionStrategy::ApprovalGate));
        assert!(d.split_score > d.merge_score, "高危任务应偏向拆分");
        assert!(d.merge_score <= 0.3, "高危任务合并倾向应被压制");
    }

    #[test]
    fn sandbox_keyword_triggers_single_expert() {
        let d = classify_simple("帮我审计这份财报");
        assert_eq!(d.isolation_need, SecurityIsolationNeed::SandboxRequired);
        assert!(matches!(d.recommended_strategy, ExecutionStrategy::DelegateSingleExpert { .. }));
    }

    #[test]
    fn workspace_write_long_running_triggers_dag() {
        let d = classify_simple("重构这个项目的认证模块");
        assert_eq!(d.isolation_need, SecurityIsolationNeed::WorkspaceWrite);
        assert_eq!(d.context_cost, ContextRetentionCost::LongRunningProject);
        assert!(matches!(d.recommended_strategy, ExecutionStrategy::OrchestratorDag));
    }

    #[test]
    fn persistent_readonly_triggers_parallel_experts() {
        // 使用 PERSISTENT_KEYWORDS 专属词（跨天/持续演进），避免与 LONG_RUNNING_KEYWORDS 重叠
        let d = classify_simple("这是一个跨天持续演进的研究课题");
        assert_eq!(d.context_cost, ContextRetentionCost::PersistentJourney);
        assert_eq!(d.isolation_need, SecurityIsolationNeed::ReadOnlyThought);
        assert!(matches!(d.recommended_strategy, ExecutionStrategy::ParallelExperts { .. }));
    }

    #[test]
    fn simple_question_defaults_to_local() {
        let d = classify_simple("今天天气怎么样");
        assert_eq!(d.context_cost, ContextRetentionCost::Ephemeral);
        assert_eq!(d.isolation_need, SecurityIsolationNeed::ReadOnlyThought);
        assert!(matches!(d.recommended_strategy, ExecutionStrategy::HandleLocally));
        assert!(d.should_merge(), "简单问题应倾向合并");
    }

    #[test]
    fn follow_up_indicator_raises_context_cost() {
        let d = classify_simple("然后继续上次的任务");
        assert!(d.context_cost >= ContextRetentionCost::Resumable);
    }

    #[test]
    fn prompt_permission_reduces_merge_score() {
        let d_ws = classify_input("帮我分析一下数据", PermissionMode::WorkspaceWrite);
        let d_prompt = classify_input("帮我分析一下数据", PermissionMode::Prompt);
        assert!(d_prompt.merge_score <= d_ws.merge_score, "Prompt 模式应降低合并倾向");
    }

    #[test]
    fn evidence_records_matched_keywords() {
        let d = classify_simple("重构项目并审计代码");
        assert!(!d.evidence.is_empty(), "应记录命中证据");
        assert!(d.evidence.iter().any(|e| e.contains("keyword")));
    }

    #[test]
    fn english_keywords_matched_case_insensitive() {
        let d = classify_simple("Please deploy to prod environment");
        assert_eq!(d.isolation_need, SecurityIsolationNeed::ApprovalAndAirGap);
    }
}
