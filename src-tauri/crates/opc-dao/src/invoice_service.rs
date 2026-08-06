// SPDX-License-Identifier: AGPL-3.0-only

//! 发票服务实现 — SeaORM CRUD + 状态机

use async_trait::async_trait;
use sea_orm::QuerySelect;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json;
use std::str::FromStr;
// use tracing;

use axagent_harness::util_fns::{gen_id, now_ts};
use axagent_opc_entities::opc_invoices;
use axagent_opc_types::{
    CreateInvoiceInput, Invoice, InvoiceFilter, InvoiceLineItem, InvoiceService, InvoiceStatus,
    OpcError, OpcResult, UpdateInvoiceInput,
};

/// 默认发票服务实现
pub struct DefaultInvoiceService {
    pub db: DatabaseConnection,
}

impl DefaultInvoiceService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Entity ↔ DTO 转换 ─────────────────────────────────────────────

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

// ── Service 实现 ───────────────────────────────────────────────────

#[async_trait]
impl InvoiceService for DefaultInvoiceService {
    async fn create_invoice(&self, input: CreateInvoiceInput) -> OpcResult<Invoice> {
        let id = gen_id();
        let now = now_ts();
        let (subtotal, tax_total, total) = calculate_totals(&input.line_items);

        // Generate invoice number: INV-YYYYMMDD-XXXX
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

        // Set timestamps on specific transitions
        match &target {
            InvoiceStatus::Sent => am.issued_at = Set(Some(now)),
            InvoiceStatus::Paid => am.paid_at = Set(Some(now)),
            _ => {},
        }

        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        self.get_invoice(id).await
    }
}
