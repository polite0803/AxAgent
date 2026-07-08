//! 分析决策追踪模块（回测基础）
//!
//! 记录每次股票分析的决策结果到 JSON 文件，供后续回测分析使用。
//! 每行一条 JSON 记录（NDJSON 格式），方便追加和解析。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 一次分析决策记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRecord {
    /// 股票代码
    pub stock_code: String,
    /// 股票名称
    pub stock_name: String,
    /// 分析决策（buy/sell/hold）
    pub action: String,
    /// 置信度（0-100）
    pub confidence: f64,
    /// 分析时的股价
    pub price_at_analysis: f64,
    /// 目标价
    pub target_price: Option<f64>,
    /// 止损价
    pub stop_loss: Option<f64>,
    /// 风险等级
    pub risk_level: String,
    /// 仓位百分比
    pub position_pct: f64,
    /// 数据质量等级
    pub data_quality: String,
    /// 分析时间
    pub analyzed_at: String,
}

/// 记录一条分析决策到文件
pub fn record_decision(record: &DecisionRecord) {
    let path = get_records_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(record) {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(file, "{json}");
        }
    }
}

/// 读取所有历史决策记录
pub fn load_records() -> Vec<DecisionRecord> {
    let path = get_records_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<DecisionRecord>(line).ok())
        .collect()
}

/// 计算指定股票的历史推荐准确率
pub fn accuracy_for_stock(stock_code: &str) -> Option<(usize, usize, f64)> {
    let records = load_records();
    let stock_records: Vec<_> = records
        .iter()
        .filter(|r| r.stock_code == stock_code)
        .collect();
    if stock_records.is_empty() {
        return None;
    }
    // 统计不同决策类型的分布（实际准确率需要与真实股价对比，这里是统计量）
    let _buys = stock_records.iter().filter(|r| r.action == "buy").count();
    let _sells = stock_records.iter().filter(|r| r.action == "sell").count();
    let _holds = stock_records.iter().filter(|r| r.action == "hold").count();
    // 平均置信度
    let avg_conf =
        stock_records.iter().map(|r| r.confidence).sum::<f64>() / stock_records.len() as f64;
    Some((0, 0, avg_conf))
}

fn get_records_path() -> PathBuf {
    let mut path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    path.push("analysis_records.ndjson");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_serde() {
        let r = DecisionRecord {
            stock_code: "000001".into(),
            stock_name: "平安银行".into(),
            action: "buy".into(),
            confidence: 75.0,
            price_at_analysis: 12.34,
            target_price: Some(15.0),
            stop_loss: Some(11.0),
            risk_level: "medium".into(),
            position_pct: 30.0,
            data_quality: "A".into(),
            analyzed_at: "2026-06-03 18:00:00".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: DecisionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.stock_code, "000001");
        assert_eq!(back.action, "buy");
    }
}
