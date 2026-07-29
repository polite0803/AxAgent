use super::helpers::{
    FEW_SHOT_EXAMPLES, NODE_SCHEMAS_DOC, NodeRecommendation, WorkflowGenerationResult,
    build_roles_and_experts_brief, extract_json_from_response, parse_llm_response,
    resolve_ai_provider,
};
use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::provider as provider_err;
use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest};
use serde::{Deserialize, Serialize};
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn generate_workflow_from_prompt(
    state: State<'_, AppState>,
    prompt: String,
    current_nodes: Option<Vec<serde_json::Value>>,
    current_edges: Option<Vec<serde_json::Value>>,
) -> Result<WorkflowGenerationResult, String> {
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&resolved.provider_type);
    let adapter = state.harness.provider_registry().get(registry_key).ok_or_else(|| {
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
                            let et =
                                e.get("edge_type").and_then(|v| v.as_str()).unwrap_or("direct");
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

=== 可用业务岗位与专家清单 ===
{roles_brief}
提示：agent 节点的 config 中可引用上述业务岗位 ID（business_role_id）和专家 ID（expert_id），
让节点执行时自动拼接对应的 system_prompt（详见 4 层 prompt 层级）。

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
10. 知识检索/文档问答用 vectorRetrieve + documentParser；不要用 llm 凭空生成。{context_section}"#,
        roles_brief = build_roles_and_experts_brief()
            .await
            .unwrap_or_else(|| "（暂无可用业务岗位/专家）".to_string())
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
        response_format: None,
    };

    let response = adapter
        .chat(&resolved.ctx, request.into())
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

    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&resolved.provider_type);
    let adapter = state.harness.provider_registry().get(registry_key).ok_or_else(|| {
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
        response_format: None,
    };

    let response = adapter
        .chat(&resolved.ctx, request.into())
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

    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&resolved.provider_type);
    let adapter = state.harness.provider_registry().get(registry_key).ok_or_else(|| {
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
        response_format: None,
    };

    let response = adapter
        .chat(&resolved.ctx, request.into())
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
        b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
    });
    recommendations.truncate(5);

    recommendations
}

// ============================================================
// NL2Skill / NL2UI — 自然语言→技能定义 / 动态 UI Schema
// ============================================================

/// NL2Skill 生成结果（与前端 NL2SkillResult 对齐，phases 由前端构造）
#[derive(Debug, Serialize, Deserialize)]
pub struct SkillGenerationResult {
    /// 技能定义（SkillDefinition）
    pub skill: serde_json::Value,
    /// 置信度 0.0-1.0
    pub confidence: f32,
    /// 后续建议
    pub suggestions: Vec<String>,
    /// 备选技能定义
    #[serde(default)]
    pub alternatives: Vec<serde_json::Value>,
}

/// NL2UI 生成结果（与前端 NL2UIResult 对齐，phases 由前端构造）
#[derive(Debug, Serialize, Deserialize)]
pub struct UIGenerationResult {
    /// UI Schema
    pub schema: serde_json::Value,
    /// 置信度 0.0-1.0
    pub confidence: f32,
    /// 后续建议
    pub suggestions: Vec<String>,
    /// 备选方案
    #[serde(default)]
    pub alternatives: Vec<AlternativeUI>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlternativeUI {
    pub schema: serde_json::Value,
    pub description: String,
}

/// NL2Skill：从自然语言生成技能定义
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn generate_skill_from_prompt(
    state: State<'_, AppState>,
    prompt: String,
    skill_type: Option<String>,
) -> Result<SkillGenerationResult, String> {
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&resolved.provider_type);
    let adapter = state.harness.provider_registry().get(registry_key).ok_or_else(|| {
        ErrorResponse::err_with_detail(
            provider_err::ADAPTER_NOT_FOUND,
            format!("Provider adapter not found for type: {}", registry_key),
        )
    })?;

    let skill_type_str = skill_type.as_deref().unwrap_or("chat");
    let system_prompt = format!(
        r#"You are a skill design assistant. Generate a skill definition based on the user's description.

=== 技能类型 ===
当前目标类型：{skill_type}（chat=对话型 / tool=工具型 / workflow=工作流型 / automation=自动化型）

=== 输出格式（严格遵循 JSON）===
{{
  "skill": {{
    "id": "skill-<简短英文ID>",
    "name": "<中文名称>",
    "description": "<技能用途描述>",
    "type": "{skill_type}",
    "triggers": ["<触发关键词1>", "<触发关键词2>"],
    "prompt_template": "<技能执行时的 prompt 模板，可用 {{{{param_name}}}} 占位>",
    "parameters": [
      {{
        "name": "<参数名>",
        "type": "string|number|boolean|enum|object",
        "description": "<参数说明>",
        "required": true,
        "default": "<可选默认值>",
        "options": ["<enum 类型时的选项>"]
      }}
    ],
    "tools": ["<依赖工具名>"],
    "icon": "<emoji 或图标名>",
    "tags": ["<标签>"]
  }},
  "confidence": 0.0,
  "suggestions": ["<改进建议>"],
  "alternatives": []
}}

=== 强制规则 ===
1. skill.id 用小写英文+短横线，如 "code-review"、"data-extractor"。
2. skill.type 必须与目标类型一致。
3. prompt_template 使用 {{{{param}}}} 双花括号语法标记占位符。
4. parameters 中 required 为 bool，options 仅 enum 类型需要。
5. confidence 范围 0.0-1.0，反映对生成结果的信心。
6. 仅输出 JSON，不要额外解释。"#,
        skill_type = skill_type_str
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
        response_format: None,
    };

    let response = adapter
        .chat(&resolved.ctx, request.into())
        .await
        .map_err(|e| format!("LLM API error: {}", e))?;

    let json_str = extract_json_from_response(&response.content)
        .ok_or_else(|| format!("Failed to parse LLM response as JSON: {}", response.content))?;

    serde_json::from_str::<SkillGenerationResult>(json_str)
        .map_err(|e| format!("Failed to deserialize skill generation result: {}", e))
}

/// NL2UI：从自然语言生成动态 UI Schema
#[tauri::command]
#[tracing::instrument(skip(state))]
pub async fn generate_ui_from_prompt(
    state: State<'_, AppState>,
    prompt: String,
    ui_type: Option<String>,
) -> Result<UIGenerationResult, String> {
    let resolved = resolve_ai_provider(&state).await?;

    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&resolved.provider_type);
    let adapter = state.harness.provider_registry().get(registry_key).ok_or_else(|| {
        ErrorResponse::err_with_detail(
            provider_err::ADAPTER_NOT_FOUND,
            format!("Provider adapter not found for type: {}", registry_key),
        )
    })?;

    let ui_type_str = ui_type.as_deref().unwrap_or("form");
    let system_prompt = format!(
        r#"You are a UI design assistant. Generate a dynamic UI Schema based on the user's description.

=== UI 类型 ===
当前目标类型：{ui_type}（form=表单 / dashboard=仪表盘 / settings=设置面板 / report=报告 / custom=自定义）

=== 输出格式（严格遵循 JSON）===
{{
  "schema": {{
    "type": "object",
    "title": "<UI 标题>",
    "description": "<UI 描述>",
    "properties": {{
      "<field_name>": {{
        "type": "string|number|boolean|array|object",
        "title": "<字段标题>",
        "description": "<字段说明>",
        "default": "<默认值>",
        "enum": ["<枚举选项>"],
        "format": "<date|time|datetime|email|url|textarea|color>",
        "minimum": 0,
        "maximum": 100
      }}
    }},
    "required": ["<必填字段名>"]
  }},
  "confidence": 0.0,
  "suggestions": ["<改进建议>"],
  "alternatives": []
}}

=== 强制规则 ===
1. schema 遵循 JSON Schema 规范，字段命名用 snake_case。
2. type 为 {ui_type} 时应生成对应的合理结构（form=扁平字段、dashboard=分组卡片、settings=分类标签）。
3. confidence 范围 0.0-1.0，反映对生成结果的信心。
4. 仅输出 JSON，不要额外解释。"#,
        ui_type = ui_type_str
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
        response_format: None,
    };

    let response = adapter
        .chat(&resolved.ctx, request.into())
        .await
        .map_err(|e| format!("LLM API error: {}", e))?;

    let json_str = extract_json_from_response(&response.content)
        .ok_or_else(|| format!("Failed to parse LLM response as JSON: {}", response.content))?;

    serde_json::from_str::<UIGenerationResult>(json_str)
        .map_err(|e| format!("Failed to deserialize UI generation result: {}", e))
}
