// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 数据服务 — 行业适配器与数据层的桥梁
//!
//! `OpcDataService` 提供行业适配器所需的数据访问接口：
//! - 实体查询（客户数、项目数、发票金额等）
//! - 聚合统计（按时间范围、按状态分组）
//! - 规则评估上下文构建

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, Statement,
};
use serde::{Deserialize, Serialize};

use axagent_entities::{
    opc_blog_posts, opc_content_assets, opc_customers, opc_invoices, opc_landing_pages,
    opc_projects, opc_publish_schedules,
};

use super::customer::CustomerStatus;
use super::error::{OpcError, OpcResult};
use super::invoice::InvoiceStatus;
use super::project::ProjectStatus;

// ── 时间范围 ──────────────────────────────────────────────────

/// 时间范围，用于 KPI 计算和数据聚合
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

impl TimeRange {
    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }

    pub fn days(days: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self { start: now - days * 86400, end: now }
    }

    pub fn hours(hours: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self { start: now - hours * 3600, end: now }
    }
}

// ── 查询上下文 ──────────────────────────────────────────────────

/// 规则评估查询上下文
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleContext {
    pub entity_type: String,
    pub entity_id: String,
    pub status: Option<String>,
    pub overdue_days: Option<u32>,
    pub created_days: Option<u32>,
    pub fields: serde_json::Value,
}

impl RuleContext {
    pub fn new(entity_type: impl Into<String>, entity_id: impl Into<String>) -> Self {
        Self {
            entity_type: entity_type.into(),
            entity_id: entity_id.into(),
            status: None,
            overdue_days: None,
            created_days: None,
            fields: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn with_overdue_days(mut self, days: u32) -> Self {
        self.overdue_days = Some(days);
        self
    }

    pub fn with_created_days(mut self, days: u32) -> Self {
        self.created_days = Some(days);
        self
    }

    pub fn with_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.fields[key.into()] = value;
        self
    }
}

// ── 聚合结果 ──────────────────────────────────────────────────

/// 通用聚合结果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregateResult {
    pub count: u64,
    pub total: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

// ── OpcDataService Trait ──────────────────────────────────────

/// OPC 数据服务 trait
#[async_trait]
pub trait OpcDataService: Send + Sync {
    /// 统计指定时间范围内的客户数量（按状态筛选）
    async fn count_customers(
        &self,
        statuses: &[CustomerStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<u64>;

    /// 统计指定时间范围内的项目数量（按状态筛选）
    async fn count_projects(
        &self,
        statuses: &[ProjectStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<u64>;

    /// 统计指定时间范围内的发票数量（按状态筛选）
    async fn count_invoices(
        &self,
        statuses: &[InvoiceStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<u64>;

    /// 聚合指定时间范围内的发票金额
    async fn aggregate_invoice_amounts(
        &self,
        statuses: &[InvoiceStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<AggregateResult>;

    /// 聚合指定时间范围内的项目预算
    async fn aggregate_project_budgets(
        &self,
        statuses: &[ProjectStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<AggregateResult>;

    /// 聚合指定客户的总营收
    async fn aggregate_customer_revenue(&self, customer_id: &str) -> OpcResult<f64>;

    /// 获取实体的规则评估上下文
    async fn get_rule_context(&self, entity_type: &str, entity_id: &str) -> OpcResult<RuleContext>;

    /// 检查字段值在指定表中是否唯一
    async fn is_field_unique(
        &self,
        entity_type: &str,
        field: &str,
        value: &str,
        exclude_id: Option<&str>,
    ) -> OpcResult<bool>;

    /// 检查关联实体是否存在
    async fn check_relation_exists(
        &self,
        parent_type: &str,
        parent_id: &str,
        child_type: &str,
        child_id: &str,
    ) -> OpcResult<bool>;

    /// 更新实体状态
    async fn update_entity_status(
        &self,
        entity_type: &str,
        entity_id: &str,
        new_status: &str,
    ) -> OpcResult<()>;

    /// 创建实体记录
    async fn create_entity_record(
        &self,
        entity_type: &str,
        data: &serde_json::Value,
    ) -> OpcResult<String>;

    /// 统计指定时间范围内的博客文章数量（按发布时间筛选）
    async fn count_blog_posts(&self, from: i64, to: i64) -> OpcResult<u64>;

    /// 聚合指定时间范围内的博客文章总阅读量
    async fn sum_blog_post_views(&self, from: i64, to: i64) -> OpcResult<f64>;

    /// 统计内容资产总数（按类型分组可选）
    async fn count_content_assets(&self, from: i64, to: i64) -> OpcResult<u64>;

    /// 统计落地页数量
    async fn count_landing_pages(&self, from: i64, to: i64) -> OpcResult<u64>;

    /// 统计待发布的发布计划数（status=pending）
    async fn count_publish_schedules_pending(&self) -> OpcResult<u64>;

    /// 统计已发布的发布计划数（status=published）
    async fn count_publish_schedules_published(&self, from: i64, to: i64) -> OpcResult<u64>;
}

// ── Mock 实现（测试用） ──────────────────────────────────────

/// Mock 数据服务，用于测试行业适配器
#[derive(Debug)]
pub struct MockDataService {
    pub customer_count: u64,
    pub project_count: u64,
    pub invoice_count: u64,
    pub invoice_total: f64,
    pub invoice_average: f64,
    pub invoice_min: f64,
    pub invoice_max: f64,
    pub project_total: f64,
    pub customer_revenue: f64,
    pub blog_post_count: u64,
    pub blog_post_views: f64,
    pub content_assets_count: u64,
    pub landing_pages_count: u64,
    pub publish_schedules_pending: u64,
    pub publish_schedules_published: u64,
}

impl Default for MockDataService {
    fn default() -> Self {
        Self {
            customer_count: 100,
            project_count: 50,
            invoice_count: 200,
            invoice_total: 100000.0,
            invoice_average: 500.0,
            invoice_min: 100.0,
            invoice_max: 5000.0,
            project_total: 500000.0,
            customer_revenue: 15000.0,
            blog_post_count: 25,
            blog_post_views: 12500.0,
            content_assets_count: 42,
            landing_pages_count: 8,
            publish_schedules_pending: 5,
            publish_schedules_published: 12,
        }
    }
}

#[async_trait]
impl OpcDataService for MockDataService {
    async fn count_customers(
        &self,
        _statuses: &[CustomerStatus],
        _from: i64,
        _to: i64,
    ) -> OpcResult<u64> {
        Ok(self.customer_count)
    }

    async fn count_projects(
        &self,
        _statuses: &[ProjectStatus],
        _from: i64,
        _to: i64,
    ) -> OpcResult<u64> {
        Ok(self.project_count)
    }

    async fn count_invoices(
        &self,
        _statuses: &[InvoiceStatus],
        _from: i64,
        _to: i64,
    ) -> OpcResult<u64> {
        Ok(self.invoice_count)
    }

    async fn aggregate_invoice_amounts(
        &self,
        _statuses: &[InvoiceStatus],
        _from: i64,
        _to: i64,
    ) -> OpcResult<AggregateResult> {
        Ok(AggregateResult {
            count: self.invoice_count,
            total: self.invoice_total,
            average: self.invoice_average,
            min: self.invoice_min,
            max: self.invoice_max,
        })
    }

    async fn aggregate_project_budgets(
        &self,
        _statuses: &[ProjectStatus],
        _from: i64,
        _to: i64,
    ) -> OpcResult<AggregateResult> {
        Ok(AggregateResult {
            count: self.project_count,
            total: self.project_total,
            average: self.project_total / self.project_count.max(1) as f64,
            min: 1000.0,
            max: 100000.0,
        })
    }

    async fn aggregate_customer_revenue(&self, _customer_id: &str) -> OpcResult<f64> {
        Ok(self.customer_revenue)
    }

    async fn get_rule_context(&self, entity_type: &str, entity_id: &str) -> OpcResult<RuleContext> {
        let mut ctx = RuleContext::new(entity_type, entity_id);
        ctx.fields = serde_json::json!({
            "mock": true,
            "entity_type": entity_type,
            "entity_id": entity_id,
        });
        Ok(ctx)
    }

    async fn is_field_unique(
        &self,
        _entity_type: &str,
        _field: &str,
        _value: &str,
        _exclude_id: Option<&str>,
    ) -> OpcResult<bool> {
        Ok(true)
    }

    async fn check_relation_exists(
        &self,
        _parent_type: &str,
        _parent_id: &str,
        _child_type: &str,
        _child_id: &str,
    ) -> OpcResult<bool> {
        Ok(true)
    }

    async fn update_entity_status(
        &self,
        _entity_type: &str,
        _entity_id: &str,
        _new_status: &str,
    ) -> OpcResult<()> {
        Ok(())
    }

    async fn create_entity_record(
        &self,
        entity_type: &str,
        _data: &serde_json::Value,
    ) -> OpcResult<String> {
        Ok(format!("mock-{}-{}", entity_type, uuid::Uuid::new_v4()))
    }

    async fn count_blog_posts(&self, _from: i64, _to: i64) -> OpcResult<u64> {
        Ok(self.blog_post_count)
    }

    async fn sum_blog_post_views(&self, _from: i64, _to: i64) -> OpcResult<f64> {
        Ok(self.blog_post_views)
    }

    async fn count_content_assets(&self, _from: i64, _to: i64) -> OpcResult<u64> {
        Ok(self.content_assets_count)
    }

    async fn count_landing_pages(&self, _from: i64, _to: i64) -> OpcResult<u64> {
        Ok(self.landing_pages_count)
    }

    async fn count_publish_schedules_pending(&self) -> OpcResult<u64> {
        Ok(self.publish_schedules_pending)
    }

    async fn count_publish_schedules_published(&self, _from: i64, _to: i64) -> OpcResult<u64> {
        Ok(self.publish_schedules_published)
    }
}

// ── SeaORM 实现 ──────────────────────────────────────────────

/// 默认数据服务实现（SeaORM）
pub struct DefaultDataService {
    pub db: DatabaseConnection,
}

impl DefaultDataService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// 根据发票状态选择业务时间列：已付款按 `paid_at`，已发出/逾期按 `issued_at`，其余按 `created_at`
    fn invoice_time_column(statuses: &[InvoiceStatus]) -> opc_invoices::Column {
        if statuses.contains(&InvoiceStatus::Paid) {
            opc_invoices::Column::PaidAt
        } else if statuses.contains(&InvoiceStatus::Sent)
            || statuses.contains(&InvoiceStatus::Overdue)
        {
            opc_invoices::Column::IssuedAt
        } else {
            opc_invoices::Column::CreatedAt
        }
    }

    /// 根据项目状态选择业务时间列：已完成按 `completed_at`，进行中按 `started_at`，其余按 `created_at`
    fn project_time_column(statuses: &[ProjectStatus]) -> opc_projects::Column {
        if statuses.contains(&ProjectStatus::Completed) {
            opc_projects::Column::CompletedAt
        } else if statuses.contains(&ProjectStatus::Active) {
            opc_projects::Column::StartedAt
        } else {
            opc_projects::Column::CreatedAt
        }
    }
}

#[async_trait]
impl OpcDataService for DefaultDataService {
    async fn count_customers(
        &self,
        statuses: &[CustomerStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<u64> {
        let status_strs: Vec<&str> = statuses.iter().map(|s| s.as_str()).collect();
        let mut query = opc_customers::Entity::find()
            .filter(opc_customers::Column::CreatedAt.between(from, to));

        if !status_strs.is_empty() {
            query = query.filter(opc_customers::Column::Status.is_in(status_strs));
        }

        let count = query.count(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(count)
    }

    async fn count_projects(
        &self,
        statuses: &[ProjectStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<u64> {
        let status_strs: Vec<&str> = statuses.iter().map(|s| s.as_str()).collect();
        let time_col = Self::project_time_column(statuses);
        let mut query = opc_projects::Entity::find().filter(time_col.between(from, to));

        if !status_strs.is_empty() {
            query = query.filter(opc_projects::Column::Status.is_in(status_strs));
        }

        let count = query.count(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(count)
    }

    async fn count_invoices(
        &self,
        statuses: &[InvoiceStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<u64> {
        let status_strs: Vec<&str> = statuses.iter().map(|s| s.as_str()).collect();
        let time_col = Self::invoice_time_column(statuses);
        let mut query = opc_invoices::Entity::find().filter(time_col.between(from, to));

        if !status_strs.is_empty() {
            query = query.filter(opc_invoices::Column::Status.is_in(status_strs));
        }

        let count = query.count(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(count)
    }

    async fn aggregate_invoice_amounts(
        &self,
        statuses: &[InvoiceStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<AggregateResult> {
        let status_strs: Vec<&str> = statuses.iter().map(|s| s.as_str()).collect();

        let time_col = Self::invoice_time_column(statuses);
        let mut query = opc_invoices::Entity::find().filter(time_col.between(from, to));

        if !status_strs.is_empty() {
            query = query.filter(opc_invoices::Column::Status.is_in(status_strs));
        }

        let invoices = query.all(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;

        let count = invoices.len() as u64;
        let totals: Vec<f64> = invoices.iter().map(|inv| inv.total).collect();
        let total = totals.iter().sum();
        let average = if count > 0 { total / count as f64 } else { 0.0 };
        let min = totals.iter().copied().fold(f64::INFINITY, f64::min);
        let max = totals.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        Ok(AggregateResult {
            count,
            total,
            average,
            min: if min == f64::INFINITY { 0.0 } else { min },
            max: if max == f64::NEG_INFINITY { 0.0 } else { max },
        })
    }

    async fn aggregate_project_budgets(
        &self,
        statuses: &[ProjectStatus],
        from: i64,
        to: i64,
    ) -> OpcResult<AggregateResult> {
        let status_strs: Vec<&str> = statuses.iter().map(|s| s.as_str()).collect();

        let time_col = Self::project_time_column(statuses);
        let mut query = opc_projects::Entity::find().filter(time_col.between(from, to));

        if !status_strs.is_empty() {
            query = query.filter(opc_projects::Column::Status.is_in(status_strs));
        }

        let projects = query.all(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;

        let count = projects.len() as u64;
        let budgets: Vec<f64> = projects.iter().map(|p| p.budget.unwrap_or(0.0)).collect();
        let total = budgets.iter().sum();
        let average = if count > 0 { total / count as f64 } else { 0.0 };
        let min = budgets.iter().copied().fold(f64::INFINITY, f64::min);
        let max = budgets.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        Ok(AggregateResult {
            count,
            total,
            average,
            min: if min == f64::INFINITY { 0.0 } else { min },
            max: if max == f64::NEG_INFINITY { 0.0 } else { max },
        })
    }

    async fn aggregate_customer_revenue(&self, customer_id: &str) -> OpcResult<f64> {
        let invoices = opc_invoices::Entity::find()
            .filter(opc_invoices::Column::CustomerId.eq(customer_id))
            .filter(opc_invoices::Column::Status.is_in(["paid", "sent"]))
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;

        let total: f64 = invoices.iter().map(|inv| inv.total).sum();
        Ok(total)
    }

    async fn get_rule_context(&self, entity_type: &str, entity_id: &str) -> OpcResult<RuleContext> {
        let ctx = match entity_type {
            "customer" => {
                let model = opc_customers::Entity::find_by_id(entity_id)
                    .one(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?;

                if let Some(c) = model {
                    let mut ctx =
                        RuleContext::new(entity_type, entity_id).with_status(c.status.clone());
                    ctx.fields = serde_json::json!({
                        "id": c.id,
                        "name": c.name,
                        "email": c.email,
                        "company": c.company,
                        "status": c.status,
                        "total_revenue": c.total_revenue,
                        "invoice_count": c.invoice_count,
                        "tags": c.tags_json,
                    });
                    ctx
                } else {
                    RuleContext::new(entity_type, entity_id)
                }
            },
            "project" => {
                let model = opc_projects::Entity::find_by_id(entity_id)
                    .one(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?;

                if let Some(p) = model {
                    let started_at = p.started_at.unwrap_or(p.created_at);
                    let now = chrono::Utc::now().timestamp();
                    let created_days = ((now - started_at) / 86400).max(0) as u32;

                    let mut ctx = RuleContext::new(entity_type, entity_id)
                        .with_status(p.status.clone())
                        .with_created_days(created_days);
                    ctx.fields = serde_json::json!({
                        "id": p.id,
                        "title": p.title,
                        "customer_id": p.customer_id,
                        "status": p.status,
                        "budget": p.budget,
                        "currency": p.currency,
                    });
                    ctx
                } else {
                    RuleContext::new(entity_type, entity_id)
                }
            },
            "invoice" => {
                let model = opc_invoices::Entity::find_by_id(entity_id)
                    .one(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?;

                if let Some(inv) = model {
                    let now = chrono::Utc::now().timestamp();
                    let created_days = ((now - inv.created_at) / 86400).max(0) as u32;

                    let mut ctx = RuleContext::new(entity_type, entity_id)
                        .with_status(inv.status.clone())
                        .with_created_days(created_days);

                    if let Some(due_at) = inv.due_at {
                        let overdue_days = ((now - due_at) / 86400).max(0) as u32;
                        ctx = ctx.with_overdue_days(overdue_days);
                    }

                    ctx.fields = serde_json::json!({
                        "id": inv.id,
                        "customer_id": inv.customer_id,
                        "invoice_number": inv.invoice_number,
                        "status": inv.status,
                        "subtotal": inv.subtotal,
                        "tax_total": inv.tax_total,
                        "total": inv.total,
                        "currency": inv.currency,
                    });
                    ctx
                } else {
                    RuleContext::new(entity_type, entity_id)
                }
            },
            _ => RuleContext::new(entity_type, entity_id),
        };

        Ok(ctx)
    }

    async fn is_field_unique(
        &self,
        entity_type: &str,
        field: &str,
        value: &str,
        exclude_id: Option<&str>,
    ) -> OpcResult<bool> {
        let table_name = match entity_type {
            "customer" => "opc_customers",
            "project" => "opc_projects",
            "invoice" => "opc_invoices",
            _ => return Ok(true),
        };

        let (sql, values) = if let Some(exclude) = exclude_id {
            (
                format!("SELECT id FROM {} WHERE {} = $1 AND id != $2 LIMIT 1", table_name, field),
                vec![sea_orm::Value::from(value), sea_orm::Value::from(exclude)],
            )
        } else {
            (
                format!("SELECT id FROM {} WHERE {} = $1 LIMIT 1", table_name, field),
                vec![sea_orm::Value::from(value)],
            )
        };

        let backend = self.db.get_database_backend();
        let stmt = Statement::from_sql_and_values(backend, sql, values);
        let row =
            self.db.query_one_raw(stmt).await.map_err(|e| OpcError::Database(e.to_string()))?;

        Ok(row.is_none())
    }

    async fn check_relation_exists(
        &self,
        parent_type: &str,
        _parent_id: &str,
        child_type: &str,
        child_id: &str,
    ) -> OpcResult<bool> {
        match (parent_type, child_type) {
            ("customer", "invoice") => {
                let exists = opc_invoices::Entity::find_by_id(child_id)
                    .one(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(exists.is_some())
            },
            ("customer", "project") => {
                let exists = opc_projects::Entity::find_by_id(child_id)
                    .one(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(exists.is_some())
            },
            _ => Ok(false),
        }
    }

    async fn update_entity_status(
        &self,
        entity_type: &str,
        entity_id: &str,
        new_status: &str,
    ) -> OpcResult<()> {
        let now = chrono::Utc::now().timestamp();
        match entity_type {
            "customer" => {
                let model = opc_customers::Entity::find_by_id(entity_id)
                    .one(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?
                    .ok_or_else(|| {
                        OpcError::NotFound(format!("customer not found: {}", entity_id))
                    })?;

                let mut am: opc_customers::ActiveModel = model.into();
                am.status = sea_orm::Set(new_status.to_string());
                am.updated_at = sea_orm::Set(now);
                am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(())
            },
            "project" => {
                let model = opc_projects::Entity::find_by_id(entity_id)
                    .one(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?
                    .ok_or_else(|| {
                        OpcError::NotFound(format!("project not found: {}", entity_id))
                    })?;

                let mut am: opc_projects::ActiveModel = model.into();
                am.status = sea_orm::Set(new_status.to_string());
                am.updated_at = sea_orm::Set(now);
                am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(())
            },
            "invoice" => {
                let model = opc_invoices::Entity::find_by_id(entity_id)
                    .one(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?
                    .ok_or_else(|| {
                        OpcError::NotFound(format!("invoice not found: {}", entity_id))
                    })?;

                let mut am: opc_invoices::ActiveModel = model.into();
                am.status = sea_orm::Set(new_status.to_string());
                am.updated_at = sea_orm::Set(now);
                am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(())
            },
            _ => {
                tracing::warn!(
                    "[opc-data-service] update_entity_status: 不支持的实体类型: {}",
                    entity_type
                );
                Ok(())
            },
        }
    }

    async fn create_entity_record(
        &self,
        entity_type: &str,
        data: &serde_json::Value,
    ) -> OpcResult<String> {
        let now = chrono::Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();

        match entity_type {
            "customer" => {
                let new_customer = opc_customers::ActiveModel {
                    id: sea_orm::Set(id.clone()),
                    name: sea_orm::Set(data["name"].as_str().unwrap_or("Unknown").to_string()),
                    email: sea_orm::Set(data["email"].as_str().unwrap_or("").to_string()),
                    phone: sea_orm::Set(data["phone"].as_str().map(|s| s.to_string())),
                    company: sea_orm::Set(data["company"].as_str().map(|s| s.to_string())),
                    source: sea_orm::Set(data["source"].as_str().map(|s| s.to_string())),
                    tags_json: sea_orm::Set(
                        data["tags"]
                            .as_array()
                            .map(|t| serde_json::to_string(t).unwrap_or_default())
                            .unwrap_or_else(|| "[]".to_string()),
                    ),
                    notes: sea_orm::Set(data["notes"].as_str().unwrap_or("").to_string()),
                    total_revenue: sea_orm::Set(data["total_revenue"].as_f64().unwrap_or(0.0)),
                    invoice_count: sea_orm::Set(data["invoice_count"].as_i64().unwrap_or(0) as u32),
                    status: sea_orm::Set(data["status"].as_str().unwrap_or("active").to_string()),
                    created_at: sea_orm::Set(now),
                    updated_at: sea_orm::Set(now),
                };

                new_customer
                    .insert(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(id)
            },
            "project" => {
                let new_project = opc_projects::ActiveModel {
                    id: sea_orm::Set(id.clone()),
                    customer_id: sea_orm::Set(data["customer_id"].as_str().map(|s| s.to_string())),
                    title: sea_orm::Set(data["title"].as_str().unwrap_or("Untitled").to_string()),
                    description: sea_orm::Set(
                        data["description"].as_str().unwrap_or("").to_string(),
                    ),
                    status: sea_orm::Set(data["status"].as_str().unwrap_or("planned").to_string()),
                    milestones_json: sea_orm::Set(
                        data["milestones"]
                            .as_array()
                            .map(|m| serde_json::to_string(m).unwrap_or_default())
                            .unwrap_or_else(|| "[]".to_string()),
                    ),
                    budget: sea_orm::Set(data["budget"].as_f64()),
                    currency: sea_orm::Set(data["currency"].as_str().unwrap_or("CNY").to_string()),
                    started_at: sea_orm::Set(data["started_at"].as_i64()),
                    deadline: sea_orm::Set(data["deadline"].as_i64()),
                    completed_at: sea_orm::Set(data["completed_at"].as_i64()),
                    notes: sea_orm::Set(data["notes"].as_str().unwrap_or("").to_string()),
                    created_at: sea_orm::Set(now),
                    updated_at: sea_orm::Set(now),
                };

                new_project
                    .insert(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(id)
            },
            "invoice" => {
                let new_invoice = opc_invoices::ActiveModel {
                    id: sea_orm::Set(id.clone()),
                    customer_id: sea_orm::Set(
                        data["customer_id"].as_str().unwrap_or("").to_string(),
                    ),
                    invoice_number: sea_orm::Set(
                        data["invoice_number"].as_str().unwrap_or("").to_string(),
                    ),
                    status: sea_orm::Set(data["status"].as_str().unwrap_or("draft").to_string()),
                    line_items_json: sea_orm::Set(
                        data["line_items"]
                            .as_array()
                            .map(|l| serde_json::to_string(l).unwrap_or_default())
                            .unwrap_or_else(|| "[]".to_string()),
                    ),
                    subtotal: sea_orm::Set(data["subtotal"].as_f64().unwrap_or(0.0)),
                    tax_total: sea_orm::Set(data["tax_total"].as_f64().unwrap_or(0.0)),
                    total: sea_orm::Set(data["total"].as_f64().unwrap_or(0.0)),
                    currency: sea_orm::Set(data["currency"].as_str().unwrap_or("CNY").to_string()),
                    issued_at: sea_orm::Set(data["issued_at"].as_i64()),
                    due_at: sea_orm::Set(data["due_at"].as_i64()),
                    paid_at: sea_orm::Set(data["paid_at"].as_i64()),
                    notes: sea_orm::Set(data["notes"].as_str().unwrap_or("").to_string()),
                    created_at: sea_orm::Set(now),
                    updated_at: sea_orm::Set(now),
                };

                new_invoice
                    .insert(&self.db)
                    .await
                    .map_err(|e| OpcError::Database(e.to_string()))?;
                Ok(id)
            },
            _ => {
                tracing::warn!(
                    "[opc-data-service] create_entity_record: 不支持的实体类型: {}",
                    entity_type
                );
                Err(OpcError::NotFound(format!("不支持的实体类型: {}", entity_type)))
            },
        }
    }

    async fn count_blog_posts(&self, from: i64, to: i64) -> OpcResult<u64> {
        let count = opc_blog_posts::Entity::find()
            .filter(opc_blog_posts::Column::PublishedAt.between(from, to))
            .filter(opc_blog_posts::Column::Published.eq(true))
            .count(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(count)
    }

    async fn sum_blog_post_views(&self, from: i64, to: i64) -> OpcResult<f64> {
        let posts = opc_blog_posts::Entity::find()
            .filter(opc_blog_posts::Column::PublishedAt.between(from, to))
            .filter(opc_blog_posts::Column::Published.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        let total: u32 = posts.iter().map(|p| p.view_count).sum();
        Ok(total as f64)
    }

    async fn count_content_assets(&self, from: i64, to: i64) -> OpcResult<u64> {
        let count = opc_content_assets::Entity::find()
            .filter(opc_content_assets::Column::CreatedAt.between(from, to))
            .count(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(count)
    }

    async fn count_landing_pages(&self, from: i64, to: i64) -> OpcResult<u64> {
        let count = opc_landing_pages::Entity::find()
            .filter(opc_landing_pages::Column::CreatedAt.between(from, to))
            .count(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(count)
    }

    async fn count_publish_schedules_pending(&self) -> OpcResult<u64> {
        let count = opc_publish_schedules::Entity::find()
            .filter(opc_publish_schedules::Column::Status.eq("pending"))
            .count(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(count)
    }

    async fn count_publish_schedules_published(&self, from: i64, to: i64) -> OpcResult<u64> {
        let count = opc_publish_schedules::Entity::find()
            .filter(opc_publish_schedules::Column::Status.eq("published"))
            .filter(opc_publish_schedules::Column::PublishedAt.between(from, to))
            .count(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;
        Ok(count)
    }
}
