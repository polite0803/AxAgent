// SPDX-License-Identifier: AGPL-3.0-only

mod agent_executor;
pub mod aggregator_executor;
pub mod approval_executor;
mod code_executor;
mod condition_executor;
pub mod data_transformer_executor;
pub mod database_query_executor;
mod debate_executor;
mod delay_executor;
mod document_parser_executor;
pub mod email_executor;
mod end_executor;
mod fallback_executor;
pub mod file_operation_executor;
pub mod http_request_executor;
pub mod llm_classifier_executor;
mod llm_executor;
pub mod logging_executor;
mod loop_executor;
mod merge_executor;
pub mod notification_executor;
mod parallel_executor;
mod storage_executor;
mod subworkflow_executor;
mod swarm_executor;
pub mod switch_executor;
mod tool_executor;
mod trigger_executor;
mod validation_executor;
mod vector_retrieve_executor;
pub mod webhook_send_executor;
pub use aggregator_executor::AggregatorExecutor;
pub use approval_executor::ApprovalExecutor;
pub use data_transformer_executor::DataTransformerExecutor;
pub use database_query_executor::DatabaseQueryExecutor;
pub use email_executor::EmailExecutor;
pub use file_operation_executor::FileOperationExecutor;
pub use http_request_executor::HttpRequestExecutor;
pub use llm_classifier_executor::LlmClassifierExecutor;
pub use logging_executor::LoggingExecutor;
pub use notification_executor::NotificationExecutor;
pub use storage_executor::StorageExecutor;
pub use switch_executor::SwitchExecutor;
pub use webhook_send_executor::WebhookSendExecutor;

pub use agent_executor::{
    AgentExecutor, PlanApprovalCallback, PlanApprovalRequest, PlanCallbacks, PlanPhaseSummary,
    PlanStepCallback, PlanStepEvent, RagCallback,
};
pub(crate) use agent_executor::{ProfileCache, ProviderCache};
pub use code_executor::CodeExecutor;
pub use condition_executor::ConditionExecutor;
pub use debate_executor::DebateExecutor;
pub use delay_executor::DelayExecutor;
pub use document_parser_executor::DocumentParserExecutor;
pub use end_executor::EndExecutor;
pub use fallback_executor::FallbackExecutor;
pub use llm_executor::LlmExecutor;
pub use loop_executor::LoopExecutor;
pub use merge_executor::MergeExecutor;
pub use parallel_executor::ParallelExecutor;
pub use subworkflow_executor::{SubWorkflowCallback, SubWorkflowExecutor};
pub use swarm_executor::SwarmExecutor;
pub use tool_executor::{ToolCallback, ToolExecutor};
pub use trigger_executor::TriggerExecutor;
pub use validation_executor::ValidationExecutor;
pub use vector_retrieve_executor::VectorRetrieveExecutor;

/// 获取节点类型名称（从 node_executor_trait 导入，供执行器使用）。
pub use crate::work_engine::node_executor_trait::node_type_name;

// ── Workflow 上下文变量名常量 ──
// 这些 key 用于在 ExecutionState.variables 与 input_params 之间传递
// LLM 选择/Provider 解析等元信息。集中定义避免散落字符串。
pub const WORKFLOW_MODEL_VAR: &str = "__workflow_model__";
pub const WORKFLOW_PROVIDER_ID_VAR: &str = "__workflow_provider_id__";

// ── 公共 LLM 解析助手 ──
// 4 个 executor（agent/condition/llm/llm_classifier）都重复
// `resolve_model_for_node → decrypt_key → registry.get(registry_key)` 三步。
// 抽成公共 helper 消除 4 处字节级同义代码。
pub(crate) mod llm_resolve;
pub(crate) use llm_resolve::resolve_provider_and_adapter;

// ── 共享变量路径解析器 ──
// 多个 executor 需要从 ExecutionState.variables 中按点号路径（如 "node_id.output.field"）
// 取出嵌套值。每个 executor 此前都有重复的私有实现，这里统一提供。
// 注意：与 prompt_template::resolve_dot_path 不同——后者在模板渲染阶段对
// `{{path}}` 占位符做类型转换，本函数返回 `Option<Value>` 供代码逻辑使用。

/// 从 variables HashMap 中按点号分隔路径解析嵌套值。
///
/// 首段为节点 ID（variables 顶层 key），后续段递归进入 JSON 嵌套值。
/// 若顶层 key 不存在，将整个 path 作为 plain key 直查（原始 fallback 行为）。
pub(crate) fn resolve_var_path(
    path: &str,
    variables: &std::collections::HashMap<String, serde_json::Value>,
) -> Option<serde_json::Value> {
    if path.is_empty() {
        return None;
    }
    let parts: Vec<&str> = path.split('.').collect();
    if let Some(root) = variables.get(parts[0]) {
        let mut current = root.clone();
        for part in &parts[1..] {
            current = current.get(part)?.clone();
        }
        return Some(current);
    }
    variables.get(path).cloned()
}

// ── 共享字符串相似度 ──
// swarm_executor 的收敛检测依赖相似度比较；放在 executors 模块根以便复用。
// 采用 bigram Jaccard 系数：实现简单、零依赖、对短文本敏感，适合 swarm 收敛判断。

/// 计算两个字符串的 bigram Jaccard 相似度，返回 [0.0, 1.0]。
///
/// - 空串对空串返回 1.0（视为完全相同）
/// - 任一为空返回 0.0
/// - 否则 |A∩B| / |A∪B|，A/B 为相邻字符对集合
pub(crate) fn simple_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_bigrams: std::collections::HashSet<&str> =
        a.as_bytes().windows(2).filter_map(|w| std::str::from_utf8(w).ok()).collect();
    let b_bigrams: std::collections::HashSet<&str> =
        b.as_bytes().windows(2).filter_map(|w| std::str::from_utf8(w).ok()).collect();
    let inter = a_bigrams.intersection(&b_bigrams).count();
    let union = a_bigrams.union(&b_bigrams).count();
    if union == 0 {
        return 0.0;
    }
    inter as f64 / union as f64
}

// ── 共享：Swarm/Debate 容器多轮协作收敛检测与共识构建 ──
// 两者语义一致（多 Agent × 多轮 + 相邻轮次相似度收敛），
// 提取为 pub(crate) 自由函数避免重复实现。

/// 简单收敛检测：基于相邻轮次输出内容相似度。
///
/// 对每个 step_id 比较当前轮与上一轮输出：
/// - 完全相等计 matching
/// - 不等但长度差 <10% 且 bigram Jaccard 相似度 >0.85 也计 matching
/// - matching/total >= 0.80 视为收敛
pub(crate) fn check_round_convergence(
    current: &std::collections::HashMap<String, serde_json::Value>,
    previous: &std::collections::HashMap<String, serde_json::Value>,
) -> bool {
    let mut matching = 0u32;
    let mut total = 0u32;
    for (key, cur_val) in current {
        if let Some(prev_val) = previous.get(key) {
            total += 1;
            if cur_val == prev_val {
                matching += 1;
            } else {
                let cur_str = serde_json::to_string(cur_val).unwrap_or_default();
                let prev_str = serde_json::to_string(prev_val).unwrap_or_default();
                if !cur_str.is_empty()
                    && !prev_str.is_empty()
                    && (cur_str.len().abs_diff(prev_str.len()) as f64
                        / prev_str.len().max(1) as f64)
                        < 0.10
                    && simple_similarity(&cur_str, &prev_str) > 0.85
                {
                    matching += 1;
                    total += 1;
                }
            }
        }
    }
    if total == 0 {
        return false;
    }
    matching as f64 / total as f64 >= 0.80
}

/// 从各轮输出中构建共识结果（取最后一轮各 step 输出作为 entries）。
pub(crate) fn build_round_consensus(
    round_outputs: &[std::collections::HashMap<String, serde_json::Value>],
) -> serde_json::Value {
    if let Some(last_round) = round_outputs.last() {
        let entries: Vec<serde_json::Value> = last_round
            .iter()
            .map(|(step_id, output)| {
                serde_json::json!({
                    "agent": step_id,
                    "output": output,
                })
            })
            .collect();
        serde_json::json!({
            "entries": entries,
            "total_rounds": round_outputs.len(),
        })
    } else {
        serde_json::json!({
            "entries": [],
            "total_rounds": 0,
        })
    }
}
