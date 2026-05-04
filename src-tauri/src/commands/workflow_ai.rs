use crate::AppState;
use axagent_core::crypto::decrypt_key;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, ProviderType};
use axagent_core::workflow_types::*;
use axagent_providers::registry::ProviderRegistry;
use axagent_providers::{resolve_base_url_for_type, ProviderRequestContext};
use serde::{Deserialize, Serialize};
use tauri::State;

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

fn provider_type_to_registry_key(pt: &ProviderType) -> &'static str {
    match pt {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    }
}

fn get_node_type_and_position(index: usize, total: usize) -> (String, Position) {
    let x = 100.0 + (index as f64) * 200.0;
    let y = 300.0;
    let node_type = match index {
        0 => "trigger".to_string(),
        _ if index == total - 1 => "end".to_string(),
        _ => "agent".to_string(),
    };
    (node_type, Position { x, y })
}

fn parse_llm_response(
    prompt: &str,
    response_content: &str,
) -> Result<WorkflowGenerationResult, String> {
    let trimmed = response_content.trim();

    let json_str = if trimmed.contains("```json") {
        trimmed
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .map(|s| s.trim())
    } else if trimmed.starts_with('{') || trimmed.starts_with('[') {
        Some(trimmed)
    } else {
        None
    }
    .ok_or_else(|| {
        format!(
            "Failed to parse LLM response as JSON: {}",
            &response_content[..response_content.len().min(200)]
        )
    })?;

    #[derive(Deserialize)]
    struct LlmWorkflowResponse {
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

    let total_nodes = parsed.nodes.len().max(2);
    let mut nodes = Vec::new();
    let mut id_to_node_id = std::collections::HashMap::new();

    for (i, llm_node) in parsed.nodes.iter().enumerate() {
        let (_node_type, position) = get_node_type_and_position(i, total_nodes);
        let node_id = if llm_node.id.is_empty() {
            format!("{}-{}", llm_node.node_type, i + 1)
        } else {
            llm_node.id.clone()
        };
        id_to_node_id.insert(node_id.clone(), node_id.clone());

        let base = WorkflowNodeBase {
            id: node_id.clone(),
            title: llm_node.title.clone(),
            description: llm_node.description.clone(),
            position,
            enabled: true,
            retry: RetryConfig {
                enabled: false,
                max_retries: 3,
                backoff_type: BackoffType::Exponential,
                base_delay_ms: 1000,
                max_delay_ms: 5000,
            },
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
                        role: AgentRole::Researcher,
                        system_prompt: format!(
                            "You are an AI assistant. {}",
                            llm_node.description.clone().unwrap_or_default()
                        ),
                        model: Some("gpt-4".to_string()),
                        temperature: Some(0.7),
                        max_tokens: Some(2048),
                        output_mode: OutputMode::Text,
                        output_var: "result".to_string(),
                        tools: vec![],
                        context_sources: vec![],
                        agent_profile_id: None,
                    });
                WorkflowNode::Agent(AgentNode {
                    base,
                    config: agent_config,
                })
            },
            "llm" => {
                let llm_config: LLMNodeConfig = serde_json::from_value(llm_node.config.clone())
                    .unwrap_or(LLMNodeConfig {
                        model: "gpt-4".to_string(),
                        prompt: llm_node.description.clone().unwrap_or_default(),
                        temperature: Some(0.7),
                        max_tokens: Some(2048),
                        tools: None,
                        functions: None,
                        messages: None,
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
                        continue_condition: None,
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
                    });
                WorkflowNode::Code(CodeNode {
                    base,
                    config: code_config,
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
            _ => WorkflowNode::Agent(AgentNode {
                base,
                config: AgentNodeConfig {
                    role: AgentRole::Researcher,
                    system_prompt: llm_node.description.clone().unwrap_or_default(),
                    model: Some("gpt-4".to_string()),
                    temperature: Some(0.7),
                    max_tokens: Some(2048),
                    output_mode: OutputMode::Text,
                    output_var: "result".to_string(),
                    tools: vec![],
                    context_sources: vec![],
                    agent_profile_id: None,
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
) -> Result<WorkflowGenerationResult, String> {
    let providers = axagent_core::repo::provider::list_providers(&state.sea_db)
        .await
        .map_err(|e| format!("Failed to list providers: {}", e))?;

    let provider = providers.iter().find(|p| p.enabled).ok_or_else(|| {
        "No enabled provider found. Please configure a provider in settings.".to_string()
    })?;

    let provider_key = axagent_core::repo::provider::get_active_key(&state.sea_db, &provider.id)
        .await
        .map_err(|e| format!("Failed to get provider key: {}", e))?;

    let decrypted_key = decrypt_key(&provider_key.key_encrypted, &state.master_key)
        .map_err(|e| format!("Failed to decrypt API key: {}", e))?;

    let registry = ProviderRegistry::create_default();
    let registry_key = provider_type_to_registry_key(&provider.provider_type);
    let adapter = registry
        .get(registry_key)
        .ok_or_else(|| format!("Provider adapter not found for type: {}", registry_key))?;

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

    let system_prompt = r#"You are a workflow design assistant. Generate a workflow based on the user's natural language description.

Output a valid JSON object with this structure:
{
  "nodes": [
    {
      "id": "node-1",
      "node_type": "trigger|agent|llm|condition|parallel|loop|tool|code|end",
      "title": "Node Title",
      "description": "Optional description",
      "config": {} // Node-specific configuration
    }
  ],
  "edges": [
    {
      "id": "edge-1",
      "source": "node-1",
      "target": "node-2",
      "edge_type": "direct|conditionTrue|conditionFalse|loopBack|parallelBranch"
    }
  ],
  "explanation": "Brief explanation of the generated workflow"
}

Rules:
1. Always start with a trigger node
2. Always end with an end node
3. For condition nodes, use edge_type "conditionTrue" or "conditionFalse"
4. Use descriptive node titles in Chinese when possible
5. Include at least one agent or llm node for processing
6. Node IDs should be unique and match in edges"#;

    let model_id = provider
        .models
        .iter()
        .find(|m| m.enabled)
        .map(|m| m.model_id.clone())
        .unwrap_or_else(|| "gpt-4".to_string());

    let request = ChatRequest {
        model: model_id.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(prompt.clone()),
                tool_calls: None,
                tool_call_id: None,
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
        .chat(&ctx, request)
        .await
        .map_err(|e| format!("LLM API error: {}", e))?;

    parse_llm_response(&prompt, &response.content)
}

#[tauri::command]
pub async fn optimize_agent_prompt(
    _state: State<'_, AppState>,
    prompt: String,
) -> Result<String, String> {
    let optimized = format!(
        r#"# Role
You are an expert AI assistant specialized in the task described below.

# Task
{}

# Requirements
1. Be clear and specific about goals and constraints
2. Break down complex tasks into manageable steps
3. Define clear success criteria
4. Consider edge cases and error handling
5. Specify any necessary tools or resources

# Output Format
Provide your response in a structured format with:
- Main content
- Supporting details
- Any relevant examples
"#,
        prompt
    );

    Ok(optimized)
}

#[tauri::command]
pub async fn recommend_nodes(
    _state: State<'_, AppState>,
    context: String,
) -> Result<Vec<NodeRecommendation>, String> {
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
    }

    recommendations.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    recommendations.truncate(5);

    Ok(recommendations)
}
