// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 业务领域数据访问层
//!
//! 实现 opc-types 中定义的 Service trait，提供 SeaORM 完整的 CRUD。
//! 包含 Entity ↔ DTO 转换、状态机验证、JSON 字段序列化。

pub mod analytics_service;
pub mod audit_log_service;
pub mod automation_service;
pub mod customer_service;
pub mod data_service;
pub mod finance_service;
pub mod invoice_service;
pub mod project_service;
pub mod rules;
pub mod site_service;

pub use analytics_service::DefaultAnalyticsService;
pub use audit_log_service::AuditLogService;
pub use automation_service::DbAutomationService;
pub use customer_service::DefaultCustomerService;
pub use data_service::DefaultDataService;
pub use finance_service::DefaultFinanceService;
pub use invoice_service::DefaultInvoiceService;
pub use project_service::DefaultProjectService;
pub use site_service::DefaultSiteService;
