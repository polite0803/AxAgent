// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务领域 Tauri 命令层
//!
//! 将 opc-dao 的 Service 暴露给前端 React 调用。

use std::str::FromStr;

use crate::AppState;
use tauri::State;

use axagent_opc_dao::{
    DefaultAnalyticsService, DefaultCustomerService, DefaultFinanceService,
    DefaultInvoiceService, DefaultProjectService, DefaultSiteService,
};
use axagent_opc_types::{
    AnalyticsService, CreateCustomerInput, CreateInvoiceInput, CreateProjectInput, CustomerFilter,
    CustomerService, FinanceService, InvoiceFilter, InvoiceService, InvoiceStatus, ProjectFilter,
    ProjectService, SiteService, UpdateCustomerInput, UpdateInvoiceInput, UpdateProjectInput,
};

/// 记录 OPC 操作轨迹到 trajectory 系统供学习
async fn record_opc_trajectory(
    storage: &axagent_trajectory::TrajectoryStorage,
    op_name: &str,
    entity_id: &str,
    outcome: axagent_trajectory::TrajectoryOutcome,
    details: &str,
) {
    let now = chrono::Utc::now();
    let step = axagent_trajectory::TrajectoryStep {
        timestamp_ms: now.timestamp_millis() as u64,
        role: axagent_trajectory::MessageRole::User,
        content: format!("OPC:{op_name} id={entity_id} {details}"),
        reasoning: None,
        tool_calls: Some(vec![axagent_trajectory::ToolCall {
            id: axagent_harness::util_fns::gen_id(),
            name: format!("opc_{op_name}"),
            arguments: serde_json::json!({"id": entity_id, "details": details}).to_string(),
        }]),
        tool_results: Some(vec![axagent_trajectory::TrajectoryToolResult {
            tool_use_id: String::new(),
            tool_name: format!("opc_{op_name}"),
            output: format!("OPC {op_name} completed: {entity_id}"),
            is_error: outcome == axagent_trajectory::TrajectoryOutcome::Failure,
        }]),
    };

    let trajectory = axagent_trajectory::Trajectory::new(
        format!("opc_session"),
        "system".to_string(),
        format!("OPC:{op_name}"),
        format!("{op_name}: {entity_id}"),
        outcome,
        0u64,
        vec![step],
    );

    if let Err(e) = storage.save_trajectory(&trajectory).await {
        tracing::warn!("[opc-trajectory] Failed to save: {e}");
    }
}

/// 成功轨迹的简便封装
async fn record_opc_success(storage: &axagent_trajectory::TrajectoryStorage, op: &str, id: &str, details: &str) {
    record_opc_trajectory(storage, op, id, axagent_trajectory::TrajectoryOutcome::Success, details).await;
}

/// 失败轨迹的简便封装
#[allow(dead_code)]
async fn record_opc_failure(storage: &axagent_trajectory::TrajectoryStorage, op: &str, id: &str, err: &str) {
    record_opc_trajectory(storage, op, id, axagent_trajectory::TrajectoryOutcome::Failure, err).await;
}

// ── Invoice Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn opc_create_invoice(
    state: State<'_, AppState>,
    input: CreateInvoiceInput,
) -> Result<axagent_opc_types::Invoice, String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    let result = svc.create_invoice(input).await.map_err(|e| e.to_string());
    if let Ok(ref inv) = result {
        record_opc_success(&state.trajectory_storage, "create_invoice", &inv.id, &format!("amount={}", inv.total)).await;
    }
    result
}

#[tauri::command]
pub async fn opc_get_invoice(
    state: State<'_, AppState>,
    id: String,
) -> Result<axagent_opc_types::Invoice, String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    svc.get_invoice(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_list_invoices(
    state: State<'_, AppState>,
    filter: InvoiceFilter,
) -> Result<Vec<axagent_opc_types::Invoice>, String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    svc.list_invoices(filter).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_update_invoice(
    state: State<'_, AppState>,
    id: String,
    input: UpdateInvoiceInput,
) -> Result<axagent_opc_types::Invoice, String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    svc.update_invoice(&id, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_delete_invoice(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    svc.delete_invoice(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_transition_invoice(
    state: State<'_, AppState>,
    id: String,
    target_status: String,
) -> Result<axagent_opc_types::Invoice, String> {
    let status = InvoiceStatus::from_str(&target_status)
        .map_err(|_| format!("invalid invoice status: {target_status}"))?;
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    let result = svc.transition_status(&id, status).await.map_err(|e| e.to_string());
    if let Ok(ref inv) = result {
        record_opc_success(&state.trajectory_storage, "transition_invoice", &inv.id, &format!("status={}", inv.status.as_str())).await;
    }
    result
}

// ── Customer Commands ─────────────────────────────────────────────

#[tauri::command]
pub async fn opc_create_customer(
    state: State<'_, AppState>,
    input: CreateCustomerInput,
) -> Result<axagent_opc_types::Customer, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    let result = svc.create_customer(input).await.map_err(|e| e.to_string());
    if let Ok(ref c) = result {
        record_opc_success(&state.trajectory_storage, "create_customer", &c.id, &format!("name={}", c.name)).await;
    }
    result
}

#[tauri::command]
pub async fn opc_get_customer(
    state: State<'_, AppState>,
    id: String,
) -> Result<axagent_opc_types::Customer, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.get_customer(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_list_customers(
    state: State<'_, AppState>,
    filter: CustomerFilter,
) -> Result<Vec<axagent_opc_types::Customer>, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.list_customers(filter).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_update_customer(
    state: State<'_, AppState>,
    id: String,
    input: UpdateCustomerInput,
) -> Result<axagent_opc_types::Customer, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.update_customer(&id, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_delete_customer(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.delete_customer(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_find_customer_by_email(
    state: State<'_, AppState>,
    email: String,
) -> Result<Option<axagent_opc_types::Customer>, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.find_by_email(&email).await.map_err(|e| e.to_string())
}

// ── Project Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn opc_create_project(
    state: State<'_, AppState>,
    input: CreateProjectInput,
) -> Result<axagent_opc_types::Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    let result = svc.create_project(input).await.map_err(|e| e.to_string());
    if let Ok(ref p) = result {
        record_opc_success(&state.trajectory_storage, "create_project", &p.id, &format!("title={}", p.title)).await;
    }
    result
}

#[tauri::command]
pub async fn opc_get_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<axagent_opc_types::Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.get_project(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_list_projects(
    state: State<'_, AppState>,
    filter: ProjectFilter,
) -> Result<Vec<axagent_opc_types::Project>, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.list_projects(filter).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_update_project(
    state: State<'_, AppState>,
    id: String,
    input: UpdateProjectInput,
) -> Result<axagent_opc_types::Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.update_project(&id, input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_delete_project(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.delete_project(&id).await.map_err(|e| e.to_string())
}

// ── Site / Landing Page Commands ────────────────────────────────────

#[tauri::command]
pub async fn opc_create_landing_page(
    state: State<'_, AppState>,
    input: axagent_opc_types::CreateLandingPageInput,
) -> Result<axagent_opc_types::LandingPage, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.create_landing_page(input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_list_landing_pages(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_opc_types::LandingPage>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.list_landing_pages().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_publish_landing_page(
    state: State<'_, AppState>,
    id: String,
) -> Result<axagent_opc_types::LandingPage, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.publish_landing_page(&id).await.map_err(|e| e.to_string())
}

// ── Blog Post Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn opc_create_blog_post(
    state: State<'_, AppState>,
    input: axagent_opc_types::CreateBlogPostInput,
) -> Result<axagent_opc_types::BlogPost, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.create_blog_post(input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_list_blog_posts(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_opc_types::BlogPost>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.list_blog_posts().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_publish_blog_post(
    state: State<'_, AppState>,
    id: String,
) -> Result<axagent_opc_types::BlogPost, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.publish_blog_post(&id).await.map_err(|e| e.to_string())
}

// ── Contact Commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn opc_list_contacts(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_opc_types::ContactSubmission>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.list_contacts().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_mark_contact_read(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.mark_contact_read(&id).await.map_err(|e| e.to_string())
}

// ── Analytics Commands ──────────────────────────────────────────────

#[tauri::command]
pub async fn opc_record_kpi(
    state: State<'_, AppState>,
    input: axagent_opc_types::CreateKpiInput,
) -> Result<axagent_opc_types::KpiRecord, String> {
    let svc = DefaultAnalyticsService::new(state.harness.db().clone());
    svc.record_kpi(input).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_list_kpis(
    state: State<'_, AppState>,
    period: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<axagent_opc_types::KpiRecord>, String> {
    let svc = DefaultAnalyticsService::new(state.harness.db().clone());
    svc.list_kpis(period, limit).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_list_revenue(
    state: State<'_, AppState>,
    category: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<axagent_opc_types::RevenueRecord>, String> {
    let svc = DefaultAnalyticsService::new(state.harness.db().clone());
    svc.list_revenue(category, limit).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_get_dashboard_summary(
    state: State<'_, AppState>,
) -> Result<axagent_opc_types::DashboardSummary, String> {
    let svc = DefaultAnalyticsService::new(state.harness.db().clone());
    svc.get_dashboard_summary().await.map_err(|e| e.to_string())
}

// ── Finance Commands ────────────────────────────────────────────────

#[tauri::command]
pub async fn opc_get_financial_report(
    state: State<'_, AppState>,
    period: String,
) -> Result<axagent_opc_types::FinancialReport, String> {
    let svc = DefaultFinanceService::new(state.harness.db().clone());
    svc.get_financial_report(&period).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_get_investment_advice(
    state: State<'_, AppState>,
    period: String,
) -> Result<axagent_opc_types::InvestmentAdvice, String> {
    let svc = DefaultFinanceService::new(state.harness.db().clone());
    let report = svc.get_financial_report(&period).await.map_err(|e| e.to_string())?;
    Ok(svc.get_investment_advice(&report).await)
}
