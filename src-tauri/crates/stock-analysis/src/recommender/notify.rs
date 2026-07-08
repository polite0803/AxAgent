//! 荐股定时任务的"扫 + 通知"核心逻辑
//!
//! 跟 [`super::recommend_stocks`] 的区别：
//! - **过滤兜底 pick** (`synthetic == true`)：用户明确要求推送"真实"推荐
//! - **过滤低 confidence**：通过 `min_confidence` 阈值
//! - **取 top N**：按 confidence 降序
//! - **生成 OS 通知文本**：`build_notification`
//!
//! 不依赖 Tauri，方便在 Tauri command 之外（如 CLI / 测试）复用。

use std::sync::Arc;

use axagent_astock_data::AStockClient;

use super::types::{Period, RecoPick};
use super::{recommend_stocks, RecoResponse};

/// 跑一次"定时荐股扫描"，返回符合要求的 picks（按 confidence 降序）
///
/// 流程：
/// 1. 并行跑 `recommend_stocks(p)` × `periods` 中每个周期
/// 2. 合并所有 RecoResponse.picks（去重：同 stock_code 留 confidence 最高）
/// 3. 过滤 `synthetic == true`（兜底合成）—— **关键** 用户要求不推兜底
/// 4. 过滤 `confidence < min_confidence`
/// 5. 按 confidence 降序
/// 6. 截取前 `top_n` 条
pub async fn run_recommendation_scan(
    client: Arc<AStockClient>,
    periods: &[Period],
    template_vars: &[(String, serde_json::Value)],
    min_confidence: u8,
    top_n: usize,
) -> Vec<RecoPick> {
    if periods.is_empty() || top_n == 0 {
        return Vec::new();
    }

    // 1. 拉取所有 period 的 RecoResponse
    //    先把 periods 复制成 owned Vec，避免借用逃逸到 spawned task
    let owned_periods: Vec<Period> = periods.to_vec();
    let mut futs = Vec::with_capacity(owned_periods.len());
    for p in owned_periods {
        let client = client.clone();
        let vars: Vec<(String, serde_json::Value)> = template_vars.to_vec();
        futs.push(tokio::spawn(async move { (p, recommend_stocks(client, p, &vars).await) }));
    }

    let mut all_picks: Vec<RecoPick> = Vec::new();
    for h in futs {
        match h.await {
            Ok((_period, Ok(resp))) => collect_real_picks(&resp, &mut all_picks),
            Ok((_period, Err(e))) => {
                tracing::warn!("[recommendation_cron] period {:?} 扫描失败: {e}", _period);
            },
            Err(e) => {
                tracing::warn!("[recommendation_cron] join error: {e}");
            },
        }
    }

    // 2. 同 stock_code 去重：留 confidence 最高
    //    （多策略命中同一只时，dedup_and_merge 已在 recommender 内部处理过；
    //     这里只处理"短/中/长"三个 period 之间的重复——比如短线 + 中线都看多同一只）
    let mut by_code: std::collections::HashMap<String, RecoPick> = std::collections::HashMap::new();
    for p in all_picks {
        match by_code.get(&p.stock_code) {
            Some(existing) if existing.confidence >= p.confidence => {
                // 保留已有
            },
            _ => {
                by_code.insert(p.stock_code.clone(), p);
            },
        }
    }

    // 3. 过滤 synthetic + min_confidence
    let mut filtered: Vec<RecoPick> = by_code
        .into_values()
        .filter(|p| !p.synthetic && p.confidence >= min_confidence)
        .collect();

    // 4. 按 confidence 降序
    filtered.sort_by(|a, b| b.confidence.cmp(&a.confidence));

    // 5. top N
    filtered.truncate(top_n);
    filtered
}

fn collect_real_picks(resp: &RecoResponse, out: &mut Vec<RecoPick>) {
    for picks in resp.picks.values() {
        for p in picks {
            if !p.synthetic {
                out.push(p.clone());
            }
        }
    }
}

/// 生成 (title, body) 用于 OS 通知
///
/// 格式：
/// - 标题：`📈 智能荐股更新 · {N} 只新推荐`
/// - 正文：每只一行 `{name}({code}) ¥{price} 目标+{upside}%` 最多 8 行（OS 通知有长度限制）
pub fn build_notification(picks: &[RecoPick]) -> (String, String) {
    if picks.is_empty() {
        return ("📈 智能荐股更新".to_string(), "本次未发现符合条件的新推荐".to_string());
    }

    let title = format!("📈 智能荐股更新 · {} 只", picks.len());
    // OS 通知有长度限制；最多 8 行：前 7 只详情 + 1 行 "等共 N"
    let max_detail_lines = 7;
    let mut body = String::new();
    for (i, p) in picks.iter().take(max_detail_lines).enumerate() {
        let upside_pct = if p.price > 0.0 {
            (p.target_price / p.price - 1.0) * 100.0
        } else {
            0.0
        };
        let line = format!(
            "{}. {} ({}) ¥{:.2} → 目标 +{:.1}%\n",
            i + 1,
            p.stock_name,
            p.stock_code,
            p.price,
            upside_pct
        );
        body.push_str(&line);
    }
    if picks.len() > max_detail_lines {
        body.push_str(&format!("…等共 {} 只", picks.len()));
    } else {
        body = body.trim_end().to_string();
    }

    (title, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommender::types::Style;

    fn pick(
        code: &str,
        name: &str,
        conf: u8,
        price: f64,
        target: f64,
        synthetic: bool,
    ) -> RecoPick {
        RecoPick {
            stock_code: code.into(),
            stock_name: name.into(),
            sector: None,
            style: Style::Trend,
            period: Period::Short,
            price,
            entry_low: price * 0.98,
            entry_high: price * 1.02,
            stop_loss: price * 0.95,
            target_price: target,
            position_pct: 5.0,
            holding_days: 5,
            confidence: conf,
            reasons: vec![],
            risk_notes: vec![],
            secondary_styles: vec![],
            synthetic,
            kline_as_of: None,
        }
    }

    #[test]
    fn build_notification_empty() {
        let (title, body) = build_notification(&[]);
        assert!(title.contains("智能荐股更新"));
        assert!(body.contains("未发现"));
    }

    #[test]
    fn build_notification_truncates_to_8_lines() {
        let picks: Vec<RecoPick> = (0..15)
            .map(|i| pick(&format!("6{i:05}"), "X", 80, 10.0, 11.0, false))
            .collect();
        let (title, body) = build_notification(&picks);
        assert!(title.contains("15"));
        // 最多 8 行：前 7 只 + 1 行 "等共 N"
        let line_count = body.lines().count();
        assert!(line_count <= 8, "got {} lines: {}", line_count, body);
        assert!(body.contains("等共 15"), "应包含 footer: {body}");
    }

    #[test]
    fn build_notification_uses_upside_pct() {
        let p = pick("688056", "莱伯泰科", 85, 48.0, 53.0, false);
        let (_, body) = build_notification(&[p]);
        // 53/48 - 1 = 0.1042 → +10.4%
        assert!(body.contains("+10.4%"), "body: {body}");
    }

    #[test]
    fn build_notification_handles_zero_price() {
        let mut p = pick("688056", "X", 80, 0.0, 10.0, false);
        p.price = 0.0;
        p.target_price = 10.0;
        let (_, body) = build_notification(&[p]);
        // upside 应该是 0% 而非 NaN/Inf
        assert!(body.contains("+0.0%"), "body: {body}");
    }
}
