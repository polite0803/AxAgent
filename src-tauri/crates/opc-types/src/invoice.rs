// SPDX-License-Identifier: AGPL-3.0-only

//! 发票/账单领域 — DTO 定义与 trait 接口

use serde::{Deserialize, Serialize};

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

    /// 合法状态转换
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

impl std::str::FromStr for InvoiceStatus {
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

use crate::OpcResult;

#[async_trait::async_trait]
pub trait InvoiceService: Send + Sync {
    async fn create_invoice(&self, input: CreateInvoiceInput) -> OpcResult<Invoice>;
    async fn get_invoice(&self, id: &str) -> OpcResult<Invoice>;
    async fn list_invoices(&self, filter: InvoiceFilter) -> OpcResult<Vec<Invoice>>;
    async fn update_invoice(&self, id: &str, input: UpdateInvoiceInput) -> OpcResult<Invoice>;
    async fn delete_invoice(&self, id: &str) -> OpcResult<()>;
    async fn transition_status(&self, id: &str, target: InvoiceStatus) -> OpcResult<Invoice>;
}

/// Noop 实现（用于尚未接入时的桩）
#[derive(Debug)]
pub struct NoopInvoiceService;

#[async_trait::async_trait]
impl InvoiceService for NoopInvoiceService {
    async fn create_invoice(&self, _input: CreateInvoiceInput) -> OpcResult<Invoice> {
        Err(crate::OpcError::NotFound("InvoiceService not implemented".into()))
    }
    async fn get_invoice(&self, _id: &str) -> OpcResult<Invoice> {
        Err(crate::OpcError::NotFound("InvoiceService not implemented".into()))
    }
    async fn list_invoices(&self, _filter: InvoiceFilter) -> OpcResult<Vec<Invoice>> {
        Ok(Vec::new())
    }
    async fn update_invoice(&self, _id: &str, _input: UpdateInvoiceInput) -> OpcResult<Invoice> {
        Err(crate::OpcError::NotFound("InvoiceService not implemented".into()))
    }
    async fn delete_invoice(&self, _id: &str) -> OpcResult<()> {
        Err(crate::OpcError::NotFound("InvoiceService not implemented".into()))
    }
    async fn transition_status(&self, _id: &str, _target: InvoiceStatus) -> OpcResult<Invoice> {
        Err(crate::OpcError::NotFound("InvoiceService not implemented".into()))
    }
}
