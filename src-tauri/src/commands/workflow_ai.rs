// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::provider as provider_err;
use axagent_core::crypto::decrypt_key;
use axagent_core::workflow_types::*;
use axagent_harness::types::{
    ChatContent, ChatMessage, ChatRequest, ChatStreamChunk, ChatStreamErrorEvent, ChatStreamEvent,
    ProviderType,
};
use axagent_harness::{ProviderRequestContext, url_utils::resolve_base_url_for_type};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::Emitter;
use tauri::State;
use tokio::sync::Mutex;

fn get_cancel_store() -> &'static Mutex<HashMap<String, Arc<std::sync::atomic::AtomicBool>>> {
    static STORE: OnceLock<Mutex<HashMap<String, Arc<std::sync::atomic::AtomicBool>>>> =
        OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowGenerationResult {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub explanation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeRecommendation {
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiChatMessage {
    pub role: String,
    pub content: String,
}

struct ResolvedProvider {
    ctx: ProviderRequestContext,
    model_id: String,
    provider_type: ProviderType,
}

async fn resolve_ai_provider(state: &AppState) -> Result<ResolvedProvider, String> {
    let providers = axagent_core::repo::provider::list_providers(state.harness.db())
        .await
        .map_err(|e| format!("Failed to list providers: {}", e))?;

    let provider = providers.iter().find(|p| p.enabled).ok_or_else(|| {
        "No enabled provider found. Please configure a provider in settings.".to_string()
    })?;

    let provider_key =
        axagent_core::repo::provider::get_active_key(state.harness.db(), &provider.id)
            .await
            .map_err(|e| format!("Failed to get provider key: {}", e))?;

    let decrypted_key = decrypt_key(&provider_key.key_encrypted, state.harness.master_key())
        .map_err(|e| format!("Failed to decrypt API key: {}", e))?;

    let base_url = resolve_base_url_for_type(&provider.api_host, &provider.provider_type);

    let ctx = ProviderRequestContext {
        api_key: decrypted_key,
        key_id: provider_key.id,
        provider_id: provider.id.clone(),
        base_url: Some(base_url),
        api_path: provider.api_path.clone(),
        proxy_config: provider.proxy_config.clone(),
        custom_headers: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let model_id = provider
        .models
        .iter()
        .find(|m| m.enabled)
        .map(|m| m.model_id.clone())
        .unwrap_or_else(|| "gpt-4".to_string());

    Ok(ResolvedProvider {
        ctx,
        model_id,
        provider_type: provider.provider_type.clone(),
    })
}

/// 27 种节点类型的完整 JSON Schema 文档。
/// LLM 生成工作流时，每个 `config` 对象必须遵循对应 schema 字段。
const NODE_SCHEMAS_DOC: &str = r#"
=== 节点 config 完整 Schema（每个 node_type 对应一组必填/可选字段）===

trigger
  config: {
    "trigger_type": "manual" | "schedule" | "webhook" | "event",
    "config": { ...trigger-type-specific 内层配置 }
  }
  - schedule: { "cron": "* * * * *", "timezone": "Asia/Shanghai", "enabled": true }
  - webhook:  { "path": "/webhook/...", "method": "POST", "auth_type": "none" }
  - event:    { "event_type": "...", "filter": { ... } }

agent
  config: {
    "system_prompt": "string (必填，Agent 角色/指令)",
    "model": "string | null",
    "temperature": 0.0~2.0 | null,
    "max_tokens": int | null,
    "output_mode": "json" | "text" | "artifact",
    "output_var": "string (Agent 产出变量名)",
    "tools": [{ "name": "string", "description": "string", "parameters": {...JSON Schema} }],
    "exposed_tools": ["string"],
    "context_sources": ["string"],
    "agent_profile_id": "string | null",
    "max_tool_rounds": int | null,
    "execution_mode": "react" | "plan" | null,
    "rag_source_ids": ["knowledge:<id>", "memory:<id>", "wiki:<id>"],
    "model_role": "quick_think | deep_think | null (映射到全局模型配置, 优先级低于 model 字段)"
  }

llm
  config: {
    "model": "string (必填)",
    "prompt": "string (必填)",
    "messages": [{ "role": "user|system|assistant", "content": "..." }] | null,
    "temperature": 0.0~2.0 | null,
    "max_tokens": int | null,
    "tools": ["string"] | null,
    "functions": [...] | null
  }

condition
  config: {
    "conditions": [
      { "var_path": "string (变量路径)", "operator": "eq|ne|gt|lt|gte|lte|contains|notContains|startsWith|endsWith|regexMatch|isEmpty|isNotEmpty", "value": <any> }
    ],
    "logical_op": "and" | "or",
    "judge_by_llm": bool | null,
    "routing_prompt": "string | null (LLM 路由提示)",
    "routing_model": "string | null"
  }
  输出边: conditionTrue / conditionFalse

switch
  config: {
    "input_var": "string (输入变量名)",
    "cases": [{ "value": "string", "label": "string" }],
    "default_case": "string | null",
    "match_mode": "exact" | "contains" | "regex" | "range",
    "output_var": "string"
  }

parallel
  config: {
    "branches": [{ "id": "string", "title": "string", "steps": ["node_id", ...] }],
    "wait_for_all": true | false,
    "timeout": int (秒) | null,
    "aggregation": "all" | "any" | "race" | "majority" | null,
    "auto_input_from_parent": true | false
  }
  输出边: parallelBranch (sourceHandle 形如 "branch-0", "branch-1"...)

loop
  config: {
    "loop_type": "forEach" | "while" | "doWhile" | "until",
    "items_var": "string | null (被遍历的集合变量名)",
    "iteratee_var": "string | null (单元素变量名)",
    "max_iterations": int | null,
    "continue_condition": "string | null (条件表达式)",
    "continue_on_error": true | false,
    "body_steps": ["node_id", ...]
  }
  输出边: loopBack (回到 loop 自身)

merge
  config: {
    "merge_type": "all" | "any" | "race" | "majority",
    "inputs": ["node_id", ...],
    "auto_inputs_from_branches": true | false
  }

delay
  config: {
    "delay_type": "fixed" | "until",
    "seconds": int,
    "until": "ISO8601 timestamp | null"
  }

httpRequest
  config: {
    "url": "string (必填, 完整 URL)",
    "method": "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS",
    "headers": { "Header-Name": "value" },
    "body": "string | null (JSON / text / form)",
    "body_type": "json" | "form" | "text" | "xml" | "binary",
    "timeout_secs": int (默认 30),
    "output_var": "string"
  }

databaseQuery
  config: {
    "query": "string (SQL 语句，必填)",
    "params": ["string", ...],
    "connection_name": "string | null (数据源名称)",
    "timeout_secs": int (默认 30),
    "output_var": "string"
  }

tool
  config: {
    "tool_name": "string (必填, 已注册的工具名。可用工具由系统工具注册表提供，包括 file_read/file_write/shell/network/system/agent/vcs/automation/communication/ai_media/integration/storage/knowledge/browser/desktop 等分类下的工具。下游可扩展注册自定义工具)",
    "input_mapping": { "arg_name": "var_path_or_literal" },
    "output_var": "string"
  }

code
  config: {
    "language": "rhai" | "javascript" | "python",
    "code": "string (源码，必填。推荐使用 rhai — 轻量安全沙箱，支持 HTTP 调用、JSON 解析、字符串处理、数学计算等。javascript/python 需要对应运行时环境)",
    "output_var": "string",
    "tool_name": "string | null (rhai 模式下注册为工具名，供 agent/tools 节点引用)"
  }
  Rhai 示例 — HTTP GET:
    let resp = http_get("https://api.example.com/data");
    let data = parse_json(resp);
    data
  Rhai 示例 — 数据转换:
    let items = input.items;
    let result = items.filter(|i| i.score > 0.8);
    result

subWorkflow
  config: {
    "sub_workflow_id": "string (子工作流 ID，必填)",
    "input_mapping": { "input_name": "var_path" },
    "output_var": "string",
    "is_async": true | false
  }

documentParser
  config: {
    "input_var": "string (含文档内容的变量名)",
    "parser_type": "auto" | "pdf" | "docx" | "xlsx" | "image" | "audio" | "html",
    "output_var": "string"
  }

vectorRetrieve
  config: {
    "query": "string (查询语句，必填)",
    "knowledge_base_id": "string (知识库 ID，必填)",
    "top_k": int (默认 5),
    "similarity_threshold": float | null,
    "output_var": "string"
  }

validation
  config: {
    "assertions": [
      { "type": "equals" | "contains" | "matches" | "exists" | "custom", "expected": "string", "actual": "string", "expression": "string" }
    ],
    "on_fail": "stop" | "retry" | "continue",
    "max_retries": int
  }

notification
  config: {
    "channel": "email" | "sms" | "feishu" | "slack" | "webhook" | "inapp",
    "message": "string (必填)",
    "webhook_url": "string | null",
    "recipients": ["string", ...],
    "subject": "string | null",
    "enabled": true | false,
    "output_var": "string"
  }

approval
  config: {
    "message": "string (审批内容，必填)",
    "approver": "string | null (审批人 user/role id)",
    "timeout_secs": int (默认 86400 = 24h),
    "timeout_action": "auto_approve" | "auto_reject",
    "output_var": "string"
  }

fileOperation
  config: {
    "operation": "read" | "write" | "append" | "delete" | "rename" | "copy" | "mkdir" | "exists",
    "file_path": "string (必填)",
    "content": "string | null (write/append 时必填)",
    "output_var": "string"
  }

dataTransformer
  config: {
    "input_var": "string (输入变量名)",
    "expression": "string (JMESPath / JSONPath / JS 表达式)",
    "output_var": "string"
  }

webhookSend
  config: {
    "url": "string (必填)",
    "method": "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
    "body": "string | null",
    "headers": { "Header-Name": "value" },
    "output_var": "string"
  }

logging
  config: {
    "level": "debug" | "info" | "warn" | "error",
    "message": "string (必填，支持 {var} 占位符)",
    "output_var": "string"
  }

llmClassifier
  config: {
    "categories": ["string", ...],
    "prompt": "string (分类指令，必填)",
    "model": "string | null",
    "input_var": "string (被分类内容)",
    "output_var": "string (分类结果变量)"
  }

aggregator
  config: {
    "strategy": "all" | "concat" | "vote" | "first" | "last" | "merge",
    "input_sources": ["node_id", ...],
    "output_var": "string"
  }

email
  config: {
    "to": ["email@x", ...],
    "subject": "string (必填)",
    "body": "string (必填, 支持 HTML)",
    "smtp_host": "string | null (为空使用系统默认)",
    "smtp_port": int | null,
    "smtp_user": "string | null",
    "smtp_pass": "string | null",
    "output_var": "string"
  }

debate (容器节点 — 辩手为容器内子 Agent 节点)
  config: {
    "debater_steps": ["子节点ID1", "子节点2", ...] (辩手 Agent 节点 ID 列表，这些节点须设 parentId 指向本 debate 节点),
    "max_rounds": int (默认 2, 辩论轮数),
    "convergence_prompt": "string | null (收敛判断提示词, 为空则固定轮数)",
    "convergence_model": "string | null",
    "convergence_model_role": "quick_think | deep_think | null",
    "topic_var": "string (辩论主题变量名)",
    "output_var": "string"
  }
  子节点: 在 debate 容器内放置 agent 节点作为辩手，每个辩手通过 system_prompt 定义立场，通过 model_role 选择推理深度。
  输出边: debateRound (每轮辩论输出)

end
  config: { "output_var": "string | null" }
"#;

/// Few-shot 范例（覆盖典型业务场景，提升生成质量）。
const FEW_SHOT_EXAMPLES: &str = r#"
=== 标准工作流示例（供你参考结构与节点选型）===

【示例 1: RAG 问答】
{
  "nodes": [
    { "id": "n1", "node_type": "trigger",   "title": "Webhook 触发",      "config": { "trigger_type": "webhook", "config": { "path": "/qa", "method": "POST", "auth_type": "none" } } },
    { "id": "n2", "node_type": "vectorRetrieve", "title": "检索知识库",   "config": { "query": "${input.question}", "knowledge_base_id": "kb_main", "top_k": 5, "output_var": "ctx" } },
    { "id": "n3", "node_type": "llm",       "title": "LLM 合成答案",      "config": { "model": "gpt-5.4-mini", "prompt": "基于以下上下文回答用户问题...\n上下文:\n${ctx}\n问题:\n${input.question}", "temperature": 0.3, "max_tokens": 1024 } },
    { "id": "n4", "node_type": "validation","title": "校验答案非空",      "config": { "assertions": [{ "type": "isNotEmpty", "actual": "${n3.output}" }], "on_fail": "retry", "max_retries": 1 } },
    { "id": "n5", "node_type": "end",       "title": "返回结果",          "config": { "output_var": "n3" } }
  ],
  "edges": [
    { "id": "e1", "source": "n1", "target": "n2", "edge_type": "direct" },
    { "id": "e2", "source": "n2", "target": "n3", "edge_type": "direct" },
    { "id": "e3", "source": "n3", "target": "n4", "edge_type": "direct" },
    { "id": "e4", "source": "n4", "target": "n5", "edge_type": "direct" }
  ]
}

【示例 2: 多路路由审批】
{
  "nodes": [
    { "id": "n1", "node_type": "trigger",   "title": "定时触发",        "config": { "trigger_type": "schedule", "config": { "cron": "0 9 * * *", "timezone": "Asia/Shanghai", "enabled": true } } },
    { "id": "n2", "node_type": "condition", "title": "金额判定",        "config": { "conditions": [{ "var_path": "${input.amount}", "operator": "gte", "value": 10000 }], "logical_op": "and" } },
    { "id": "n3", "node_type": "approval",  "title": "财务审批",        "config": { "message": "申请金额 ${input.amount} 元，请审批", "approver": "role:finance", "timeout_secs": 86400, "timeout_action": "auto_reject" } },
    { "id": "n4", "node_type": "notification", "title": "通知申请人",  "config": { "channel": "email", "recipients": ["${input.applicant_email}"], "subject": "审批结果", "message": "您的申请已批准" } },
    { "id": "n5", "node_type": "end",       "title": "结束",            "config": {} }
  ],
  "edges": [
    { "id": "e1", "source": "n1", "target": "n2", "edge_type": "direct" },
    { "id": "e2", "source": "n2", "target": "n3", "edge_type": "conditionTrue" },
    { "id": "e3", "source": "n2", "target": "n4", "edge_type": "conditionFalse" },
    { "id": "e4", "source": "n3", "target": "n4", "edge_type": "direct" },
    { "id": "e5", "source": "n4", "target": "n5", "edge_type": "direct" }
  ]
}

【示例 3: 并行抓取 + 聚合】
{
  "nodes": [
    { "id": "n1", "node_type": "trigger",   "title": "手动触发",          "config": { "trigger_type": "manual", "config": {} } },
    { "id": "n2", "node_type": "parallel",  "title": "并行抓取三个数据源", "config": { "branches": [{ "id": "b0", "title": "GitHub", "steps": ["n3"] }, { "id": "b1", "title": "Twitter", "steps": ["n4"] }, { "id": "b2", "title": "RSS", "steps": ["n5"] }], "wait_for_all": true, "auto_input_from_parent": true } },
    { "id": "n3", "node_type": "httpRequest", "title": "GitHub API",   "config": { "url": "https://api.github.com/...", "method": "GET", "headers": {}, "timeout_secs": 15, "output_var": "gh" } },
    { "id": "n4", "node_type": "httpRequest", "title": "Twitter API",  "config": { "url": "https://api.twitter.com/...", "method": "GET", "headers": {}, "timeout_secs": 15, "output_var": "tw" } },
    { "id": "n5", "node_type": "httpRequest", "title": "RSS Feed",     "config": { "url": "https://example.com/feed.xml", "method": "GET", "headers": {}, "timeout_secs": 15, "output_var": "rss" } },
    { "id": "n6", "node_type": "aggregator","title": "合并结果",          "config": { "strategy": "concat", "input_sources": ["n3","n4","n5"], "output_var": "all" } },
    { "id": "n7", "node_type": "llm",       "title": "LLM 总结",          "config": { "model": "gpt-5.4-mini", "prompt": "总结以下多源信息:\n${all}", "max_tokens": 800 } },
    { "id": "n8", "node_type": "end",       "title": "结束",              "config": { "output_var": "n7" } }
  ],
  "edges": [
    { "id": "e1", "source": "n1", "target": "n2", "edge_type": "direct" },
    { "id": "e2", "source": "n2", "target": "n3", "edge_type": "parallelBranch", "label": "branch-0" },
    { "id": "e3", "source": "n2", "target": "n4", "edge_type": "parallelBranch", "label": "branch-1" },
    { "id": "e4", "source": "n2", "target": "n5", "edge_type": "parallelBranch", "label": "branch-2" },
    { "id": "e5", "source": "n3", "target": "n6", "edge_type": "direct" },
    { "id": "e6", "source": "n4", "target": "n6", "edge_type": "direct" },
    { "id": "e7", "source": "n5", "target": "n6", "edge_type": "direct" },
    { "id": "e8", "source": "n6", "target": "n7", "edge_type": "direct" },
    { "id": "e9", "source": "n7", "target": "n8", "edge_type": "direct" }
  ]
}

【示例 4: 多 Agent 辩论决策（debate 为容器节点，辩手为子 Agent 节点）】
{
  "nodes": [
    { "id": "n1", "node_type": "trigger",   "title": "手动触发",          "config": { "trigger_type": "manual", "config": {} } },
    { "id": "n2", "node_type": "llm",       "title": "生成辩题",          "config": { "model": "gpt-5.4-mini", "prompt": "根据用户输入生成一个需要多方论证的议题:\n${input.topic}", "temperature": 0.7, "max_tokens": 512, "output_var": "debate_topic" } },
    { "id": "n3", "node_type": "debate",    "title": "多角色对抗辩论",    "config": { "debater_steps": ["n3a","n3b","n3c"], "max_rounds": 3, "convergence_prompt": "综合以上辩论，给出最终结论和建议。", "convergence_model_role": "deep_think", "topic_var": "debate_topic", "output_var": "debate_result" } },
    { "id": "n3a", "node_type": "agent",    "title": "正方辩手",         "config": { "system_prompt": "你是正方辩手，请从支持的角度论证议题，提供有力论据。", "model_role": "deep_think", "tools": [], "output_var": "proponent_output" }, "parentId": "n3" },
    { "id": "n3b", "node_type": "agent",    "title": "反方辩手",         "config": { "system_prompt": "你是反方辩手，请从反对的角度论证议题，指出潜在风险。", "model_role": "deep_think", "tools": [], "output_var": "opponent_output" }, "parentId": "n3" },
    { "id": "n3c", "node_type": "agent",    "title": "主持人",           "config": { "system_prompt": "你是主持人，在每轮辩论后总结双方观点并指出共识与分歧。", "model_role": "quick_think", "tools": [], "output_var": "moderator_output" }, "parentId": "n3" },
    { "id": "n4", "node_type": "agent",     "title": "决策总结 Agent",    "config": { "system_prompt": "你是一个决策助手，根据辩论结果提炼可执行的行动方案。", "model_role": "deep_think", "tools": [], "output_var": "action_plan" } },
    { "id": "n5", "node_type": "end",       "title": "结束",              "config": { "output_var": "action_plan" } }
  ],
  "edges": [
    { "id": "e1", "source": "n1", "target": "n2", "edge_type": "direct" },
    { "id": "e2", "source": "n2", "target": "n3", "edge_type": "direct" },
    { "id": "e3", "source": "n3", "target": "n4", "edge_type": "direct" },
    { "id": "e4", "source": "n4", "target": "n5", "edge_type": "direct" }
  ]
}
"#;

fn extract_json_from_response(content: &str) -> Option<&str> {
    let trimmed = content.trim();
    if trimmed.contains("```json") {
        return trimmed
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .map(|s| s.trim());
    }
    if trimmed.contains("```") {
        if let Some(start_idx) = trimmed.find("```") {
            let after_first = &trimmed[start_idx + 3..];
            if let Some(end_idx) = after_first.find("```") {
                let extracted = after_first[..end_idx].trim();
                if extracted.starts_with('{') || extracted.starts_with('[') {
                    return Some(extracted);
                }
            }
        }
    }
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Some(trimmed);
    }
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            if end > start {
                return Some(trimmed[start..=end].trim());
            }
        }
    }
    if let Some(start) = trimmed.find('[') {
        if let Some(end) = trimmed.rfind(']') {
            if end > start {
                return Some(trimmed[start..=end].trim());
            }
        }
    }
    None
}

fn layout_workflow_nodes(
    node_ids: &[String],
    edge_pairs: &[(String, String)],
    parent_map: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, Position> {
    let mut positions = std::collections::HashMap::new();
    if node_ids.is_empty() {
        return positions;
    }

    let mut children: std::collections::HashMap<&str, Vec<&str>> = std::collections::HashMap::new();
    let mut has_parent: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for (src, tgt) in edge_pairs {
        children.entry(src.as_str()).or_default().push(tgt.as_str());
        has_parent.insert(tgt.as_str());
    }

    let roots: Vec<&str> = node_ids
        .iter()
        .map(|s| s.as_str())
        .filter(|id| !has_parent.contains(id) && !parent_map.contains_key(*id))
        .collect();

    let root = roots.first().copied().unwrap_or(node_ids[0].as_str());

    let mut depths: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back((root, 0usize));

    while let Some((nid, depth)) = queue.pop_front() {
        if depths.contains_key(nid) {
            continue;
        }
        depths.insert(nid, depth);
        if let Some(kids) = children.get(nid) {
            for kid in kids {
                if !depths.contains_key(kid) {
                    queue.push_back((kid, depth + 1));
                }
            }
        }
    }

    let mut depth_groups: std::collections::HashMap<usize, Vec<&str>> =
        std::collections::HashMap::new();
    for (id, &depth) in &depths {
        depth_groups.entry(depth).or_default().push(*id);
    }

    let max_depth = depths.values().copied().max().unwrap_or(0);

    for depth in 0..=max_depth {
        let nodes_at_depth = depth_groups
            .get(&depth)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let count = nodes_at_depth.len().max(1);
        let total_width = (count as f64) * 220.0;
        let center_x = 400.0;
        let start_x = center_x - total_width / 2.0 + 110.0;

        for (i, id) in nodes_at_depth.iter().enumerate() {
            positions.insert(
                (*id).to_string(),
                Position {
                    x: start_x + (i as f64) * 220.0,
                    y: 80.0 + (depth as f64) * 140.0,
                },
            );
        }
    }

    let mut idx = 0usize;
    for id in node_ids {
        if !positions.contains_key(id.as_str()) {
            positions.insert(
                id.clone(),
                Position {
                    x: 100.0 + (idx as f64) * 220.0,
                    y: 80.0 + ((max_depth + 1 + idx) as f64) * 140.0,
                },
            );
            idx += 1;
        }
    }

    let mut container_children: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (child_id, parent_id) in parent_map {
        container_children
            .entry(parent_id.clone())
            .or_default()
            .push(child_id.clone());
    }

    for (parent_id, child_ids) in &container_children {
        let count = child_ids.len().max(1);
        let start_x = 50.0;
        let start_y = 60.0;
        for (i, child_id) in child_ids.iter().enumerate() {
            positions.insert(
                child_id.clone(),
                Position {
                    x: start_x,
                    y: start_y + (i as f64) * 80.0,
                },
            );
        }
        if let Some(parent_pos) = positions.get_mut(parent_id) {
            let needed_height = (count as f64) * 80.0 + 80.0;
            if parent_pos.y < needed_height {}
        }
    }

    positions
}

fn parse_llm_response(
    prompt: &str,
    response_content: &str,
    model_id: &str,
) -> Result<WorkflowGenerationResult, String> {
    let json_str = extract_json_from_response(response_content).ok_or_else(|| {
        format!(
            "Failed to parse LLM response as JSON: {}",
            &response_content[..response_content.len().min(200)]
        )
    })?;

    #[derive(Deserialize)]
    struct LlmWorkflowResponse {
        #[serde(default)]
        intent: Option<String>,
        nodes: Vec<LlmNode>,
        edges: Vec<LlmEdge>,
        explanation: Option<String>,
    }

    #[derive(Deserialize)]
    struct LlmNode {
        id: String,
        node_type: String,
        title: String,
        description: Option<String>,
        config: serde_json::Value,
        #[serde(default)]
        parent_id: Option<String>,
    }

    #[derive(Deserialize)]
    struct LlmEdge {
        id: String,
        source: String,
        target: String,
        edge_type: Option<String>,
    }

    let parsed: LlmWorkflowResponse = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse workflow JSON: {}", e))?;

    if let Some(intent) = parsed.intent.as_deref() {
        match intent {
            "refuse" => {
                return Ok(WorkflowGenerationResult {
                    nodes: vec![],
                    edges: vec![],
                    explanation: Some(format!(
                        "[AI 拒绝生成] {}",
                        parsed
                            .explanation
                            .unwrap_or_else(|| "请求被拒绝".to_string())
                    )),
                });
            },
            "clarify" => {
                return Ok(WorkflowGenerationResult {
                    nodes: vec![],
                    edges: vec![],
                    explanation: Some(format!(
                        "[AI 请求澄清] {}",
                        parsed
                            .explanation
                            .unwrap_or_else(|| "请补充更详细的需求".to_string())
                    )),
                });
            },
            _ => {},
        }
    }

    let node_ids: Vec<String> = parsed
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if n.id.is_empty() {
                format!("{}-{}", n.node_type, i + 1)
            } else {
                n.id.clone()
            }
        })
        .collect();

    let edge_pairs: Vec<(String, String)> = parsed
        .edges
        .iter()
        .map(|e| (e.source.clone(), e.target.clone()))
        .collect();

    let mut parent_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for llm_node in &parsed.nodes {
        if let Some(ref pid) = llm_node.parent_id {
            parent_map.insert(llm_node.id.clone(), pid.clone());
        }
    }

    let positions = layout_workflow_nodes(&node_ids, &edge_pairs, &parent_map);

    let mut nodes = Vec::new();
    let mut id_to_node_id = std::collections::HashMap::new();

    for (i, llm_node) in parsed.nodes.iter().enumerate() {
        let node_id = node_ids[i].clone();
        id_to_node_id.insert(llm_node.id.clone(), node_id.clone());

        let position = positions.get(&node_id).cloned().unwrap_or(Position {
            x: 100.0 + (i as f64) * 200.0,
            y: 80.0 + (i as f64) * 140.0,
        });

        let base = WorkflowNodeBase {
            compensation: None,
            id: node_id.clone(),
            title: llm_node.title.clone(),
            description: llm_node.description.clone(),
            position,
            enabled: true,
            parent_id: llm_node.parent_id.clone(),
            retry: RetryConfig::default(),
            timeout: None,
        };

        let node = match llm_node.node_type.as_str() {
            "trigger" => WorkflowNode::Trigger(TriggerNode {
                base,
                config: TriggerConfig {
                    trigger_type: TriggerType::Manual,
                    config: llm_node.config.clone(),
                },
            }),
            "agent" => {
                let agent_config: AgentNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(AgentNodeConfig {
                        system_prompt: format!(
                            "You are an AI assistant. {}",
                            llm_node.description.clone().unwrap_or_default()
                        ),
                        model: Some(model_id.to_string()),
                        temperature: Some(0.7),
                        max_tokens: Some(2048),
                        output_mode: OutputMode::Text,
                        output_var: "result".to_string(),
                        tools: vec![],
                        exposed_tools: vec![],
                        context_sources: vec![],
                        agent_profile_id: None,
                        max_tool_rounds: None,
                        execution_mode: None,
                        rag_source_ids: vec![],
                        model_role: None,
                        consistency_check: None,
                        hallucination_guard: None,
                        input_mapping: std::collections::HashMap::new(),
                    });
                WorkflowNode::Agent(AgentNode {
                    base,
                    config: agent_config,
                })
            },
            "llm" => {
                let llm_config: LLMNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(LLMNodeConfig {
                        model: model_id.to_string(),
                        prompt: llm_node.description.clone().unwrap_or_default(),
                        temperature: Some(0.7),
                        max_tokens: Some(2048),
                        tools: None,
                        functions: None,
                        messages: None,
                        consistency_check: None,
                        max_context_tokens: None,
                        reserved_output_tokens: None,
                    });
                WorkflowNode::Llm(LLMNode {
                    base,
                    config: llm_config,
                })
            },
            "condition" => {
                let cond_config: ConditionNodeConfig = serde_json::from_value(
                    llm_node.config.clone(),
                )
                .unwrap_or(ConditionNodeConfig {
                    conditions: vec![],
                    logical_op: LogicalOperator::And,
                    judge_by_llm: None,
                    routing_prompt: None,
                    routing_model: None,
                    confidence_threshold: None,
                });
                WorkflowNode::Condition(ConditionNode {
                    base,
                    config: cond_config,
                })
            },
            "parallel" => {
                let para_config: ParallelNodeConfig =
                    serde_json::from_value(llm_node.config.clone()).unwrap_or(ParallelNodeConfig {
                        branches: vec![],
                        wait_for_all: true,
                        timeout: None,
                        aggregation: None,
                        auto_input_from_parent: true,
                        sub_graph: None,
                    });
                WorkflowNode::Parallel(ParallelNode {
                    base,
                    config: para_config,
                })
            },
            "loop" => {
                let loop_config: LoopNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(LoopNodeConfig {
                        loop_type: LoopType::ForEach,
                        max_iterations: Some(100),
                        continue_on_error: false,
                        body_steps: vec![],
                        items_var: None,
                        iteratee_var: None,
                        iter_input_var: None,
                        iter_output_var: None,
                        partial_result_var: None,
                        continue_condition: None,
                        sub_graph: None,
                        interrupt_after_each: false,
                        interrupt_nodes: vec![],
                    });
                WorkflowNode::Loop(LoopNode {
                    base,
                    config: loop_config,
                })
            },
            "tool" => {
                let tool_config: ToolNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(ToolNodeConfig {
                        tool_name: "".to_string(),
                        input_mapping: std::collections::HashMap::new(),
                        output_var: "".to_string(),
                    });
                WorkflowNode::Tool(ToolNode {
                    base,
                    config: tool_config,
                })
            },
            "code" => {
                let code_config: CodeNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(CodeNodeConfig {
                        language: "javascript".to_string(),
                        code: "".to_string(),
                        output_var: "".to_string(),
                        tool_name: None,
                    });
                WorkflowNode::Code(CodeNode {
                    base,
                    config: code_config,
                })
            },
            "merge" => {
                let merge_config: MergeNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(MergeNodeConfig {
                        merge_type: MergeStrategy::All,
                        inputs: vec![],
                        auto_inputs_from_branches: false,
                    });
                WorkflowNode::Merge(MergeNode {
                    base,
                    config: merge_config,
                })
            },
            "delay" => {
                let delay_config: DelayNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(DelayNodeConfig {
                        delay_type: "fixed".to_string(),
                        seconds: 5,
                        until: None,
                    });
                WorkflowNode::Delay(DelayNode {
                    base,
                    config: delay_config,
                })
            },
            "validation" => {
                let val_config: ValidationNodeConfig = serde_json::from_value(
                    llm_node.config.clone(),
                )
                .unwrap_or(ValidationNodeConfig {
                    assertions: vec![],
                    on_fail: "abort".to_string(),
                    max_retries: 0,
                });
                WorkflowNode::Validation(ValidationNode {
                    base,
                    config: val_config,
                })
            },
            "subWorkflow" => {
                let sub_config: SubWorkflowNodeConfig = serde_json::from_value(
                    llm_node.config.clone(),
                )
                .unwrap_or(SubWorkflowNodeConfig {
                    sub_workflow_id: "".to_string(),
                    input_mapping: std::collections::HashMap::new(),
                    output_var: "result".to_string(),
                    is_async: false,
                    sub_graph: None,
                });
                WorkflowNode::SubWorkflow(SubWorkflowNode {
                    base,
                    config: sub_config,
                })
            },
            "documentParser" => {
                let doc_config: DocumentParserNodeConfig = serde_json::from_value(
                    llm_node.config.clone(),
                )
                .unwrap_or(DocumentParserNodeConfig {
                    input_var: "input".to_string(),
                    parser_type: "auto".to_string(),
                    output_var: "parsed".to_string(),
                });
                WorkflowNode::DocumentParser(DocumentParserNode {
                    base,
                    config: doc_config,
                })
            },
            "vectorRetrieve" => {
                let vec_config: VectorRetrieveNodeConfig = serde_json::from_value(
                    llm_node.config.clone(),
                )
                .unwrap_or(VectorRetrieveNodeConfig {
                    query: "".to_string(),
                    knowledge_base_id: "".to_string(),
                    top_k: 5,
                    similarity_threshold: None,
                    output_var: "retrieved".to_string(),
                });
                WorkflowNode::VectorRetrieve(VectorRetrieveNode {
                    base,
                    config: vec_config,
                })
            },
            "end" => {
                let end_config: EndNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(EndNodeConfig { output_var: None });
                WorkflowNode::End(EndNode {
                    base,
                    config: end_config,
                })
            },
            "switch" => {
                let cfg: SwitchNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(SwitchNodeConfig {
                        input_var: "input".to_string(),
                        cases: vec![],
                        default_case: None,
                        match_mode: "exact".to_string(),
                        output_var: "switched".to_string(),
                        use_llm: None,
                        llm_model: None,
                        llm_prompt: None,
                    });
                WorkflowNode::Switch(SwitchNode { base, config: cfg })
            },
            "httpRequest" => {
                let cfg: HttpRequestNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(HttpRequestNodeConfig {
                        url: "".to_string(),
                        method: "GET".to_string(),
                        headers: std::collections::HashMap::new(),
                        body: None,
                        body_type: "json".to_string(),
                        timeout_secs: 30,
                        output_var: "response".to_string(),
                    });
                WorkflowNode::HttpRequest(HttpRequestNode { base, config: cfg })
            },
            "databaseQuery" => {
                let cfg: DatabaseQueryNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(DatabaseQueryNodeConfig {
                        query: "".to_string(),
                        params: vec![],
                        connection_name: None,
                        timeout_secs: 30,
                        output_var: "db_result".to_string(),
                    });
                WorkflowNode::DatabaseQuery(DatabaseQueryNode { base, config: cfg })
            },
            "notification" => {
                let cfg: NotificationNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(NotificationNodeConfig {
                        channel: "inapp".to_string(),
                        message: "".to_string(),
                        webhook_url: None,
                        recipients: vec![],
                        subject: None,
                        enabled: true,
                        output_var: "notified".to_string(),
                    });
                WorkflowNode::Notification(NotificationNode { base, config: cfg })
            },
            "approval" => {
                let cfg: ApprovalNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(ApprovalNodeConfig {
                        message: "".to_string(),
                        approver: None,
                        timeout_secs: 86400,
                        timeout_action: "auto_reject".to_string(),
                        output_var: "approved".to_string(),
                    });
                WorkflowNode::Approval(ApprovalNode { base, config: cfg })
            },
            "fileOperation" => {
                let cfg: FileOperationNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(FileOperationNodeConfig {
                        operation: "read".to_string(),
                        file_path: "".to_string(),
                        content: None,
                        output_var: "file_result".to_string(),
                    });
                WorkflowNode::FileOperation(FileOperationNode { base, config: cfg })
            },
            "dataTransformer" => {
                let cfg: DataTransformerNodeConfig = serde_json::from_value(
                    llm_node.config.clone(),
                )
                .unwrap_or(DataTransformerNodeConfig {
                    input_var: "input".to_string(),
                    expression: "".to_string(),
                    output_var: "transformed".to_string(),
                });
                WorkflowNode::DataTransformer(DataTransformerNode { base, config: cfg })
            },
            "webhookSend" => {
                let cfg: WebhookSendNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(WebhookSendNodeConfig {
                        url: "".to_string(),
                        method: "POST".to_string(),
                        body: None,
                        headers: std::collections::HashMap::new(),
                        output_var: "webhook_result".to_string(),
                    });
                WorkflowNode::WebhookSend(WebhookSendNode { base, config: cfg })
            },
            "logging" => {
                let cfg: LoggingNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(LoggingNodeConfig {
                        level: "info".to_string(),
                        message: "".to_string(),
                        output_var: "logged".to_string(),
                    });
                WorkflowNode::Logging(LoggingNode { base, config: cfg })
            },
            "llmClassifier" => {
                let cfg: LlmClassifierNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(LlmClassifierNodeConfig {
                        categories: vec![],
                        prompt: "".to_string(),
                        model: None,
                        input_var: "input".to_string(),
                        output_var: "category".to_string(),
                        confidence_threshold: None,
                        fallback_label: None,
                        consistency_check: None,
                    });
                WorkflowNode::LlmClassifier(LlmClassifierNode { base, config: cfg })
            },
            "aggregator" => {
                let cfg: AggregatorNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(AggregatorNodeConfig {
                        strategy: "all".to_string(),
                        input_sources: vec![],
                        output_var: "aggregated".to_string(),
                        wait_for_all: true,
                        weights: vec![],
                        sub_graph: None,
                        summarize_prompt: None,
                        summarize_model: None,
                    });
                WorkflowNode::Aggregator(AggregatorNode { base, config: cfg })
            },
            "email" => {
                let cfg: EmailNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(EmailNodeConfig {
                        to: vec![],
                        subject: "".to_string(),
                        body: "".to_string(),
                        smtp_host: None,
                        smtp_port: None,
                        smtp_user: None,
                        smtp_pass: None,
                        output_var: "sent".to_string(),
                    });
                WorkflowNode::Email(EmailNode { base, config: cfg })
            },
            "debate" => {
                let cfg: DebateNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(DebateNodeConfig {
                        debater_steps: vec![],
                        max_rounds: 2,
                        convergence_prompt: None,
                        convergence_model: None,
                        convergence_model_role: None,
                        topic_var: "topic".to_string(),
                        output_var: "debate_result".to_string(),
                        sub_graph: None,
                    });
                WorkflowNode::Debate(DebateNode { base, config: cfg })
            },
            _ => WorkflowNode::Agent(AgentNode {
                base,
                config: AgentNodeConfig {
                    system_prompt: llm_node.description.clone().unwrap_or_default(),
                    model: Some(model_id.to_string()),
                    temperature: Some(0.7),
                    max_tokens: Some(2048),
                    output_mode: OutputMode::Text,
                    output_var: "result".to_string(),
                    tools: vec![],
                    exposed_tools: vec![],
                    context_sources: vec![],
                    agent_profile_id: None,
                    max_tool_rounds: None,
                    execution_mode: None,
                    rag_source_ids: vec![],
                    model_role: None,
                    consistency_check: None,
                    hallucination_guard: None,
                    input_mapping: std::collections::HashMap::new(),
                },
            }),
        };
        nodes.push(node);
    }

    let mut edges = Vec::new();
    for (i, llm_edge) in parsed.edges.iter().enumerate() {
        let source_id = id_to_node_id
            .get(&llm_edge.source)
            .cloned()
            .unwrap_or(llm_edge.source.clone());
        let target_id = id_to_node_id
            .get(&llm_edge.target)
            .cloned()
            .unwrap_or(llm_edge.target.clone());

        let edge_type = match llm_edge.edge_type.as_deref() {
            Some("conditionTrue") => EdgeType::ConditionTrue,
            Some("conditionFalse") => EdgeType::ConditionFalse,
            Some("loopBack") => EdgeType::LoopBack,
            Some("parallelBranch") => EdgeType::ParallelBranch,
            Some("merge") => EdgeType::Merge,
            Some("error") => EdgeType::Error,
            Some("debateRound") => EdgeType::DebateRound,
            _ => EdgeType::Direct,
        };

        edges.push(WorkflowEdge {
            id: if llm_edge.id.is_empty() {
                format!("edge-{}", i + 1)
            } else {
                llm_edge.id.clone()
            },
            source: source_id,
            source_handle: None,
            target: target_id,
            target_handle: None,
            edge_type,
            label: None,
        });
    }

    Ok(WorkflowGenerationResult {
        nodes,
        edges,
        explanation: parsed
            .explanation
            .or_else(|| Some(format!("基于您的描述 '{}' 生成了工作流", prompt))),
    })
}

#[tauri::command]
pub async fn generate_workflow_from_prompt(
    state: State<'_, AppState>,
    prompt: String,
    current_nodes: Option<Vec<serde_json::Value>>,
    current_edges: Option<Vec<serde_json::Value>>,
) -> Result<WorkflowGenerationResult, String> {
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key = resolved.provider_type.registry_key();
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| {
            ErrorResponse::err_with_detail(
                provider_err::ADAPTER_NOT_FOUND,
                format!("Provider adapter not found for type: {}", registry_key),
            )
        })?;

    let mut context_section = String::new();
    if let Some(nodes) = &current_nodes {
        if !nodes.is_empty() {
            let node_summary: Vec<String> = nodes
                .iter()
                .map(|n| {
                    let nt = n.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let title = n.get("title").and_then(|v| v.as_str()).unwrap_or(nt);
                    let id = n.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    format!("- [{}] {} ({})", id, title, nt)
                })
                .collect();
            let mut edge_section = String::new();
            if let Some(edges) = &current_edges {
                if !edges.is_empty() {
                    let edge_summary: Vec<String> = edges
                        .iter()
                        .map(|e| {
                            let src = e.get("source").and_then(|v| v.as_str()).unwrap_or("?");
                            let tgt = e.get("target").and_then(|v| v.as_str()).unwrap_or("?");
                            let et = e
                                .get("edge_type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("direct");
                            format!("{} --[{}]--> {}", src, et, tgt)
                        })
                        .collect();
                    edge_section =
                        format!("\nEdges ({}):\n{}", edges.len(), edge_summary.join("\n"));
                }
            }
            context_section = format!(
                "\n\nCurrent workflow already has these nodes:\n{}\n{}Please generate nodes that integrate with the existing workflow. Use the existing node IDs in edges where appropriate.",
                node_summary.join("\n"),
                edge_section
            );
        }
    }

    let system_prompt = format!(
        r#"You are a workflow design assistant. Generate a workflow based on the user's natural language description.

=== 任务边界 ===
- 仅当用户希望"创建/重写/批量修改"工作流时输出完整 JSON。
- 若用户只是"修改某个节点"、"询问某节点配置"或"删除某节点"，应只输出单点修改（参见 chat 模式协议）。
- 若需求模糊（如"帮我优化一下"），先简短澄清再生成。

=== 完整节点类型（共 27 种，必须从下列中选）===
trigger, agent, llm, condition, switch, parallel, loop, merge, delay,
httpRequest, databaseQuery, tool, code, subWorkflow, documentParser,
vectorRetrieve, validation, notification, approval, fileOperation,
dataTransformer, webhookSend, logging, llmClassifier, aggregator, email, end

{NODE_SCHEMAS_DOC}

=== Few-shot 范例 ===
{FEW_SHOT_EXAMPLES}

=== 输出格式 ===
{{
  "intent": "generate" | "clarify" | "refuse",
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
6. 若用户描述有歧义（不知选哪个节点），intent=clarify，nodes/edges 留空数组，explanation 写明澄清问题。
7. 若请求违反平台规则（如要求越权访问），intent=refuse，explanation 写明原因。
8. 涉及并发/批量处理用 parallel；循环遍历用 loop；不要把循环当并发。
9. 跨多个服务编排时优先用 subWorkflow 复用已有工作流。
10. 知识检索/文档问答用 vectorRetrieve + documentParser；不要用 llm 凭空生成。{context_section}"#
    );

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
                content: ChatContent::Text(prompt.clone()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        temperature: Some(0.7),
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
    };

    let response = adapter
        .chat(&resolved.ctx, request)
        .await
        .map_err(|e| format!("LLM API error: {}", e))?;

    parse_llm_response(&prompt, &response.content, &resolved.model_id)
}

#[tauri::command]
pub async fn optimize_agent_prompt(
    state: State<'_, AppState>,
    prompt: String,
) -> Result<String, String> {
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key = resolved.provider_type.registry_key();
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| {
            ErrorResponse::err_with_detail(
                provider_err::ADAPTER_NOT_FOUND,
                format!("Provider adapter not found for type: {}", registry_key),
            )
        })?;

    let system_prompt = r#"You are an expert prompt engineer. Your task is to optimize the given agent prompt to make it more effective, clear, and structured.

Rules for optimization:
1. Add a clear role definition at the beginning
2. Break down complex instructions into numbered steps
3. Add specific constraints and boundaries
4. Include output format specifications
5. Add error handling guidance
6. Make the prompt more specific and actionable
7. Remove ambiguity and vague language
8. Keep the original intent and purpose intact

Language rules:
- If the input prompt is in Chinese, output the optimized prompt in Chinese
- If the input prompt is in English, output the optimized prompt in English
- Match the language of the input prompt

Output ONLY the optimized prompt text, without any explanation or meta-commentary."#;

    let request = ChatRequest {
        model: resolved.model_id.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(format!(
                    "Please optimize the following agent prompt:\n\n{}",
                    prompt
                )),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        temperature: Some(0.7),
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
    };

    let response = adapter
        .chat(&resolved.ctx, request)
        .await
        .map_err(|e| format!("LLM API error: {}", e))?;

    Ok(response.content)
}

#[tauri::command]
pub async fn recommend_nodes(
    state: State<'_, AppState>,
    context: String,
    current_node_types: Option<Vec<String>>,
) -> Result<Vec<NodeRecommendation>, String> {
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key = resolved.provider_type.registry_key();
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| {
            ErrorResponse::err_with_detail(
                provider_err::ADAPTER_NOT_FOUND,
                format!("Provider adapter not found for type: {}", registry_key),
            )
        })?;

    let system_prompt = r#"You are a workflow design assistant. Based on the user's description of their workflow needs, recommend the most suitable node types.

Available node types:
- trigger: Workflow trigger (manual, schedule, webhook, event)
- agent: AI Agent node for autonomous task execution with role, tools, and context
- llm: Direct LLM call node for text generation or analysis
- condition: Conditional branching node (if/else logic)
- parallel: Parallel execution node for concurrent tasks
- loop: Loop iteration node (forEach, while, doWhile, until)
- merge: Merge multiple branches into one
- delay: Delay/wait node
- tool: External tool/API call node
- code: Custom code execution node (JavaScript/Python)
- subWorkflow: Sub-workflow invocation node
- documentParser: Document parsing and extraction node
- vectorRetrieve: Vector similarity search from knowledge base
- validation: Data validation and assertion node
- end: Workflow end node

Output a valid JSON array with this structure:
[
  {
    "node_type": "agent",
    "label": "Agent 节点",
    "description": "Description of why this node type is recommended",
    "confidence": 0.9
  }
]

Rules:
1. Return at most 5 recommendations, sorted by confidence (highest first)
2. Confidence should be between 0.0 and 1.0
3. Provide clear descriptions explaining why each node is recommended
4. Consider the workflow context and how nodes work together
5. Use Chinese for labels and descriptions when the input is in Chinese"#;

    let mut existing_section = String::new();
    if let Some(types) = &current_node_types {
        if !types.is_empty() {
            existing_section = format!(
                "\n\nCurrent workflow already has these node types: {}. Avoid recommending duplicate types unless the workflow specifically needs multiple instances of the same type.",
                types.join(", ")
            );
        }
    }

    let request = ChatRequest {
        model: resolved.model_id.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(format!(
                    "Based on this workflow context, recommend suitable node types:\n\n{}{}",
                    context, existing_section
                )),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ],
        temperature: Some(0.7),
        top_p: None,
        max_tokens: Some(2048),
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
    };

    let response = adapter
        .chat(&resolved.ctx, request)
        .await
        .map_err(|e| format!("LLM API error: {}", e))?;

    let json_str = match extract_json_from_response(&response.content) {
        Some(s) => s,
        None => {
            return Ok(fallback_recommendations(&context));
        },
    };

    match serde_json::from_str::<Vec<NodeRecommendation>>(json_str) {
        Ok(recs) => Ok(recs),
        Err(_) => Ok(fallback_recommendations(&context)),
    }
}

fn fallback_recommendations(context: &str) -> Vec<NodeRecommendation> {
    let context_lower = context.to_lowercase();
    let mut recommendations = Vec::new();

    if context_lower.contains("代码")
        || context_lower.contains("code")
        || context_lower.contains("审查")
    {
        recommendations.push(NodeRecommendation {
            node_type: "llm".to_string(),
            label: "LLM 节点".to_string(),
            description: "用于代码分析和审查".to_string(),
            confidence: 0.95,
        });
        recommendations.push(NodeRecommendation {
            node_type: "code".to_string(),
            label: "代码节点".to_string(),
            description: "执行代码进行静态分析".to_string(),
            confidence: 0.9,
        });
    }

    if context_lower.contains("测试") || context_lower.contains("test") {
        recommendations.push(NodeRecommendation {
            node_type: "agent".to_string(),
            label: "Agent 节点".to_string(),
            description: "自动化测试执行".to_string(),
            confidence: 0.9,
        });
        recommendations.push(NodeRecommendation {
            node_type: "condition".to_string(),
            label: "条件节点".to_string(),
            description: "根据测试结果进行分支".to_string(),
            confidence: 0.85,
        });
    }

    if context_lower.contains("并行")
        || context_lower.contains("parallel")
        || context_lower.contains("并发")
    {
        recommendations.push(NodeRecommendation {
            node_type: "parallel".to_string(),
            label: "并行节点".to_string(),
            description: "并行执行多个任务".to_string(),
            confidence: 0.95,
        });
    }

    if context_lower.contains("循环")
        || context_lower.contains("loop")
        || context_lower.contains("迭代")
    {
        recommendations.push(NodeRecommendation {
            node_type: "loop".to_string(),
            label: "循环节点".to_string(),
            description: "重复执行任务直到满足条件".to_string(),
            confidence: 0.95,
        });
    }

    if context_lower.contains("延迟")
        || context_lower.contains("delay")
        || context_lower.contains("等待")
    {
        recommendations.push(NodeRecommendation {
            node_type: "delay".to_string(),
            label: "延迟节点".to_string(),
            description: "在继续之前等待指定时间".to_string(),
            confidence: 0.8,
        });
    }

    if context_lower.contains("文档")
        || context_lower.contains("document")
        || context_lower.contains("解析")
    {
        recommendations.push(NodeRecommendation {
            node_type: "documentParser".to_string(),
            label: "文档解析节点".to_string(),
            description: "解析和提取文档内容".to_string(),
            confidence: 0.9,
        });
    }

    if context_lower.contains("搜索")
        || context_lower.contains("search")
        || context_lower.contains("检索")
    {
        recommendations.push(NodeRecommendation {
            node_type: "vectorRetrieve".to_string(),
            label: "向量检索节点".to_string(),
            description: "从知识库检索相关信息".to_string(),
            confidence: 0.85,
        });
    }

    if context_lower.contains("数据") || context_lower.contains("data") {
        recommendations.push(NodeRecommendation {
            node_type: "validation".to_string(),
            label: "校验节点".to_string(),
            description: "验证数据格式和完整性".to_string(),
            confidence: 0.85,
        });
    }

    if context_lower.contains("知识")
        || context_lower.contains("knowledge")
        || context_lower.contains("rag")
    {
        recommendations.push(NodeRecommendation {
            node_type: "vectorRetrieve".to_string(),
            label: "向量检索节点".to_string(),
            description: "从知识库检索相关信息".to_string(),
            confidence: 0.9,
        });
    }

    if context_lower.contains("合并")
        || context_lower.contains("merge")
        || context_lower.contains("汇聚")
    {
        recommendations.push(NodeRecommendation {
            node_type: "merge".to_string(),
            label: "合并节点".to_string(),
            description: "合并多个分支的结果".to_string(),
            confidence: 0.9,
        });
    }

    if context_lower.contains("子流程")
        || context_lower.contains("subworkflow")
        || context_lower.contains("嵌套")
    {
        recommendations.push(NodeRecommendation {
            node_type: "subWorkflow".to_string(),
            label: "子流程节点".to_string(),
            description: "调用另一个工作流作为子流程".to_string(),
            confidence: 0.9,
        });
    }

    if context_lower.contains("校验")
        || context_lower.contains("validate")
        || context_lower.contains("验证")
    {
        recommendations.push(NodeRecommendation {
            node_type: "validation".to_string(),
            label: "校验节点".to_string(),
            description: "数据验证和断言检查".to_string(),
            confidence: 0.9,
        });
    }

    if context_lower.contains("异步") || context_lower.contains("async") {
        recommendations.push(NodeRecommendation {
            node_type: "subWorkflow".to_string(),
            label: "子流程节点".to_string(),
            description: "异步执行子工作流".to_string(),
            confidence: 0.85,
        });
    }

    if recommendations.is_empty() {
        recommendations.push(NodeRecommendation {
            node_type: "agent".to_string(),
            label: "Agent 节点".to_string(),
            description: "通用 AI Agent 用于处理任务".to_string(),
            confidence: 0.7,
        });
        recommendations.push(NodeRecommendation {
            node_type: "llm".to_string(),
            label: "LLM 节点".to_string(),
            description: "调用大语言模型进行处理".to_string(),
            confidence: 0.65,
        });
        recommendations.push(NodeRecommendation {
            node_type: "tool".to_string(),
            label: "工具节点".to_string(),
            description: "调用外部工具或 API".to_string(),
            confidence: 0.6,
        });
        recommendations.push(NodeRecommendation {
            node_type: "trigger".to_string(),
            label: "触发器节点".to_string(),
            description: "工作流触发入口".to_string(),
            confidence: 0.55,
        });
        recommendations.push(NodeRecommendation {
            node_type: "end".to_string(),
            label: "结束节点".to_string(),
            description: "工作流结束节点".to_string(),
            confidence: 0.5,
        });
    }

    recommendations.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    recommendations.truncate(5);

    recommendations
}

#[tauri::command]
pub async fn workflow_ai_chat_stream(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    message: String,
    history: Vec<AiChatMessage>,
    current_nodes: Option<Vec<serde_json::Value>>,
    current_edges: Option<Vec<serde_json::Value>>,
    session_id: String,
) -> Result<(), String> {
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key = resolved.provider_type.registry_key();
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| {
            ErrorResponse::err_with_detail(
                provider_err::ADAPTER_NOT_FOUND,
                format!("Provider adapter not found for type: {}", registry_key),
            )
        })?;

    let mut canvas_section = String::new();
    if let Some(nodes) = &current_nodes {
        if !nodes.is_empty() {
            let node_summary: Vec<String> = nodes
                .iter()
                .map(|n| {
                    let nt = n.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let title = n.get("title").and_then(|v| v.as_str()).unwrap_or(nt);
                    format!("- {} ({})", title, nt)
                })
                .collect();
            let edge_count = current_edges.as_ref().map(|e| e.len()).unwrap_or(0);
            canvas_section = format!(
                "\n\nCurrent workflow canvas:\nNodes ({}):\n{}\nEdges: {}",
                nodes.len(),
                node_summary.join("\n"),
                edge_count
            );
        }
    }

    let system_prompt = format!(
        r#"You are an AI assistant for a workflow editor. You help users create, modify, and optimize workflows through conversation.

=== 意图路由（务必先判断用户意图再决定输出形式）===
- 全新生成/重写工作流 → generate_workflow action
- 添加新节点 → add_node / add_nodes action
- 修改节点属性 → update_node action（仅传需要改的字段）
- 删除节点/边 → delete_node / delete_edge action
- 修改边 → add_edge / update_edge / delete_edge
- 优化 Agent 提示词 → optimize_prompt action
- 解释/分析/问问题 → 纯文本回复，不输出 action 块
- 单纯咨询（如"什么是 parallel 节点"）→ 纯文本

=== 你能做的事 ===
1. 全新生成完整工作流
2. 修改现有工作流（add/update/delete nodes, add/update/delete edges）
3. 优化 agent prompt
4. 推荐节点类型
5. 解释工作流概念

=== 完整节点类型（27 种，add_node 时 node.type 必须是下列之一）===
trigger, agent, llm, condition, switch, parallel, loop, merge, delay,
httpRequest, databaseQuery, tool, code, subWorkflow, documentParser,
vectorRetrieve, validation, notification, approval, fileOperation,
dataTransformer, webhookSend, logging, llmClassifier, aggregator, email, end

{NODE_SCHEMAS_DOC}

=== Action 协议（必须用 :::action 包裹 JSON）===
:::action
{{"action_type": "generate_workflow", "data": {{"nodes": [...], "edges": [...]}}}}
:::

:::action
{{"action_type": "add_node", "data": {{"node": {{...}}, "position": {{"x": 0, "y": 0}}}}}}
:::

:::action
{{"action_type": "add_nodes", "data": {{"nodes": [...]}}}}
:::

:::action
{{"action_type": "update_node", "data": {{"node_id": "...", "changes": {{"title": "新标题", "config": {{...}}, "position": {{...}}}}}}}}
:::

:::action
{{"action_type": "modify_node", "data": {{"node_id": "...", "changes": {{}}}}}}
:::

:::action
{{"action_type": "delete_node", "data": {{"node_id": "..."}}}}
:::

:::action
{{"action_type": "delete_nodes", "data": {{"node_ids": ["...", "..."]}}}}
:::

:::action
{{"action_type": "add_edge", "data": {{"edge": {{"id": "...", "source": "...", "target": "...", "edge_type": "direct", "label": "..."}}}}}}
:::

:::action
{{"action_type": "update_edge", "data": {{"edge_id": "...", "changes": {{"label": "...", "edge_type": "conditionTrue"}}}}}}
:::

:::action
{{"action_type": "delete_edge", "data": {{"edge_id": "..."}}}}
:::

:::action
{{"action_type": "optimize_prompt", "data": {{"node_id": "...", "optimized_prompt": "..."}}}}
:::

=== 强制规则 ===
1. 一次回复可包含多个 :::action 块（按依赖顺序排列）。
2. add_node/add_nodes 时严格按上面对应 node_type 的 schema 写 config；缺字段的未知参数会被丢弃。
3. update_node 是部分更新：只传需要改的字段。删除属性请传 null。
4. 涉及并行批量 → parallel，循环遍历 → loop，不要混用。
5. 描述里"审批"→ approval；"邮件"→ email；"HTTP/接口/REST"→ httpRequest；"数据库/SQL"→ databaseQuery；"Webhook 回调"→ webhookSend；"分类/打标"→ llmClassifier。
6. 在 action 块之前先用一句话解释你要做什么，让用户能跟得上。
7. 所有改动会先以 diff 形式展示给用户确认，不会自动落库。
8. Respond in the same language as the user's message.{}"#,
        canvas_section
    );

    let mut chat_messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".to_string(),
        content: ChatContent::Text(system_prompt),
        tool_calls: None,
        tool_call_id: None,
        thinking: None,
    }];

    for msg in &history {
        chat_messages.push(ChatMessage {
            role: msg.role.clone(),
            content: ChatContent::Text(msg.content.clone()),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        });
    }

    chat_messages.push(ChatMessage {
        role: "user".to_string(),
        content: ChatContent::Text(message),
        tool_calls: None,
        tool_call_id: None,
        thinking: None,
    });

    let request = ChatRequest {
        model: resolved.model_id.clone(),
        messages: chat_messages,
        stream: true,
        temperature: Some(0.7),
        top_p: None,
        max_tokens: Some(4096),
        tools: None,
        thinking_budget: None,
        use_max_completion_tokens: None,
        thinking_param_style: None,
        api_mode: None,
        instructions: None,
        conversation: None,
        previous_response_id: None,
        store: None,
    };

    let cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut store = get_cancel_store().lock().await;
        store.insert(session_id.clone(), cancel_flag.clone());
    }
    let _ = app.emit(
        "workflow-ai-chat-start",
        serde_json::json!({
            "session_id": session_id,
        }),
    );

    let mut stream = adapter.chat_stream(&resolved.ctx, request, None);
    let message_id = format!("wf-ai-{}", uuid::Uuid::new_v4());

    while let Some(result) = stream.next().await {
        if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        match result {
            Ok(chunk) => {
                let is_done = chunk.done;
                let content_delta = chunk.content.clone();
                let thinking_delta = chunk.thinking.clone();

                let mut emit_content = String::new();
                if let Some(ref t) = thinking_delta {
                    if !t.is_empty() {
                        emit_content
                            .push_str(&format!("<think data-aq>\n{}\n</think data-aq>\n", t));
                    }
                }
                if let Some(ref c) = content_delta {
                    emit_content.push_str(c);
                }

                let emitted_chunk = ChatStreamChunk {
                    content: if emit_content.is_empty() {
                        None
                    } else {
                        Some(emit_content)
                    },
                    thinking: None,
                    done: is_done,
                    is_final: if is_done { Some(true) } else { None },
                    usage: chunk.usage.clone(),
                    tool_calls: None,
                };

                let _ = app.emit(
                    "workflow-ai-chat-chunk",
                    ChatStreamEvent {
                        conversation_id: session_id.clone(),
                        message_id: message_id.clone(),
                        model_id: Some(resolved.model_id.clone()),
                        provider_id: Some(resolved.ctx.provider_id.clone()),
                        chunk: emitted_chunk,
                    },
                );

                if is_done {
                    break;
                }
            },
            Err(e) => {
                let _ = app.emit(
                    "workflow-ai-chat-error",
                    ChatStreamErrorEvent {
                        conversation_id: session_id.clone(),
                        message_id: message_id.clone(),
                        error: format!("{}", e),
                    },
                );
                break;
            },
        }
    }

    let _ = app.emit(
        "workflow-ai-chat-done",
        serde_json::json!({
            "session_id": session_id,
            "message_id": message_id,
        }),
    );

    {
        let mut store = get_cancel_store().lock().await;
        store.remove(&session_id);
    }

    Ok(())
}

#[tauri::command]
pub async fn workflow_ai_chat_cancel(
    _state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let store = get_cancel_store().lock().await;
    if let Some(flag) = store.get(&session_id) {
        flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    Ok(())
}
