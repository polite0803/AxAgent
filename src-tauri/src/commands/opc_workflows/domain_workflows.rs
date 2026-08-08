// SPDX-License-Identifier: AGPL-3.0-only

//! 领域工作流命令 — 暴露 DomainAdapterFactory 的能力给前端

use agent_macro::agent_command;
use axagent_analysis_engine::opc::domain::{DomainAdapterFactory, DomainWorkflowDef};
use serde::Serialize;

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
    domain_id: Option<String>,
    tag: Option<String>,
) -> Result<Vec<DomainWorkflowSummary>, String> {
    let all = DomainAdapterFactory::create_all();

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
                if !wf.tags.iter().any(|tag_item| tag_item == t) {
                    return false;
                }
            }
            true
        })
        .map(|wf| {
            let domain = extract_domain_from_id(&wf.id);
            DomainWorkflowSummary {
                id: wf.id.clone(),
                name: wf.name.clone(),
                description: wf.description.clone(),
                icon: wf.icon.clone(),
                tags: wf.tags.clone(),
                domain: domain.to_string(),
                step_count: wf.steps.len(),
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
    workflow_id: String,
) -> Result<Option<DomainWorkflowDef>, String> {
    let wf = DomainAdapterFactory::create(&workflow_id);
    Ok(wf)
}

/// 列出所有领域元数据（用于前端下拉选择）
#[tauri::command]
#[agent_command(
    domain = "OPC",
    safety = Safe,
    call_mode = StateOnly,
    description = "列出所有领域 ID 和名称"
)]
pub async fn opc_list_domains() -> Result<Vec<(String, String)>, String> {
    let domains = DomainAdapterFactory::list_all();
    Ok(domains.into_iter().map(|(id, name)| (id.to_string(), name.to_string())).collect())
}

/// 从工作流 ID 提取领域标识
fn extract_domain_from_id(id: &str) -> &str {
    // wf-acd-xxx -> acd
    // wf-eng-xxx -> eng
    if let Some(rest) = id.strip_prefix("wf-") {
        if let Some(domain) = rest.split('-').next() {
            return domain;
        }
    }
    "unknown"
}
