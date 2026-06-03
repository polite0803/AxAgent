use crate::AppState;
use axagent_core::crypto::decrypt_key;
use axagent_core::entity::provider_keys;
use axagent_core::repo::provider;
use axagent_core::workflow_types::WorkflowNode;
use axagent_rt_workflow::work_engine::node_executor_trait::node_type_name;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmDiagnoseRequest {
    pub nodes: Vec<WorkflowNode>,
    pub workflow_name: String,
    pub workflow_description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmDiagnoseResult {
    pub summary: String,
    pub issues: Vec<LlmDiagnosticIssue>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LlmDiagnosticIssue {
    pub severity: String, // "error" | "warning" | "info"
    pub category: String,
    pub node_id: Option<String>,
    pub title: String,
    pub detail: String,
    pub suggestion: String,
}

/// LLM 增强诊断：分析 prompt 质量、语义冲突、最佳实践
#[tauri::command]
pub async fn llm_diagnose_workflow(
    state: State<'_, AppState>,
    request: LlmDiagnoseRequest,
) -> Result<LlmDiagnoseResult, String> {
    // 构造诊断 prompt
    let mut node_summaries = Vec::new();
    for node in &request.nodes {
        let summary = match node {
            WorkflowNode::Agent(n) => format!(
                "- Agent '{}': system_prompt='{}' ({} chars), tools={:?}
",
                n.base.id,
                n.config.system_prompt.chars().take(100).collect::<String>(),
                n.config.system_prompt.len(),
                n.config.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
            ),
            WorkflowNode::Llm(n) => format!(
                "- LLM '{}': prompt='{}' ({} chars), model={:?}
",
                n.base.id,
                n.config.prompt.chars().take(100).collect::<String>(),
                n.config.prompt.len(),
                n.config.model
            ),
            WorkflowNode::Condition(n) => format!(
                "- Condition '{}': {} conditions, logical_op={:?}, judge_by_llm={:?}
",
                n.base.id,
                n.config.conditions.len(),
                n.config.logical_op,
                n.config.judge_by_llm
            ),
            WorkflowNode::HttpRequest(n) => format!(
                "- HttpRequest '{}': {} {} (timeout={}s, retry={:?})
",
                n.base.id, n.config.method, n.config.url, n.config.timeout_secs, n.base.retry
            ),
            _ => format!(
                "- {} '{}'
",
                node_type_name(node),
                node.base_id()
            ),
        };
        node_summaries.push(summary);
    }

    let prompt = format!(
        "You are a workflow diagnostic expert. Analyze this workflow and identify issues across 5 dimensions.

Workflow name: {name}
Description: {desc}

Nodes:
{nodes}

Please analyze and provide a JSON response with:
1. summary: Overall workflow health summary (1-2 sentences)
2. issues: Array of issues found, each with:
   - severity: error | warning | info
   - category: prompt_quality | performance | cost | security | best_practice
   - node_id: Node ID if applicable, or null
   - title: Short issue title
   - detail: Detailed description
   - suggestion: How to fix it
3. suggestions: Array of general improvement suggestions (3-5 items)

Diagnostic dimensions:
1. prompt_quality — agent/llm system_prompt 是否清晰、有角色定义、输出约束、错误处理
2. performance — 长链同步串行、缺少 parallel 加速、循环无 max_iterations、httpRequest/dbQuery 无超时、documentParser 缺 parser_type
3. cost — LLM/Agent 节点无 max_tokens、max_tool_rounds 未设置、温度未调优（默认 0.7 适合生成但不适合分类）、vectorRetrieve top_k 过大
4. security — httpRequest/webhookSend/notification URL 走 http 非 https、approval 无 approver、email 凭据硬编码、approval timeout 过长、vectorRetrieve 缺 similarity_threshold、SubWorkflow 无沙箱提示
5. best_practice — 变量命名、错误处理、链路完整性（有无 start/end）、condition 后是否双出口、loop 是否有终止条件、validation 覆盖关键节点
6. structure — debate 容器是否有至少 2 个辩手子节点、debater_steps 是否引用了存在的子节点、容器子节点是否正确设置 parentId

Respond ONLY with valid JSON.",
        name = request.workflow_name,
        desc = request.workflow_description.as_deref().unwrap_or(""),
        nodes = node_summaries.join(""),
    );

    // 查找默认 provider 调用 LLM
    let db = &state.sea_db;
    let providers = provider::list_providers(db)
        .await
        .map_err(|e| e.to_string())?;
    let default_prov = providers
        .iter()
        .find(|p| p.enabled)
        .ok_or("No enabled provider found")?;
    let key = provider_keys::Entity::find()
        .filter(provider_keys::Column::ProviderId.eq(&default_prov.id))
        .filter(provider_keys::Column::Enabled.eq(1))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("No enabled API key found")?;
    let api_key =
        decrypt_key(&key.key_encrypted, state.harness.master_key()).map_err(|e| e.to_string())?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "model": "deepseek-chat",
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0.1,
        "max_tokens": 2000,
        "response_format": { "type": "json_object" }
    });

    let resp = client
        .post(format!("{}/chat/completions", default_prov.api_host.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LLM call failed: {e}"))?;

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {e}"))?;
    let content = result["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("{}");
    let parsed: LlmDiagnoseResult =
        serde_json::from_str(content).map_err(|e| format!("LLM response parse failed: {e}"))?;

    Ok(parsed)
}
