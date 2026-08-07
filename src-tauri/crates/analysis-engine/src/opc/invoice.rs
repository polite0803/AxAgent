// SPDX-License-Identifier: AGPL-3.0-only

//! 发票/账单领域 — DTO 定义、trait 接口与 SeaORM 实现

use async_trait::async_trait;
use sea_orm::QuerySelect;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use axagent_entities::opc_invoices;
use axagent_harness::util_fns::{gen_id, now_ts};

use super::error::{OpcError, OpcResult};

// ── DTO 定义 ──────────────────────────────────────────────────

/// 发票状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Paid,
    Overdue,
    Cancelled,
    Refunded,
}

impl InvoiceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Sent => "sent",
            Self::Paid => "paid",
            Self::Overdue => "overdue",
            Self::Cancelled => "cancelled",
            Self::Refunded => "refunded",
        }
    }

    pub fn can_transition_to(&self, target: &Self) -> bool {
        matches!(
            (self, target),
            (Self::Draft, Self::Sent)
                | (Self::Draft, Self::Cancelled)
                | (Self::Sent, Self::Paid)
                | (Self::Sent, Self::Overdue)
                | (Self::Sent, Self::Cancelled)
                | (Self::Overdue, Self::Paid)
                | (Self::Overdue, Self::Cancelled)
                | (Self::Paid, Self::Refunded)
        )
    }
}

impl FromStr for InvoiceStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "sent" => Ok(Self::Sent),
            "paid" => Ok(Self::Paid),
            "overdue" => Ok(Self::Overdue),
            "cancelled" => Ok(Self::Cancelled),
            "refunded" => Ok(Self::Refunded),
            _ => Err(format!("Unknown InvoiceStatus: {s}")),
        }
    }
}

/// 发票行项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLineItem {
    pub description: String,
    pub quantity: f64,
    pub unit_price: f64,
    pub tax_rate: f64,
    pub total: f64,
}

/// 发票 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub customer_id: String,
    pub invoice_number: String,
    pub status: InvoiceStatus,
    pub line_items: Vec<InvoiceLineItem>,
    pub subtotal: f64,
    pub tax_total: f64,
    pub total: f64,
    pub currency: String,
    pub issued_at: Option<i64>,
    pub due_at: Option<i64>,
    pub paid_at: Option<i64>,
    pub notes: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 创建发票请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInvoiceInput {
    pub customer_id: String,
    pub line_items: Vec<InvoiceLineItem>,
    pub currency: String,
    pub due_at: Option<i64>,
    pub notes: String,
}

/// 更新发票请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInvoiceInput {
    pub line_items: Option<Vec<InvoiceLineItem>>,
    pub notes: Option<String>,
    pub due_at: Option<Option<i64>>,
}

/// 发票查询过滤
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InvoiceFilter {
    pub status: Option<InvoiceStatus>,
    pub customer_id: Option<String>,
    pub date_from: Option<i64>,
    pub date_to: Option<i64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// ── Invoice Service Trait ──────────────────────────────────────────

#[async_trait]
pub trait InvoiceService: Send + Sync {
    async fn create_invoice(&self, input: CreateInvoiceInput) -> OpcResult<Invoice>;
    async fn get_invoice(&self, id: &str) -> OpcResult<Invoice>;
    async fn list_invoices(&self, filter: InvoiceFilter) -> OpcResult<Vec<Invoice>>;
    async fn update_invoice(&self, id: &str, input: UpdateInvoiceInput) -> OpcResult<Invoice>;
    async fn delete_invoice(&self, id: &str) -> OpcResult<()>;
    async fn transition_status(&self, id: &str, target: InvoiceStatus) -> OpcResult<Invoice>;
}

/// Noop 实现
#[derive(Debug)]
pub struct NoopInvoiceService;

#[async_trait]
impl InvoiceService for NoopInvoiceService {
    async fn create_invoice(&self, _input: CreateInvoiceInput) -> OpcResult<Invoice> {
        Err(OpcError::NotFound("InvoiceService not implemented".into()))
    }
    async fn get_invoice(&self, _id: &str) -> OpcResult<Invoice> {
        Err(OpcError::NotFound("InvoiceService not implemented".into()))
    }
    async fn list_invoices(&self, _filter: InvoiceFilter) -> OpcResult<Vec<Invoice>> {
        Ok(Vec::new())
    }
    async fn update_invoice(&self, _id: &str, _input: UpdateInvoiceInput) -> OpcResult<Invoice> {
        Err(OpcError::NotFound("InvoiceService not implemented".into()))
    }
    async fn delete_invoice(&self, _id: &str) -> OpcResult<()> {
        Err(OpcError::NotFound("InvoiceService not implemented".into()))
    }
    async fn transition_status(&self, _id: &str, _target: InvoiceStatus) -> OpcResult<Invoice> {
        Err(OpcError::NotFound("InvoiceService not implemented".into()))
    }
}

// ── SeaORM 实现 ───────────────────────────────────────────────────

/// 默认发票服务实现
pub struct DefaultInvoiceService {
    pub db: DatabaseConnection,
}

impl DefaultInvoiceService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn entity_to_dto(e: opc_invoices::Model) -> OpcResult<Invoice> {
    let line_items: Vec<InvoiceLineItem> =
        serde_json::from_str(&e.line_items_json).unwrap_or_default();

    let status = InvoiceStatus::from_str(&e.status).unwrap_or(InvoiceStatus::Draft);

    Ok(Invoice {
        id: e.id,
        customer_id: e.customer_id,
        invoice_number: e.invoice_number,
        status,
        line_items,
        subtotal: e.subtotal,
        tax_total: e.tax_total,
        total: e.total,
        currency: e.currency,
        issued_at: e.issued_at,
        due_at: e.due_at,
        paid_at: e.paid_at,
        notes: e.notes,
        created_at: e.created_at,
        updated_at: e.updated_at,
    })
}

fn calculate_totals(items: &[InvoiceLineItem]) -> (f64, f64, f64) {
    let mut subtotal = 0.0;
    let mut tax_total = 0.0;
    for item in items {
        let line_total = item.quantity * item.unit_price;
        subtotal += line_total;
        tax_total += line_total * item.tax_rate;
    }
    let total = subtotal + tax_total;
    (subtotal, tax_total, total)
}

#[async_trait]
impl InvoiceService for DefaultInvoiceService {
    async fn create_invoice(&self, input: CreateInvoiceInput) -> OpcResult<Invoice> {
        let id = gen_id();
        let now = now_ts();
        let (subtotal, tax_total, total) = calculate_totals(&input.line_items);

        let ts = chrono::Utc::now();
        let inv_num = format!("INV-{}-{}", ts.format("%Y%m%d"), id[..6].to_uppercase());

        let line_items_json = serde_json::to_string(&input.line_items)
            .map_err(|e| OpcError::Database(e.to_string()))?;

        opc_invoices::ActiveModel {
            id: Set(id.clone()),
            customer_id: Set(input.customer_id),
            invoice_number: Set(inv_num),
            status: Set(InvoiceStatus::Draft.as_str().to_string()),
            line_items_json: Set(line_items_json),
            subtotal: Set(subtotal),
            tax_total: Set(tax_total),
            total: Set(total),
            currency: Set(input.currency),
            issued_at: Set(None),
            due_at: Set(input.due_at),
            paid_at: Set(None),
            notes: Set(input.notes),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await
        .map_err(|e| OpcError::Database(e.to_string()))?;

        self.get_invoice(&id).await
    }

    async fn get_invoice(&self, id: &str) -> OpcResult<Invoice> {
        let entity = opc_invoices::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("invoice {id}")))?;

        entity_to_dto(entity)
    }

    async fn list_invoices(&self, filter: InvoiceFilter) -> OpcResult<Vec<Invoice>> {
        let mut query = opc_invoices::Entity::find().order_by_desc(opc_invoices::Column::CreatedAt);

        if let Some(status) = &filter.status {
            query = query.filter(opc_invoices::Column::Status.eq(status.as_str()));
        }
        if let Some(cid) = &filter.customer_id {
            query = query.filter(opc_invoices::Column::CustomerId.eq(cid));
        }
        if let Some(from) = filter.date_from {
            query = query.filter(opc_invoices::Column::CreatedAt.gte(from));
        }
        if let Some(to) = filter.date_to {
            query = query.filter(opc_invoices::Column::CreatedAt.lte(to));
        }
        if let Some(limit) = filter.limit {
            query = query.limit(limit as u64);
        }
        if let Some(offset) = filter.offset {
            query = query.offset(offset as u64);
        }

        let entities = query.all(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        entities.into_iter().map(entity_to_dto).collect()
    }

    async fn update_invoice(&self, id: &str, input: UpdateInvoiceInput) -> OpcResult<Invoice> {
        let entity = opc_invoices::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("invoice {id}")))?;

        let mut am: opc_invoices::ActiveModel = entity.into();
        am.updated_at = Set(now_ts());

        if let Some(notes) = input.notes {
            am.notes = Set(notes);
        }
        if let Some(line_items) = input.line_items {
            let (subtotal, tax_total, total) = calculate_totals(&line_items);
            am.line_items_json = Set(serde_json::to_string(&line_items)
                .map_err(|e| OpcError::Database(e.to_string()))?);
            am.subtotal = Set(subtotal);
            am.tax_total = Set(tax_total);
            am.total = Set(total);
        }
        if let Some(due_at) = input.due_at {
            am.due_at = Set(due_at);
        }

        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        self.get_invoice(id).await
    }

    async fn delete_invoice(&self, id: &str) -> OpcResult<()> {
        let result = opc_invoices::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(OpcError::NotFound(format!("invoice {id}")));
        }
        Ok(())
    }

    async fn transition_status(&self, id: &str, target: InvoiceStatus) -> OpcResult<Invoice> {
        let entity = opc_invoices::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("invoice {id}")))?;

        let current = InvoiceStatus::from_str(&entity.status).map_err(OpcError::Validation)?;

        if !current.can_transition_to(&target) {
            return Err(OpcError::InvalidStateTransition {
                from: current.as_str().to_string(),
                to: target.as_str().to_string(),
            });
        }

        let now = now_ts();
        let mut am: opc_invoices::ActiveModel = entity.into();
        am.status = Set(target.as_str().to_string());
        am.updated_at = Set(now);

        match &target {
            InvoiceStatus::Sent => am.issued_at = Set(Some(now)),
            InvoiceStatus::Paid => am.paid_at = Set(Some(now)),
            _ => {},
        }

        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        self.get_invoice(id).await
    }
}
