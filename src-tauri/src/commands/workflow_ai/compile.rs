// SPDX-License-Identifier: AGPL-3.0-only

//! 设计时 mission 编译命令：`compile_mission_to_template`
//!
//! 与 `generate_workflow_from_prompt` 的关键区别：
//! - **设计时编译**：在设计阶段把 mission 文本编译成 workflow_template 落库，
//!   运行时直接使用已编译的 template，不调 LLM，保证运行时稳定性。
//! - **去重缓存**：基于 mission 文本的 SHA-256 哈希查重，命中则直接返回已有
//!   template_id，避免重复调用 LLM。
//! - **不可变产物**：生成的 template 标记 `mission_hash` 字段，与手动创建的
//!   template（mission_hash=None）区分。
//!
//! 详见 AGENTS.md「运行时边界」铁律：运行时不调 LLM 拆任务，所有工作流
//! 必须基于已编译的 template 执行。

use super::helpers::{
    FEW_SHOT_EXAMPLES, NODE_SCHEMAS_DOC, WorkflowGenerationResult, build_roles_and_experts_brief,
    parse_llm_response, resolve_ai_provider,
};
use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::provider as provider_err;
use agent_macro::agent_command;
use axagent_dao::repo::workflow_template as db_repo;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_harness::workflow_types::WorkflowTemplateData;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::State;

/// mission 编译结果
#[derive(Debug, Serialize)]
pub struct CompileMissionResult {
    /// 命中缓存时为已有模板 ID；新生成时为刚落库的模板 ID
    pub template_id: String,
    /// true=命中缓存（未调 LLM）；false=新生成并落库
    pub is_cached: bool,
    /// LLM 生成时的 explanation（缓存命中时为 None）
    pub explanation: Option<String>,
}

/// 对 mission 文本标准化后计算 SHA-256 哈希。
///
/// 标准化规则：去除首尾空白 + 把连续空白（含 \r\n\t）压缩为单个空格。
/// 这样「同一 mission 文本的不同缩进/换行」也能命中缓存。
fn mission_hash(mission: &str) -> String {
    // split_whitespace 已自动跳过首尾和中间的空白，无需额外 trim
    let normalized: String = mission.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    hex::encode(hasher.finalize())
}

/// 把 mission 文本截断为模板名称（最多 50 字符）。
fn mission_to_name(mission: &str) -> String {
    let trimmed = mission.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 50 {
        trimmed.to_string()
    } else {
        let head: String = chars.iter().take(47).collect();
        format!("{}...", head)
    }
}

/// 构造 mission 编译专用 system prompt。
///
/// 相比 `generate_workflow_from_prompt`，本 prompt 强调：
/// - 生成「可重复执行」的稳定 template（不允许 intent=clarify）
/// - 必须输出完整 nodes/edges（不允许留空）
async fn build_system_prompt() -> String {
    let roles_brief = build_roles_and_experts_brief()
        .await
        .unwrap_or_else(|| "（暂无可用业务岗位/专家）".to_string());

    format!(
        r#"You are a workflow compilation assistant. Compile the user's mission statement into a stable, repeatable workflow template.

=== 任务边界（与 generate_workflow_from_prompt 不同）===
- 这是「设计时编译」，必须输出完整的 nodes/edges，不允许 intent=clarify 或 intent=refuse。
- 若 mission 描述有歧义，按最合理的方式解读并生成，在 explanation 中说明你的解读。
- 生成的 template 将被反复执行，必须保证稳定可重复。

=== 完整节点类型（共 27 种，必须从下列中选）===
trigger, agent, llm, condition, switch, parallel, loop, merge, delay,
httpRequest, databaseQuery, tool, code, subWorkflow, documentParser,
vectorRetrieve, validation, notification, approval, fileOperation,
dataTransformer, webhookSend, logging, llmClassifier, aggregator, email, end

{NODE_SCHEMAS_DOC}

=== Few-shot 范例 ===
{FEW_SHOT_EXAMPLES}

=== 可用业务岗位与专家清单 ===
{roles_brief}
提示：agent 节点的 config 中可引用上述业务岗位 ID（business_role_id）和专家 ID（expert_id），
让节点执行时自动拼接对应的 system_prompt（详见 4 层 prompt 层级）。

=== 输出格式 ===
{{
  "intent": "generate",
  "nodes": [
    {{
      "id": "n1",
      "node_type": "见上方完整列表",
      "title": "中文/英文标题",
      "description": "可选，节点作用",
      "config": {{ ...严格遵循上面对应 node_type 的 schema... }}
    }}
  ],
  "edges": [
    {{
      "id": "e1",
      "source": "n1",
      "target": "n2",
      "edge_type": "direct" | "conditionTrue" | "conditionFalse" | "loopBack" | "parallelBranch" | "merge" | "error",
      "label": "可选，parallelBranch 时填 'branch-N'"
    }}
  ],
  "explanation": "一段中文解释：为什么这样设计、关键节点的作用、潜在风险"
}}

=== 强制规则 ===
1. 总是以 trigger 节点开始、end 节点结束。
2. 节点 ID 用 n1, n2, n3... 这种简短形式，edges 中 source/target 引用必须一致。
3. 每个 config 字段必须遵循上方对应 node_type 的 schema —— 必填字段不能省略、可选字段可省略。
4. condition 节点配 conditionTrue/conditionFalse 边；parallel 节点的每条分支用 parallelBranch 边，label 形如 "branch-0"、"branch-1"。
5. 若用户描述里有"审批"→ approval；"邮件"→ email；"HTTP/接口/REST"→ httpRequest；"数据库/SQL"→ databaseQuery；"Webhook 回调"→ webhookSend；"分类/打标"→ llmClassifier；"汇总/合并"→ aggregator。
6. 涉及并发/批量处理用 parallel；循环遍历用 loop；不要把循环当并发。
7. 跨多个服务编排时优先用 subWorkflow 复用已有工作流。
8. 知识检索/文档问答用 vectorRetrieve + documentParser；不要用 llm 凭空生成。"#
    )
}

/// 设计时编译 mission 文本为 workflow_template。
///
/// 流程：
/// 1. 计算 mission 的 SHA-256 哈希
/// 2. 查 workflow_templates.mission_hash 命中缓存 → 直接返回 template_id
/// 3. 未命中 → 调 LLM 生成 nodes/edges → 落库（填充 mission_hash）→ 返回 template_id
///
/// 运行时执行工作流时不再调用此命令，直接使用已落库的 template。
#[agent_command(domain = workflow, safety = Caution, call_mode = StateInput, description = "编译mission为工作流模板")]
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn compile_mission_to_template(
    state: State<'_, AppState>,
    mission: String,
) -> Result<CompileMissionResult, String> {
    if mission.trim().is_empty() {
        return Err("Mission text must not be empty".to_string());
    }

    let db = state.harness.db();
    let hash = mission_hash(&mission);

    // 1. 命中缓存直接返回
    if let Some(existing) = db_repo::find_latest_by_mission_hash(db, &hash).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })? {
        tracing::info!(
            "[compile_mission] mission_hash={} 命中缓存 template_id={}",
            hash,
            existing.id
        );
        return Ok(CompileMissionResult {
            template_id: existing.id,
            is_cached: true,
            explanation: None,
        });
    }

    // 2. 未命中 → 调 LLM 生成
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&resolved.provider_type);
    let adapter = state.harness.provider_registry().get(registry_key).ok_or_else(|| {
        ErrorResponse::err_with_detail(
            provider_err::ADAPTER_NOT_FOUND,
            format!("Provider adapter not found for type: {}", registry_key),
        )
    })?;

    let system_prompt = build_system_prompt().await;
    let request = ChatRequest {
        model: resolved.model_id.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(mission.clone()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        temperature: Some(0.4),
        top_p: None,
        max_tokens: Some(4096),
        stream: false,
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
        response_format: None,
    };

    let response = adapter
        .chat(&resolved.ctx, request.into())
        .await
        .map_err(|e| format!("LLM API error: {}", e))?;

    let generated: WorkflowGenerationResult =
        parse_llm_response(&mission, &response.content, &resolved.model_id)?;

    // 3. 校验 LLM 输出非空（mission 编译不允许 clarify/refuse）
    if generated.nodes.is_empty() {
        return Err(format!(
            "LLM returned empty nodes for mission (explanation: {})",
            generated.explanation.unwrap_or_else(|| "none".to_string())
        ));
    }

    // 4. 落库为新的 workflow_template（填充 mission_hash）
    let now = chrono::Utc::now().timestamp_millis();
    let template_id = uuid::Uuid::new_v4().to_string();
    let template = WorkflowTemplateData {
        id: template_id.clone(),
        name: mission_to_name(&mission),
        description: Some(mission.clone()),
        icon: "mission".to_string(),
        tags: vec!["mission-compiled".to_string()],
        version: 1,
        is_preset: false,
        is_editable: true,
        is_public: false,
        trigger_config: None,
        nodes: generated.nodes,
        edges: generated.edges,
        input_schema: None,
        output_schema: None,
        variables: vec![],
        error_config: None,
        tool_defs: vec![],
        error_workflow_id: None,
        mission_hash: Some(hash.clone()),
        created_at: now,
        updated_at: now,
    };

    let active_model = db_repo::build_active_model_from_data(&template);
    db_repo::insert_workflow_template(db, active_model).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 5. 预编译 Rhai 工具（虽然 mission 编译目前不带 tool_defs，保留调用以兼容未来扩展）
    state.work_engine.precompile_tool_defs(&template_id, &[]).await;

    tracing::info!("[compile_mission] mission_hash={} 新编译 template_id={}", hash, template_id);

    Ok(CompileMissionResult { template_id, is_cached: false, explanation: generated.explanation })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_hash_normalizes_whitespace() {
        let h1 = mission_hash("hello world");
        let h2 = mission_hash("  hello   world  ");
        let h3 = mission_hash("hello\tworld\n");
        assert_eq!(h1, h2);
        assert_eq!(h1, h3);
        assert_eq!(h1.len(), 64); // SHA-256 hex 长度
    }

    #[test]
    fn mission_hash_is_case_sensitive() {
        // 大小写敏感（mission 是业务文本，大小写差异可能意味着不同任务）
        let h1 = mission_hash("Analyze stock");
        let h2 = mission_hash("analyze stock");
        assert_ne!(h1, h2);
    }

    #[test]
    fn mission_to_name_truncates_long_text() {
        let long = "a".repeat(100);
        let name = mission_to_name(&long);
        assert!(name.len() <= 50);
        assert!(name.ends_with("..."));
    }

    #[test]
    fn mission_to_name_preserves_short_text() {
        let short = "分析股票市场";
        let name = mission_to_name(short);
        assert_eq!(name, short);
    }

    #[tokio::test]
    async fn build_system_prompt_contains_key_sections() {
        axagent_harness::test_support::register_noop_role_and_expert_repos();
        let prompt = build_system_prompt().await;
        assert!(prompt.contains("设计时编译"));
        assert!(prompt.contains(NODE_SCHEMAS_DOC));
        assert!(prompt.contains(FEW_SHOT_EXAMPLES));
        assert!(prompt.contains("可用业务岗位与专家清单"));
        // 确保不包含 context_section 占位符（mission 编译无上下文）
        assert!(!prompt.contains("{context_section}"));
    }
}
