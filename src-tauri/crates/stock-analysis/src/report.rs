use std::collections::HashMap;

/// Generate an HTML visualization report from all analysis data
#[allow(clippy::too_many_arguments)]
pub fn generate_html_report(
    stock_code: &str,
    stock_name: &str,
    analysis_date: &str,
    quote_json: &str,
    indicators: &axagent_astock_data::indicators::TechnicalIndicators,
    score_json: &str,
    analyst_reports: &HashMap<String, String>,
    decision_json: &str,
    quality_summary: &str,
    rule_check_result: &str,
    value_assessment_json: &str,
    block_trades_json: &str,
    institutional_visits_json: &str,
    index_quotes_json: &str,
    peers_json: &str,
    option_pcr_json: &str,
) -> String {
    let quote: serde_json::Value = serde_json::from_str(quote_json).unwrap_or_default();
    let price = quote["price"].as_f64().unwrap_or(0.0);
    let change_pct = quote["changePct"].as_f64().unwrap_or(0.0);
    let score: serde_json::Value = serde_json::from_str(score_json).unwrap_or_default();
    let decision: serde_json::Value = serde_json::from_str(decision_json).unwrap_or_default();
    let value: serde_json::Value = serde_json::from_str(value_assessment_json).unwrap_or_default();
    let buffett_verdict = value["buffett_verdict"].as_str().unwrap_or("-");
    let margin_of_safety = value["margin_of_safety_pct"].as_f64().unwrap_or(0.0);
    let f_score = value["f_score"].as_f64().unwrap_or(0.0);
    let moat = value["moat_score"].as_f64().unwrap_or(0.0);

    let block_trades: Vec<serde_json::Value> =
        serde_json::from_str(block_trades_json).unwrap_or_default();
    let block_trades_section = if block_trades.is_empty() {
        String::new()
    } else {
        let mut rows = String::new();
        for bt in block_trades.iter().take(5) {
            let date = bt["date"].as_str().unwrap_or("-");
            let price = bt["price"].as_f64().unwrap_or(0.0);
            let vol = bt["volume"].as_f64().unwrap_or(0.0);
            let buyer = bt["buyer"].as_str().unwrap_or("-");
            let seller = bt["seller"].as_str().unwrap_or("-");
            rows.push_str(&format!(
                "<tr><td>{date}</td><td>{price:.2}</td><td>{vol:.0}</td><td>{buyer}</td><td>{seller}</td></tr>"
            ));
        }
        format!("<h3 style=\"margin:16px 0 8px\">大宗交易</h3><table style=\"width:100%;font-size:12px;border-collapse:collapse\"><tr style=\"color:#8b949e\"><th>日期</th><th>价格</th><th>数量</th><th>买方</th><th>卖方</th></tr>{rows}</table>")
    };

    let visits: Vec<serde_json::Value> =
        serde_json::from_str(institutional_visits_json).unwrap_or_default();
    let institutional_visits_section = if visits.is_empty() {
        String::new()
    } else {
        let mut rows = String::new();
        for v in visits.iter().take(5) {
            let date = v["date"].as_str().unwrap_or("-");
            let orgs = v["org_count"].as_u64().unwrap_or(0);
            let content = v["content"].as_str().unwrap_or("-");
            let short = if content.len() > 60 {
                content.chars().take(60).collect::<String>()
            } else {
                content.to_string()
            };
            rows.push_str(&format!("<tr><td>{date}</td><td>{orgs}</td><td>{short}</td></tr>"));
        }
        format!("<h3 style=\"margin:16px 0 8px\">机构调研</h3><table style=\"width:100%;font-size:12px;border-collapse:collapse\"><tr style=\"color:#8b949e\"><th>日期</th><th>机构数</th><th>内容</th></tr>{rows}</table>")
    };

    let index_quotes: Vec<serde_json::Value> =
        serde_json::from_str(index_quotes_json).unwrap_or_default();
    let index_quotes_section = if index_quotes.is_empty() {
        String::new()
    } else {
        let mut rows = String::new();
        for idx in index_quotes.iter() {
            let name = idx["name"].as_str().unwrap_or("-");
            let price = idx["price"].as_f64().unwrap_or(0.0);
            let pct = idx["changePct"].as_f64().unwrap_or(0.0);
            let color = if pct >= 0.0 { "#3fb950" } else { "#f85149" };
            let sign = if pct >= 0.0 { "+" } else { "" };
            rows.push_str(&format!(
                "<tr><td>{name}</td><td>{price:.2}</td><td style=\"color:{color}\">{sign}{pct:.2}%</td></tr>"
            ));
        }
        format!("<h3 style=\"margin:16px 0 8px\">大盘指数</h3><table style=\"width:100%;font-size:12px;border-collapse:collapse\"><tr style=\"color:#8b949e\"><th>指数</th><th>点位</th><th>涨跌幅</th></tr>{rows}</table>")
    };

    let peers: Vec<serde_json::Value> = serde_json::from_str(peers_json).unwrap_or_default();
    let peers_section = if peers.is_empty() {
        String::new()
    } else {
        let mut rows = String::new();
        for p in peers.iter().take(8) {
            let code = p["stockCode"].as_str().unwrap_or("-");
            let name = p["stockName"].as_str().unwrap_or("-");
            let pe = p["pe"]
                .as_f64()
                .map(|v| format!("{v:.1}"))
                .unwrap_or("-".into());
            let pb = p["pb"]
                .as_f64()
                .map(|v| format!("{v:.1}"))
                .unwrap_or("-".into());
            let roe = p["roe"]
                .as_f64()
                .map(|v| format!("{v:.1}%"))
                .unwrap_or("-".into());
            let pct = p["changePct"].as_f64().unwrap_or(0.0);
            let color = if pct >= 0.0 { "#3fb950" } else { "#f85149" };
            let sign = if pct >= 0.0 { "+" } else { "" };
            rows.push_str(&format!(
                "<tr><td>{code} {name}</td><td>{pe}</td><td>{pb}</td><td>{roe}</td><td style=\"color:{color}\">{sign}{pct:.2}%</td></tr>"
            ));
        }
        format!("<h3 style=\"margin:16px 0 8px\">同行业可比公司</h3><table style=\"width:100%;font-size:12px;border-collapse:collapse\"><tr style=\"color:#8b949e\"><th>股票</th><th>PE</th><th>PB</th><th>ROE</th><th>涨跌幅</th></tr>{rows}</table>")
    };

    let option_pcr: serde_json::Value = serde_json::from_str(option_pcr_json).unwrap_or_default();
    let option_pcr_section = if option_pcr.is_null() || option_pcr_json.is_empty() {
        String::new()
    } else {
        let vol_pcr = option_pcr["volumePcr"].as_f64().unwrap_or(0.0);
        let oi_pcr = option_pcr["oiPcr"].as_f64().unwrap_or(0.0);
        let call_vol = option_pcr["callVolume"].as_f64().unwrap_or(0.0);
        let put_vol = option_pcr["putVolume"].as_f64().unwrap_or(0.0);
        let vol_color = if vol_pcr > 1.0 { "#f85149" } else { "#3fb950" };
        let oi_color = if oi_pcr > 1.0 { "#f85149" } else { "#3fb950" };
        format!(
            "<h3 style=\"margin:16px 0 8px\">期权PCR（情绪指标）</h3>\
            <div class=\"grid\">\
            <div class=\"card\"><h3>成交量PCR</h3><div class=\"value\" style=\"color:{vol_color}\">{vol_pcr:.2}</div><div style=\"font-size:12px;color:#8b949e\">看跌{put_vol:.0} / 看涨{call_vol:.0}</div></div>\
            <div class=\"card\"><h3>持仓量PCR</h3><div class=\"value\" style=\"color:{oi_color}\">{oi_pcr:.2}</div><div style=\"font-size:12px;color:#8b949e\">&gt;1 看空偏强，&lt;1 看多偏强</div></div>\
            </div>"
        )
    };

    // Build analyst report HTML
    let mut analyst_html = String::new();
    for expert_id in &[
        "market-analyst",
        "sentiment-analyst",
        "news-analyst",
        "fundamentals-analyst",
        "policy-analyst",
        "hot-money-tracker",
        "lockup-watcher",
    ] {
        if let Some(report) = analyst_reports.get(*expert_id) {
            let name = get_analyst_display_name(expert_id);
            let escaped = report
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('\n', "<br>");
            analyst_html.push_str(&format!(
                r#"<div class='report-card'><h3>{}</h3><p>{}</p></div>"#,
                name, escaped
            ));
        }
    }

    let total = score["total"].as_u64().unwrap_or(0);
    let trend = score["trendScore"].as_u64().unwrap_or(0);
    let deviation = score["deviationScore"].as_u64().unwrap_or(0);
    let macd_s = score["macdScore"].as_u64().unwrap_or(0);
    let volume_s = score["volumeScore"].as_u64().unwrap_or(0);
    let rsi_s = score["rsiScore"].as_u64().unwrap_or(0);
    let support_s = score["supportScore"].as_u64().unwrap_or(0);
    let signal = score["signal"].as_str().unwrap_or("-");
    let action = decision["action"].as_str().unwrap_or("-");
    let position = decision["positionPct"].as_f64().unwrap_or(0.0);
    let reasoning = decision["reasoning"].as_str().unwrap_or("");

    let price_color = if change_pct >= 0.0 {
        "#3fb950"
    } else {
        "#f85149"
    };
    let change_sign = if change_pct >= 0.0 { "+" } else { "" };
    let score_color = if total >= 60 {
        "#3fb950"
    } else if total >= 30 {
        "#d29922"
    } else {
        "#f85149"
    };
    let action_color = match action {
        "买入" | "增持" => "#3fb950",
        "卖出" | "减持" => "#f85149",
        _ => "#d29922",
    };
    let ma5 = indicators.ma5;
    let ma10 = indicators.ma10;
    let ma20 = indicators.ma20;
    let ma60 = indicators.ma60;
    let dif = indicators.macd_dif;
    let dea = indicators.macd_dea;
    let bar = indicators.macd_bar;
    let macd_signal = &indicators.macd_signal;
    let rsi6 = indicators.rsi6;
    let rsi12 = indicators.rsi12;
    let rsi24 = indicators.rsi24;
    let vol_ratio = indicators.volume_ratio;

    format!(
        r###"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>股票分析报告 - {stock_name}({stock_code})</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;background:#0f1117;color:#e1e4e8;padding:20px;max-width:1000px;margin:0 auto}}
.header{{text-align:center;padding:30px 0;border-bottom:1px solid #30363d;margin-bottom:20px}}
.header h1{{font-size:28px;margin-bottom:8px}}
.header .sub{{color:#8b949e;font-size:14px}}
.price-row{{display:flex;gap:20px;justify-content:center;align-items:baseline;margin:16px 0}}
.price{{font-size:36px;font-weight:bold;color:{price_color}}}
.change{{font-size:20px;color:{price_color}}}
.grid{{display:grid;grid-template-columns:repeat(3,1fr);gap:12px;margin:16px 0}}
.card{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:16px}}
.card h3{{font-size:12px;color:#8b949e;text-transform:uppercase;margin-bottom:8px}}
.card .value{{font-size:20px;font-weight:bold}}
.score-bar{{height:8px;border-radius:4px;background:#21262d;margin-top:8px}}
.score-fill{{height:100%;border-radius:4px;transition:width .3s}}
.score-grid{{display:grid;grid-template-columns:repeat(3,1fr);gap:8px;margin:8px 0}}
.score-item{{background:#161b22;padding:10px;border-radius:6px;text-align:center}}
.score-item .label{{font-size:11px;color:#8b949e}}
.score-item .val{{font-size:18px;font-weight:bold}}
.reports{{margin:20px 0}}
.report-card{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:16px;margin-bottom:10px}}
.report-card h3{{font-size:14px;margin-bottom:8px;color:#58a6ff}}
.report-card p{{font-size:13px;line-height:1.6;color:#c9d1d9}}
.decision{{background:linear-gradient(135deg,#1a2332,#0f1a25);border:2px solid {action_color};border-radius:12px;padding:24px;margin:20px 0;text-align:center}}
.decision h2{{font-size:22px;margin-bottom:8px}}
.decision .action{{font-size:32px;font-weight:bold;color:{action_color};margin:8px 0}}
.footer{{text-align:center;color:#8b949e;font-size:12px;margin-top:30px;padding-top:20px;border-top:1px solid #30363d}}
</style>
</head>
<body>
<div class="header">
  <h1>{stock_name} ({stock_code})</h1>
  <div class="sub">分析日期: {analysis_date}</div>
</div>
<div class="price-row">
  <span class="price">¥{price:.2}</span>
  <span class="change">{change_sign}{change_pct:.2}%</span>
</div>
<div class="grid">
  <div class="card">
    <h3>综合评分</h3>
    <div class="value" style="color:{score_color}">{total}/100</div>
    <div class="score-bar"><div class="score-fill" style="width:{total}%;background:{score_color}"></div></div>
  </div>
  <div class="card">
    <h3>技术信号</h3>
    <div class="value" style="font-size:16px">{signal}</div>
    <div style="font-size:12px;color:#8b949e;margin-top:4px">{indicators_ma_alignment}</div>
  </div>
  <div class="card">
    <h3>投资建议</h3>
    <div class="value" style="color:{action_color}">{action}</div>
    <div style="font-size:12px;color:#8b949e;margin-top:4px">仓位 {position}%</div>
  </div>
</div>

<h3 style="margin:16px 0 8px">评分明细</h3>
<div class="score-grid">
  <div class="score-item"><div class="label">趋势</div><div class="val">{trend}/30</div></div>
  <div class="score-item"><div class="label">乖离率</div><div class="val">{deviation}/20</div></div>
  <div class="score-item"><div class="label">MACD</div><div class="val">{macd_s}/15</div></div>
  <div class="score-item"><div class="label">量能</div><div class="val">{volume_s}/15</div></div>
  <div class="score-item"><div class="label">RSI</div><div class="val">{rsi_s}/10</div></div>
  <div class="score-item"><div class="label">支撑</div><div class="val">{support_s}/10</div></div>
</div>

<h3 style="margin:16px 0 8px">技术指标</h3>
<div class="grid">
  <div class="card"><h3>均线</h3><div style="font-size:13px">MA5: {ma5:.2}<br>MA10: {ma10:.2}<br>MA20: {ma20:.2}<br>MA60: {ma60:.2}</div></div>
  <div class="card"><h3>MACD</h3><div style="font-size:13px">DIF: {dif:.2}<br>DEA: {dea:.2}<br>柱: {bar:.2}<br>{macd_signal}</div></div>
  <div class="card"><h3>RSI &amp; 量能</h3><div style="font-size:13px">RSI6: {rsi6:.0}<br>RSI12: {rsi12:.0}<br>RSI24: {rsi24:.0}<br>量比: {vol_ratio:.1}x</div></div>
</div>

<h3 style="margin:16px 0 8px">价值评估</h3>
<div class="grid">
  <div class="card"><h3>巴菲特判定</h3><div class="value" style="font-size:16px">{buffett_verdict}</div></div>
  <div class="card"><h3>安全边际</h3><div class="value">{margin_of_safety:.1}%</div></div>
  <div class="card"><h3>F-Score / 护城河</h3><div class="value">{f_score:.0}/9 · {moat:.0}/10</div></div>
</div>

{block_trades_section}

{institutional_visits_section}

{index_quotes_section}

{peers_section}

{option_pcr_section}

<div class="decision">
  <h2>最终决策</h2>
  <div class="action">{action}</div>
  <div style="font-size:14px;margin-top:8px">{reasoning}</div>
</div>

<h3 style="margin:16px 0 8px">分析师报告</h3>
<div class="reports">{analyst_html}</div>

<div style="margin:12px 0;padding:12px;background:#161b22;border-radius:8px;font-size:12px;color:#8b949e">
  <strong>质量评估:</strong> {quality_summary}<br>
  <strong>规则检查:</strong> {rule_check_result}
</div>

<div class="footer">
  <p>⚠️ 本报告由 AxInvest AI 分析引擎生成，仅供参考，不构成投资建议。</p>
  <p>股市有风险，投资需谨慎。Generated at {analysis_date}</p>
</div>
</body>
</html>"###,
        indicators_ma_alignment = indicators.ma_alignment,
    )
}

fn get_analyst_display_name(id: &str) -> &str {
    match id {
        "market-analyst" => "市场技术分析师",
        "sentiment-analyst" => "情绪面分析师",
        "news-analyst" => "消息面分析师",
        "fundamentals-analyst" => "基本面分析师",
        "policy-analyst" => "政策面分析师",
        "hot-money-tracker" => "资金面追踪者",
        "lockup-watcher" => "筹码面观察者",
        _ => id,
    }
}
