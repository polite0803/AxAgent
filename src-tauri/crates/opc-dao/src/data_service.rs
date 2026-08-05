// SPDX-License-Identifier: AGPL-3.0-only

//! OpcDataService 的 SeaORM 实现
//!
//! 为行业适配器提供数据访问能力，使用 SeaORM 查询数据库。

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, Statement,
};

use axagent_opc_entities::{opc_customers, opc_invoices, opc_projects};
use axagent_opc_types::{
    AggregateResult, CustomerStatus, InvoiceStatus, OpcDataService, OpcError, OpcResult,
    ProjectStatus, RuleContext,
};

/// 默认数据服务实现
pub struct DefaultDataService {
    pub db: DatabaseConnection,
}

impl DefaultDataService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
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
        let mut query =
            opc_projects::Entity::find().filter(opc_projects::Column::CreatedAt.between(from, to));

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
        let mut query =
            opc_invoices::Entity::find().filter(opc_invoices::Column::CreatedAt.between(from, to));

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

        let mut query =
            opc_invoices::Entity::find().filter(opc_invoices::Column::CreatedAt.between(from, to));

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

        let mut query =
            opc_projects::Entity::find().filter(opc_projects::Column::CreatedAt.between(from, to));

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
}
