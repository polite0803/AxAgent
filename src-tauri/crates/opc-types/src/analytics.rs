// SPDX-License-Identifier: AGPL-3.0-only

//! 分析仪表盘领域 — DTO 定义与 trait 接口

use serde::{Deserialize, Serialize};

/// KPI 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KpiRecord {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub period: String,
    pub recorded_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateKpiInput {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub period: String,
}

/// 收入记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueRecord {
    pub id: String,
    pub amount: f64,
    pub currency: String,
    pub category: String,
    pub description: String,
    pub recorded_at: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRevenueInput {
    pub amount: f64,
    pub currency: String,
    pub category: String,
    pub description: String,
}

/// 仪表盘摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total_revenue: f64,
    pub total_invoices: u32,
    pub active_projects: u32,
    pub total_customers: u32,
    pub recent_kpis: Vec<KpiRecord>,
    pub revenue_trend: Vec<RevenueRecord>,
}

use crate::OpcResult;

#[async_trait::async_trait]
pub trait AnalyticsService: Send + Sync {
    // KPI
    async fn record_kpi(&self, input: CreateKpiInput) -> OpcResult<KpiRecord>;
    async fn list_kpis(
        &self,
        period: Option<String>,
        limit: Option<u32>,
    ) -> OpcResult<Vec<KpiRecord>>;

    // Revenue
    async fn record_revenue(&self, input: CreateRevenueInput) -> OpcResult<RevenueRecord>;
    async fn list_revenue(
        &self,
        category: Option<String>,
        limit: Option<u32>,
    ) -> OpcResult<Vec<RevenueRecord>>;

    // Dashboard
    async fn get_dashboard_summary(&self) -> OpcResult<DashboardSummary>;
}

#[derive(Debug)]
pub struct NoopAnalyticsService;

#[async_trait::async_trait]
impl AnalyticsService for NoopAnalyticsService {
    async fn record_kpi(&self, _: CreateKpiInput) -> OpcResult<KpiRecord> {
        Err(crate::OpcError::NotFound("AnalyticsService not implemented".into()))
    }
    async fn list_kpis(&self, _: Option<String>, _: Option<u32>) -> OpcResult<Vec<KpiRecord>> {
        Ok(Vec::new())
    }
    async fn record_revenue(&self, _: CreateRevenueInput) -> OpcResult<RevenueRecord> {
        Err(crate::OpcError::NotFound("AnalyticsService not implemented".into()))
    }
    async fn list_revenue(
        &self,
        _: Option<String>,
        _: Option<u32>,
    ) -> OpcResult<Vec<RevenueRecord>> {
        Ok(Vec::new())
    }
    async fn get_dashboard_summary(&self) -> OpcResult<DashboardSummary> {
        Ok(DashboardSummary {
            total_revenue: 0.0,
            total_invoices: 0,
            active_projects: 0,
            total_customers: 0,
            recent_kpis: Vec::new(),
            revenue_trend: Vec::new(),
        })
    }
}
