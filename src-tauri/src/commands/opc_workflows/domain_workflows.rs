// SPDX-License-Identifier: AGPL-3.0-only

//! 领域工作流命令 — 从数据库读取种子化的领域工作流
//!
//! 所有领域工作流已迁移到各领域 seed 文件手动定义 WorkflowNode/Edge 并写入数据库。
//! 本模块从 DB 读取。旧代码 DomainAdapterFactory/DomainWorkflowGenerator 已彻底移除。

use axagent_agent_macro::agent_command;
use axagent_entities::workflow_template;
use axagent_harness::workflow_types::WorkflowTemplateResponse;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use tauri::State;

/// 领域工作流摘要（用于前端展示）
#[derive(Debug, Clone, Serialize)]
pub struct DomainWorkflowSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub tags: Vec<String>,
    pub domain: String,
    pub step_count: usize,
}

/// 列出所有领域工作流（可按领域 ID 或标签过滤）
#[tauri::command]
#[agent_command(
    domain = "OPC",
    safety = Safe,
    call_mode = StateOnly,
    description = "列出所有领域工作流，支持按领域或标签过滤"
)]
pub async fn opc_list_domain_workflows(
    state: State<'_, crate::AppState>,
    domain_id: Option<String>,
    tag: Option<String>,
) -> Result<Vec<DomainWorkflowSummary>, String> {
    let db = state.harness.db();

    // 查询所有以 "wf-" 开头的工作流模板（领域工作流）
    let all = workflow_template::Entity::find()
        .filter(workflow_template::Column::Id.starts_with("wf-"))
        .all(db)
        .await
        .map_err(|e| format!("查询领域工作流: {e}"))?;

    let summaries: Vec<DomainWorkflowSummary> = all
        .iter()
        .filter(|wf| {
            // 按领域 ID 过滤（从工作流 ID 推断）
            if let Some(ref did) = domain_id {
                let wf_domain = extract_domain_from_id(&wf.id);
                if wf_domain != did.as_str() {
                    return false;
                }
            }
            // 按标签过滤
            if let Some(ref t) = tag {
                let tags = parse_tags(&wf.tags);
                if !tags.iter().any(|tag_item| tag_item == t) {
                    return false;
                }
            }
            true
        })
        .map(|wf| {
            let domain = extract_domain_from_id(&wf.id);
            let tags = parse_tags(&wf.tags);
            let step_count = parse_nodes_len(&wf.nodes).unwrap_or(0);
            DomainWorkflowSummary {
                id: wf.id.clone(),
                name: wf.name.clone(),
                description: wf.description.clone().unwrap_or_default(),
                icon: wf.icon.clone(),
                tags,
                domain: domain.to_string(),
                step_count,
            }
        })
        .collect();

    Ok(summaries)
}

/// 获取指定领域工作流的详细定义
#[tauri::command]
#[agent_command(
    domain = "OPC",
    safety = Safe,
    call_mode = StateOnly,
    description = "获取指定领域工作流的完整定义"
)]
pub async fn opc_get_domain_workflow(
    state: State<'_, crate::AppState>,
    workflow_id: String,
) -> Result<Option<WorkflowTemplateResponse>, String> {
    let db = state.harness.db();

    let model = workflow_template::Entity::find_by_id(&workflow_id)
        .one(db)
        .await
        .map_err(|e| format!("查询工作流: {e}"))?;

    match model {
        Some(m) => Ok(Some(model_to_response(m)?)),
        None => Ok(None),
    }
}

/// 列出所有领域元数据（用于前端下拉选择）
#[tauri::command]
#[agent_command(
    domain = "OPC",
    safety = Safe,
    call_mode = StateOnly,
    description = "列出所有领域 ID 和名称"
)]
pub async fn opc_list_domains(
    state: State<'_, crate::AppState>,
) -> Result<Vec<(String, String)>, String> {
    let db = state.harness.db();

    // 从 DB 查询唯一的领域标识（从工作流 ID 前缀提取）
    let all = workflow_template::Entity::find()
        .filter(workflow_template::Column::Id.starts_with("wf-"))
        .all(db)
        .await
        .map_err(|e| format!("查询领域: {e}"))?;

    let mut domains: Vec<(String, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for wf in &all {
        let domain_id = extract_domain_from_id(&wf.id).to_string();
        if seen.insert(domain_id.clone()) {
            // 使用该领域第一个工作流的 name 作为领域展示名的一部分
            // 实际领域名称由领域映射函数提供
            let domain_name = domain_id_to_name(&domain_id);
            domains.push((domain_id, domain_name));
        }
    }

    // 排序保持稳定顺序
    domains.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(domains)
}

// ── 辅助函数 ─────────────────────────────────────────────────

/// 从工作流 ID 提取领域标识
/// wf-acd-xxx → "acd"
fn extract_domain_from_id(id: &str) -> &str {
    if let Some(rest) = id.strip_prefix("wf-") {
        if let Some(domain) = rest.split('-').next() {
            return domain;
        }
    }
    "unknown"
}

/// 领域 ID 到中文名称映射
fn domain_id_to_name(id: &str) -> String {
    match id {
        "academic" => "学术研究".to_string(),
        "design" => "设计与创意".to_string(),
        "engineering" => "工程与开发".to_string(),
        "finance" => "财务与会计".to_string(),
        "gamedev" => "游戏开发".to_string(),
        "gis" => "地理信息系统".to_string(),
        "marketing" => "市场营销".to_string(),
        "paidmedia" => "付费媒体".to_string(),
        "pm" => "项目管理".to_string(),
        "product" => "产品管理".to_string(),
        "sales" => "销售与商务".to_string(),
        "security" => "安全与合规".to_string(),
        "spatial" => "空间计算".to_string(),
        "specialized" => "专业服务".to_string(),
        "strategy" => "战略规划".to_string(),
        "support" => "客户支持".to_string(),
        "testing" => "测试与质量".to_string(),
        _ => id.to_string(),
    }
}

/// 解析 tags JSON 字符串
fn parse_tags(tags: &Option<String>) -> Vec<String> {
    tags.as_ref().and_then(|s| serde_json::from_str::<Vec<String>>(s).ok()).unwrap_or_default()
}

/// 解析 nodes JSON 字符串，返回节点的数量
fn parse_nodes_len(nodes_json: &str) -> Option<usize> {
    serde_json::from_str::<Vec<serde_json::Value>>(nodes_json).ok().map(|v| v.len())
}

/// 将 DB Model 转为 WorkflowTemplateResponse
fn model_to_response(m: workflow_template::Model) -> Result<WorkflowTemplateResponse, String> {
    let nodes: Vec<axagent_harness::workflow_types::WorkflowNode> =
        serde_json::from_str(&m.nodes).map_err(|e| format!("解析 nodes: {e}"))?;
    let edges: Vec<axagent_harness::workflow_types::WorkflowEdge> =
        serde_json::from_str(&m.edges).map_err(|e| format!("解析 edges: {e}"))?;
    let tags: Vec<String> =
        m.tags.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
    let is_system = m.is_preset && tags.iter().any(|t| t == "cognitive_router");
    let variables: Vec<axagent_harness::workflow_types::Variable> =
        m.variables.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
    let tool_defs: Option<Vec<axagent_harness::workflow_types::RhaiToolDef>> =
        m.tool_defs.as_deref().and_then(|s| serde_json::from_str(s).ok());
    let trigger_config: Option<axagent_harness::workflow_types::TriggerConfig> =
        m.trigger_config.as_deref().and_then(|s| serde_json::from_str(s).ok());
    let input_schema: Option<axagent_harness::workflow_types::JsonSchema> =
        m.input_schema.as_deref().and_then(|s| serde_json::from_str(s).ok());
    let output_schema: Option<axagent_harness::workflow_types::JsonSchema> =
        m.output_schema.as_deref().and_then(|s| serde_json::from_str(s).ok());
    let error_config: Option<axagent_harness::workflow_types::ErrorConfig> =
        m.error_config.as_deref().and_then(|s| serde_json::from_str(s).ok());

    Ok(WorkflowTemplateResponse {
        id: m.id,
        name: m.name,
        description: m.description,
        icon: m.icon,
        cluster_id: m.cluster_id,
        route_path: m.route_path,
        tags,
        version: m.version,
        is_preset: m.is_preset,
        is_editable: m.is_editable,
        is_public: m.is_public,
        is_system,
        trigger_config,
        nodes,
        edges,
        input_schema,
        output_schema,
        variables,
        error_config,
        tool_defs,
        mission_hash: m.mission_hash,
        created_at: m.created_at,
        updated_at: m.updated_at,
    })
}
