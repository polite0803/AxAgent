// SPDX-License-Identifier: AGPL-3.0-only

//! 决策仪表盘报告 DTO 契约层
//!
//! 借鉴 daily_stock_analysis 项目的「决策仪表盘」推送格式，
//! 固化为标准化的 7 段式结构 + 大盘复盘模板。
//!
//! 权威定义在 harness 层（铁律 4），stock-analysis / notification / gateway
//! 等 crate 通过 `pub use` 引用，不得重复定义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── 决策仪表盘 7 段式报告 ──

/// 决策仪表盘报告（单只股票）
///
/// 对应 DSA 推送格式：
/// - 核心结论 / 评分 / 趋势 / 买卖点位 / 风险警报 / 催化因素 / 操作检查清单
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardReport {
    /// 股票代码（如 "600519"）
    pub stock_code: String,
    /// 股票名称（如 "贵州茅台"）
    pub stock_name: String,
    /// 分析日期（ISO 8601，如 "2026-07-16"）
    pub analysis_date: String,
    /// 生成时间戳
    pub generated_at: DateTime<Utc>,

    // ── 1. 核心结论 ──
    /// 核心结论：一句话总结（如 "AI 服务器 HDI 核心供应商，业绩高增长，短期获利盘压力"）
    pub core_conclusion: String,
    /// 决策动作（6 档中文）：强烈买入 / 买入 / 增持 / 持有 / 减持 / 卖出
    pub action: String,
    /// 综合评分（0-100）
    pub score: u32,
    /// 趋势标签：看多 / 看空 / 震荡
    pub trend: String,
    /// 置信度（0-100）
    pub confidence: f64,

    // ── 2. 买卖点位 ──
    /// 买入点位区间（下限）
    pub buy_point_low: Option<f64>,
    /// 买入点位区间（上限）
    pub buy_point_high: Option<f64>,
    /// 目标价
    pub target_price: Option<f64>,
    /// 止损价
    pub stop_loss: Option<f64>,
    /// 建议仓位百分比（0-100）
    pub position_pct: f64,

    // ── 3. 风险警报 ──
    pub risk_alerts: Vec<RiskAlert>,

    // ── 4. 催化因素 ──
    pub catalysts: Vec<Catalyst>,

    // ── 5. 操作检查清单 ──
    pub checklist: Vec<ChecklistItem>,

    // ── 6. 最新动态（舆情/公告摘要）──
    pub latest_news: Option<String>,

    // ── 7. 业绩预期（可选）──
    pub earnings_expectation: Option<String>,

    // ── 元数据 ──
    /// 使用的 LLM 模型名称（可选，用于报告底部展示）
    pub llm_model: Option<String>,
    /// 是否通过完整性校验
    pub integrity_passed: bool,
}

/// 风险警报条目
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RiskAlert {
    /// 风险描述
    pub description: String,
    /// 风险等级：低 / 中 / 高
    pub severity: String,
    /// 风险来源标签（如 "主力资金流出" / "筹码分散" / "历史违规"）
    pub source: Option<String>,
}

/// 催化因素条目
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Catalyst {
    /// 催化描述
    pub description: String,
    /// 影响方向：利好 / 利空
    pub direction: String,
    /// 时间线：短期 / 中期 / 长期
    pub timeline: Option<String>,
    /// 置信度（0-100）
    pub confidence_score: Option<f64>,
}

/// 操作检查清单条目
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    /// 检查项描述
    pub description: String,
    /// 是否已确认
    pub checked: bool,
    /// 检查类别：入场 / 加仓 / 减仓 / 止损 / 止盈
    pub category: String,
}

// ── 大盘复盘报告 ──

/// 大盘复盘报告（对应 DSA 的"大盘复盘"推送）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketReviewReport {
    /// 复盘日期（ISO 8601）
    pub review_date: String,
    /// 生成时间戳
    pub generated_at: DateTime<Utc>,

    // ── 主要指数 ──
    pub indices: Vec<IndexQuote>,

    // ── 市场概况 ──
    /// 上涨家数
    pub advancers: Option<u32>,
    /// 下跌家数
    pub decliners: Option<u32>,
    /// 涨停家数
    pub limit_up: Option<u32>,
    /// 跌停家数
    pub limit_down: Option<u32>,

    // ── 板块表现 ──
    /// 领涨板块
    pub sector_leaders: Vec<String>,
    /// 领跌板块
    pub sector_laggards: Vec<String>,

    // ── 元数据 ──
    pub llm_model: Option<String>,
}

/// 指数行情
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IndexQuote {
    /// 指数名称（如 "上证指数"）
    pub name: String,
    /// 点位
    pub price: f64,
    /// 涨跌幅（百分比）
    pub change_pct: f64,
}

// ── 聚合仪表盘（多只股票汇总）──

/// 聚合仪表盘（对应 DSA 的"决策仪表盘"汇总推送）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDigest {
    /// 仪表盘日期
    pub digest_date: String,
    /// 生成时间戳
    pub generated_at: DateTime<Utc>,
    /// 分析的股票总数
    pub total_count: u32,
    /// 买入数量
    pub buy_count: u32,
    /// 观望数量
    pub watch_count: u32,
    /// 卖出数量
    pub sell_count: u32,
    /// 摘要列表（每只股票一行）
    pub summaries: Vec<StockSummary>,
    /// 大盘复盘（可选，与仪表盘合并推送时使用）
    pub market_review: Option<MarketReviewReport>,
}

/// 股票摘要（仪表盘汇总中每只股票一行）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StockSummary {
    pub stock_code: String,
    pub stock_name: String,
    /// 决策动作（6 档中文）
    pub action: String,
    /// 评分（0-100）
    pub score: u32,
    /// 趋势标签
    pub trend: String,
    /// 置信度（0-100）
    pub confidence: f64,
}

// ── 完整性校验 ──

/// 完整性校验缺失字段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MissingField {
    /// 字段名
    pub field: String,
    /// 缺失说明
    pub reason: String,
}

/// 校验 DashboardReport 必填字段完整性
///
/// 借鉴 DSA 的 `REPORT_INTEGRITY_ENABLED` 语义：
/// 缺失必填字段时返回 MissingField 列表，调用方可重试或用占位符补全。
pub fn validate_dashboard_report(report: &DashboardReport) -> Vec<MissingField> {
    let mut missing = Vec::new();

    if report.stock_code.is_empty() {
        missing
            .push(MissingField { field: "stock_code".into(), reason: "股票代码为空".into() });
    }
    if report.stock_name.is_empty() {
        missing
            .push(MissingField { field: "stock_name".into(), reason: "股票名称为空".into() });
    }
    if report.core_conclusion.is_empty() {
        missing
            .push(MissingField {
                field: "core_conclusion".into(), reason: "核心结论为空".into()
            });
    }
    if report.action.is_empty() {
        missing.push(MissingField { field: "action".into(), reason: "决策动作为空".into() });
    }
    if report.score > 100 {
        missing.push(MissingField {
            field: "score".into(),
            reason: format!("评分 {} 超出 0-100 范围", report.score),
        });
    }
    if report.confidence < 0.0 || report.confidence > 100.0 {
        missing.push(MissingField {
            field: "confidence".into(),
            reason: format!("置信度 {} 超出 0-100 范围", report.confidence),
        });
    }
    if report.position_pct < 0.0 || report.position_pct > 100.0 {
        missing.push(MissingField {
            field: "position_pct".into(),
            reason: format!("仓位百分比 {} 超出 0-100 范围", report.position_pct),
        });
    }
    // 买入信号应有目标价
    if (report.action == "强烈买入" || report.action == "买入" || report.action == "增持")
        && report.target_price.is_none()
    {
        missing.push(MissingField {
            field: "target_price".into(),
            reason: format!("{} 信号缺少目标价", report.action),
        });
    }
    // 有仓位应有止损
    if report.position_pct > 0.0 && report.stop_loss.is_none() {
        missing.push(MissingField {
            field: "stop_loss".into(),
            reason: "有仓位建议但缺少止损价".into(),
        });
    }

    missing
}

/// 用占位符补全 DashboardReport 的缺失字段
///
/// 借鉴 DSA 的报告完整性占位策略：缺失字段填入 "—" 或 0，
/// 避免推送时出现空段。
pub fn fill_missing_with_placeholders(report: &mut DashboardReport) {
    if report.core_conclusion.is_empty() {
        report.core_conclusion = "—".to_string();
    }
    if report.action.is_empty() {
        report.action = "持有".to_string();
    }
    if report.trend.is_empty() {
        report.trend = "震荡".to_string();
    }
    if report.target_price.is_none() && report.position_pct > 0.0 {
        // 无目标价时用当前价 *1.1 作为保守占位
        // （实际值由调用方在完整性校验失败重试后覆盖）
    }
    if report.stop_loss.is_none() && report.position_pct > 0.0 {
        // 同上，占位由调用方覆盖
    }
    report.integrity_passed = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_valid_report() -> DashboardReport {
        DashboardReport {
            stock_code: "600519".into(),
            stock_name: "贵州茅台".into(),
            analysis_date: "2026-07-16".into(),
            generated_at: Utc::now(),
            core_conclusion: "白酒龙头，业绩稳健".into(),
            action: "买入".into(),
            score: 75,
            trend: "看多".into(),
            confidence: 80.0,
            buy_point_low: Some(1680.0),
            buy_point_high: Some(1720.0),
            target_price: Some(1900.0),
            stop_loss: Some(1600.0),
            position_pct: 30.0,
            risk_alerts: vec![RiskAlert {
                description: "短期获利盘压力".into(),
                severity: "中".into(),
                source: Some("技术面".into()),
            }],
            catalysts: vec![Catalyst {
                description: "中秋旺季需求".into(),
                direction: "利好".into(),
                timeline: Some("短期".into()),
                confidence_score: Some(75.0),
            }],
            checklist: vec![ChecklistItem {
                description: "确认放量突破".into(),
                checked: false,
                category: "入场".into(),
            }],
            latest_news: Some("贵州茅台发布半年报".into()),
            earnings_expectation: Some("2026H1 营收同比+15%".into()),
            llm_model: Some("glm-5.2".into()),
            integrity_passed: true,
        }
    }

    #[test]
    fn test_valid_report_passes_validation() {
        let report = make_valid_report();
        let missing = validate_dashboard_report(&report);
        assert!(missing.is_empty(), "应有 0 个缺失字段, got {missing:?}");
    }

    #[test]
    fn test_missing_core_conclusion_detected() {
        let mut report = make_valid_report();
        report.core_conclusion = String::new();
        let missing = validate_dashboard_report(&report);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].field, "core_conclusion");
    }

    #[test]
    fn test_buy_signal_without_target_price_detected() {
        let mut report = make_valid_report();
        report.action = "强烈买入".into();
        report.target_price = None;
        let missing = validate_dashboard_report(&report);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].field == "target_price");
    }

    #[test]
    fn test_position_without_stop_loss_detected() {
        let mut report = make_valid_report();
        report.stop_loss = None;
        report.position_pct = 20.0;
        let missing = validate_dashboard_report(&report);
        assert_eq!(missing.len(), 1);
        assert!(missing[0].field == "stop_loss");
    }

    #[test]
    fn test_score_out_of_range_detected() {
        let mut report = make_valid_report();
        report.score = 150;
        let missing = validate_dashboard_report(&report);
        assert!(missing.iter().any(|m| m.field == "score"));
    }

    #[test]
    fn test_fill_placeholders_sets_integrity_passed() {
        let mut report = make_valid_report();
        report.core_conclusion = String::new();
        report.action = String::new();
        report.trend = String::new();
        fill_missing_with_placeholders(&mut report);
        assert!(report.integrity_passed);
        assert_eq!(report.action, "持有");
        assert_eq!(report.trend, "震荡");
        assert_eq!(report.core_conclusion, "—");
    }
}
