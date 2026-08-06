// SPDX-License-Identifier: AGPL-3.0-only

//! 客户管理领域 — DTO 定义与 trait 接口

use serde::{Deserialize, Serialize};

/// 客户来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CustomerSource {
    Referral,
    Website,
    SocialMedia,
    Marketplace,
    Direct,
    Other(String),
}

/// 客户 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub source: Option<CustomerSource>,
    pub tags: Vec<String>,
    pub notes: String,
    pub total_revenue: f64,
    pub invoice_count: u32,
    pub status: CustomerStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 客户状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CustomerStatus {
    Lead,
    Prospect,
    Active,
    Inactive,
    Churned,
}

impl CustomerStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lead => "lead",
            Self::Prospect => "prospect",
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Churned => "churned",
        }
    }
}

impl std::str::FromStr for CustomerStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lead" => Ok(Self::Lead),
            "prospect" => Ok(Self::Prospect),
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "churned" => Ok(Self::Churned),
            _ => Err(format!("Unknown CustomerStatus: {s}")),
        }
    }
}

/// 创建客户请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCustomerInput {
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub source: Option<CustomerSource>,
    pub tags: Vec<String>,
    pub notes: String,
}

/// 更新客户请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCustomerInput {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<Option<String>>,
    pub company: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// 客户查询过滤
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomerFilter {
    pub status: Option<CustomerStatus>,
    pub search: Option<String>,
    pub tags: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

// ── Customer Service Trait ────────────────────────────────────────

use crate::OpcResult;

#[async_trait::async_trait]
pub trait CustomerService: Send + Sync {
    async fn create_customer(&self, input: CreateCustomerInput) -> OpcResult<Customer>;
    async fn get_customer(&self, id: &str) -> OpcResult<Customer>;
    async fn list_customers(&self, filter: CustomerFilter) -> OpcResult<Vec<Customer>>;
    async fn update_customer(&self, id: &str, input: UpdateCustomerInput) -> OpcResult<Customer>;
    async fn delete_customer(&self, id: &str) -> OpcResult<()>;
    /// 按邮箱查找
    async fn find_by_email(&self, email: &str) -> OpcResult<Option<Customer>>;
}

/// Noop 实现
#[derive(Debug)]
pub struct NoopCustomerService;

#[async_trait::async_trait]
impl CustomerService for NoopCustomerService {
    async fn create_customer(&self, _input: CreateCustomerInput) -> OpcResult<Customer> {
        Err(crate::OpcError::NotFound("CustomerService not implemented".into()))
    }
    async fn get_customer(&self, _id: &str) -> OpcResult<Customer> {
        Err(crate::OpcError::NotFound("CustomerService not implemented".into()))
    }
    async fn list_customers(&self, _filter: CustomerFilter) -> OpcResult<Vec<Customer>> {
        Ok(Vec::new())
    }
    async fn update_customer(&self, _id: &str, _input: UpdateCustomerInput) -> OpcResult<Customer> {
        Err(crate::OpcError::NotFound("CustomerService not implemented".into()))
    }
    async fn delete_customer(&self, _id: &str) -> OpcResult<()> {
        Err(crate::OpcError::NotFound("CustomerService not implemented".into()))
    }
    async fn find_by_email(&self, _email: &str) -> OpcResult<Option<Customer>> {
        Ok(None)
    }
}
