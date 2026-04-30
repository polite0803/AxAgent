use crate::AppState;
use axagent_core::entity::agency_experts;
use axagent_core::repo::provider::{self as provider_repo, get_active_key};
use axagent_core::repo::settings::get_settings;
use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};
use axagent_providers::registry::ProviderRegistry;
use axagent_providers::resolve_base_url_for_type;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportAgencyExpertsRequest {
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub count: u32,
    pub workflows_created: u32,
    pub tools_matched: u32,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgencyExpertRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub system_prompt: String,
    pub color: Option<String>,
    pub source_dir: String,
    pub is_enabled: bool,
    pub recommended_workflows: Option<Vec<String>>,
    pub recommended_tools: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendedWorkflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<String>,
    pub expert_role_id: String,
}

fn map_directory_to_category(dir: &str) -> &str {
    match dir {
        "engineering" => "development",
        "security" => "security",
        "data" | "finance" => "data",
        "devops" | "infrastructure" => "devops",
        "design" | "game-development" => "design",
        "marketing" | "writing" | "content" => "writing",
        "legal" | "hr" | "sales" | "product" | "project-management" | "strategy"
        | "supply-chain" => "business",
        _ => "general",
    }
}

fn map_color_to_category(color: &str) -> Option<&str> {
    match color.to_lowercase().as_str() {
        "purple" | "blue" => Some("development"),
        "red" => Some("security"),
        "green" => Some("data"),
        "orange" | "amber" => Some("business"),
        "teal" | "cyan" => Some("devops"),
        "pink" => Some("design"),
        _ => None,
    }
}

fn parse_frontmatter(content: &str) -> (String, String, String, String) {
    let mut name = String::new();
    let mut description = String::new();
    let mut color = String::new();

    if !content.starts_with("---") {
        return (name, description, color, content.to_string());
    }

    let rest = &content[3..];
    let end_idx = rest.find("\n---").unwrap_or(0);
    let frontmatter = &rest[..end_idx];
    let body = if end_idx > 0 {
        rest[end_idx + 4..].trim().to_string()
    } else {
        content.to_string()
    };

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            name = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("description:") {
            description = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("color:") {
            color = value.trim().to_string();
        }
    }

    (name, description, color, body)
}

/// Parse workflow steps from system prompt.
/// Looks for:
/// - Numbered lists (1. xxx 2. xxx)
/// - Section headers like "## 工作流程", "## 审查策略", "## 协作模式"
fn parse_workflow_from_prompt(prompt: &str, expert_id: &str) -> Vec<RecommendedWorkflow> {
    let mut workflows: Vec<RecommendedWorkflow> = Vec::new();
    let mut current_name = String::new();
    let mut current_desc = String::new();
    let mut current_steps: Vec<String> = Vec::new();
    let mut in_workflow_section = false;

    for line in prompt.lines() {
        let trimmed = line.trim();

        // Detect workflow section headers
        if trimmed.starts_with("## ") {
            let header = trimmed[3..].to_lowercase();
            if header.contains("工作流程")
                || header.contains("协作模式")
                || header.contains("审查策略")
                || header.contains("执行步骤")
                || header.contains("workflow")
            {
                // Save previous workflow if any
                if !current_steps.is_empty() {
                    workflows.push(RecommendedWorkflow {
                        name: if current_name.is_empty() {
                            "默认流程".to_string()
                        } else {
                            current_name.clone()
                        },
                        description: current_desc.clone(),
                        steps: current_steps.clone(),
                        expert_role_id: expert_id.to_string(),
                    });
                }
                current_name = trimmed[3..].to_string();
                current_desc = String::new();
                current_steps = Vec::new();
                in_workflow_section = true;
                continue;
            } else {
                if in_workflow_section && !current_steps.is_empty() {
                    workflows.push(RecommendedWorkflow {
                        name: current_name.clone(),
                        description: current_desc.clone(),
                        steps: current_steps.clone(),
                        expert_role_id: expert_id.to_string(),
                    });
                }
                in_workflow_section = false;
                current_name = String::new();
                current_desc = String::new();
                current_steps = Vec::new();
            }
        }

        if in_workflow_section {
            // Capture numbered steps: "1. xxx", "2. xxx"
            if let Some(rest) = trimmed
                .strip_prefix(|c: char| c.is_ascii_digit())
                .and_then(|s| s.strip_prefix(". "))
            {
                current_steps.push(rest.to_string());
            } else if let Some(rest) = trimmed.strip_prefix("- ") {
                if current_steps.is_empty() {
                    // First bullet might be the description
                    current_desc = rest.to_string();
                } else {
                    current_steps.push(rest.to_string());
                }
            }
        }
    }

    // Save last workflow
    if !current_steps.is_empty() {
        workflows.push(RecommendedWorkflow {
            name: current_name,
            description: current_desc,
            steps: current_steps,
            expert_role_id: expert_id.to_string(),
        });
    }

    // If no structured workflows found, try numbered-list fallback
    if workflows.is_empty() {
        let mut steps: Vec<String> = Vec::new();
        for line in prompt.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed
                .strip_prefix(|c: char| c.is_ascii_digit())
                .and_then(|s| s.strip_prefix(". "))
            {
                if steps.len() < 10 {
                    steps.push(rest.to_string());
                }
            }
        }
        if steps.len() >= 2 {
            workflows.push(RecommendedWorkflow {
                name: "推荐流程".to_string(),
                description: String::new(),
                steps,
                expert_role_id: expert_id.to_string(),
            });
        }
    }

    workflows
}

/// Parse tool references from system prompt.
/// Looks for patterns like: read_file, write_file, search_content, grep, browse, execute, etc.
fn parse_tools_from_prompt(prompt: &str) -> Vec<String> {
    let tool_patterns = [
        "read_file",
        "write_file",
        "edit_file",
        "delete_file",
        "search_content",
        "search_file",
        "grep",
        "glob",
        "execute_command",
        "bash",
        "shell",
        "terminal",
        "web_search",
        "web_fetch",
        "browse",
        "url",
        "list_files",
        "list_directory",
        "run_tests",
        "cargo",
        "npm",
        "git",
        "database",
        "sql",
        "query",
        "docker",
        "kubectl",
        "deploy",
        "agent",
        "task",
        "subagent",
    ];

    let prompt_lower = prompt.to_lowercase();
    let mut found: Vec<String> = Vec::new();
    let mut seen: HashMap<String, bool> = HashMap::new();

    for pattern in &tool_patterns {
        if prompt_lower.contains(pattern) && !seen.contains_key(*pattern) {
            found.push(pattern.to_string());
            seen.insert(pattern.to_string(), true);
        }
    }

    found
}

/// Generate workflow template IDs from parsed workflows.
/// Stores the steps in agency_experts.recommended_workflows as JSON.
/// Frontend can later create actual workflow templates from this data.
fn generate_workflow_ids(expert_id: &str, workflows: &[RecommendedWorkflow]) -> Vec<String> {
    workflows
        .iter()
        .enumerate()
        .filter(|(_, wf)| wf.steps.len() >= 2)
        .map(|(i, _)| format!("auto-{}-wf{}", expert_id, i))
        .collect()
}

#[tauri::command]
pub async fn import_agency_experts(
    state: State<'_, AppState>,
    request: ImportAgencyExpertsRequest,
) -> Result<ImportResult, String> {
    let db = &state.sea_db;
    let path = Path::new(&request.path);

    if !path.exists() || !path.is_dir() {
        return Err(format!("路径不存在或不是目录: {}", request.path));
    }

    let now = chrono::Utc::now().timestamp();
    let mut count: u32 = 0;
    let mut workflows_created: u32 = 0;
    let mut tools_matched: u32 = 0;
    let mut errors: Vec<String> = Vec::new();

    let entries = fs::read_dir(path).map_err(|e| format!("读取目录失败: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let dir_name = entry_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if dir_name.starts_with('.')
            || dir_name == "scripts"
            || dir_name == "examples"
            || dir_name == "integrations"
        {
            continue;
        }

        let category = map_directory_to_category(&dir_name);
        let md_files = fs::read_dir(&entry_path).map_err(|e| format!("读取目录失败: {}", e))?;

        for md_entry in md_files {
            let md_entry = md_entry.map_err(|e| format!("读取文件条目失败: {}", e))?;
            let md_path = md_entry.path();
            if md_path.extension().map_or(true, |ext| ext != "md") {
                continue;
            }

            let file_stem = md_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let content = match fs::read_to_string(&md_path) {
                Ok(c) => c,
                Err(e) => {
                    errors.push(format!("读取文件失败 {}: {}", md_path.display(), e));
                    continue;
                }
            };

            let (name, description, color, body) = parse_frontmatter(&content);
            let display_name = if name.is_empty() {
                file_stem
                    .strip_prefix(&format!("{}-", dir_name))
                    .unwrap_or(&file_stem)
                    .replace('-', " ")
            } else {
                name.clone()
            };

            if display_name.trim().is_empty() {
                continue;
            }

            let id = format!("agency-{}-{}", dir_name, file_stem);
            let final_category = map_color_to_category(&color)
                .unwrap_or(category)
                .to_string();

            // Parse workflows and tools
            let parsed_workflows = parse_workflow_from_prompt(&body, &id);
            let created_wf_ids = generate_workflow_ids(&id, &parsed_workflows);
            workflows_created += created_wf_ids.len() as u32;

            // Parse and match tools
            let parsed_tools = parse_tools_from_prompt(&body);
            tools_matched += parsed_tools.len() as u32;

            let recommended_workflows_json = if created_wf_ids.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&created_wf_ids).unwrap_or_default())
            };
            let recommended_tools_json = if parsed_tools.is_empty() {
                None
            } else {
                Some(serde_json::to_string(&parsed_tools).unwrap_or_default())
            };

            let model = agency_experts::ActiveModel {
                id: Set(id.clone()),
                name: Set(display_name),
                description: Set(if description.is_empty() {
                    None
                } else {
                    Some(description)
                }),
                category: Set(final_category),
                system_prompt: Set(body),
                color: Set(if color.is_empty() { None } else { Some(color) }),
                source_dir: Set(dir_name.clone()),
                is_enabled: Set(1),
                imported_at: Set(now),
                recommended_workflows: Set(recommended_workflows_json),
                recommended_tools: Set(recommended_tools_json),
            };

            match model.save(db).await {
                Ok(_) => count += 1,
                Err(e) => {
                    errors.push(format!("保存失败 {}: {}", id, e));
                }
            }
        }
    }

    Ok(ImportResult {
        count,
        workflows_created,
        tools_matched,
        errors,
    })
}

#[tauri::command]
pub async fn list_agency_experts(
    state: State<'_, AppState>,
) -> Result<Vec<AgencyExpertRow>, String> {
    let db = &state.sea_db;
    let models = agency_experts::Entity::find()
        .filter(agency_experts::Column::IsEnabled.eq(1))
        .all(db)
        .await
        .map_err(|e| format!("查询失败: {}", e))?;

    let rows: Vec<AgencyExpertRow> = models
        .into_iter()
        .map(|m| AgencyExpertRow {
            id: m.id,
            name: m.name,
            description: m.description,
            category: m.category,
            system_prompt: m.system_prompt,
            color: m.color,
            source_dir: m.source_dir,
            is_enabled: m.is_enabled != 0,
            recommended_workflows: m
                .recommended_workflows
                .and_then(|s| serde_json::from_str(&s).ok()),
            recommended_tools: m
                .recommended_tools
                .and_then(|s| serde_json::from_str(&s).ok()),
        })
        .collect();

    Ok(rows)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedWorkflowStep {
    pub title: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub estimated_tools: Vec<String>,
    pub depends_on: Vec<String>,
    /** Condition for executing this step (optional) */
    pub condition: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedWorkflow {
    pub name: String,
    pub description: String,
    pub steps: Vec<ExtractedWorkflowStep>,
    pub parallel_groups: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractExpertStructureRequest {
    pub expert_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractExpertStructureResult {
    pub workflows: Vec<ExtractedWorkflow>,
    pub tools: Vec<String>,
}

fn extract_json_from_text(text: &str) -> Result<serde_json::Value, String> {
    if let Some(start) = text.find("```json") {
        let body = &text[start + 7..];
        if let Some(end) = body.find("```") {
            return serde_json::from_str(&body[..end])
                .map_err(|e| format!("JSON parse error: {}", e));
        }
    }
    if let Some(start) = text.find('{') {
        let trimmed = &text[start..];
        if let Some(end) = trimmed.rfind('}') {
            return serde_json::from_str(&trimmed[..end + 1])
                .map_err(|e| format!("JSON parse error: {}", e));
        }
    }
    Err("No JSON found in response".to_string())
}

#[tauri::command]
pub async fn extract_expert_structure(
    state: State<'_, AppState>,
    request: ExtractExpertStructureRequest,
) -> Result<ExtractExpertStructureResult, String> {
    let db = &state.sea_db;

    // Load the expert
    let expert = agency_experts::Entity::find_by_id(&request.expert_id)
        .one(db)
        .await
        .map_err(|e| format!("查询失败: {}", e))?
        .ok_or_else(|| format!("专家不存在: {}", request.expert_id))?;

    // Get default provider/model from settings
    let settings = get_settings(db)
        .await
        .map_err(|e| format!("加载设置失败: {}", e))?;
    let provider_id = settings.default_provider_id.ok_or("未配置默认模型供应商")?;
    let model_id = settings.default_model_id.ok_or("未配置默认模型")?;

    // Load provider
    let provider_config = provider_repo::get_provider(db, &provider_id)
        .await
        .map_err(|e| format!("加载供应商失败: {}", e))?;
    let key_row = get_active_key(db, &provider_id)
        .await
        .map_err(|e| format!("无活跃密钥: {}", e))?;
    let api_key = axagent_core::crypto::decrypt_key(&key_row.key_encrypted, &state.master_key)
        .map_err(|e| format!("密钥解密失败: {}", e))?;

    let registry_key = format!("{:?}", provider_config.provider_type).to_lowercase();
    let registry = ProviderRegistry::create_default();
    let adapter = registry
        .get(&registry_key)
        .ok_or_else(|| format!("未找到供应商适配器: {}", registry_key))?;

    let ctx = axagent_providers::ProviderRequestContext {
        api_key,
        key_id: key_row.id.clone(),
        provider_id: provider_id.clone(),
        base_url: Some(resolve_base_url_for_type(
            &provider_config.api_host,
            &provider_config.provider_type,
        )),
        api_path: provider_config.api_path.clone(),
        proxy_config: provider_config.proxy_config.clone(),
        custom_headers: provider_config
            .custom_headers
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok()),
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let system_prompt = r#"你是一个工作流结构化提取器。根据专家的系统提示词，提取该专家的完整工作流定义。

输出 JSON 格式，必须包含以下字段：
{
  "workflows": [{
    "name": "工作流名称",
    "description": "工作流描述",
    "steps": [{
      "title": "步骤标题",
      "description": "该步骤做什么",
      "inputs": ["需要的输入"],
      "outputs": ["产出的输出"],
      "estimated_tools": ["可能用到的工具"],
      "depends_on": ["依赖的前置步骤ID"],
      "condition": "执行该步骤的条件（可选，没有则为null）"
    }],
    "parallel_groups": [["可并行的步骤ID列表"]]
  }],
  "tools": ["推荐工具列表"]
}

规则：
1. 步骤间有依赖关系的填 depends_on，独立的留空
2. 可以从系统提示词的"工作流程""审查策略""协作模式"等章节提取
3. 如果没有明确结构，根据经验推断典型的工作流
4. 只输出 JSON，不要其他文字"#;

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: ChatContent::Text(system_prompt.to_string()),
            tool_calls: None,
            tool_call_id: None,
        },
        ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Text(format!(
                "专家名称: {}\n专家描述: {}\n系统提示词:\n{}",
                expert.name,
                expert.description.unwrap_or_default(),
                expert.system_prompt
            )),
            tool_calls: None,
            tool_call_id: None,
        },
    ];

    let chat_request = ChatRequest {
        model: model_id.to_string(),
        messages,
        stream: false,
        temperature: Some(0.2),
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

    let response = adapter
        .chat(&ctx, chat_request)
        .await
        .map_err(|e| format!("LLM调用失败: {}", e))?;

    let extracted = extract_json_from_text(&response.content).map_err(|e| {
        let preview = &response.content[..200.min(response.content.len())];
        format!("JSON解析失败: {}. 响应预览: {}", e, preview)
    })?;

    let workflows: Vec<ExtractedWorkflow> =
        serde_json::from_value(extracted["workflows"].clone()).unwrap_or_default();

    let tools: Vec<String> = extracted["tools"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Ok(ExtractExpertStructureResult { workflows, tools })
}

#[tauri::command]
pub async fn clear_agency_experts(state: State<'_, AppState>) -> Result<ImportResult, String> {
    let db = &state.sea_db;
    let result = agency_experts::Entity::delete_many()
        .exec(db)
        .await
        .map_err(|e| format!("删除失败: {}", e))?;

    Ok(ImportResult {
        count: result.rows_affected as u32,
        workflows_created: 0,
        tools_matched: 0,
        errors: vec![],
    })
}

#[derive(Debug, Deserialize)]
pub struct UpdateExpertRequest {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub system_prompt: Option<String>,
    pub is_enabled: Option<bool>,
}

#[tauri::command]
pub async fn update_agency_expert(
    state: State<'_, AppState>,
    request: UpdateExpertRequest,
) -> Result<(), String> {
    let db = &state.sea_db;

    let expert = agency_experts::Entity::find_by_id(&request.id)
        .one(db)
        .await
        .map_err(|e| format!("查询失败: {}", e))?
        .ok_or_else(|| format!("专家不存在: {}", request.id))?;

    let mut am: agency_experts::ActiveModel = expert.into();

    if let Some(name) = request.name {
        am.name = Set(name);
    }
    if let Some(desc) = request.description {
        am.description = Set(Some(desc));
    }
    if let Some(cat) = request.category {
        am.category = Set(cat);
    }
    if let Some(sp) = request.system_prompt {
        am.system_prompt = Set(sp);
    }
    if let Some(enabled) = request.is_enabled {
        am.is_enabled = Set(if enabled { 1 } else { 0 });
    }

    am.update(db)
        .await
        .map_err(|e| format!("更新失败: {}", e))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct DeleteExpertRequest {
    pub id: String,
}

#[tauri::command]
pub async fn delete_agency_expert(
    state: State<'_, AppState>,
    request: DeleteExpertRequest,
) -> Result<(), String> {
    let db = &state.sea_db;
    agency_experts::Entity::delete_by_id(&request.id)
        .exec(db)
        .await
        .map_err(|e| format!("删除失败: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn export_agency_experts(state: State<'_, AppState>) -> Result<String, String> {
    let db = &state.sea_db;
    let models = agency_experts::Entity::find()
        .filter(agency_experts::Column::IsEnabled.eq(1))
        .all(db)
        .await
        .map_err(|e| format!("查询失败: {}", e))?;

    let rows: Vec<AgencyExpertRow> = models
        .into_iter()
        .map(|m| AgencyExpertRow {
            id: m.id,
            name: m.name,
            description: m.description,
            category: m.category,
            system_prompt: m.system_prompt,
            color: m.color,
            source_dir: m.source_dir,
            is_enabled: m.is_enabled != 0,
            recommended_workflows: m
                .recommended_workflows
                .and_then(|s| serde_json::from_str(&s).ok()),
            recommended_tools: m
                .recommended_tools
                .and_then(|s| serde_json::from_str(&s).ok()),
        })
        .collect();

    serde_json::to_string_pretty(&rows).map_err(|e| format!("序列化失败: {}", e))
}
