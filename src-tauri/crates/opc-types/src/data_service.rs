// SPDX-License-Identifier: AGPL-3.0-only

//! OPC 数据服务 — 行业适配器与数据层的桥梁
//!
//! `OpcDataService` 提供行业适配器所需的数据访问接口：
//! - 实体查询（客户数、项目数、发票金额等）
//! - 聚合统计（按时间范围、按状态分组）
//! - 规则评估上下文构建
//!
//! 设计原则：
//! - 只暴露行业适配器需要的查询方法
//! - 返回纯 DTO，不暴露 SeaORM 实体
//! - 支持同步执行（在 tokio 运行时内异步运行）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{CustomerStatus, InvoiceStatus, OpcResult, ProjectStatus};

// ── 查询上下文 ──────────────────────────────────────────────────

/// 规则评估查询上下文
///
/// 行业适配器的自动化规则通过此上下文获取实体的运行时数据。
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
///
/// 行业适配器通过此 trait 查询业务数据，用于：
/// - 计算 KPI（需要查询时间范围内的数据量和金额）
/// - 评估自动化规则（需要获取实体的当前状态和字段值）
/// - 执行验证（需要检查数据的唯一性、关联性等）
#[async_trait]
pub trait OpcDataService: Send + Sync {
    // ── 实体计数 ──

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

    // ── 金额聚合 ──

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

    // ── 规则评估 ──

    /// 获取实体的规则评估上下文
    ///
    /// 用于自动化规则引擎在执行规则前获取实体的当前状态。
    async fn get_rule_context(&self, entity_type: &str, entity_id: &str) -> OpcResult<RuleContext>;

    // ── 验证辅助 ──

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

    // ── 规则动作执行 ──

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
}

// ── Mock 实现（测试用） ──────────────────────────────────────

/// Mock 数据服务，用于测试行业适配器
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
        }
    }
}

#[async_trait::async_trait]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_context_new() {
        let ctx = RuleContext::new("invoice", "inv-001");
        assert_eq!(ctx.entity_type, "invoice");
        assert_eq!(ctx.entity_id, "inv-001");
        assert!(ctx.status.is_none());
        assert!(ctx.overdue_days.is_none());
        assert!(ctx.created_days.is_none());
    }

    #[test]
    fn test_rule_context_with_status() {
        let ctx = RuleContext::new("customer", "cust-001").with_status("active");
        assert_eq!(ctx.status, Some("active".to_string()));
    }

    #[test]
    fn test_rule_context_with_overdue_days() {
        let ctx = RuleContext::new("invoice", "inv-001").with_overdue_days(30);
        assert_eq!(ctx.overdue_days, Some(30));
    }

    #[test]
    fn test_rule_context_with_created_days() {
        let ctx = RuleContext::new("project", "proj-001").with_created_days(90);
        assert_eq!(ctx.created_days, Some(90));
    }

    #[test]
    fn test_rule_context_with_field() {
        let ctx =
            RuleContext::new("invoice", "inv-001").with_field("amount", serde_json::json!(1000.0));
        assert_eq!(ctx.fields["amount"], serde_json::json!(1000.0));
    }

    #[test]
    fn test_aggregate_result_default() {
        let result = AggregateResult::default();
        assert_eq!(result.count, 0);
        assert_eq!(result.total, 0.0);
        assert_eq!(result.average, 0.0);
        assert_eq!(result.min, 0.0);
        assert_eq!(result.max, 0.0);
    }

    #[tokio::test]
    async fn test_mock_data_service_count_customers() {
        let service = MockDataService::default();
        let result = service.count_customers(&[], 0, 9999999999).await.unwrap();
        assert_eq!(result, 100);
    }

    #[tokio::test]
    async fn test_mock_data_service_count_projects() {
        let service = MockDataService::default();
        let result = service.count_projects(&[], 0, 9999999999).await.unwrap();
        assert_eq!(result, 50);
    }

    #[tokio::test]
    async fn test_mock_data_service_count_invoices() {
        let service = MockDataService::default();
        let result = service.count_invoices(&[], 0, 9999999999).await.unwrap();
        assert_eq!(result, 200);
    }

    #[tokio::test]
    async fn test_mock_data_service_aggregate_invoice_amounts() {
        let service = MockDataService::default();
        let result = service.aggregate_invoice_amounts(&[], 0, 9999999999).await.unwrap();
        assert_eq!(result.count, 200);
        assert_eq!(result.total, 100000.0);
        assert_eq!(result.average, 500.0);
        assert_eq!(result.min, 100.0);
        assert_eq!(result.max, 5000.0);
    }

    #[tokio::test]
    async fn test_mock_data_service_aggregate_project_budgets() {
        let service = MockDataService::default();
        let result = service.aggregate_project_budgets(&[], 0, 9999999999).await.unwrap();
        assert_eq!(result.count, 50);
        assert_eq!(result.total, 500000.0);
    }

    #[tokio::test]
    async fn test_mock_data_service_aggregate_customer_revenue() {
        let service = MockDataService::default();
        let result = service.aggregate_customer_revenue("cust-001").await.unwrap();
        assert_eq!(result, 15000.0);
    }

    #[tokio::test]
    async fn test_mock_data_service_get_rule_context() {
        let service = MockDataService::default();
        let ctx = service.get_rule_context("invoice", "inv-001").await.unwrap();
        assert_eq!(ctx.entity_type, "invoice");
        assert_eq!(ctx.entity_id, "inv-001");
        assert_eq!(ctx.fields["mock"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn test_mock_data_service_is_field_unique() {
        let service = MockDataService::default();
        let result = service.is_field_unique("customer", "name", "test", None).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_mock_data_service_check_relation_exists() {
        let service = MockDataService::default();
        let result = service
            .check_relation_exists("customer", "cust-001", "invoice", "inv-001")
            .await
            .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_mock_data_service_custom_values() {
        let service = MockDataService {
            customer_count: 50,
            project_count: 25,
            invoice_count: 100,
            invoice_total: 50000.0,
            invoice_average: 500.0,
            invoice_min: 50.0,
            invoice_max: 2000.0,
            project_total: 250000.0,
            customer_revenue: 8000.0,
        };
        let result = service.count_customers(&[], 0, 9999999999).await.unwrap();
        assert_eq!(result, 50);

        let result = service.aggregate_customer_revenue("cust-002").await.unwrap();
        assert_eq!(result, 8000.0);
    }
}
