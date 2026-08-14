// SPDX-License-Identifier: AGPL-3.0-only

//! OPC (一人公司) 业务工具
//!
//! 将 opc-dao 的数据操作暴露为 Agent Tool，
//! 使 AI Agent 能够直接管理发票、客户、项目等 OPC 业务数据。
//!
//! 所有工具使用 `ToolDomain::Automation` 分类，
//! 可通过域过滤仅暴露给 OPC 角色的 Agent。

use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::OnceLock;

/// OPC 通知消息（通过 channel 发送到后台 worker）
pub struct OpcNotification {
    pub platform: String,
    pub chat_id: String,
    pub message: String,
}

/// OPC 通知发送 channel（由 wiring 层注入）
static OPC_NOTIFY_TX: OnceLock<tokio::sync::mpsc::UnboundedSender<OpcNotification>> =
    OnceLock::new();

/// 注入通知发送 channel（由 wiring 层在初始化时调用一次）
pub fn set_opc_notify_tx(tx: tokio::sync::mpsc::UnboundedSender<OpcNotification>) {
    let _ = OPC_NOTIFY_TX.set(tx);
}

fn get_notify_tx() -> &'static tokio::sync::mpsc::UnboundedSender<OpcNotification> {
    OPC_NOTIFY_TX.get().expect("OPC notify tx not initialized")
}

fn get_db() -> Result<Arc<sea_orm::DatabaseConnection>, ToolError> {
    crate::global_state::get_sea_db()
        .ok_or_else(|| ToolError::execution_failed("OPC 数据库未初始化".to_string()))
}

// ═══════════════════════════════════════════════════════════════════
// 发票工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcListInvoicesTool;

#[async_trait]
impl Tool for OpcListInvoicesTool {
    fn name(&self) -> &str {
        "OpcListInvoices"
    }
    fn description(&self) -> &str {
        "查询一人公司(OPC)的发票列表。支持按状态、客户、日期范围过滤。返回发票摘要信息。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["draft", "sent", "paid", "overdue", "cancelled", "refunded"],
                    "description": "按状态过滤（可选）"
                },
                "customer_id": { "type": "string", "description": "按客户ID过滤（可选）" },
                "date_from": { "type": "integer", "description": "起始时间戳（可选）" },
                "date_to": { "type": "integer", "description": "结束时间戳（可选）" },
                "limit": { "type": "integer", "description": "返回数量上限（默认10）" }
            }
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultInvoiceService;
        use axagent_analysis_engine::opc::{InvoiceFilter, InvoiceService, InvoiceStatus};

        let db = get_db()?;
        let svc = DefaultInvoiceService::new((*db).clone());

        let status = input
            .get("status")
            .and_then(|v| v.as_str())
            .and_then(|s| InvoiceStatus::from_str(s).ok());
        let filter = InvoiceFilter {
            status,
            customer_id: input.get("customer_id").and_then(|v| v.as_str()).map(String::from),
            date_from: input.get("date_from").and_then(|v| v.as_i64()),
            date_to: input.get("date_to").and_then(|v| v.as_i64()),
            limit: input.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32),
            offset: None,
        };

        match svc.list_invoices(filter).await {
            Ok(invoices) => {
                if invoices.is_empty() {
                    return Ok(ToolResult::success("## 发票列表\n\n暂无发票记录。"));
                }
                let lines: Vec<String> = invoices
                    .iter()
                    .map(|inv| {
                        format!(
                            "- **{}** | {} | ¥{:.2} | {} | 客户: {}",
                            inv.invoice_number,
                            inv.status.as_str(),
                            inv.total,
                            chrono::DateTime::from_timestamp(inv.created_at, 0)
                                .map(|dt| dt.format("%Y-%m-%d").to_string())
                                .unwrap_or_default(),
                            inv.customer_id,
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 发票列表 ({} 条)\n\n{}\n\n> 共 {} 条结果。使用 `OpcGetInvoice` 查看详情。",
                    invoices.len(),
                    lines.join("\n"),
                    invoices.len(),
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询发票失败: {e}"))),
        }
    }
}

pub struct OpcCreateInvoiceTool;

#[async_trait]
impl Tool for OpcCreateInvoiceTool {
    fn name(&self) -> &str {
        "OpcCreateInvoice"
    }
    fn description(&self) -> &str {
        "创建一张新发票。需要提供客户ID、行项目（描述、数量、单价、税率）、货币和可选备注。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "customer_id": { "type": "string", "description": "客户ID" },
                "line_items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "description": { "type": "string" },
                            "quantity": { "type": "number" },
                            "unit_price": { "type": "number" },
                            "tax_rate": { "type": "number", "description": "税率，如 0.13 表示13%" }
                        },
                        "required": ["description", "quantity", "unit_price"]
                    },
                    "description": "行项目列表。quantity×unit_price×(1+tax_rate) 自动计算 total"
                },
                "currency": { "type": "string", "default": "CNY", "description": "货币代码（默认CNY）" },
                "due_at": { "type": "integer", "description": "到期时间戳（可选）" },
                "notes": { "type": "string", "description": "备注（可选）" }
            },
            "required": ["customer_id", "line_items"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_destructive(&self) -> bool {
        false
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultInvoiceService;
        use axagent_analysis_engine::opc::{CreateInvoiceInput, InvoiceLineItem, InvoiceService};

        let db = get_db()?;
        let svc = DefaultInvoiceService::new((*db).clone());

        let customer_id = input
            .get("customer_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: customer_id"))?
            .to_string();

        let line_items = input
            .get("line_items")
            .and_then(|v| v.as_array())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: line_items"))?
            .iter()
            .map(|item| {
                let qty = item.get("quantity").and_then(|v| v.as_f64()).unwrap_or(1.0);
                let price = item.get("unit_price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let tax = item.get("tax_rate").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let total = qty * price * (1.0 + tax);
                InvoiceLineItem {
                    description: item
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    quantity: qty,
                    unit_price: price,
                    tax_rate: tax,
                    total,
                }
            })
            .collect::<Vec<_>>();

        let input = CreateInvoiceInput {
            customer_id,
            line_items,
            currency: input.get("currency").and_then(|v| v.as_str()).unwrap_or("CNY").to_string(),
            due_at: input.get("due_at").and_then(|v| v.as_i64()),
            notes: input.get("notes").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        };

        match svc.create_invoice(input).await {
            Ok(inv) => Ok(ToolResult::success(format!(
                "## 发票创建成功\n\n- **编号**: {}\n- **金额**: ¥{:.2}\n- **状态**: {}\n- **ID**: `{}`\n\n可使用 `OpcTransitionInvoice` 更改状态。",
                inv.invoice_number,
                inv.total,
                inv.status.as_str(),
                inv.id,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("创建发票失败: {e}"))),
        }
    }
}

pub struct OpcTransitionInvoiceTool;

#[async_trait]
impl Tool for OpcTransitionInvoiceTool {
    fn name(&self) -> &str {
        "OpcTransitionInvoice"
    }
    fn description(&self) -> &str {
        "变更发票状态。合法转换: draft→sent, draft→cancelled, sent→paid, sent→overdue, sent→cancelled, overdue→paid, overdue→cancelled, paid→refunded。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "发票ID" },
                "status": {
                    "type": "string",
                    "enum": ["draft", "sent", "paid", "overdue", "cancelled", "refunded"],
                    "description": "目标状态"
                }
            },
            "required": ["id", "status"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultInvoiceService;
        use axagent_analysis_engine::opc::{InvoiceService, InvoiceStatus};

        let db = get_db()?;
        let svc = DefaultInvoiceService::new((*db).clone());

        let id = input
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: id"))?;
        let target = input
            .get("status")
            .and_then(|v| v.as_str())
            .and_then(|s| InvoiceStatus::from_str(s).ok())
            .ok_or_else(|| ToolError::invalid_input("无效的状态值"))?;

        match svc.transition_status(id, target).await {
            Ok(inv) => Ok(ToolResult::success(format!(
                "## 发票状态已更新\n\n- **编号**: {}\n- **新状态**: **{}**\n- **金额**: ¥{:.2}",
                inv.invoice_number,
                inv.status.as_str(),
                inv.total,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("状态变更失败: {e}"))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 客户工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcListCustomersTool;

#[async_trait]
impl Tool for OpcListCustomersTool {
    fn name(&self) -> &str {
        "OpcListCustomers"
    }
    fn description(&self) -> &str {
        "查询一人公司(OPC)的客户列表。支持按状态、名称/邮箱搜索过滤。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["lead", "prospect", "active", "inactive", "churned"],
                    "description": "按客户状态过滤（可选）"
                },
                "search": { "type": "string", "description": "按名称/邮箱/公司模糊搜索（可选）" },
                "limit": { "type": "integer", "description": "返回数量上限（默认10）" }
            }
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultCustomerService;
        use axagent_analysis_engine::opc::{CustomerFilter, CustomerService, CustomerStatus};

        let db = get_db()?;
        let svc = DefaultCustomerService::new((*db).clone());

        let status = input
            .get("status")
            .and_then(|v| v.as_str())
            .and_then(|s| CustomerStatus::from_str(s).ok());
        let filter = CustomerFilter {
            status,
            search: input.get("search").and_then(|v| v.as_str()).map(String::from),
            tags: None,
            limit: input.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32),
            offset: None,
        };

        match svc.list_customers(filter).await {
            Ok(customers) => {
                if customers.is_empty() {
                    return Ok(ToolResult::success("## 客户列表\n\n暂无客户记录。"));
                }
                let lines: Vec<String> = customers
                    .iter()
                    .map(|c| {
                        format!(
                            "- **{}** | {} | {} | 消费: ¥{:.2} | {}",
                            c.name,
                            c.status.as_str(),
                            c.email,
                            c.total_revenue,
                            c.company.as_deref().unwrap_or(""),
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 客户列表 ({} 条)\n\n{}\n\n> 共 {} 条结果。",
                    customers.len(),
                    lines.join("\n"),
                    customers.len(),
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询客户失败: {e}"))),
        }
    }
}

pub struct OpcCreateCustomerTool;

#[async_trait]
impl Tool for OpcCreateCustomerTool {
    fn name(&self) -> &str {
        "OpcCreateCustomer"
    }
    fn description(&self) -> &str {
        "创建新客户记录。需要提供姓名和邮箱，可选电话、公司、来源、标签和备注。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "客户姓名" },
                "email": { "type": "string", "description": "客户邮箱" },
                "phone": { "type": "string", "description": "电话号码（可选）" },
                "company": { "type": "string", "description": "公司名称（可选）" },
                "source": {
                    "type": "string",
                    "enum": ["Referral", "Website", "SocialMedia", "Marketplace", "Direct"],
                    "description": "客户来源（可选）"
                },
                "notes": { "type": "string", "description": "备注（可选）" }
            },
            "required": ["name", "email"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultCustomerService;
        use axagent_analysis_engine::opc::{CreateCustomerInput, CustomerService, CustomerSource};

        let db = get_db()?;
        let svc = DefaultCustomerService::new((*db).clone());

        let name = input
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: name"))?;
        let email = input
            .get("email")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: email"))?;

        let source = input.get("source").and_then(|v| v.as_str()).map(|s| match s {
            "Referral" => CustomerSource::Referral,
            "Website" => CustomerSource::Website,
            "SocialMedia" => CustomerSource::SocialMedia,
            "Marketplace" => CustomerSource::Marketplace,
            "Direct" => CustomerSource::Direct,
            other => CustomerSource::Other(other.to_string()),
        });

        let inp = CreateCustomerInput {
            name: name.to_string(),
            email: email.to_string(),
            phone: input.get("phone").and_then(|v| v.as_str()).map(String::from),
            company: input.get("company").and_then(|v| v.as_str()).map(String::from),
            source,
            tags: Vec::new(),
            notes: input.get("notes").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        };

        match svc.create_customer(inp).await {
            Ok(c) => Ok(ToolResult::success(format!(
                "## 客户创建成功\n\n- **姓名**: {}\n- **邮箱**: {}\n- **状态**: {}\n- **ID**: `{}`",
                c.name,
                c.email,
                c.status.as_str(),
                c.id,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("创建客户失败: {e}"))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 项目工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcListProjectsTool;

#[async_trait]
impl Tool for OpcListProjectsTool {
    fn name(&self) -> &str {
        "OpcListProjects"
    }
    fn description(&self) -> &str {
        "查询一人公司(OPC)的项目列表。支持按状态、客户过滤和关键词搜索。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["planning", "active", "paused", "completed", "cancelled"],
                    "description": "按项目状态过滤（可选）"
                },
                "customer_id": { "type": "string", "description": "按客户ID过滤（可选）" },
                "search": { "type": "string", "description": "按标题/描述搜索（可选）" },
                "limit": { "type": "integer", "description": "返回数量上限（默认10）" }
            }
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultProjectService;
        use axagent_analysis_engine::opc::{
            MilestoneStatus, ProjectFilter, ProjectService, ProjectStatus,
        };

        let db = get_db()?;
        let svc = DefaultProjectService::new((*db).clone());

        let status = input
            .get("status")
            .and_then(|v| v.as_str())
            .and_then(|s| ProjectStatus::from_str(s).ok());
        let filter = ProjectFilter {
            status,
            customer_id: input.get("customer_id").and_then(|v| v.as_str()).map(String::from),
            search: input.get("search").and_then(|v| v.as_str()).map(String::from),
            limit: input.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32),
            offset: None,
        };

        match svc.list_projects(filter).await {
            Ok(projects) => {
                if projects.is_empty() {
                    return Ok(ToolResult::success("## 项目列表\n\n暂无项目记录。"));
                }
                let lines: Vec<String> = projects
                    .iter()
                    .map(|p| {
                        let milestone_info = if p.milestones.is_empty() {
                            String::new()
                        } else {
                            let done = p
                                .milestones
                                .iter()
                                .filter(|m| m.status == MilestoneStatus::Completed)
                                .count();
                            format!(" (里程碑: {}/{})", done, p.milestones.len())
                        };
                        let budget_str =
                            p.budget.map(|b| format!(" | 预算: ¥{:.2}", b)).unwrap_or_default();
                        format!(
                            "- **{}** | {}{}{}",
                            p.title,
                            p.status.as_str(),
                            milestone_info,
                            budget_str,
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 项目列表 ({} 条)\n\n{}",
                    projects.len(),
                    lines.join("\n"),
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询项目失败: {e}"))),
        }
    }
}

pub struct OpcCreateProjectTool;

#[async_trait]
impl Tool for OpcCreateProjectTool {
    fn name(&self) -> &str {
        "OpcCreateProject"
    }
    fn description(&self) -> &str {
        "创建新项目。需要提供项目标题，可选关联客户、描述、预算和截止日期。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "项目标题" },
                "description": { "type": "string", "description": "项目描述（可选）" },
                "customer_id": { "type": "string", "description": "关联客户ID（可选）" },
                "budget": { "type": "number", "description": "项目预算（可选）" },
                "currency": { "type": "string", "default": "CNY", "description": "货币代码（默认CNY）" },
                "deadline": { "type": "integer", "description": "截止时间戳（可选）" },
                "notes": { "type": "string", "description": "备注（可选）" }
            },
            "required": ["title"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultProjectService;
        use axagent_analysis_engine::opc::{CreateProjectInput, ProjectService};

        let db = get_db()?;
        let svc = DefaultProjectService::new((*db).clone());

        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: title"))?;

        let inp = CreateProjectInput {
            customer_id: input.get("customer_id").and_then(|v| v.as_str()).map(String::from),
            title: title.to_string(),
            description: input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            budget: input.get("budget").and_then(|v| v.as_f64()),
            currency: input.get("currency").and_then(|v| v.as_str()).unwrap_or("CNY").to_string(),
            deadline: input.get("deadline").and_then(|v| v.as_i64()),
            notes: input.get("notes").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        };

        match svc.create_project(inp).await {
            Ok(p) => Ok(ToolResult::success(format!(
                "## 项目创建成功\n\n- **标题**: {}\n- **状态**: {}\n- **ID**: `{}`\n\n可使用 `OpcAddMilestone` 添加里程碑。",
                p.title,
                p.status.as_str(),
                p.id,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("创建项目失败: {e}"))),
        }
    }
}

pub struct OpcAddMilestoneTool;

#[async_trait]
impl Tool for OpcAddMilestoneTool {
    fn name(&self) -> &str {
        "OpcAddMilestone"
    }
    fn description(&self) -> &str {
        "为项目添加里程碑。需要项目ID、里程碑标题和描述。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project_id": { "type": "string", "description": "项目ID" },
                "title": { "type": "string", "description": "里程碑标题" },
                "description": { "type": "string", "description": "里程碑描述（可选）" },
                "due_at": { "type": "integer", "description": "到期时间戳（可选）" }
            },
            "required": ["project_id", "title"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultProjectService;
        use axagent_analysis_engine::opc::{Milestone, MilestoneStatus, ProjectService};

        let db = get_db()?;
        let svc = DefaultProjectService::new((*db).clone());

        let project_id = input
            .get("project_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: project_id"))?;
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: title"))?;

        let milestone = Milestone {
            id: axagent_harness::util_fns::gen_id(),
            title: title.to_string(),
            description: input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            due_at: input.get("due_at").and_then(|v| v.as_i64()),
            completed_at: None,
            status: MilestoneStatus::Pending,
        };

        match svc.add_milestone(project_id, milestone).await {
            Ok(p) => Ok(ToolResult::success(format!(
                "## 里程碑已添加\n\n- **项目**: {}\n- **里程碑数**: {}\n\n可使用 `OpcCompleteMilestone` 标记完成。",
                p.title,
                p.milestones.len(),
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("添加里程碑失败: {e}"))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 仪表盘工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcGetDashboardTool;

#[async_trait]
impl Tool for OpcGetDashboardTool {
    fn name(&self) -> &str {
        "OpcGetDashboard"
    }
    fn description(&self) -> &str {
        "获取一人公司(OPC)的运营概览。返回总收入、发票数、活跃项目、客户总数等关键指标。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({ "type": "object", "properties": {} })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::{
            CustomerFilter, CustomerService, CustomerStatus, InvoiceFilter, InvoiceService,
            InvoiceStatus, ProjectFilter, ProjectService, ProjectStatus,
        };
        use axagent_analysis_engine::opc::{
            DefaultCustomerService, DefaultInvoiceService, DefaultProjectService,
        };

        let db = get_db()?;
        let inv_svc = DefaultInvoiceService::new((*db).clone());
        let cust_svc = DefaultCustomerService::new((*db).clone());
        let proj_svc = DefaultProjectService::new((*db).clone());

        // 并行查询各业务指标
        let (invoices, customers, projects) = tokio::join!(
            inv_svc.list_invoices(InvoiceFilter {
                status: None,
                customer_id: None,
                date_from: None,
                date_to: None,
                limit: Some(100),
                offset: None
            }),
            cust_svc.list_customers(CustomerFilter {
                status: None,
                search: None,
                tags: None,
                limit: Some(100),
                offset: None
            }),
            proj_svc.list_projects(ProjectFilter {
                status: None,
                customer_id: None,
                search: None,
                limit: Some(100),
                offset: None
            }),
        );

        let invoices = invoices.unwrap_or_default();
        let customers = customers.unwrap_or_default();
        let projects = projects.unwrap_or_default();

        let total_revenue: f64 =
            invoices.iter().filter(|i| i.status == InvoiceStatus::Paid).map(|i| i.total).sum();
        let pending_invoices = invoices
            .iter()
            .filter(|i| {
                matches!(
                    i.status,
                    InvoiceStatus::Draft | InvoiceStatus::Sent | InvoiceStatus::Overdue
                )
            })
            .count();
        let active_projects = projects
            .iter()
            .filter(|p| matches!(p.status, ProjectStatus::Active | ProjectStatus::Planning))
            .count();
        let active_customers =
            customers.iter().filter(|c| c.status == CustomerStatus::Active).count();

        Ok(ToolResult::success(format!(
            r#"## 📊 OPC 运营概览

| 指标 | 数值 |
|------|------|
| **总收入 (已收款)** | ¥{total_revenue:.2} |
| **待处理发票** | {pending_invoices} 张 |
| **活跃项目** | {active_projects} 个 |
| **活跃客户** | {active_customers} 位 |
| **客户总数** | {total_customers} 位 |
| **项目总数** | {total_projects} 个 |

> 使用 `OpcListInvoices` / `OpcListCustomers` / `OpcListProjects` 查看详情。"#,
            total_revenue = total_revenue,
            pending_invoices = pending_invoices,
            active_projects = active_projects,
            active_customers = active_customers,
            total_customers = customers.len(),
            total_projects = projects.len(),
        )))
    }
}

// ═══════════════════════════════════════════════════════════════════
// 站点/博客工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcListLandingPagesTool;

#[async_trait]
impl Tool for OpcListLandingPagesTool {
    fn name(&self) -> &str {
        "OpcListLandingPages"
    }
    fn description(&self) -> &str {
        "列出所有落地页。返回标题、slug、发布状态等摘要信息。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultSiteService;
        use axagent_analysis_engine::opc::SiteService;

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());
        match svc.list_landing_pages().await {
            Ok(pages) => {
                if pages.is_empty() {
                    return Ok(ToolResult::success("## 落地页\n\n暂无落地页。"));
                }
                let lines: Vec<String> = pages
                    .iter()
                    .map(|p| {
                        format!(
                            "- **{}** (`{}`) {} | 创建: {}",
                            p.title,
                            p.slug,
                            if p.published {
                                "✅ 已发布"
                            } else {
                                "📝 草稿"
                            },
                            chrono::DateTime::from_timestamp(p.created_at, 0)
                                .map(|dt| dt.format("%Y-%m-%d").to_string())
                                .unwrap_or_default(),
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 落地页 ({} 个)\n\n{}",
                    pages.len(),
                    lines.join("\n")
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询落地页失败: {e}"))),
        }
    }
}

pub struct OpcListBlogPostsTool;

#[async_trait]
impl Tool for OpcListBlogPostsTool {
    fn name(&self) -> &str {
        "OpcListBlogPosts"
    }
    fn description(&self) -> &str {
        "列出所有博客文章。返回标题、slug、标签、发布状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultSiteService;
        use axagent_analysis_engine::opc::SiteService;

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());
        match svc.list_blog_posts().await {
            Ok(posts) => {
                if posts.is_empty() {
                    return Ok(ToolResult::success("## 博客文章\n\n暂无文章。"));
                }
                let lines: Vec<String> = posts
                    .iter()
                    .map(|p| {
                        let tags = p.tags.join(", ");
                        format!(
                            "- **{}** (`{}`) {} | 阅读: {} | 标签: {}",
                            p.title,
                            p.slug,
                            if p.published {
                                "✅ 已发布"
                            } else {
                                "📝 草稿"
                            },
                            p.view_count,
                            tags,
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 博客文章 ({} 篇)\n\n{}",
                    posts.len(),
                    lines.join("\n")
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询博客失败: {e}"))),
        }
    }
}

pub struct OpcListContactsTool;

#[async_trait]
impl Tool for OpcListContactsTool {
    fn name(&self) -> &str {
        "OpcListContacts"
    }
    fn description(&self) -> &str {
        "列出所有联系表单提交。返回姓名、邮箱、来源、读取状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultSiteService;
        use axagent_analysis_engine::opc::SiteService;

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());
        match svc.list_contacts().await {
            Ok(contacts) => {
                if contacts.is_empty() {
                    return Ok(ToolResult::success("## 联系表单\n\n暂无提交。"));
                }
                let lines: Vec<String> = contacts
                    .iter()
                    .map(|c| {
                        format!(
                            "- **{}** <{}> | {} | {}",
                            c.name,
                            c.email,
                            c.source,
                            if c.read { "✅ 已读" } else { "🆕 未读" },
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 联系表单 ({} 条)\n\n{}",
                    contacts.len(),
                    lines.join("\n")
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询联系表单失败: {e}"))),
        }
    }
}

pub struct OpcCreateLandingPageTool;

#[async_trait]
impl Tool for OpcCreateLandingPageTool {
    fn name(&self) -> &str {
        "OpcCreateLandingPage"
    }
    fn description(&self) -> &str {
        "创建新的落地页。需要提供标题、slug 和正文内容，可选描述。创建后为草稿状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "落地页标题" },
                "slug": { "type": "string", "description": "URL slug（小写，空格自动转横线）" },
                "description": { "type": "string", "description": "页面描述（可选）" },
                "content": { "type": "string", "description": "页面正文内容（支持 Markdown）" }
            },
            "required": ["title", "slug", "content"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultSiteService;
        use axagent_analysis_engine::opc::{CreateLandingPageInput, SiteService};

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());

        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: title"))?;
        let slug = input
            .get("slug")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: slug"))?;
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: content"))?;

        let inp = CreateLandingPageInput {
            title: title.to_string(),
            slug: slug.to_string(),
            description: input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            content: content.to_string(),
        };

        match svc.create_landing_page(inp).await {
            Ok(lp) => Ok(ToolResult::success(format!(
                "## 落地页创建成功\n\n- **标题**: {}\n- **Slug**: `{}`\n- **状态**: 📝 草稿\n- **ID**: `{}`",
                lp.title, lp.slug, lp.id,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("创建落地页失败: {e}"))),
        }
    }
}

pub struct OpcCreateBlogPostTool;

#[async_trait]
impl Tool for OpcCreateBlogPostTool {
    fn name(&self) -> &str {
        "OpcCreateBlogPost"
    }
    fn description(&self) -> &str {
        "创建新的博客文章。需要提供标题、slug 和正文内容，可选摘要和标签。创建后为草稿状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "文章标题" },
                "slug": { "type": "string", "description": "URL slug（小写，空格自动转横线）" },
                "excerpt": { "type": "string", "description": "文章摘要（可选）" },
                "content": { "type": "string", "description": "文章正文内容（支持 Markdown）" },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "标签列表（可选）"
                }
            },
            "required": ["title", "slug", "content"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultSiteService;
        use axagent_analysis_engine::opc::{CreateBlogPostInput, SiteService};

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());

        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: title"))?;
        let slug = input
            .get("slug")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: slug"))?;
        let content = input
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: content"))?;

        let tags = input
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let inp = CreateBlogPostInput {
            title: title.to_string(),
            slug: slug.to_string(),
            excerpt: input.get("excerpt").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            content: content.to_string(),
            tags,
        };

        match svc.create_blog_post(inp).await {
            Ok(p) => Ok(ToolResult::success(format!(
                "## 博客文章创建成功\n\n- **标题**: {}\n- **Slug**: `{}`\n- **标签**: {}\n- **状态**: 📝 草稿\n- **ID**: `{}`",
                p.title,
                p.slug,
                if p.tags.is_empty() {
                    "无".to_string()
                } else {
                    p.tags.join(", ")
                },
                p.id,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("创建博客文章失败: {e}"))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 通知发送工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcSendNotificationTool;

#[async_trait]
impl Tool for OpcSendNotificationTool {
    fn name(&self) -> &str {
        "OpcSendNotification"
    }
    fn description(&self) -> &str {
        "通过已配置的消息平台（Telegram/钉钉/飞书等）发送通知给客户或群组。\
         需要先在设置中配置消息渠道。支持的平台：telegram, dingtalk, feishu, slack, discord, wechat, whatsapp, qq。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "platform": {
                    "type": "string",
                    "enum": ["telegram", "dingtalk", "feishu", "slack", "discord", "wechat", "whatsapp", "qq"],
                    "description": "消息平台名称"
                },
                "chat_id": { "type": "string", "description": "目标用户或群的ID" },
                "message": { "type": "string", "description": "消息内容" }
            },
            "required": ["platform", "chat_id", "message"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Communication
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let platform = input
            .get("platform")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: platform"))?
            .to_string();
        let chat_id = input
            .get("chat_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: chat_id"))?
            .to_string();
        let message = input
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: message"))?
            .to_string();

        let tx = get_notify_tx();
        match tx.send(OpcNotification { platform, chat_id, message }) {
            Ok(_) => Ok(ToolResult::success(
                "## 通知已排队发送\n\n消息已提交到通知队列，后台 worker 将异步发送。",
            )),
            Err(e) => Err(ToolError::execution_failed(format!(
                "通知发送失败（队列已满或 worker 未启动）: {e}"
            ))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 分析仪表盘工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcRecordKpiTool;

#[async_trait]
impl Tool for OpcRecordKpiTool {
    fn name(&self) -> &str {
        "OpcRecordKpi"
    }
    fn description(&self) -> &str {
        "记录一个 KPI 指标。需要名称、数值、单位和周期（如 '2026-07'）。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"name":{"type":"string"},"value":{"type":"number"},"unit":{"type":"string"},"period":{"type":"string"}},"required":["name","value","unit","period"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultAnalyticsService;
        use axagent_analysis_engine::opc::{AnalyticsService, CreateKpiInput};
        let db = get_db()?;
        let svc = DefaultAnalyticsService::new((*db).clone());
        let inp = CreateKpiInput {
            name: input.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            value: input.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0),
            unit: input.get("unit").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            period: input.get("period").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        };
        match svc.record_kpi(inp).await {
            Ok(kpi) => Ok(ToolResult::success(format!(
                "## KPI 已记录\n\n- {}: {} {}\n- 周期: {}",
                kpi.name, kpi.value, kpi.unit, kpi.period
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("记录 KPI 失败: {e}"))),
        }
    }
}

pub struct OpcListKpisTool;

#[async_trait]
impl Tool for OpcListKpisTool {
    fn name(&self) -> &str {
        "OpcListKpis"
    }
    fn description(&self) -> &str {
        "查询 KPI 记录。可按周期过滤。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"period":{"type":"string"},"limit":{"type":"integer"}}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::AnalyticsService;
        use axagent_analysis_engine::opc::DefaultAnalyticsService;
        let db = get_db()?;
        let svc = DefaultAnalyticsService::new((*db).clone());
        let period = input.get("period").and_then(|v| v.as_str()).map(String::from);
        let limit = input.get("limit").and_then(|v| v.as_u64()).map(|n| n as u32);
        match svc.list_kpis(period, limit).await {
            Ok(kpis) => {
                if kpis.is_empty() {
                    return Ok(ToolResult::success("## KPI 记录\n\n暂无记录。"));
                }
                Ok(ToolResult::success(format!(
                    "## KPI 记录\n\n{}",
                    kpis.iter()
                        .map(|k| format!("- {}: {} {}", k.name, k.value, k.unit))
                        .collect::<Vec<_>>()
                        .join("\n")
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询 KPI 失败: {e}"))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 知识源工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcSearchWikiTool;

#[async_trait]
impl Tool for OpcSearchWikiTool {
    fn name(&self) -> &str {
        "OpcSearchWiki"
    }
    fn description(&self) -> &str {
        "在 OPC 业务 Wiki 中搜索知识文档（发票规范、客户管理、税法、模板等）。返回最匹配的文档内容摘要。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"query":{"type":"string"},"top_k":{"type":"integer","default":3}},"required":["query"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Knowledge
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = input
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少 query"))?
            .to_string();
        let top_k = input.get("top_k").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        let db = get_db()?;

        // 查找 OPC Wiki ID（走 dao repo，保持 tools 不依赖 entities）
        let wikis = axagent_dao::repo::wiki::list_wikis(&db)
            .await
            .map_err(|e| ToolError::execution_failed(format!("查询 Wiki 失败: {e}")))?;
        let wiki_id = match wikis.iter().find(|w| w.name == "OPC 业务知识库") {
            Some(w) => w.id.clone(),
            None => {
                return Ok(ToolResult::success(
                    "## OPC Wiki\n\nOPC 业务知识库尚未创建。系统将在首次启动后自动创建。",
                ));
            },
        };

        // 在 notes 表中按标题和内容模糊搜索（dao repo 已实现排序 + limit）
        let notes = axagent_dao::repo::note::search_notes(&db, &wiki_id, &query, top_k)
            .await
            .map_err(|e| ToolError::execution_failed(format!("搜索失败: {e}")))?;

        if notes.is_empty() {
            return Ok(ToolResult::success(format!(
                "## OPC 知识库搜索\n\n未找到包含「{query}」的文档。"
            )));
        }

        let lines: Vec<String> = notes
            .iter()
            .map(|n| {
                let excerpt = n
                    .content
                    .lines()
                    .filter(|l| l.contains(&query) || l.starts_with("# "))
                    .take(3)
                    .collect::<Vec<_>>()
                    .join("\n  ");
                format!("- **{}**\n  {}", n.title, excerpt)
            })
            .collect();

        Ok(ToolResult::success(format!(
            "## OPC 知识库搜索「{}」\n\n找到 {} 条结果\n\n{}",
            query,
            notes.len(),
            lines.join("\n\n")
        )))
    }
}

// ═══════════════════════════════════════════════════════════════════
// 金融投资工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcGetFinancialReportTool;

#[async_trait]
impl Tool for OpcGetFinancialReportTool {
    fn name(&self) -> &str {
        "OpcGetFinancialReport"
    }
    fn description(&self) -> &str {
        "生成 OPC 财务报表。按期间（如 '2026-07' 或 '2026-Q2'）统计收入、利润、待收款、可投资金额。给出投资建议。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"period":{"type":"string","description":"期间，如 '2026-07'（月度）或 '2026-Q2'（季度）"}},"required":["period"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::DefaultFinanceService;
        use axagent_analysis_engine::opc::FinanceService;

        let period = input
            .get("period")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少 period"))?;

        let db = get_db()?;
        let svc = DefaultFinanceService::new((*db).clone());
        match svc.get_financial_report(period).await {
            Ok(rpt) => {
                let advice = svc.get_investment_advice(&rpt).await;
                Ok(ToolResult::success(format!(
                    r#"## 📊 OPC 财务报表: {}

| 指标 | 数值 |
|------|------|
| **总收入** | ¥{:.2} |
| **待收款** | ¥{:.2} |
| **退款** | ¥{:.2} |
| **净利润** | **¥{:.2}** |
| **可投资金额** | ¥{:.2} |
| **税前收入(估)** | ¥{:.2} |
| **活跃项目** | {} |
| **新增客户** | {} |
| **客户流失** | {} |

## 💡 投资建议

- **风险等级**: {}
- **建议金额**: ¥{:.2}
- **建议**: {}"#,
                    rpt.period,
                    rpt.total_revenue,
                    rpt.pending_revenue,
                    rpt.refunded_amount,
                    rpt.net_profit,
                    rpt.investable_amount,
                    rpt.pretax_revenue,
                    rpt.active_projects,
                    rpt.new_customers,
                    rpt.churned_customers,
                    advice.risk_level,
                    advice.recommended_amount,
                    advice.advice,
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("生成财务报表失败: {e}"))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 内容资产工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcCreateContentAssetTool;

#[async_trait]
impl Tool for OpcCreateContentAssetTool {
    fn name(&self) -> &str {
        "OpcCreateContentAsset"
    }
    fn description(&self) -> &str {
        "创建新的内容资产（文章、视频、图片等）。需要提供标题、内容类型和正文内容。创建后为草稿状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string", "description": "内容标题" },
                "content_type": {
                    "type": "string",
                    "enum": ["article", "video", "image"],
                    "description": "内容类型"
                },
                "body": { "type": "string", "description": "正文内容（支持 Markdown）" },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "标签列表（可选）"
                }
            },
            "required": ["title", "content_type", "body"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::{
            ContentAssetService, CreateContentAssetInput, DefaultSiteService,
        };

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());

        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: title"))?;
        let content_type = input
            .get("content_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: content_type"))?;
        let body = input
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: body"))?;

        let tags = input
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let inp = CreateContentAssetInput {
            title: title.to_string(),
            content_type: content_type.to_string(),
            body: body.to_string(),
            tags,
        };

        match svc.create_content_asset(inp).await {
            Ok(asset) => Ok(ToolResult::success(format!(
                "## 内容资产创建成功\n\n- **标题**: {}\n- **类型**: {}\n- **状态**: 📝 草稿\n- **ID**: `{}`",
                asset.title, asset.content_type, asset.id,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("创建内容资产失败: {e}"))),
        }
    }
}

pub struct OpcListContentAssetsTool;

#[async_trait]
impl Tool for OpcListContentAssetsTool {
    fn name(&self) -> &str {
        "OpcListContentAssets"
    }
    fn description(&self) -> &str {
        "列出所有内容资产。返回标题、类型、标签、状态等摘要信息。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::{ContentAssetService, DefaultSiteService};

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());
        match svc.list_content_assets().await {
            Ok(assets) => {
                if assets.is_empty() {
                    return Ok(ToolResult::success("## 内容资产\n\n暂无内容资产。"));
                }
                let lines: Vec<String> = assets
                    .iter()
                    .map(|a| {
                        let tags = a.tags.join(", ");
                        format!(
                            "- **{}** | {} | {} | 标签: {}",
                            a.title,
                            a.content_type,
                            if a.status == "published" {
                                "✅ 已发布"
                            } else {
                                "📝 草稿"
                            },
                            tags,
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 内容资产 ({} 个)\n\n{}",
                    assets.len(),
                    lines.join("\n")
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询内容资产失败: {e}"))),
        }
    }
}

pub struct OpcUpdateContentAssetTool;

#[async_trait]
impl Tool for OpcUpdateContentAssetTool {
    fn name(&self) -> &str {
        "OpcUpdateContentAsset"
    }
    fn description(&self) -> &str {
        "更新内容资产。可选择性更新标题、类型、正文、标签和状态。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "内容资产ID" },
                "title": { "type": "string", "description": "新标题（可选）" },
                "content_type": {
                    "type": "string",
                    "enum": ["article", "video", "image"],
                    "description": "新内容类型（可选）"
                },
                "body": { "type": "string", "description": "新正文内容（可选）" },
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "新标签列表（可选）"
                },
                "status": {
                    "type": "string",
                    "enum": ["draft", "published"],
                    "description": "新状态（可选）"
                }
            },
            "required": ["id"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::{
            ContentAssetService, DefaultSiteService, UpdateContentAssetInput,
        };

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());

        let id = input
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: id"))?;

        let inp = UpdateContentAssetInput {
            title: input.get("title").and_then(|v| v.as_str()).map(String::from),
            content_type: input.get("content_type").and_then(|v| v.as_str()).map(String::from),
            body: input.get("body").and_then(|v| v.as_str()).map(String::from),
            tags: input
                .get("tags")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect()),
            status: input.get("status").and_then(|v| v.as_str()).map(String::from),
        };

        match svc.update_content_asset(id, inp).await {
            Ok(asset) => Ok(ToolResult::success(format!(
                "## 内容资产已更新\n\n- **标题**: {}\n- **类型**: {}\n- **状态**: {}\n- **ID**: `{}`",
                asset.title, asset.content_type, asset.status, asset.id,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("更新内容资产失败: {e}"))),
        }
    }
}

pub struct OpcDeleteContentAssetTool;

#[async_trait]
impl Tool for OpcDeleteContentAssetTool {
    fn name(&self) -> &str {
        "OpcDeleteContentAsset"
    }
    fn description(&self) -> &str {
        "删除指定内容资产。需要提供内容资产ID。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "内容资产ID" }
            },
            "required": ["id"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::ContentAssetService;
        use axagent_analysis_engine::opc::DefaultSiteService;

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());

        let id = input
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: id"))?;

        match svc.delete_content_asset(id).await {
            Ok(_) => Ok(ToolResult::success(format!("## 内容资产已删除\n\n- **ID**: `{id}`"))),
            Err(e) => Err(ToolError::execution_failed(format!("删除内容资产失败: {e}"))),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// 发布计划工具
// ═══════════════════════════════════════════════════════════════════

pub struct OpcCreatePublishScheduleTool;

#[async_trait]
impl Tool for OpcCreatePublishScheduleTool {
    fn name(&self) -> &str {
        "OpcCreatePublishSchedule"
    }
    fn description(&self) -> &str {
        "创建新的发布计划。需要提供内容引用类型、ID 和计划发布时间。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "content_ref_type": {
                    "type": "string",
                    "enum": ["blog_post", "content_asset"],
                    "description": "内容引用类型"
                },
                "content_ref_id": { "type": "string", "description": "内容引用ID" },
                "scheduled_at": { "type": "integer", "description": "计划发布时间戳" }
            },
            "required": ["content_ref_type", "content_ref_id", "scheduled_at"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::{
            CreatePublishScheduleInput, DefaultSiteService, PublishScheduleService,
        };

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());

        let content_ref_type = input
            .get("content_ref_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: content_ref_type"))?;
        let content_ref_id = input
            .get("content_ref_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: content_ref_id"))?;
        let scheduled_at = input
            .get("scheduled_at")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: scheduled_at"))?;

        let inp = CreatePublishScheduleInput {
            content_ref_type: content_ref_type.to_string(),
            content_ref_id: content_ref_id.to_string(),
            scheduled_at,
        };

        match svc.create_publish_schedule(inp).await {
            Ok(schedule) => Ok(ToolResult::success(format!(
                "## 发布计划创建成功\n\n- **类型**: {}\n- **计划时间**: {}\n- **状态**: 📋 待发布\n- **ID**: `{}`",
                schedule.content_ref_type,
                chrono::DateTime::from_timestamp(schedule.scheduled_at, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default(),
                schedule.id,
            ))),
            Err(e) => Err(ToolError::execution_failed(format!("创建发布计划失败: {e}"))),
        }
    }
}

pub struct OpcListPublishSchedulesTool;

#[async_trait]
impl Tool for OpcListPublishSchedulesTool {
    fn name(&self) -> &str {
        "OpcListPublishSchedules"
    }
    fn description(&self) -> &str {
        "列出所有发布计划。返回内容类型、计划时间、状态等信息。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }
    fn is_read_only(&self) -> bool {
        true
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::{DefaultSiteService, PublishScheduleService};

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());
        match svc.list_publish_schedules().await {
            Ok(schedules) => {
                if schedules.is_empty() {
                    return Ok(ToolResult::success("## 发布计划\n\n暂无发布计划。"));
                }
                let lines: Vec<String> = schedules
                    .iter()
                    .map(|s| {
                        format!(
                            "- **{}** | {} | {} | 计划: {}{}",
                            s.content_ref_type,
                            s.content_ref_id,
                            if s.status == "published" {
                                "✅ 已发布"
                            } else if s.status == "failed" {
                                "❌ 失败"
                            } else {
                                "📋 待发布"
                            },
                            chrono::DateTime::from_timestamp(s.scheduled_at, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_default(),
                            if let Some(published_at) = s.published_at {
                                format!(
                                    " | 发布于: {}",
                                    chrono::DateTime::from_timestamp(published_at, 0)
                                        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                        .unwrap_or_default()
                                )
                            } else {
                                String::new()
                            },
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 发布计划 ({} 个)\n\n{}",
                    schedules.len(),
                    lines.join("\n")
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("查询发布计划失败: {e}"))),
        }
    }
}

pub struct OpcCancelPublishScheduleTool;

#[async_trait]
impl Tool for OpcCancelPublishScheduleTool {
    fn name(&self) -> &str {
        "OpcCancelPublishSchedule"
    }
    fn description(&self) -> &str {
        "取消/删除发布计划。需要提供发布计划ID。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "发布计划ID" }
            },
            "required": ["id"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::{DefaultSiteService, PublishScheduleService};

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());

        let id = input
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("缺少必需参数: id"))?;

        match svc.delete_publish_schedule(id).await {
            Ok(_) => Ok(ToolResult::success(format!("## 发布计划已取消\n\n- **ID**: `{id}`"))),
            Err(e) => Err(ToolError::execution_failed(format!("取消发布计划失败: {e}"))),
        }
    }
}

pub struct OpcProcessDueSchedulesTool;

#[async_trait]
impl Tool for OpcProcessDueSchedulesTool {
    fn name(&self) -> &str {
        "OpcProcessDueSchedules"
    }
    fn description(&self) -> &str {
        "处理所有到期的发布计划。将状态为pending且计划时间已到的内容发布出去。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{}})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Integration
    }
    fn domain(&self) -> ToolDomain {
        ToolDomain::Automation
    }

    async fn call(&self, _input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        use axagent_analysis_engine::opc::{DefaultSiteService, PublishScheduleService};

        let db = get_db()?;
        let svc = DefaultSiteService::new((*db).clone());

        match svc.process_due_schedules().await {
            Ok(results) => {
                if results.is_empty() {
                    return Ok(ToolResult::success("## 处理发布计划\n\n暂无到期的发布计划。"));
                }
                let lines: Vec<String> = results
                    .iter()
                    .map(|s| {
                        format!(
                            "- **{}** | {} | {}",
                            s.content_ref_type,
                            s.content_ref_id,
                            if s.status == "published" {
                                "✅ 发布成功"
                            } else {
                                "❌ 发布失败"
                            },
                        )
                    })
                    .collect();
                Ok(ToolResult::success(format!(
                    "## 处理发布计划 ({} 个)\n\n{}",
                    results.len(),
                    lines.join("\n")
                )))
            },
            Err(e) => Err(ToolError::execution_failed(format!("处理发布计划失败: {e}"))),
        }
    }
}
