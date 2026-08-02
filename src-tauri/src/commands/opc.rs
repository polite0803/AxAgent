// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务领域 Tauri 命令层
//!
//! 将 opc-dao 的 Service 暴露给前端 React 调用。

use std::str::FromStr;

use crate::AppState;
use sea_orm::ActiveModelTrait;
use tauri::State;

use axagent_opc_dao::{
    DefaultAnalyticsService, DefaultCustomerService, DefaultFinanceService, DefaultInvoiceService,
    DefaultProjectService, DefaultSiteService,
};
use axagent_opc_types::{
    AnalyticsService, CreateCustomerInput, CreateInvoiceInput, CreateProjectInput, CustomerFilter,
    CustomerService, FinanceService, InvoiceFilter, InvoiceService, InvoiceStatus, Milestone,
    ProjectFilter, ProjectService, SiteService, UpdateCustomerInput, UpdateInvoiceInput,
    UpdateProjectInput,
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
async fn record_opc_success(
    storage: &axagent_trajectory::TrajectoryStorage,
    op: &str,
    id: &str,
    details: &str,
) {
    record_opc_trajectory(storage, op, id, axagent_trajectory::TrajectoryOutcome::Success, details)
        .await;
}

/// 失败轨迹的简便封装
#[allow(dead_code)]
async fn record_opc_failure(
    storage: &axagent_trajectory::TrajectoryStorage,
    op: &str,
    id: &str,
    err: &str,
) {
    record_opc_trajectory(storage, op, id, axagent_trajectory::TrajectoryOutcome::Failure, err)
        .await;
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
        record_opc_success(
            &state.trajectory_storage,
            "create_invoice",
            &inv.id,
            &format!("amount={}", inv.total),
        )
        .await;
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
pub async fn opc_delete_invoice(state: State<'_, AppState>, id: String) -> Result<(), String> {
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
        record_opc_success(
            &state.trajectory_storage,
            "transition_invoice",
            &inv.id,
            &format!("status={}", inv.status.as_str()),
        )
        .await;
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
        record_opc_success(
            &state.trajectory_storage,
            "create_customer",
            &c.id,
            &format!("name={}", c.name),
        )
        .await;
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
pub async fn opc_delete_customer(state: State<'_, AppState>, id: String) -> Result<(), String> {
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
        record_opc_success(
            &state.trajectory_storage,
            "create_project",
            &p.id,
            &format!("title={}", p.title),
        )
        .await;
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
pub async fn opc_delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.delete_project(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_add_milestone(
    state: State<'_, AppState>,
    project_id: String,
    milestone: Milestone,
) -> Result<axagent_opc_types::Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.add_milestone(&project_id, milestone).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn opc_complete_milestone(
    state: State<'_, AppState>,
    project_id: String,
    milestone_id: String,
) -> Result<axagent_opc_types::Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.complete_milestone(&project_id, &milestone_id).await.map_err(|e| e.to_string())
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
pub async fn opc_mark_contact_read(state: State<'_, AppState>, id: String) -> Result<(), String> {
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

// ── Industry Pack .opcip 导出/导入 ──────────────────────────────

/// 导出行业包为 .opcip 归档（打包 manifest + workflows）。
/// out_dir 为前端选择的保存目录（通过对话框）。
#[tauri::command]
pub async fn opc_export_industry_pack(
    state: State<'_, AppState>,
    id: String,
    out_dir: String,
) -> Result<String, String> {
    // 包源：app_dir/config/opc/industries（生产）或仓库根（开发）
    let app_dir = &state.app_data_dir;
    let base = crate::commands::opc_workflows::resolve_industries_dir(Some(app_dir));
    let out_path = std::path::PathBuf::from(&out_dir);
    crate::commands::opc_workflows::export_industry_pack(&base, &id, &out_path).await
}

/// 导入 .opcip 行业包：解包到 app_dir/config/opc/industries/ 并注册 seed。
/// archive_path 为前端选择的 .opcip 文件路径。
#[tauri::command]
pub async fn opc_import_industry_pack(
    state: State<'_, AppState>,
    archive_path: String,
) -> Result<String, String> {
    let app_dir = &state.app_data_dir;
    let archive = std::path::PathBuf::from(&archive_path);
    crate::commands::opc_workflows::import_industry_pack(
        &state.harness.db().clone(),
        app_dir,
        &archive,
    )
    .await
}

// ── Company Runtime：看板投影 + 阻塞升级链（P3-3/4）──────────────

/// 看板投影：按 phase 列聚合 work items（Kanban）。
/// 返回 {列名: [item...]}，列为 待办/进行中/阻塞/评审/已完成/终止。
#[tauri::command]
pub async fn opc_kanban_board(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    use axagent_company_runtime::work_item::Phase;
    use std::collections::BTreeMap;

    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);
    let items = svc.list_by_phase(None).await.map_err(|e| e.to_string())?;

    let mut board: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for it in &items {
        let phase = Phase::from_str(&it.phase).unwrap_or(Phase::Queued);
        let col = phase.kanban_column().to_string();
        let entry = serde_json::json!({
            "id": it.id,
            "title": it.title,
            "phase": it.phase,
            "owner_role_id": it.owner_role_id,
            "assignee_agent_id": it.assignee_agent_id,
            "manager_role_id": it.manager_role_id,
            "last_error": it.last_error,
            "deps": serde_json::from_str::<Vec<String>>(&it.deps_json).unwrap_or_default(),
            "created_at": it.created_at,
            "updated_at": it.updated_at,
        });
        board.entry(col).or_default().push(entry);
    }
    Ok(serde_json::to_value(board).map_err(|e| e.to_string())?)
}

/// 认领 work item（Start）。
#[tauri::command]
pub async fn opc_work_item_start(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);
    let model = svc.start(&id).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(&model).map_err(|e| e.to_string())?)
}

/// 提交评审（SubmitForReview）。
#[tauri::command]
pub async fn opc_work_item_review(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    use axagent_company_runtime::work_item::Transition;
    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);
    let model = svc.apply(&id, Transition::SubmitForReview).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(&model).map_err(|e| e.to_string())?)
}

/// 阻塞升级链：置 BLOCKED + 记录 last_error（原因），通知 manager。
#[tauri::command]
pub async fn opc_escalate_work_item(
    state: State<'_, AppState>,
    id: String,
    reason: String,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    use axagent_company_runtime::work_item::{Phase, Transition};
    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);

    // 1. 状态机置 BLOCKED
    let model = svc.apply(&id, Transition::Block).await.map_err(|e| e.to_string())?;

    // 2. 记录 last_error（升级原因）
    let mut am: axagent_opc_entities::opc_work_items::ActiveModel = model.clone().into();
    am.last_error = sea_orm::Set(Some(reason.clone()));
    am.phase = sea_orm::Set(Phase::Blocked.as_str().to_string());
    am.updated_at = sea_orm::Set(chrono::Utc::now().timestamp());
    let updated = am.update(&db).await.map_err(|e| e.to_string())?;

    // 3. 通知 manager（rt-messaging 渠道；无 manager 则跳过并记日志）
    if let Some(mgr) = &updated.manager_role_id {
        tracing::info!("[opc-escalate] {} 升级给 {}: {}", updated.id, mgr, reason);
    } else {
        tracing::warn!("[opc-escalate] {} 无 manager_role_id，升级仅记录", updated.id);
    }

    Ok(serde_json::to_value(&updated).map_err(|e| e.to_string())?)
}

/// 解除阻塞（Unblock）。
#[tauri::command]
pub async fn opc_work_item_unblock(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    use axagent_company_runtime::work_item::Transition;
    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);
    let model = svc.apply(&id, Transition::Unblock).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(&model).map_err(|e| e.to_string())?)
}

// ── P4-1：人才库导入 + 市场包 ─────────────────────────────────

/// 导入人才库：扫描 agency-agents-src 目录 → 填充 opc_talent_templates。
/// 每个专家 md 生成一条 talent template（分类 = 目录名）。
/// 幂等：已存在的 template id 跳过。
#[tauri::command]
pub async fn opc_import_talent_library(
    state: State<'_, AppState>,
    path: String,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::org::OrgService;

    let base = std::path::PathBuf::from(&path);
    if !base.is_dir() {
        return Err(format!("目录不存在: {path}"));
    }
    let db = state.harness.db().clone();
    let org = OrgService::new(&db);

    let mut imported: u32 = 0;
    let mut skipped: u32 = 0;

    // 遍历分类目录（跳过非专家目录）
    for entry in std::fs::read_dir(&base).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if dir_name.starts_with('.')
            || dir_name == "scripts"
            || dir_name == "examples"
            || dir_name == "integrations"
        {
            continue;
        }
        // 读目录下每个 md 的 frontmatter（name/description）
        for md in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let md = md.map_err(|e| e.to_string())?;
            let md_path = md.path();
            if md_path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let stem = md_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let content = std::fs::read_to_string(&md_path).map_err(|e| e.to_string())?;
            let (name, description) = parse_frontmatter_brief(&content, &stem);
            let tid = format!("tt-{dir_name}-{stem}");

            // 幂等：已存在跳过
            let existing = org.list_talent_templates(None).await.map_err(|e| e.to_string())?;
            if existing.iter().any(|t| t.id == tid) {
                skipped += 1;
                continue;
            }
            org.add_talent_template(
                &tid,
                &dir_name,
                &name,
                &description,
                "agency-agents-src",
                Some(&[format!("agency-agents-src/{dir_name}/{stem}.md")]),
                None,
                Some(&[dir_name.clone()]),
            )
            .await
            .map_err(|e| e.to_string())?;
            imported += 1;
        }
    }

    Ok(serde_json::json!({ "imported": imported, "skipped": skipped }))
}

/// 解析专家 md 的 frontmatter（name/description）。
fn parse_frontmatter_brief(content: &str, fallback_stem: &str) -> (String, String) {
    let mut name = String::new();
    let mut description = String::new();
    for line in content.lines().take(20) {
        if let Some(v) = line.strip_prefix("name:") {
            name = v.trim().trim_matches('"').to_string();
        } else if let Some(v) = line.strip_prefix("description:") {
            description = v.trim().trim_matches('"').to_string();
        }
        if !name.is_empty() && !description.is_empty() {
            break;
        }
    }
    if name.is_empty() {
        name = fallback_stem.replace('-', " ");
    }
    (name, description)
}

/// 市场包列表：扫描内置行业包目录 + app_dir 已装状态。
/// 返回 [{id, name, icon, version, installed, path}]
#[tauri::command]
pub async fn opc_market_list(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let app_dir = &state.app_data_dir;
    let builtin = crate::commands::opc_workflows::resolve_industries_dir(Some(app_dir));
    let installed_root = app_dir.join("config/opc/industries");

    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&builtin) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let manifest_path = dir.join("manifest.yaml");
            let Ok(raw) = std::fs::read_to_string(&manifest_path) else { continue };
            let Ok(manifest) =
                serde_yaml::from_str::<crate::commands::opc_workflows::IndustryManifest>(&raw)
            else {
                continue;
            };
            let installed = installed_root.join(&manifest.id).is_dir();
            items.push(serde_json::json!({
                "id": manifest.id,
                "name": manifest.name,
                "icon": manifest.icon,
                "version": manifest.version,
                "enabled": manifest.enabled,
                "installed": installed,
                "path": dir.display().to_string(),
            }));
        }
    }
    Ok(serde_json::to_value(items).map_err(|e| e.to_string())?)
}
