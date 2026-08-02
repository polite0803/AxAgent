// SPDX-License-Identifier: AGPL-3.0-only

//! 金融投资领域 — 财务报表 DTO 与 trait 接口
//!
//! 提供一人公司的财务报表能力，桥接到 AxInvest 投资追踪系统。
//! 核心概念：经营利润 → 投资资本

use serde::{Deserialize, Serialize};

/// 期间财务报表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    /// 报表期间，如 "2026-Q2" 或 "2026-07"
    pub period: String,
    /// 总收入（已收款发票金额）
    pub total_revenue: f64,
    /// 待收款（已发送/逾期发票）
    pub pending_revenue: f64,
    /// 退款金额
    pub refunded_amount: f64,
    /// 活跃项目数
    pub active_projects: u32,
    /// 新增客户数（本期创建的客户）
    pub new_customers: u32,
    /// 净利润 ≈ 总收入 - 退款
    pub net_profit: f64,
    /// 可投资金额（建议转入 AxInvest）
    pub investable_amount: f64,
    /// 客户流失数
    pub churned_customers: u32,
    /// 未税收入（总收入 / (1+税率) 简化估算，按 6%）
    pub pretax_revenue: f64,
}

/// 投资建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentAdvice {
    /// 建议投资金额
    pub recommended_amount: f64,
    /// 风险等级
    pub risk_level: String,
    /// 建议说明
    pub advice: String,
}

use crate::OpcResult;

#[async_trait::async_trait]
pub trait FinanceService: Send + Sync {
    /// 生成指定周期的财务报表
    async fn get_financial_report(&self, period: &str) -> OpcResult<FinancialReport>;
    /// 生成投资建议
    async fn get_investment_advice(&self, report: &FinancialReport) -> InvestmentAdvice;
}
