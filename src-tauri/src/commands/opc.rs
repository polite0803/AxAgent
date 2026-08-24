// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务领域 Tauri 命令层
//!
//! 将 opc-dao 的 Service 暴露给前端 React 调用。

use std::str::FromStr;

use axagent_agent_macro::agent_command;

use crate::AppState;
use sea_orm::ActiveModelTrait;
use tauri::State;

use axagent_analysis_engine::opc::{
    AnalyticsService, BlogPost, ContactSubmission, ContentAsset, ContentAssetService,
    CreateBlogPostInput, CreateContentAssetInput, CreateCustomerInput, CreateInvoiceInput,
    CreateKpiInput, CreateLandingPageInput, CreateProjectInput, CreatePublishScheduleInput,
    Customer, CustomerFilter, CustomerService, DashboardSummary, DefaultAnalyticsService,
    DefaultCustomerService, DefaultFinanceService, DefaultInvoiceService, DefaultProjectService,
    DefaultSiteService, FinanceService, FinancialReport, InvestmentAdvice, Invoice, InvoiceFilter,
    InvoiceService, InvoiceStatus, KpiRecord, LandingPage, Milestone, Project, ProjectFilter,
    ProjectService, PublishSchedule, PublishScheduleService, RevenueRecord, SiteService,
    UpdateContentAssetInput, UpdateCustomerInput, UpdateInvoiceInput, UpdateProjectInput,
    UpdatePublishScheduleInput,
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
        "opc_session".to_string(),
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

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建发票")]
#[tauri::command]
pub async fn opc_create_invoice(
    state: State<'_, AppState>,
    input: CreateInvoiceInput,
) -> Result<Invoice, String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    let result = svc.create_invoice(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    });
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

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取发票")]
#[tauri::command]
pub async fn opc_get_invoice(state: State<'_, AppState>, id: String) -> Result<Invoice, String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    svc.get_invoice(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出发票")]
#[tauri::command]
pub async fn opc_list_invoices(
    state: State<'_, AppState>,
    filter: InvoiceFilter,
) -> Result<Vec<Invoice>, String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    svc.list_invoices(filter).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "更新发票")]
#[tauri::command]
pub async fn opc_update_invoice(
    state: State<'_, AppState>,
    id: String,
    input: UpdateInvoiceInput,
) -> Result<Invoice, String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    svc.update_invoice(&id, input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Dangerous, call_mode = StateInput, description = "删除发票")]
#[tauri::command]
pub async fn opc_delete_invoice(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    svc.delete_invoice(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "转换发票状态")]
#[tauri::command]
pub async fn opc_transition_invoice(
    state: State<'_, AppState>,
    id: String,
    target_status: String,
) -> Result<Invoice, String> {
    let status = InvoiceStatus::from_str(&target_status)
        .map_err(|_| format!("invalid invoice status: {target_status}"))?;
    let svc = DefaultInvoiceService::new(state.harness.db().clone());
    let result = svc.transition_status(&id, status).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    });
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

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建客户")]
#[tauri::command]
pub async fn opc_create_customer(
    state: State<'_, AppState>,
    input: CreateCustomerInput,
) -> Result<Customer, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    let result = svc.create_customer(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    });
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

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取客户")]
#[tauri::command]
pub async fn opc_get_customer(state: State<'_, AppState>, id: String) -> Result<Customer, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.get_customer(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出客户")]
#[tauri::command]
pub async fn opc_list_customers(
    state: State<'_, AppState>,
    filter: CustomerFilter,
) -> Result<Vec<Customer>, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.list_customers(filter).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "更新客户")]
#[tauri::command]
pub async fn opc_update_customer(
    state: State<'_, AppState>,
    id: String,
    input: UpdateCustomerInput,
) -> Result<Customer, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.update_customer(&id, input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Dangerous, call_mode = StateInput, description = "删除客户")]
#[tauri::command]
pub async fn opc_delete_customer(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.delete_customer(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "按邮箱查找客户")]
#[tauri::command]
pub async fn opc_find_customer_by_email(
    state: State<'_, AppState>,
    email: String,
) -> Result<Option<Customer>, String> {
    let svc = DefaultCustomerService::new(state.harness.db().clone());
    svc.find_by_email(&email).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── Project Commands ──────────────────────────────────────────────

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建项目")]
#[tauri::command]
pub async fn opc_create_project(
    state: State<'_, AppState>,
    input: CreateProjectInput,
) -> Result<Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    let result = svc.create_project(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    });
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

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取项目")]
#[tauri::command]
pub async fn opc_get_project(state: State<'_, AppState>, id: String) -> Result<Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.get_project(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出项目")]
#[tauri::command]
pub async fn opc_list_projects(
    state: State<'_, AppState>,
    filter: ProjectFilter,
) -> Result<Vec<Project>, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.list_projects(filter).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "更新项目")]
#[tauri::command]
pub async fn opc_update_project(
    state: State<'_, AppState>,
    id: String,
    input: UpdateProjectInput,
) -> Result<Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.update_project(&id, input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Dangerous, call_mode = StateInput, description = "删除项目")]
#[tauri::command]
pub async fn opc_delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.delete_project(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "添加项目里程碑")]
#[tauri::command]
pub async fn opc_add_milestone(
    state: State<'_, AppState>,
    project_id: String,
    milestone: Milestone,
) -> Result<Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.add_milestone(&project_id, milestone).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "完成项目里程碑")]
#[tauri::command]
pub async fn opc_complete_milestone(
    state: State<'_, AppState>,
    project_id: String,
    milestone_id: String,
) -> Result<Project, String> {
    let svc = DefaultProjectService::new(state.harness.db().clone());
    svc.complete_milestone(&project_id, &milestone_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── Site / Landing Page Commands ────────────────────────────────────

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建着陆页")]
#[tauri::command]
pub async fn opc_create_landing_page(
    state: State<'_, AppState>,
    input: CreateLandingPageInput,
) -> Result<LandingPage, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.create_landing_page(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出着陆页")]
#[tauri::command]
pub async fn opc_list_landing_pages(
    state: State<'_, AppState>,
) -> Result<Vec<LandingPage>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.list_landing_pages().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "发布着陆页")]
#[tauri::command]
pub async fn opc_publish_landing_page(
    state: State<'_, AppState>,
    id: String,
) -> Result<LandingPage, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.publish_landing_page(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── Blog Post Commands ──────────────────────────────────────────────

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建博客文章")]
#[tauri::command]
pub async fn opc_create_blog_post(
    state: State<'_, AppState>,
    input: CreateBlogPostInput,
) -> Result<BlogPost, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.create_blog_post(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出博客文章")]
#[tauri::command]
pub async fn opc_list_blog_posts(state: State<'_, AppState>) -> Result<Vec<BlogPost>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.list_blog_posts().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "发布博客文章")]
#[tauri::command]
pub async fn opc_publish_blog_post(
    state: State<'_, AppState>,
    id: String,
) -> Result<BlogPost, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.publish_blog_post(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── Contact Commands ────────────────────────────────────────────────

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出联系提交")]
#[tauri::command]
pub async fn opc_list_contacts(
    state: State<'_, AppState>,
) -> Result<Vec<ContactSubmission>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.list_contacts().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "标记联系已读")]
#[tauri::command]
pub async fn opc_mark_contact_read(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.mark_contact_read(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── Analytics Commands ──────────────────────────────────────────────

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "记录 KPI")]
#[tauri::command]
pub async fn opc_record_kpi(
    state: State<'_, AppState>,
    input: CreateKpiInput,
) -> Result<KpiRecord, String> {
    let svc = DefaultAnalyticsService::new(state.harness.db().clone());
    svc.record_kpi(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出 KPI 记录")]
#[tauri::command]
pub async fn opc_list_kpis(
    state: State<'_, AppState>,
    period: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<KpiRecord>, String> {
    let svc = DefaultAnalyticsService::new(state.harness.db().clone());
    svc.list_kpis(period, limit).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出收入记录")]
#[tauri::command]
pub async fn opc_list_revenue(
    state: State<'_, AppState>,
    category: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<RevenueRecord>, String> {
    let svc = DefaultAnalyticsService::new(state.harness.db().clone());
    svc.list_revenue(category, limit).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取看板摘要")]
#[tauri::command]
pub async fn opc_get_dashboard_summary(
    state: State<'_, AppState>,
) -> Result<DashboardSummary, String> {
    let svc = DefaultAnalyticsService::new(state.harness.db().clone());
    svc.get_dashboard_summary().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── Finance Commands ────────────────────────────────────────────────

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取财务报告")]
#[tauri::command]
pub async fn opc_get_financial_report(
    state: State<'_, AppState>,
    period: String,
) -> Result<FinancialReport, String> {
    let svc = DefaultFinanceService::new(state.harness.db().clone());
    svc.get_financial_report(&period).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取投资建议")]
#[tauri::command]
pub async fn opc_get_investment_advice(
    state: State<'_, AppState>,
    period: String,
) -> Result<InvestmentAdvice, String> {
    let svc = DefaultFinanceService::new(state.harness.db().clone());
    let report = svc.get_financial_report(&period).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(svc.get_investment_advice(&report).await)
}

// ── Industry Pack .opcip 导出/导入 ──────────────────────────────

/// 导出行业包为 .opcip 归档（打包 manifest + workflows）。
/// out_dir 为前端选择的保存目录（通过对话框）。
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "导出行业包")]
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
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "导入行业包")]
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

/// 创建工作项（修复 P0-2：此前 opc_work_items 无生产创建入口，看板/自改进/质量门全无数据源）。
/// id 强制 `wi-` 前缀（与 self_improving::OpcWorkItemRound 的 task 契约一致）。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建工作项")]
#[tauri::command]
pub async fn opc_create_work_item(
    state: State<'_, AppState>,
    title: String,
    owner_role_id: Option<String>,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    use axagent_company_runtime::work_item_service::NewWorkItem;

    if title.trim().is_empty() {
        return Err("工作项标题不能为空".to_string());
    }
    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);
    let id = format!("wi-{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    let mut input = NewWorkItem::new(id, title.trim().to_string());
    if let Some(owner) = owner_role_id.filter(|s| !s.trim().is_empty()) {
        input = input.owner(owner.trim().to_string());
    }
    let model = svc.create(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    serde_json::to_value(&model).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 删除落地页（修复 P0-5：前端 SitesTab 调用但后端缺失）。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "删除落地页")]
#[tauri::command]
pub async fn opc_delete_landing_page(state: State<'_, AppState>, id: String) -> Result<(), String> {
    use axagent_entities::opc_landing_pages;
    use sea_orm::EntityTrait;
    let db = state.harness.db();
    opc_landing_pages::Entity::delete_by_id(&id).exec(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(())
}

/// 删除博客文章（修复 P0-5）。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "删除博客文章")]
#[tauri::command]
pub async fn opc_delete_blog_post(state: State<'_, AppState>, id: String) -> Result<(), String> {
    use axagent_entities::opc_blog_posts;
    use sea_orm::EntityTrait;
    let db = state.harness.db();
    opc_blog_posts::Entity::delete_by_id(&id).exec(db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    Ok(())
}

// ── Content Asset Commands ──────────────────────────────────────────

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建内容资产")]
#[tauri::command]
pub async fn opc_create_content_asset(
    state: State<'_, AppState>,
    input: CreateContentAssetInput,
) -> Result<ContentAsset, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.create_content_asset(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出内容资产")]
#[tauri::command]
pub async fn opc_list_content_assets(
    state: State<'_, AppState>,
) -> Result<Vec<ContentAsset>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.list_content_assets().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取内容资产详情")]
#[tauri::command]
pub async fn opc_get_content_asset(
    state: State<'_, AppState>,
    id: String,
) -> Result<ContentAsset, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.get_content_asset(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "更新内容资产")]
#[tauri::command]
pub async fn opc_update_content_asset(
    state: State<'_, AppState>,
    id: String,
    input: UpdateContentAssetInput,
) -> Result<ContentAsset, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.update_content_asset(&id, input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

#[agent_command(domain = "automation", safety = Dangerous, call_mode = StateInput, description = "删除内容资产")]
#[tauri::command]
pub async fn opc_delete_content_asset(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.delete_content_asset(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 看板投影：按 phase 列聚合 work items（Kanban）。
/// 返回 {列名: [item...]}，列为 待办/进行中/阻塞/评审/已完成/终止。
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "获取看板块")]
#[tauri::command]
pub async fn opc_kanban_board(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    use axagent_company_runtime::work_item::Phase;
    use std::collections::BTreeMap;

    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);
    let items = svc.list_by_phase(None).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let mut board: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    for it in &items {
        let phase = it.phase.parse::<Phase>().unwrap_or(Phase::Queued);
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
    serde_json::to_value(board).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 认领 work item（Start）。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "认领工作项")]
#[tauri::command]
pub async fn opc_work_item_start(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);
    let model = svc.start(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    serde_json::to_value(&model).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 提交评审（质量门前置，方案 A）：先跑一轮自改进评估，质量达标才允许进 REVIEW。
///
/// 流程：执行一轮 OpcWorkItemRound → 5 维规则评估 → 评估经 QualityGateService
/// 落经验（归因+信号）→ score >= 0.80 才 apply(SubmitForReview)；未达标返回
/// 缺口清单（前端展示原因），产出无法进入评审流。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "提交工作项评审")]
#[tauri::command]
pub async fn opc_work_item_review(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::work_item::Transition;
    use axagent_company_runtime::{OpcWorkItemRound, QualityGateService, WorkItemService};
    use axagent_harness::self_improving_loop::SelfImprovingRound;

    // P1-3：准入线 0.70——允许无历史经验（无 playbook）的首次执行也进入评审；
    // 收敛判定（decide_next 0.85 Accept）语义不同，保留在自改进循环内。
    const QUALITY_THRESHOLD: f64 = 0.70;

    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);

    // 1. 加载 work item（拿 owner 做归因）
    let item = svc.get(&id).await.map_err(|e| format!("加载 work item 失败: {e}"))?;

    // 2. 质量门前置：跑一轮自改进评估
    let mut round = OpcWorkItemRound::new(db.clone());
    let result =
        round.execute_round(&id, None).await.map_err(|e| format!("自改进评估失败: {e}"))?;
    let evaluation =
        round.evaluate_round(&id, &result).await.map_err(|e| format!("自改进评估失败: {e}"))?;

    // 3. 评估落经验（归因铁律：评估者=owner，写入 owner 档案）
    if let Some(owner) = &item.owner_role_id {
        if !owner.is_empty() {
            let gate = QualityGateService::new(&db);
            let _ = gate.apply(owner, &id, owner, &evaluation, QUALITY_THRESHOLD).await;
        }
    }

    // 4. 质量门判定
    if evaluation.score < QUALITY_THRESHOLD {
        let pct = evaluation.score * 100.0;
        return Err(format!(
            "质量门未通过（{pct:.0}% < {}%）。缺口：{}",
            QUALITY_THRESHOLD * 100.0,
            if evaluation.gaps.is_empty() {
                "综合质量不足".to_string()
            } else {
                evaluation.gaps.join("；")
            }
        ));
    }

    // 5. 达标 → 进入 REVIEW
    let model = svc.apply(&id, Transition::SubmitForReview).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    serde_json::to_value(&model).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 阻塞升级链：置 BLOCKED + 记录 last_error（原因），通知 manager。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "升级阻塞工作项")]
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
    let model = svc.apply(&id, Transition::Block).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 2. 记录 last_error（升级原因）
    let mut am: axagent_entities::opc_work_items::ActiveModel = model.clone().into();
    am.last_error = sea_orm::Set(Some(reason.clone()));
    am.phase = sea_orm::Set(Phase::Blocked.as_str().to_string());
    am.updated_at = sea_orm::Set(chrono::Utc::now().timestamp());
    let updated = am.update(&db).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 3. 通知 manager（rt-messaging 渠道；无 manager 则跳过并记日志）
    if let Some(mgr) = &updated.manager_role_id {
        tracing::info!("[opc-escalate] {} 升级给 {}: {}", updated.id, mgr, reason);
    } else {
        tracing::warn!("[opc-escalate] {} 无 manager_role_id，升级仅记录", updated.id);
    }

    serde_json::to_value(&updated).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 解除阻塞（Unblock）。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "解除工作项阻塞")]
#[tauri::command]
pub async fn opc_work_item_unblock(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use axagent_company_runtime::WorkItemService;
    use axagent_company_runtime::work_item::Transition;
    let db = state.harness.db().clone();
    let svc = WorkItemService::new(&db);
    let model = svc.apply(&id, Transition::Unblock).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    serde_json::to_value(&model).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── P4-1：人才库导入 + 市场包 ─────────────────────────────────

/// 导入人才库：扫描 agency-agents-src 目录 → 填充 opc_talent_templates。
/// 每个专家 md 生成一条 talent template（分类 = 目录名）。
/// 幂等：已存在的 template id 跳过。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "导入人才库")]
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

    // P2-1：一次性加载现有模板 id 集合（此前每个 md 文件都全表查询 list_talent_templates，
    // 274 个专家 → 274 次全表扫描；改为一次加载 + HashSet 判重）
    let existing = org.list_talent_templates(None).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    let mut existing_ids: std::collections::HashSet<String> =
        existing.into_iter().map(|t| t.id).collect();

    // 遍历分类目录（跳过非专家目录）
    for entry in std::fs::read_dir(&base).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })? {
        let entry = entry.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
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
        for md in std::fs::read_dir(&dir).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })? {
            let md = md.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            let md_path = md.path();
            if md_path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let stem = md_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let content = std::fs::read_to_string(&md_path).map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
            let (name, description) = parse_frontmatter_brief(&content, &stem);
            let tid = format!("tt-{dir_name}-{stem}");

            // 幂等：已存在跳过（P2-1：用预加载的 HashSet 判重）
            if existing_ids.contains(&tid) {
                skipped += 1;
                continue;
            }
            existing_ids.insert(tid.clone());
            org.add_talent_template(axagent_company_runtime::org::NewTalentTemplate {
                id: tid.clone(),
                category: dir_name.clone(),
                name: name.clone(),
                description: description.clone(),
                source_repo: "agency-agents-src".to_string(),
                prompt_refs: Some(vec![format!("agency-agents-src/{dir_name}/{stem}.md")]),
                skill_refs: None,
                tags: Some(vec![dir_name.clone()]),
            })
            .await
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
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
#[agent_command(domain = "automation", safety = Safe, call_mode = StateInput, description = "列出市场行业包")]
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
    serde_json::to_value(items).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

// ── OPC 自改进循环（对接上游 Loop Engineering，参照 stock_analysis）──

/// 自改进 WorkItem 循环：OPC 领域实现 OpcWorkItemRound（company-runtime）
/// 通过上游 harness::SelfImprovingRound trait + agent::SelfImprovementExecutor
/// 跑"执行 → 自评估 → 收敛/改进"回合制闭环。返回最终产出 + 评估分 + 轮次。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "自改进 OPC 工作项循环")]
#[tauri::command]
pub async fn run_self_improving_opc_work_item(
    state: State<'_, AppState>,
    task: String,
    max_rounds: Option<u32>,
) -> Result<serde_json::Value, String> {
    use axagent_agent::self_improvement_executor::{
        SelfImprovementConfig, SelfImprovementExecutor,
    };

    let db = state.harness.db().clone();
    let round = axagent_company_runtime::OpcWorkItemRound::new(db);
    let config = SelfImprovementConfig::new(
        max_rounds.unwrap_or(3),
        0.80, // 收敛阈值：评估分高于此值直接 Accept
        3,    // 连续无进展多少次后 Escalate
    );
    let mut executor = SelfImprovementExecutor::new(Box::new(round), config);

    match executor.run(&task).await {
        Ok(output) => Ok(serde_json::json!({
            "text": output.text,
            "totalRounds": output.total_rounds,
            "finalScore": output.final_evaluation.score,
            "confidence": output.final_evaluation.confidence,
            "strengths": output.final_evaluation.strengths,
            "gaps": output.final_evaluation.gaps,
        })),
        Err(e) => Err(e.to_string()),
    }
}

// ── 方案 B：OPC 角色进 Fleet（办公室接真实 Agent 状态）────────────

/// 同步 OPC 员工为舰队成员（幂等）：扫描 opc_org_employees(active) →
/// 注册/更新到 Fleet，成员状态由该角色最新 work item phase 驱动。
/// 办公室（Fleet 视图）从此显示真实角色状态，与看板形成"人/事"互补。
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "同步员工到舰队")]
#[tauri::command]
pub async fn opc_sync_fleet(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    use axagent_entities::opc_org_employees;
    use axagent_harness::fleet::{Fleet, FleetMember, FleetMetadata, FleetStatus};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = state.harness.db().clone();
    let fleet_repo = state.fleet_repository.clone();

    // 1. 找/建默认舰队
    let fleet_id = {
        let fleets = fleet_repo.list_fleets(None).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        match fleets.into_iter().find(|f| f.name == "OPC 一人公司") {
            Some(f) => f.id,
            None => {
                let fleet = Fleet {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "OPC 一人公司".to_string(),
                    scene_template_slug: None,
                    status: FleetStatus::Active,
                    created_at: chrono::Utc::now().timestamp_millis(),
                    updated_at: chrono::Utc::now().timestamp_millis(),
                    metadata: FleetMetadata {
                        description: "一人公司 6 角色（OpenOPC 三机制）".to_string(),
                        max_members: 16,
                        strategy: None,
                        tags: vec!["opc".to_string()],
                    },
                };
                fleet_repo
                    .create_fleet(fleet)
                    .await
                    .map_err(|e| {
                        String::from(crate::commands::error::ErrorResponse::from_error(
                            e,
                            crate::commands::error::ErrorCategory::Unrecoverable,
                        ))
                    })?
                    .id
            },
        }
    };

    // 2. 扫描 active 员工
    let employees = opc_org_employees::Entity::find()
        .filter(opc_org_employees::Column::Status.eq("active"))
        .all(&db)
        .await
        .map_err(|e| format!("查询员工失败: {e}"))?;

    // 已有成员（按 agent_slug 判重）
    let members = fleet_repo.list_members(&fleet_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    let mut synced = 0u32;
    let mut updated = 0u32;

    let role_ids: Vec<String> = employees.iter().map(|e| e.role_id.clone()).collect();
    let role_status_map = opc_batch_role_status(&db, &role_ids).await;

    for emp in &employees {
        let slug = emp.role_id.clone();
        let status = role_status_map
            .get(&slug)
            .cloned()
            .unwrap_or(axagent_harness::fleet::FleetMemberStatus::Idle);
        match members.iter().find(|m| m.agent_slug == slug) {
            Some(existing) => {
                if existing.status != status {
                    fleet_repo.update_member_status(&existing.id, status).await.map_err(|e| {
                        String::from(crate::commands::error::ErrorResponse::from_error(
                            e,
                            crate::commands::error::ErrorCategory::Unrecoverable,
                        ))
                    })?;
                    updated += 1;
                }
            },
            None => {
                let member = FleetMember {
                    id: uuid::Uuid::new_v4().to_string(),
                    fleet_id: fleet_id.clone(),
                    agent_id: emp.employee_id.clone(),
                    agent_slug: slug.clone(),
                    display_name: emp.employee_id.clone(),
                    role: slug.clone(),
                    agent_profile_id: None,
                    room_id: "default".to_string(),
                    status,
                    joined_at: chrono::Utc::now().timestamp_millis(),
                    today_tokens: 0,
                    total_tokens: 0,
                };
                fleet_repo.add_member(member).await.map_err(|e| {
                    String::from(crate::commands::error::ErrorResponse::from_error(
                        e,
                        crate::commands::error::ErrorCategory::Unrecoverable,
                    ))
                })?;
                synced += 1;
            },
        }
    }

    let total = fleet_repo
        .list_members(&fleet_id)
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?
        .len();
    Ok(serde_json::json!({
        "fleetId": fleet_id,
        "synced": synced,
        "updatedStatus": updated,
        "totalMembers": total,
    }))
}

/// 批量查询多个角色的成员状态（避免 N+1 查询）。
/// 一次性加载所有相关 work items，在内存中按 role_id 分组取最新一条。
async fn opc_batch_role_status(
    db: &sea_orm::DatabaseConnection,
    role_ids: &[String],
) -> std::collections::HashMap<String, axagent_harness::fleet::FleetMemberStatus> {
    use axagent_entities::opc_work_items;
    use axagent_harness::fleet::FleetMemberStatus;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    let mut map = std::collections::HashMap::new();

    if role_ids.is_empty() {
        return map;
    }

    let work_items = opc_work_items::Entity::find()
        .filter(opc_work_items::Column::OwnerRoleId.is_in(role_ids.iter().map(|s| s.as_str())))
        .order_by_desc(opc_work_items::Column::UpdatedAt)
        .all(db)
        .await
        .unwrap_or_default();

    for w in work_items {
        if let Some(role_id) = &w.owner_role_id {
            if role_id.is_empty() || map.contains_key(role_id) {
                continue;
            }
            let status = match w.phase.as_str() {
                "IN_PROGRESS" | "REVIEW" | "APPROVED" => FleetMemberStatus::Busy,
                "BLOCKED" | "FAILED" => FleetMemberStatus::Error,
                _ => FleetMemberStatus::Idle,
            };
            map.insert(role_id.clone(), status);
        }
    }

    for rid in role_ids {
        map.entry(rid.clone()).or_insert(FleetMemberStatus::Idle);
    }

    map
}

// ── 发布计划 Tauri 命令 ─────────────────────────────────────────

/// 创建发布计划
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "创建发布计划")]
#[tauri::command]
pub async fn opc_create_publish_schedule(
    state: State<'_, AppState>,
    input: CreatePublishScheduleInput,
) -> Result<PublishSchedule, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.create_publish_schedule(input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 列出所有发布计划
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "列出发布计划")]
#[tauri::command]
pub async fn opc_list_publish_schedules(
    state: State<'_, AppState>,
) -> Result<Vec<PublishSchedule>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.list_publish_schedules().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取单个发布计划
#[agent_command(domain = "automation", safety = Safe, call_mode = StateOnly, description = "获取发布计划详情")]
#[tauri::command]
pub async fn opc_get_publish_schedule(
    state: State<'_, AppState>,
    id: String,
) -> Result<PublishSchedule, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.get_publish_schedule(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 更新发布计划
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "更新发布计划")]
#[tauri::command]
pub async fn opc_update_publish_schedule(
    state: State<'_, AppState>,
    id: String,
    input: UpdatePublishScheduleInput,
) -> Result<PublishSchedule, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.update_publish_schedule(&id, input).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 删除发布计划
#[agent_command(domain = "automation", safety = Caution, call_mode = StateInput, description = "删除发布计划")]
#[tauri::command]
pub async fn opc_delete_publish_schedule(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.delete_publish_schedule(&id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 处理到期的发布计划
#[agent_command(domain = "automation", safety = Caution, call_mode = Manual, description = "处理到期发布计划")]
#[tauri::command]
pub async fn opc_process_due_schedules(
    state: State<'_, AppState>,
) -> Result<Vec<PublishSchedule>, String> {
    let svc = DefaultSiteService::new(state.harness.db().clone());
    svc.process_due_schedules().await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
