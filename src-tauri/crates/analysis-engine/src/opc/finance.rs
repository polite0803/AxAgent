// SPDX-License-Identifier: AGPL-3.0-only

//! 金融投资领域 — 财务报表 DTO、trait 接口与 SeaORM 实现

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use axagent_entities::{opc_customers, opc_invoices, opc_projects};

use super::error::OpcResult;

// ── DTO 定义 ──────────────────────────────────────────────────

/// 期间财务报表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    pub period: String,
    pub total_revenue: f64,
    pub pending_revenue: f64,
    pub refunded_amount: f64,
    pub active_projects: u32,
    pub new_customers: u32,
    pub net_profit: f64,
    pub investable_amount: f64,
    pub churned_customers: u32,
    pub pretax_revenue: f64,
}

/// 投资建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentAdvice {
    pub recommended_amount: f64,
    pub risk_level: String,
    pub advice: String,
}

// ── Finance Service Trait ─────────────────────────────────────

#[async_trait]
pub trait FinanceService: Send + Sync {
    async fn get_financial_report(&self, period: &str) -> OpcResult<FinancialReport>;
    async fn get_investment_advice(&self, report: &FinancialReport) -> InvestmentAdvice;
}

/// Noop 实现
#[derive(Debug)]
pub struct NoopFinanceService;

#[async_trait]
impl FinanceService for NoopFinanceService {
    async fn get_financial_report(&self, _period: &str) -> OpcResult<FinancialReport> {
        Ok(FinancialReport {
            period: String::new(),
            total_revenue: 0.0,
            pending_revenue: 0.0,
            refunded_amount: 0.0,
            active_projects: 0,
            new_customers: 0,
            net_profit: 0.0,
            investable_amount: 0.0,
            churned_customers: 0,
            pretax_revenue: 0.0,
        })
    }
    async fn get_investment_advice(&self, _report: &FinancialReport) -> InvestmentAdvice {
        InvestmentAdvice {
            recommended_amount: 0.0,
            risk_level: "conservative".to_string(),
            advice: "暂无建议".to_string(),
        }
    }
}

// ── SeaORM 实现 ──────────────────────────────────────────────

/// 默认金融服务实现
pub struct DefaultFinanceService {
    pub db: DatabaseConnection,
}

impl DefaultFinanceService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl FinanceService for DefaultFinanceService {
    async fn get_financial_report(&self, period: &str) -> OpcResult<FinancialReport> {
        let (year_str, month_or_q) = match period.split_once('-') {
            Some((y, m)) => (y, Some(m)),
            None => (period, None),
        };
        let year: i32 = year_str.parse().unwrap_or(2026);

        let invoices = opc_invoices::Entity::find()
            .all(&self.db)
            .await
            .map_err(|e| super::error::OpcError::Database(e.to_string()))?;

        let period_invoices: Vec<_> = invoices
            .into_iter()
            .filter(|inv| {
                let ts = inv.created_at;
                let inv_year = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y").to_string())
                    .unwrap_or_default();
                if inv_year != year.to_string() {
                    return false;
                }
                if let Some(moq) = month_or_q {
                    let inv_month = chrono::DateTime::from_timestamp(ts, 0)
                        .map(|dt| dt.format("%m").to_string())
                        .unwrap_or_default();
                    if let Some(q_str) = moq.strip_prefix('Q') {
                        let q = q_str.parse::<u32>().unwrap_or(0);
                        let inv_q = inv_month.parse::<u32>().unwrap_or(0).div_ceil(3);
                        if inv_q != q {
                            return false;
                        }
                    } else if inv_month != moq {
                        return false;
                    }
                }
                true
            })
            .collect();

        let total_revenue: f64 =
            period_invoices.iter().filter(|i| i.status == "paid").map(|i| i.total).sum();

        let pending_revenue: f64 = period_invoices
            .iter()
            .filter(|i| i.status == "sent" || i.status == "overdue")
            .map(|i| i.total)
            .sum();

        let refunded_amount: f64 =
            period_invoices.iter().filter(|i| i.status == "refunded").map(|i| i.total).sum();

        let active_projects = opc_projects::Entity::find()
            .filter(opc_projects::Column::Status.eq("active"))
            .all(&self.db)
            .await
            .map_err(|e| super::error::OpcError::Database(e.to_string()))?
            .len() as u32;

        let new_customers = opc_customers::Entity::find()
            .filter(opc_customers::Column::CreatedAt.gte(period_start_ts(year, month_or_q)))
            .all(&self.db)
            .await
            .map_err(|e| super::error::OpcError::Database(e.to_string()))?
            .len() as u32;

        let churned_customers = opc_customers::Entity::find()
            .filter(opc_customers::Column::Status.eq("churned"))
            .all(&self.db)
            .await
            .map_err(|e| super::error::OpcError::Database(e.to_string()))?
            .len() as u32;

        let net_profit = total_revenue - refunded_amount;
        let investable_amount = (net_profit * 0.5).max(0.0);
        let pretax_revenue = total_revenue / 1.06;

        Ok(FinancialReport {
            period: period.to_string(),
            total_revenue,
            pending_revenue,
            refunded_amount,
            active_projects,
            new_customers,
            net_profit,
            investable_amount,
            churned_customers,
            pretax_revenue,
        })
    }

    async fn get_investment_advice(&self, report: &FinancialReport) -> InvestmentAdvice {
        let risk_level = if report.net_profit > 100_000.0 {
            "balanced"
        } else if report.net_profit > 30_000.0 {
            "moderate"
        } else {
            "conservative"
        };

        let advice = match risk_level {
            "balanced" => format!(
                "本期经营良好，净利润 ¥{:.2}。建议将 ¥{:.2} 分散投资于指数基金和优质个股。",
                report.net_profit, report.investable_amount
            ),
            "moderate" => format!(
                "经营稳健，净利润 ¥{:.2}。建议优先补充流动资金，剩余 ¥{:.2} 投入低风险理财。",
                report.net_profit, report.investable_amount
            ),
            _ => format!(
                "本期净利润 ¥{:.2}。建议保留全部利润作为运营资金，暂不对外投资。",
                report.net_profit
            ),
        };

        InvestmentAdvice {
            recommended_amount: report.investable_amount,
            risk_level: risk_level.to_string(),
            advice,
        }
    }
}

fn period_start_ts(year: i32, month_or_q: Option<&str>) -> i64 {
    match month_or_q {
        Some(moq) if moq.starts_with('Q') => {
            let q = moq[1..].parse::<u32>().unwrap_or(1);
            let month = (q - 1) * 3 + 1;
            chrono::NaiveDate::from_ymd_opt(year, month, 1)
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
                .unwrap_or(0)
        },
        Some(m) if m.len() == 2 => {
            let month: u32 = m.parse().unwrap_or(1);
            chrono::NaiveDate::from_ymd_opt(year, month, 1)
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
                .unwrap_or(0)
        },
        _ => chrono::NaiveDate::from_ymd_opt(year, 1, 1)
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
            .unwrap_or(0),
    }
}
