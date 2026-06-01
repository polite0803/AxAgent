use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::provider as provider_err;
use axagent_core::crypto::decrypt_key;
use axagent_core::types::{
    ChatContent, ChatMessage, ChatRequest, ChatStreamChunk, ChatStreamErrorEvent, ChatStreamEvent,
    ProviderType,
};
use axagent_core::workflow_types::*;
use axagent_providers::registry::ProviderRegistry;
use axagent_providers::{ProviderRequestContext, resolve_base_url_for_type};
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
        .filter(|id| !has_parent.contains(id))
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
    let positions = layout_workflow_nodes(&node_ids, &edge_pairs);

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
            id: node_id.clone(),
            title: llm_node.title.clone(),
            description: llm_node.description.clone(),
            position,
            enabled: true,
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
    let registry = ProviderRegistry::create_default();
    let registry_key = provider_type_to_registry_key(&resolved.provider_type);
    let adapter = registry.get(registry_key).ok_or_else(|| {
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

Output a valid JSON object with this structure:
{{
  "nodes": [
    {{
      "id": "node-1",
      "node_type": "trigger|agent|llm|condition|parallel|loop|merge|delay|tool|code|subWorkflow|documentParser|vectorRetrieve|validation|end",
      "title": "Node Title",
      "description": "Optional description",
      "config": {{}} // Node-specific configuration
    }}
  ],
  "edges": [
    {{
      "id": "edge-1",
      "source": "node-1",
      "target": "node-2",
      "edge_type": "direct|conditionTrue|conditionFalse|loopBack|parallelBranch"
    }}
  ],
  "explanation": "Brief explanation of the generated workflow"
}}

Rules:
1. Always start with a trigger node
2. Always end with an end node
3. For condition nodes, use edge_type "conditionTrue" or "conditionFalse"
4. Use descriptive node titles in Chinese when possible
5. Include at least one agent or llm node for processing
6. Node IDs should be unique and match in edges
7. Use merge nodes to combine parallel branches back together
8. Use delay nodes when waiting is needed between steps
9. Use subWorkflow nodes to invoke other workflows
10. Use documentParser nodes for document extraction
11. Use vectorRetrieve nodes for knowledge base search
12. Use validation nodes for data quality checks{context_section}"#
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
    let registry = ProviderRegistry::create_default();
    let registry_key = provider_type_to_registry_key(&resolved.provider_type);
    let adapter = registry.get(registry_key).ok_or_else(|| {
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
    let registry = ProviderRegistry::create_default();
    let registry_key = provider_type_to_registry_key(&resolved.provider_type);
    let adapter = registry.get(registry_key).ok_or_else(|| {
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
    let registry = ProviderRegistry::create_default();
    let registry_key = provider_type_to_registry_key(&resolved.provider_type);
    let adapter = registry.get(registry_key).ok_or_else(|| {
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

You can:
1. Generate complete workflows based on descriptions
2. Modify existing workflows (add/remove nodes, change connections)
3. Optimize agent prompts
4. Recommend node types
5. Explain workflow concepts

When you want to perform an action on the workflow, include a special action block in your response using this format:
:::action
{{"action_type": "generate_workflow", "data": {{"nodes": [...], "edges": [...]}}}}
:::

:::action
{{"action_type": "add_nodes", "data": {{"nodes": [...]}}}}
:::

:::action
{{"action_type": "modify_node", "data": {{"node_id": "...", "changes": {{}}}}}}
:::

:::action
{{"action_type": "optimize_prompt", "data": {{"node_id": "...", "optimized_prompt": "..."}}}}
:::

:::action
{{"action_type": "delete_nodes", "data": {{"node_ids": [...]}}}}
:::

You can include multiple action blocks in a single response. Always explain what you're doing before the action block.
For generate_workflow and add_nodes, use the same node/edge JSON format as the workflow schema.
Respond in the same language as the user's message.{}"#,
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
