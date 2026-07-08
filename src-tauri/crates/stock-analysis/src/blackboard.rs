//! 工作流结果 → blackboard_snapshot 转换器
//!
//! 时间旅行模式下，snapshot 自动注入 `_meta` 块（mode / as_of_date / source / built_at），
//! 供回放审计与跨日报告聚合使用。

use axagent_astock_data::as_of::{AsOfContext, DegradationEntry};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// 工作流结果 → blackboard_snapshot（JSON 对象，键名采用约定前缀）
///
/// 键名约定（与 `stock_analysis.rs` 读取端保持一致）：
///   - `report.<nodeId>`         分析师报告（如 `report.a-fundamentals`）
///   - `report.investment-plan`  交易员方案（trader 节点映射到该键）
///   - `value.assessment`        价值投资（巴菲特）评估
///   - `rule_check.summary`      规则检查结果
///   - `data_quality_summary`    数据质量总评
///   - `raw.combined`            原始数据聚合
///   - `degraded`                时间锚定降级报告（spec §4.1）
///   - `_meta`                   元数据：mode / as_of_date / source / built_at
///   - `params.<nodeId>`         结构化参数（Agent 节点的 .params + CodeNode 的 .result）
///   - 其余节点（debate/risk/research-mgr/portfolio-mgr 等）按 nodeId 存
///
/// `degradation` 由调用方在 workflow 完成时通过 `as_of::take_asof_degradation_report()`
/// 取出后传入。live 模式下该函数返回空 Vec,生成的 `degraded` 块也为空对象。
pub fn build_blackboard_snapshot(
    results: &HashMap<String, Value>,
    as_of_ctx: Option<&AsOfContext>,
    degradation: &[DegradationEntry],
) -> HashMap<String, Value> {
    let mut bb: HashMap<String, Value> = HashMap::new();
    for (node_id, raw_output) in results {
        let key = match node_id.as_str() {
            // 9 个分析师 + value-investor：归到 report.* 前缀
            id if id.starts_with("a-") => format!("report.{id}"),
            // 交易员：归到 report.investment-plan
            "trader" => "report.investment-plan".to_string(),
            // 价值投资评估
            "value-investor" => "value.assessment".to_string(),
            // 规则检查
            "rule-check" => "rule_check.summary".to_string(),
            // 数据质量
            "data-quality" => "data_quality_summary".to_string(),
            // 原始数据聚合
            "raw-data" => "raw.combined".to_string(),
            // _meta 保留给元数据，节点不会命名为这个
            "_meta" => continue,
            // 其余按 nodeId 直接存
            _ => node_id.clone(),
        };

        // 辩论/风险/研究经理/投资组合经理等结构化节点：保留原始 JSON
        // 前端 loadAnalysis 根据 key 前缀做结构化还原。这些节点的输出含有嵌套
        // 结构（如 { content, params, rounds }），flatten 成纯文本会丢失信息。
        let is_structured = node_id.starts_with("bull-r")
            || node_id.starts_with("bear-r")
            || node_id.starts_with("risk-")
            || *node_id == "agg-risk"
            || *node_id == "debate-bull-bear"
            || *node_id == "debate-convergence"
            || *node_id == "value-investor"
            || *node_id == "research-mgr"
            || *node_id == "portfolio-mgr"
            // V40 修复: data-quality(JSON模式含grade/score字段)、
            // rule-check(含violations/corrections)、
            // quality-fallback(含决策JSON)的输出均含结构化字段，
            // 需保留JSON结构而非用extract_node_text提取纯文本。
            || *node_id == "data-quality"
            || *node_id == "rule-check"
            || *node_id == "quality-fallback";
        if is_structured {
            // 结构化节点：优先用 report + verdict 重构带 VERDICT 标签的文本
            let mut value_to_store = raw_output.clone();
            if let Some(obj) = raw_output.as_object() {
                if let Some(verdict) = obj.get("verdict") {
                    if let Some(report) = obj.get("report").and_then(|v| v.as_str()) {
                        let reconstructed = format!("{}<!-- VERDICT: {} -->", report, verdict);
                        value_to_store = Value::String(reconstructed);
                    }
                }
            }
            bb.insert(key, value_to_store);
        } else {
            let mut text = extract_node_text(raw_output);
            // 如果 content 是 JSON 且含有 verdict 字段（strict_mode 重构格式），
            // 把 verdict 补回 <!-- VERDICT: ... --> 标签，让前端 tryParseVerdictFormat 能识别
            if let Some(obj) = raw_output.as_object() {
                if let Some(content_str) = obj.get("content").and_then(|v| v.as_str()) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(content_str) {
                        if let Some(verdict) = json.get("verdict") {
                            if let Some(report) = json.get("report").and_then(|v| v.as_str()) {
                                text = format!("{}<!-- VERDICT: {} -->", report, verdict);
                            }
                        }
                    }
                }
            }
            bb.insert(key, Value::String(text));
        }

        // ── 结构化参数专用存储（结构化参数方案 Phase 4）──
        // 保存每个节点的结构化参数，供 What-If 回测 UI 读取原始参数值
        // 并允许用户修改后重算。
        // 注意：extract_node_text 会丢失 JSON 结构，因此需要单独保存。
        if let Some(obj) = raw_output.as_object() {
            // V37 修复: AgentNode 输出无顶层 .params 字段（只有 CodeNode 有）。
            // AgentNode 的业务参数在 .content JSON 字符串内部，需解析后提取。
            // CodeNode 的决策结果在 .result 字段，存到 result.{node_id} 以示区分。
            if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                if let Ok(parsed) = serde_json::from_str::<Value>(content) {
                    if parsed.is_object() {
                        bb.insert(format!("params.{node_id}"), parsed.clone());
                    }
                }
            }
            // CodeNode 直接执行：.result 在顶层，存到 result.{node_id}
            if let Some(result) = obj.get("result") {
                bb.insert(format!("result.{node_id}"), result.clone());
            }
            // 兼容：如果 AgentNode 的顶层有 .params（非标准但保留），
            // 同时存到 .content 解析结果之后（后者优先）
            if let Some(params) = obj.get("params") {
                if !params.is_null() && !bb.contains_key(&format!("params.{node_id}")) {
                    bb.insert(format!("params.{node_id}"), params.clone());
                }
            }
        }
    }
    bb.insert("_meta".into(), build_meta(as_of_ctx));
    // 降级报告: spec §4.1 统一降级协议
    // 结构: { count, sources: [{ vendor, method, reason, asOf }] }
    let sources: Vec<Value> = degradation
        .iter()
        .map(|d| {
            json!({
                "vendor": d.vendor,
                "method": d.method,
                "reason": d.reason,
                "asOf": d.as_of,
            })
        })
        .collect();
    bb.insert(
        "degraded".into(),
        json!({
            "count": degradation.len(),
            "sources": sources,
        }),
    );
    // 注入原始节点输出（以 _raw.{nodeId} 为键），供 rerun_decision 等工具读取
    // 上游节点使用原始 nodeId 而非被黑板上 key remapping 改写后的键。
    for (node_id, raw_output) in results {
        bb.insert(format!("_raw.{node_id}"), raw_output.clone());
    }
    bb
}

fn build_meta(as_of_ctx: Option<&AsOfContext>) -> Value {
    let mut meta = Map::new();
    meta.insert(
        "mode".into(),
        json!(if as_of_ctx.is_some() {
            "replay"
        } else {
            "live"
        }),
    );
    meta.insert(
        "built_at".into(),
        json!(chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()),
    );
    if let Some(ctx) = as_of_ctx {
        meta.insert("as_of_date".into(), json!(ctx.as_string()));
        meta.insert("source".into(), json!(ctx.source.to_string()));
    }
    Value::Object(meta)
}

fn extract_node_text(v: &Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    if let Some(obj) = v.as_object() {
        for k in ["output", "text", "content", "message", "result", "data"] {
            if let Some(s) = obj.get(k).and_then(|v| v.as_str()) {
                return s.to_string();
            }
        }
        if let Some(nested) = obj.get("output").and_then(|o| o.as_object()) {
            for k in ["text", "content", "message", "result"] {
                if let Some(s) = nested.get(k).and_then(|v| v.as_str()) {
                    return s.to_string();
                }
            }
        }
    }
    v.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_astock_data::as_of::AsOfSource;
    use chrono::NaiveDate;

    #[test]
    fn live_meta_block_has_mode_live_and_no_asof() {
        let bb = build_blackboard_snapshot(&HashMap::new(), None, &[]);
        let meta = bb.get("_meta").expect("must have _meta");
        assert_eq!(meta["mode"], json!("live"));
        assert!(meta.get("as_of_date").is_none());
        assert!(meta["built_at"].is_string());
    }

    #[test]
    fn replay_meta_block_includes_asof_date_and_source() {
        let ctx =
            AsOfContext::new(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(), AsOfSource::UserReplay)
                .unwrap();
        let bb = build_blackboard_snapshot(&HashMap::new(), Some(&ctx), &[]);
        let meta = bb.get("_meta").expect("must have _meta");
        assert_eq!(meta["mode"], json!("replay"));
        assert_eq!(meta["as_of_date"], json!("2026-06-01"));
        assert_eq!(meta["source"], json!("user_replay"));
    }

    #[test]
    fn backtest_sweep_source_serialized() {
        let ctx = AsOfContext::new(
            NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
            AsOfSource::BacktestSweep,
        )
        .unwrap();
        let bb = build_blackboard_snapshot(&HashMap::new(), Some(&ctx), &[]);
        assert_eq!(bb["_meta"]["source"], json!("backtest_sweep"));
    }

    #[test]
    fn meta_block_does_not_clobber_existing_node_keys() {
        let mut results = HashMap::new();
        results.insert("a-fundamentals".into(), json!("text"));
        results.insert("trader".into(), json!("trader text"));
        let bb = build_blackboard_snapshot(&results, None, &[]);
        assert_eq!(bb["report.a-fundamentals"], json!("text"));
        assert_eq!(bb["report.investment-plan"], json!("trader text"));
        assert!(bb.contains_key("_meta"));
    }

    #[test]
    fn node_id_with_dash_prefix_uses_report_key() {
        let mut results = HashMap::new();
        results.insert("a-policy".into(), json!("policy text"));
        let bb = build_blackboard_snapshot(&results, None, &[]);
        assert!(bb.contains_key("report.a-policy"));
    }

    #[test]
    fn unknown_node_id_stored_verbatim() {
        let mut results = HashMap::new();
        results.insert("debate-bull-bear".into(), json!("debate"));
        let bb = build_blackboard_snapshot(&results, None, &[]);
        assert_eq!(bb["debate-bull-bear"], json!("debate"));
    }

    #[test]
    fn degraded_block_is_empty_in_live_mode() {
        let bb = build_blackboard_snapshot(&HashMap::new(), None, &[]);
        assert_eq!(bb["degraded"]["count"], json!(0));
        assert_eq!(bb["degraded"]["sources"], json!([]));
    }

    #[test]
    fn degraded_block_records_entries_with_asof_string() {
        let entries = vec![DegradationEntry {
            vendor: "eastmoney".into(),
            method: "get_industry_ranking".into(),
            reason: "as-of 截断后,排名无 N 日前对比语义".into(),
            as_of: "2026-06-01".into(),
        }];
        let bb = build_blackboard_snapshot(&HashMap::new(), None, &entries);
        assert_eq!(bb["degraded"]["count"], json!(1));
        assert_eq!(bb["degraded"]["sources"][0]["vendor"], json!("eastmoney"));
        assert_eq!(bb["degraded"]["sources"][0]["asOf"], json!("2026-06-01"));
    }

    #[test]
    fn extract_node_text_handles_nested_output() {
        let v = json!({"output": {"text": "hello"}});
        assert_eq!(extract_node_text(&v), "hello");
        let v2 = json!({"text": "x"});
        assert_eq!(extract_node_text(&v2), "x");
        let v3 = json!("raw string");
        assert_eq!(extract_node_text(&v3), "raw string");
    }
}
