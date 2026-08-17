// SPDX-License-Identifier: AGPL-3.0-only

//! 任务形态 LLM 兜底分类器 — `TaskShapeLlmClassifier` 的 wiring 层实现。
//!
//! 当规则分类器（`classify_input`）置信度不足时（`is_ambiguous`），
//! `classify_hybrid` 会调用此适配器发起轻量 LLM 请求，让 LLM 修正两轴判断。
//!
//! ## 设计约束
//!
//! - 任何失败都返回 `Ok(None)`（回退规则结论），**绝不阻断主流程**
//! - LLM 只判断 `context_cost` + `isolation_need` + 证据 + 评分，
//!   `recommended_strategy` 由代码用 `decide_strategy` 重新推导
//!   （避免 LLM 不熟悉策略映射表）
//! - 复用 `llm_helpers::chat_with_default_provider` 统一 LLM 调用入口

use axagent_harness::task_shape::{
    ContextRetentionCost, ExecutionStrategy, SecurityIsolationNeed, TaskShapeDecision,
    TaskShapeLlmClassifier,
};

use crate::harness::RuntimeHarness;
use crate::llm_helpers::chat_with_default_provider;

/// LLM 兜底分类器的 wiring 层实现。
///
/// 持有 `RuntimeHarness`（Clone），用默认提供商做单轮非流式 chat。
/// 由 `init/state.rs` 注入到 `AppState.task_shape_llm_classifier`。
pub struct ProviderTaskShapeLlmClassifier {
    harness: RuntimeHarness,
}

impl ProviderTaskShapeLlmClassifier {
    pub fn new(harness: RuntimeHarness) -> Self {
        Self { harness }
    }
}

/// LLM 输出的 JSON 结构（只含 LLM 需要判断的字段）。
///
/// 注意：字段名用 snake_case（与 harness 中 enum 的 serde rename_all = "snake_case" 对齐），
/// 而非 TaskShapeDecision 的 camelCase。因为 LLM 直接输出 enum 变体名，
/// 我们在反序列化后手动组装完整的 TaskShapeDecision。
#[derive(serde::Deserialize)]
struct LlmClassificationResult {
    context_cost: ContextRetentionCost,
    isolation_need: SecurityIsolationNeed,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default = "default_merge")]
    merge_score: f32,
    #[serde(default = "default_split")]
    split_score: f32,
}

fn default_merge() -> f32 {
    0.5
}

fn default_split() -> f32 {
    0.5
}

const SYSTEM_PROMPT: &str = r#"你是一个任务形态分类器。根据用户输入，判断两个维度：

1. **上下文保留成本**（context_cost）：任务需要保持上下文连贯性的程度
   - "ephemeral"：一次性问答，无需保留上下文
   - "resumable"：可断点续传，中等连贯性
   - "long_running_project"：长周期项目，高连贯性
   - "persistent_journey"：跨天持续演进，极高连贯性

2. **安全隔离需求**（isolation_need）：任务对权限隔离的要求
   - "read_only_thought"：纯读取/思考，不需隔离
   - "workspace_write"：工作区写入，常规操作
   - "sandbox_required"：专业领域（法务/财务/密钥），需沙箱
   - "approval_and_air_gap"：高危操作（转账/发布/删除），需审批+气隔

只输出 JSON，不要解释：
{"context_cost": "ephemeral", "isolation_need": "read_only_thought", "evidence": ["理由1"], "merge_score": 0.8, "split_score": 0.2}

merge_score：倾向合并的程度（0-1，高上下文成本→高合并倾向）
split_score：倾向拆分的程度（0-1，高隔离需求→高拆分倾向）"#;

#[async_trait::async_trait]
impl TaskShapeLlmClassifier for ProviderTaskShapeLlmClassifier {
    async fn refine(
        &self,
        user_input: &str,
        rule_result: &TaskShapeDecision,
    ) -> Result<Option<TaskShapeDecision>, String> {
        // 构造 user prompt：包含原始输入 + 规则层结论（供 LLM 参考）
        let user_prompt = format!(
            "用户输入：{user_input}\n\n规则分类器结论（供参考，可能不准确）：\n\
             - context_cost: {rule_cc:?}\n\
             - isolation_need: {rule_iso:?}\n\
             - evidence: {rule_ev:?}\n\n\
             请重新判断并输出 JSON：",
            rule_cc = rule_result.context_cost,
            rule_iso = rule_result.isolation_need,
            rule_ev = rule_result.evidence,
        );

        // 调用 LLM（max_tokens=256 足够输出短 JSON）
        let content =
            match chat_with_default_provider(&self.harness, SYSTEM_PROMPT, &user_prompt, 256).await
            {
                Ok(text) => text,
                Err(e) => {
                    tracing::warn!(error = %e, "TaskShape LLM 调用失败，回退规则结论");
                    return Ok(None);
                },
            };

        // 从 LLM 响应中提取 JSON（可能包含 ```json 包裹或多余文本）
        let json_str = extract_json(&content);

        // 反序列化
        let llm_result: LlmClassificationResult = match serde_json::from_str(json_str) {
            Ok(parsed) => parsed,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    raw = %content,
                    "TaskShape LLM 输出 JSON 解析失败，回退规则结论"
                );
                return Ok(None);
            },
        };

        // 根据 LLM 判断的两轴重新推导策略（不让 LLM 直接决定策略）
        let recommended_strategy =
            decide_strategy(llm_result.context_cost, llm_result.isolation_need);

        tracing::debug!(
            llm_context = ?llm_result.context_cost,
            llm_isolation = ?llm_result.isolation_need,
            rule_context = ?rule_result.context_cost,
            rule_isolation = ?rule_result.isolation_need,
            "🧭 LLM 分类器修正完成"
        );

        Ok(Some(TaskShapeDecision {
            context_cost: llm_result.context_cost,
            isolation_need: llm_result.isolation_need,
            recommended_strategy,
            merge_score: llm_result.merge_score,
            split_score: llm_result.split_score,
            evidence: llm_result.evidence,
        }))
    }
}

/// 从 LLM 响应文本中提取 JSON 字符串。
///
/// 处理三种常见格式：
/// 1. 纯 JSON：`{"key": "value"}`
/// 2. Markdown 包裹：```` ```json\n{"key": "value"}\n``` ````
/// 3. JSON 前后有解释文字：`好的，结果是：{"key": "value"}`
fn extract_json(text: &str) -> &str {
    let trimmed = text.trim();

    // Case 2: ```json ... ```
    if let Some(start) = trimmed.find("```json") {
        let after = &trimmed[start + 7..];
        if let Some(end) = after.rfind("```") {
            return after[..end].trim();
        }
    }

    // Case 2b: ``` ... ```
    if let Some(start) = trimmed.find("```") {
        let after = &trimmed[start + 3..];
        if let Some(end) = after.rfind("```") {
            return after[..end].trim();
        }
    }

    // Case 1 & 3: 找第一个 { 和最后一个 }
    if let Some(start) = trimmed.find('{')
        && let Some(end) = trimmed.rfind('}')
    {
        return &trimmed[start..=end];
    }

    trimmed
}

/// 根据两轴维度推导执行策略。
///
/// 与 orchestrator 中 `decide_strategy` 逻辑一致（铁律 12：复用而非重定义），
/// 但此处为内联副本——因为 orchestrator 的 `decide_strategy` 是私有函数，
/// 且 runtime（wiring）→ orchestrator（consumer）的依赖方向不成立
/// （runtime 依赖 orchestrator 是 wiring → consumer，允许但 orchestrator
/// 不应暴露内部启发式）。若后续 orchestrator 公开此函数，改 pub use 引用。
fn decide_strategy(
    context_cost: ContextRetentionCost,
    isolation_need: SecurityIsolationNeed,
) -> ExecutionStrategy {
    use ContextRetentionCost as C;
    use SecurityIsolationNeed as S;

    match (context_cost, isolation_need) {
        // 高危动作 → 强制审批门
        (_, S::ApprovalAndAirGap) => ExecutionStrategy::ApprovalGate,

        // 沙箱级专业领域 → 委派单专家
        (_, S::SandboxRequired) => ExecutionStrategy::DelegateSingleExpert {
            expert_id: infer_expert_id(context_cost, isolation_need),
        },

        // 高隔离 + 高上下文 → DAG 编排
        (C::LongRunningProject | C::PersistentJourney, S::WorkspaceWrite) => {
            ExecutionStrategy::OrchestratorDag
        },

        // 极高上下文 + 低隔离 → 并行专家
        (C::PersistentJourney, S::ReadOnlyThought) => ExecutionStrategy::ParallelExperts {
            expert_ids: vec!["researcher".to_string(), "synthesizer".to_string()],
        },

        // 默认 → 本地处理
        _ => ExecutionStrategy::HandleLocally,
    }
}

/// 推断专家 ID（简化版，与 orchestrator 中 `infer_expert_id` 对齐）。
fn infer_expert_id(
    context_cost: ContextRetentionCost,
    isolation_need: SecurityIsolationNeed,
) -> String {
    use ContextRetentionCost as C;
    use SecurityIsolationNeed as S;

    match (context_cost, isolation_need) {
        (_, S::SandboxRequired) => match context_cost {
            C::PersistentJourney => "auditor".to_string(),
            C::LongRunningProject => "analyst".to_string(),
            _ => "specialist".to_string(),
        },
        _ => "specialist".to_string(),
    }
}

// ── 测试 ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_pure() {
        let result = extract_json(r#"{"context_cost": "ephemeral"}"#);
        assert!(result.contains("ephemeral"));
    }

    #[test]
    fn extract_json_markdown_wrapped() {
        let result = extract_json("```json\n{\"context_cost\": \"resumable\"}\n```");
        assert!(result.contains("resumable"));
    }

    #[test]
    fn extract_json_with_prefix() {
        let result = extract_json("好的，结果是：{\"context_cost\": \"ephemeral\"} 就这样");
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
    }

    #[test]
    fn llm_result_deserializes() {
        let json = r#"{
            "context_cost": "long_running_project",
            "isolation_need": "workspace_write",
            "evidence": ["关键词命中: 重构"],
            "merge_score": 0.6,
            "split_score": 0.4
        }"#;
        let result: LlmClassificationResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.context_cost, ContextRetentionCost::LongRunningProject);
        assert_eq!(result.isolation_need, SecurityIsolationNeed::WorkspaceWrite);
        assert!(!result.evidence.is_empty());
    }

    #[test]
    fn decide_strategy_matches_orchestrator() {
        // 高危 → ApprovalGate
        assert!(matches!(
            decide_strategy(
                ContextRetentionCost::Ephemeral,
                SecurityIsolationNeed::ApprovalAndAirGap
            ),
            ExecutionStrategy::ApprovalGate
        ));

        // 沙箱 → DelegateSingleExpert
        assert!(matches!(
            decide_strategy(
                ContextRetentionCost::Ephemeral,
                SecurityIsolationNeed::SandboxRequired
            ),
            ExecutionStrategy::DelegateSingleExpert { .. }
        ));

        // 高上下文 + 工作区写入 → DAG
        assert!(matches!(
            decide_strategy(
                ContextRetentionCost::LongRunningProject,
                SecurityIsolationNeed::WorkspaceWrite
            ),
            ExecutionStrategy::OrchestratorDag
        ));

        // 极高上下文 + 只读 → 并行专家
        assert!(matches!(
            decide_strategy(
                ContextRetentionCost::PersistentJourney,
                SecurityIsolationNeed::ReadOnlyThought
            ),
            ExecutionStrategy::ParallelExperts { .. }
        ));

        // 简单问答 → 本地处理
        assert!(matches!(
            decide_strategy(
                ContextRetentionCost::Ephemeral,
                SecurityIsolationNeed::ReadOnlyThought
            ),
            ExecutionStrategy::HandleLocally
        ));
    }
}
