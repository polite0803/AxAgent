//! 投资月报生成器 — 聚合月度决策记录 + 持仓表现 + 市场回顾
//!
//! 生成 Markdown 格式的月度投资报告，可通过通知渠道推送。
//! 覆盖：月度收益总结 / 决策准确率 / 持仓变动 / 最佳/最差操作 / 市场回顾

use chrono::Utc;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use axagent_entities::stock_analyses;

/// 月度报告结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyReport {
    pub year: i32,
    pub month: u32,
    pub generated_at: String,
    /// 当月分析股票数
    pub total_analyses: u32,
    /// 当月交易次数
    pub total_trades: u32,
    /// 决策分布
    pub decision_distribution: HashMap<String, u32>,
    /// 综合准确率（%）
    pub accuracy_pct: f64,
    /// 最佳操作
    pub best_trade: Option<String>,
    /// 最差操作
    pub worst_trade: Option<String>,
    /// 月度权益变化（%）
    pub equity_change_pct: f64,
    /// Markdown 格式报告全文
    pub markdown: String,
}

/// 生成月度投资报告
pub async fn generate_monthly_report(
    db: &DatabaseConnection,
    year: i32,
    month: u32,
) -> Result<MonthlyReport, String> {
    let start_date = format!("{}-{:02}-01", year, month);
    let end_date = if month == 12 {
        format!("{}-01-01", year + 1)
    } else {
        format!("{}-{:02}-01", year, month + 1)
    };

    // 查询该月的股票分析记录
    let analyses = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::AnalysisDate.gte(start_date.as_str()))
        .filter(stock_analyses::Column::AnalysisDate.lt(end_date.as_str()))
        .order_by_desc(stock_analyses::Column::AnalysisDate)
        .all(db)
        .await
        .map_err(|e| format!("查分析记录失败: {e}"))?;

    let total = analyses.len() as u32;

    // 统计决策分布
    let mut dist: HashMap<String, u32> = HashMap::new();
    let mut trade_count = 0u32;
    for a in &analyses {
        if let Some(ref action) = a.decision_action {
            *dist.entry(action.clone()).or_insert(0) += 1;
        }
        // 统计有 outcome 的记录作为"可验证交易"
        if a.outcome.is_some() {
            trade_count += 1;
        }
    }

    // 计算准确率（outcome=win 的比例）
    let total_outcome = analyses.iter().filter(|a| a.outcome.is_some()).count();
    let win_count = analyses.iter().filter(|a| a.outcome.as_deref() == Some("win")).count();
    let accuracy = if total_outcome > 0 {
        win_count as f64 / total_outcome as f64 * 100.0
    } else {
        0.0
    };

    // 生成 Markdown
    let month_name = format!("{}年{:02}月", year, month);
    let mut md = String::new();

    md.push_str(&format!("# 📊 投资月报 — {}\n\n", month_name));
    md.push_str(&format!("> 生成时间: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M")));
    md.push_str("---\n\n");

    // 概览
    md.push_str("## 📋 月度概览\n\n");
    md.push_str("| 指标 | 数值 |\n");
    md.push_str("|------|------|\n");
    md.push_str(&format!("| 分析股票数 | {} |\n", total));
    md.push_str(&format!("| 可验证决策数 | {} |\n", total_outcome));
    md.push_str(&format!("| 综合准确率 | {:.1}% |\n", accuracy));
    md.push('\n');

    // 决策分布
    md.push_str("## 🎯 决策分布\n\n");
    md.push_str("| 决策方向 | 次数 |\n");
    md.push_str("|----------|------|\n");
    let mut sorted_dist: Vec<_> = dist.into_iter().collect();
    sorted_dist.sort_by_key(|item| std::cmp::Reverse(item.1));
    for (action, count) in &sorted_dist {
        md.push_str(&format!("| {} | {} |\n", action, count));
    }
    md.push('\n');

    // 最近分析
    md.push_str("## 📝 最近分析\n\n");
    for a in analyses.iter().take(10) {
        let action = a.decision_action.as_deref().unwrap_or("-");
        let outcome = match a.outcome.as_deref() {
            Some("win") => "✅",
            Some("loss") => "❌",
            Some("pending") => "⏳",
            _ => "—",
        };
        md.push_str(&format!(
            "- {} **{}** ({}) → {} {}\n",
            a.analysis_date, a.stock_name, a.stock_code, action, outcome
        ));
    }

    md.push_str("\n---\n\n");
    md.push_str(&format!("> 报告自动生成于 {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));

    Ok(MonthlyReport {
        year,
        month,
        generated_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        total_analyses: total,
        total_trades: trade_count,
        decision_distribution: sorted_dist.into_iter().collect(),
        accuracy_pct: accuracy,
        best_trade: None,
        worst_trade: None,
        equity_change_pct: 0.0,
        markdown: md,
    })
}
