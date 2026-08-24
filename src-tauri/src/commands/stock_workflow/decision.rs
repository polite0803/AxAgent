use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;
use axagent_astock_data::as_of::AsOfContext;
use axagent_entities::stock_analyses;
use axagent_harness::workflow_types::{JsonSchema, Variable, WorkflowEdge, WorkflowNode};
#[cfg(test)]
use axagent_rt_workflow::NodeRuntimeState;
use axagent_rt_workflow::{NodeStatus, Workflow};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use serde_json::json;
use tauri::State;

/// 单个数据源缺失条目（结构化报告用）
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DataMissingItem {
    pub source: String,
    /// "failed" = 全部 Vendor 降级链失败, "partial" = 成功获取但数据不完整
    pub status: String,
    pub detail: String,
}

/// 聚合预检结果：数据充分/部分缺失/完全不足
#[derive(Debug, Clone)]
pub(crate) enum QualityPrecheckResult {
    /// 数据充分，可以执行
    Pass,
    /// 部分数据缺失但可继续
    Partial(String),
    /// 数据不足，跳过（含结构化缺失清单，供前端展示数据缺失报告）
    Insufficient { summary: String, missing_sources: Vec<DataMissingItem> },
}

/// P1-3: 单数据源预检结果(供多源聚合用)
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceCheck {
    /// 该源充分
    Ok,
    /// 该源部分缺失,但可继续
    Partial(String),
    /// 该源完全失败(数据为零或 vendor 报错)
    Failed(String),
}

/// P1-3: 聚合 5 个核心数据源的预检结果, 取最差等级
pub(crate) fn aggregate_precheck(sources: Vec<(&str, SourceCheck)>) -> QualityPrecheckResult {
    let mut partial_msgs: Vec<String> = Vec::new();
    let mut missing_sources: Vec<DataMissingItem> = Vec::new();
    for (name, c) in sources {
        match c {
            SourceCheck::Ok => {},
            SourceCheck::Partial(reason) => partial_msgs.push(format!("{name}: {reason}")),
            SourceCheck::Failed(reason) => {
                missing_sources.push(DataMissingItem {
                    source: name.to_string(),
                    status: "failed".into(),
                    detail: reason,
                });
            },
        }
    }
    if !missing_sources.is_empty() {
        let summary = missing_sources
            .iter()
            .map(|item| format!("{}: {}", item.source, item.detail))
            .collect::<Vec<_>>()
            .join("; ");
        QualityPrecheckResult::Insufficient { summary, missing_sources }
    } else if !partial_msgs.is_empty() {
        QualityPrecheckResult::Partial(partial_msgs.join("; "))
    } else {
        QualityPrecheckResult::Pass
    }
}

/// 在启动 DAG 前执行快速数据质量检查。
///
/// P1-3 修复: 扩展预检覆盖 5 个核心数据源(quote / financials / klines / news /
/// money_flow),任一完全失败则整体 Insufficient;部分缺失则 Partial。as-of 模式下
/// 所有 vendor 调用走 as-of scope, 预检结果反映"截至 as_of_date 的数据是否够用"。
///
/// API 调用成本: 5 次 vs 原 2 次, 仍远低于 15~20 次 LLM 调用。
///
/// P1-1 修复(2026-08-09): 接入 astock-data/validation.rs 的字段级校验
/// (validate_quote/validate_financials/validate_klines/validate_news_batch)，
/// 将"只查非空/行数"升级为 OHLC 有效性、high<low、eps/roe 值域等字段检查。
/// P2-3 修复(2026-08-09): 新增跨源一致性校验——
///   quote.price vs 最新 K 线 close(偏差>1%)、quote.pe vs 财报 EPS 隐含 PE(>20%)、
///   K 线最新日期未来/滞后检查。命中即 Partial 告警,不阻断。
pub(crate) async fn data_quality_precheck(
    client: &axagent_astock_data::AStockClient,
    stock_code: &str,
    quote: &axagent_astock_data::StockQuote,
) -> QualityPrecheckResult {
    use axagent_astock_data::validation::{
        validate_financials, validate_klines, validate_news_batch, validate_quote,
    };

    // 1. quote — 字段级校验（code/price/name 缺失 → Failed；change_pct/pre_close 异常 → Partial）
    let quote_check = {
        let vr = validate_quote(quote);
        if !vr.missing.is_empty() {
            SourceCheck::Failed(vr.missing.join("; "))
        } else if !vr.warnings.is_empty() {
            SourceCheck::Partial(vr.warnings.join("; "))
        } else {
            SourceCheck::Ok
        }
    };

    // 2. financials — 营收/利润存在性 + 字段值域校验（eps/roe 异常）+ PE 交叉校验
    let fin_check = match client.get_financials(stock_code).await {
        Ok(financials) => {
            let has_revenue = financials.iter().any(|f| f.revenue.unwrap_or(0.0) > 0.0);
            let has_profit = financials.iter().any(|f| f.net_profit.unwrap_or(0.0) > 0.0);
            let vr = validate_financials(&financials);
            if !has_revenue && !has_profit {
                // 空财报/营收利润缺失：保持原 Partial 语义（可继续但基本面受限），不升级 Failed
                SourceCheck::Partial("营收/利润缺失".into())
            } else if !vr.missing.is_empty() {
                SourceCheck::Failed(vr.missing.join("; "))
            } else if !vr.warnings.is_empty() {
                SourceCheck::Partial(vr.warnings.join("; "))
            } else {
                // P2-3: quote.pe vs 最新财报 EPS 隐含 PE 交叉校验
                let mut cross: Vec<String> = Vec::new();
                if let (Some(pe), Some(eps)) = (quote.pe, financials.first().and_then(|f| f.eps)) {
                    if pe > 0.0 && eps > 0.0 && quote.price > 0.0 {
                        let implied_pe = quote.price / eps;
                        let dev = (implied_pe - pe).abs() / pe;
                        if dev > 0.2 {
                            cross.push(format!(
                                "行情 PE {pe} 与财报 EPS 隐含 PE {implied_pe:.1} 偏差 {:.0}%",
                                dev * 100.0
                            ));
                        }
                    }
                }
                if cross.is_empty() {
                    SourceCheck::Ok
                } else {
                    SourceCheck::Partial(cross.join("; "))
                }
            }
        },
        Err(e) => SourceCheck::Failed(format!("全部数据源获取失败: {e}")),
    };

    // V38 修复: K 线至少需要 60 日才能计算 MA(20)+MACD(26) 等关键技术指标。
    // 不足 60 日但 ≥30 日时仅降级为 Partial（可继续但技术分析受限）。
    // P1-1(2026-08-09): 行数足够时追加 validate_klines 字段校验（OHLC/high<low 等）。
    // P2-3(2026-08-09): 追加 quote.price vs 最新 close 偏差、K 线日期时效交叉校验。
    let kline_check = match client.get_klines(stock_code, "daily", 500).await {
        Ok(klines) if klines.len() >= 60 => {
            let vr = validate_klines(&klines);
            if !vr.missing.is_empty() {
                SourceCheck::Partial(format!("K 线字段异常: {}", vr.missing.join("; ")))
            } else {
                let mut cross: Vec<String> = Vec::new();
                if let Some(last) = klines.last() {
                    if last.close > 0.0 && quote.price > 0.0 {
                        let dev = (quote.price - last.close).abs() / last.close;
                        if dev > 0.01 {
                            cross.push(format!(
                                "最新 K 线收盘 {} 与行情价 {} 偏差 {:.1}%",
                                last.close,
                                quote.price,
                                dev * 100.0
                            ));
                        }
                    }
                    // 日期时效: 未来日期(>当前/回放日)或滞后 >10 自然日 → 告警
                    let asof = axagent_astock_data::as_of::current_date_or_now();
                    if let (Ok(d1), Ok(d2)) = (
                        chrono::NaiveDate::parse_from_str(&last.date, "%Y-%m-%d"),
                        chrono::NaiveDate::parse_from_str(&asof, "%Y-%m-%d"),
                    ) {
                        let days = (d2 - d1).num_days();
                        if days < 0 {
                            cross.push(format!(
                                "K 线最新日期 {} 晚于当前/回放日 {}",
                                last.date, asof
                            ));
                        } else if days > 10 {
                            cross.push(format!(
                                "K 线数据滞后 {days} 天(最新 {} vs 当前/回放日 {asof})",
                                last.date
                            ));
                        }
                    }
                }
                if cross.is_empty() {
                    SourceCheck::Ok
                } else {
                    SourceCheck::Partial(cross.join("; "))
                }
            }
        },
        Ok(klines) if klines.len() >= 30 => {
            SourceCheck::Partial(format!("仅 {} 行, 技术分析受限", klines.len()))
        },
        Ok(klines) if !klines.is_empty() => {
            SourceCheck::Partial(format!("仅 {} 行, 严重不足", klines.len()))
        },
        Ok(_) => SourceCheck::Failed("K 线为空".into()),
        Err(e) => SourceCheck::Failed(format!("全部数据源获取失败: {e}")),
    };

    // P1-3 新增: 4. news (取最近 10 条)
    // P1-1(2026-08-09): 非空时追加 title/url/publish_time 字段校验
    let news_check = match client.get_news(stock_code, 10).await {
        Ok(news) if !news.is_empty() => {
            let vr = validate_news_batch(&news);
            if !vr.missing.is_empty() {
                SourceCheck::Partial(format!("新闻字段异常: {}", vr.missing.join("; ")))
            } else {
                SourceCheck::Ok
            }
        },
        Ok(_) => SourceCheck::Partial("无新闻数据".into()),
        Err(e) => SourceCheck::Failed(format!("全部数据源获取失败: {e}")),
    };

    // P1-3 新增: 5. money_flow
    let money_flow_check = match client.get_money_flow(stock_code).await {
        Ok(Some(_)) => SourceCheck::Ok,
        Ok(None) => SourceCheck::Partial("无资金流数据".into()),
        Err(e) => SourceCheck::Failed(format!("全部数据源获取失败: {e}")),
    };

    // P2: 补充数据源检查 — 覆盖 catalyst-analyst / sector-analyst 的依赖
    let announcements_check = match client.get_announcements(stock_code).await {
        Ok(anns) if !anns.is_empty() => SourceCheck::Ok,
        Ok(_) => SourceCheck::Partial("无公告数据".into()),
        Err(e) => SourceCheck::Failed(format!("全部数据源获取失败: {e}")),
    };
    let concept_check = match client.get_concept_blocks(stock_code).await {
        Ok(Some(blocks)) if !blocks.concepts.is_empty() => SourceCheck::Ok,
        Ok(_) => SourceCheck::Partial("无概念板块数据".into()),
        Err(e) => SourceCheck::Failed(format!("概念板块数据源全部获取失败: {e}")),
    };

    // V40 修复: 补充对核心分析师依赖的数据源预检（不阻塞分析，仅标记 Partial）
    // a-sector / a-catalyst 依赖 sector_info；a-lockup 依赖 lockup_schedule
    let sector_check = match client.get_sector_info(stock_code).await {
        Ok(Some(_)) => SourceCheck::Ok,
        Ok(None) => SourceCheck::Partial("无行业板块数据".into()),
        Err(e) => SourceCheck::Failed(format!("行业板块数据源全部获取失败: {e}")),
    };
    let lockup_check = match client.get_lockup_schedule(stock_code).await {
        Ok(items) if !items.is_empty() => SourceCheck::Ok,
        Ok(_) => SourceCheck::Partial("无限售解禁数据".into()),
        Err(e) => SourceCheck::Failed(format!("限售解禁数据源全部获取失败: {e}")),
    };
    // PACE 集成: 补充 dragon_tiger 预检（筹码面 f10 增强所需的机构席位数据）
    let dragon_tiger_check = match client.get_dragon_tiger(stock_code).await {
        Ok(entries) if !entries.is_empty() => SourceCheck::Ok,
        Ok(_) => SourceCheck::Partial("无龙虎榜数据".into()),
        Err(e) => SourceCheck::Failed(format!("龙虎榜数据源全部获取失败: {e}")),
    };

    // 全部数据源统一由 aggregate_precheck 判定：
    // - 任一数据源 Failed（所有 Vendor 降级链均失败）→ 整体 Insufficient，阻断工作流
    // - 全部通过但存在 Partial（成功获取但某维度天然空）→ 整体 Partial，继续但标记警告
    // - 全部通过且无 Partial → Pass
    aggregate_precheck(vec![
        ("quote", quote_check),
        ("financials", fin_check),
        ("klines", kline_check),
        ("news", news_check),
        ("money_flow", money_flow_check),
        ("announcements", announcements_check),
        ("concept_blocks", concept_check),
        ("sector_info", sector_check),
        ("lockup_schedule", lockup_check),
        ("dragon_tiger", dragon_tiger_check),
    ])
}

pub(crate) struct LoadedTemplate {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Option<Vec<Variable>>,
}

/// 从模板变量解析 vendor_* 布尔开关，注入到 astock_client 的启用状态过滤器。
/// 未启用的 vendor 会在 find_vendor 中被跳过，避免无效调用和超时重试。
/// 应在 load_and_inject_template 之后、run_workflow 之前调用。
pub(crate) fn inject_vendor_state(
    astock_client: &axagent_astock_data::AStockClient,
    variables: Option<&Vec<Variable>>,
) {
    if let Some(vars) = variables {
        let template_vars: Vec<(String, serde_json::Value)> =
            vars.iter().map(|v| (v.name.clone(), v.value.clone())).collect();
        let enabled_set =
            axagent_analysis_engine::recommender::pool::load_enabled_vendors_from_template(
                &template_vars,
            );
        tracing::info!("[stock_workflow] vendor 启用状态已注入: {:?}", enabled_set);
        astock_client.set_enabled_vendors(Some(enabled_set));
    }
}

#[cfg(test)]
mod precheck_tests {
    use super::*;

    // P1-3: aggregate_precheck 取最差等级
    #[test]
    fn aggregate_all_ok_returns_pass() {
        let r = aggregate_precheck(vec![
            ("quote", SourceCheck::Ok),
            ("financials", SourceCheck::Ok),
            ("klines", SourceCheck::Ok),
        ]);
        assert!(matches!(r, QualityPrecheckResult::Pass));
    }

    #[test]
    fn aggregate_one_partial_returns_partial_with_joined_message() {
        let r = aggregate_precheck(vec![
            ("quote", SourceCheck::Ok),
            ("financials", SourceCheck::Partial("营收缺失".into())),
            ("klines", SourceCheck::Ok),
        ]);
        match r {
            QualityPrecheckResult::Partial(msg) => {
                assert!(msg.contains("financials"), "partial msg 应含 source 名: {msg}");
                assert!(msg.contains("营收缺失"));
            },
            _ => panic!("expected Partial"),
        }
    }

    #[test]
    fn aggregate_any_failure_returns_insufficient() {
        let r = aggregate_precheck(vec![
            ("quote", SourceCheck::Ok),
            ("klines", SourceCheck::Failed("K 线获取失败".into())),
        ]);
        match r {
            QualityPrecheckResult::Insufficient { summary, .. } => {
                assert!(
                    summary.contains("klines"),
                    "insufficient summary 应含 source 名: {summary}"
                );
                assert!(summary.contains("K 线获取失败"));
            },
            _ => panic!("expected Insufficient"),
        }
    }

    #[test]
    fn aggregate_failure_beats_partial() {
        // 5 源: 2 partial + 1 failed → overall Insufficient
        let r = aggregate_precheck(vec![
            ("quote", SourceCheck::Ok),
            ("financials", SourceCheck::Partial("缺".into())),
            ("klines", SourceCheck::Failed("空了".into())),
            ("news", SourceCheck::Partial("无".into())),
            ("money_flow", SourceCheck::Ok),
        ]);
        assert!(matches!(r, QualityPrecheckResult::Insufficient { .. }));
    }
}

pub(crate) async fn load_and_inject_template(
    db: &sea_orm::DatabaseConnection,
    stock_code: &str,
    _stock_name: &str,
    template_id: &str,
) -> Result<LoadedTemplate, String> {
    use axagent_entities::workflow_template;

    let template = workflow_template::Entity::find_by_id(template_id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询工作流模板失败: {e}"))
        })?
        .ok_or(format!("工作流模板 {template_id} 未种子化，请重启应用"))?;

    let mut nodes: Vec<WorkflowNode> = serde_json::from_str(&template.nodes).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("解析模板节点失败: {e}"))
    })?;
    let edges: Vec<WorkflowEdge> = serde_json::from_str(&template.edges).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("解析模板边失败: {e}"))
    })?;

    if nodes.is_empty() {
        tracing::warn!("[stock_workflow] 模板节点为空，自动重新种子化");
        crate::commands::stock_analysis_setup::ensure_stock_analysis_experts_seeded(db).await?;
        let template = workflow_template::Entity::find_by_id("stock-analysis")
            .one(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("重查模板失败: {e}"))
            })?
            .ok_or("模板种子化后仍不存在")?;
        nodes = serde_json::from_str(&template.nodes).map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("解析模板节点失败: {e}"))
        })?;
    }

    for node in &mut nodes {
        if let WorkflowNode::Trigger(tn) = node {
            if let Some(sc) = tn.config.config.get_mut("stock_code") {
                *sc = serde_json::Value::String(stock_code.to_string());
            }
        }
    }

    // stock_code/stock_name 已通过 AgentNodeConfig.input_mapping 自动注入到每个 Agent 节点的 system_prompt，
    // 不再需要手动遍历追加（参见 stock_analysis_setup.rs 中 agent() 宏的 input_mapping 配置）。

    let input_schema: Option<JsonSchema> =
        template.input_schema.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let output_schema: Option<JsonSchema> =
        template.output_schema.as_ref().and_then(|s| serde_json::from_str(s).ok());
    let variables: Option<Vec<Variable>> =
        template.variables.as_ref().and_then(|v| serde_json::from_str(v).ok());

    Ok(LoadedTemplate { nodes, edges, input_schema, output_schema, variables })
}

/// 工作流结果 → blackboard_snapshot — 现已委托给 axagent-stock-analysis::blackboard 模块
/// 此处保留占位以便未来重新内联
#[allow(clippy::type_complexity)]
pub(crate) fn extract_decision_fields(
    decision_json: &Option<String>,
) -> (Option<String>, Option<f64>, Option<String>, Option<String>, Option<u32>) {
    let raw = match decision_json {
        Some(s) if !s.is_empty() => s,
        _ => return (None, None, None, None, None),
    };
    let parsed: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return (None, None, None, None, None),
    };
    let action = parsed.get("action").and_then(|v| v.as_str()).map(|s| s.to_string());
    let position_pct =
        parsed.get("positionPct").or_else(|| parsed.get("position_pct")).and_then(|v| v.as_f64());
    let reasoning = parsed.get("reasoning").and_then(|v| v.as_str()).map(|s| s.to_string());
    let time_horizon = parsed
        .get("timeHorizon")
        .or_else(|| parsed.get("time_horizon"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let expected_holding_days = parsed
        .get("expectedHoldingDays")
        .or_else(|| parsed.get("expected_holding_days"))
        .and_then(|v| {
            if v.is_number() {
                v.as_u64().map(|n| n as u32)
            } else {
                None
            }
        });
    (action, position_pct, reasoning, time_horizon, expected_holding_days)
}

/// 从 Workflow 结果中提取 portfolio-mgr 节点的决策 JSON 字符串。
///
/// 优先取 `results["portfolio-mgr"]["result"]`（CodeNode 包装内 Rhai 脚本的
/// 实际输出，例如 `{ action, positionPct, confidence, ... }`），回退到
/// `results["portfolio-mgr"]` 本身（兼容非 CodeNode 包装的旧版 portfolio-mgr），
/// 最后回退到 workflow 顶层 `output`（兼容无 portfolio-mgr 节点的工作流）。
///
/// 修复"决策信息缺失"误报：之前直接用 `wf.output` 写入 decisionJson，
/// 但 stock-analysis 工作流配置了 output_schema（且未用 $source 标记字段
/// 来源节点），导致 `filter_by_schema` 退化为整个 results map。前端
/// normalizeDecision 拿到 results map 后会判定为"全零空壳"返回 null，
/// store.decision 保持空 → DecisionBanner 显示"决策信息缺失"误报。
pub(crate) fn extract_decision_json(wf: &Workflow) -> Option<String> {
    if let Some(pm) = wf.results.get("portfolio-mgr") {
        // CodeNode 包装: { status, result, input_params, node_id, params }
        // 实际决策在 .result 字段;若 .result 缺失(旧版/异常路径)则降级用
        // 整个 pm 值,让 extract_decision_fields 至少能拿到 action 等字段。
        //
        // V63 修复: portfolio-mgr 也可能是 {node_id, output, source, status}
        // 格式（不含 result/params），决策数据在 .output 字段中。
        let actual = match pm {
            serde_json::Value::Object(obj) => {
                if let Some(result) = obj.get("result") {
                    result.clone()
                } else if let Some(output) = obj.get("output") {
                    output.clone()
                } else {
                    pm.clone()
                }
            },
            _ => pm.clone(),
        };
        if let Ok(s) = serde_json::to_string(&actual) {
            return Some(s);
        }
    }
    // V40 修复: 当 quality-gate 判定为 D/F 时，portfolio-mgr 公式决策被
    // quality-fallback(AgentNode)的保守决策替代。此时取 quality-fallback 的
    // content JSON 作为最终决策，确保前端 DB 展示与质量门禁路径一致。
    if let Some(qf) = wf.results.get("quality-fallback") {
        if let Some(content_str) = qf.get("content").and_then(|v| v.as_str()) {
            // quality-fallback 输出格式: {"action":"持有/减持/卖出","positionPct":0-20,"confidence":20-40,"riskLevel":"高风险","reasoning":"..."}
            // P0 修复: 若 LLM 未严格遵循 prompt 缺失 confidence/riskLevel 字段，补充合理保守默认值
            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(content_str) {
                if let Some(obj) = v.as_object_mut() {
                    if !obj.contains_key("confidence") {
                        obj.insert("confidence".to_string(), json!(30.0));
                    }
                    if !obj.contains_key("riskLevel") {
                        obj.insert("riskLevel".to_string(), json!("高风险"));
                    }
                    if !obj.contains_key("decisionConfidence") {
                        if let Some(c) = obj.get("confidence").and_then(|c| c.as_f64()) {
                            obj.insert("decisionConfidence".to_string(), json!(c));
                        }
                    }
                }
                return Some(v.to_string());
            }
        }
    }
    // ── V57 硬化：portfolio-mgr 节点未成功产出结果时（Failed / Skipped /
    // 因上游失败被跳过 / 从未运行），rt-workflow 的 apply_node_status_update
    // 仅在 result=Some 时才写入 results（见 rt-workflow engine/mod.rs），
    // 故 results["portfolio-mgr"] 缺位，旧逻辑回退到 wf.output 会再次产出
    // "全零空壳"/决策缺失。此处携带 node_states["portfolio-mgr"].error 作可见
    // 诊断，并给出最小有效决策 action:"观望"，避免 UI 静默"决策缺失"。
    // 注意：只要节点状态不是 Completed（即没拿到有效 result）就触发，
    // 覆盖 Rhai 运行时错误(Failed)与上游失败导致 Skipped 两种空壳来源。
    if let Some(state) = wf.node_states.get("portfolio-mgr") {
        if state.status != NodeStatus::Completed {
            let error_msg = state.error.clone().unwrap_or_else(|| match state.status {
                NodeStatus::Skipped => {
                    "portfolio-mgr 节点被跳过（上游依赖失败导致，无本地错误详情）".to_string()
                },
                NodeStatus::Failed => "portfolio-mgr 节点执行失败（无错误详情）".to_string(),
                _ => "portfolio-mgr 节点未完成（状态非 Completed）".to_string(),
            });
            let mut fallback = serde_json::Map::new();
            fallback.insert("action".to_string(), json!("观望"));
            fallback.insert("positionPct".to_string(), json!(0));
            fallback.insert("confidence".to_string(), json!(0));
            fallback.insert("riskLevel".to_string(), json!("未知"));
            fallback.insert("timeHorizon".to_string(), json!("短期"));
            fallback.insert(
                "reasoning".to_string(),
                json!("组合管理节点未产出有效决策，已降级为保守观望。详见 diagnostics.nodeError。"),
            );
            let mut diag = serde_json::Map::new();
            diag.insert("node".to_string(), json!("portfolio-mgr"));
            diag.insert("nodeStatus".to_string(), json!(format!("{:?}", state.status)));
            diag.insert("nodeError".to_string(), json!(error_msg.clone()));
            let hint = if state.status == NodeStatus::Skipped {
                "portfolio-mgr 被 Skipped：检查其上游依赖节点（trader/research-mgr/a-catalyst/t-risk 等）是否失败或超时，错误在对应 node_states[上游].error。"
            } else {
                "portfolio-mgr Rhai 运行失败：检查未走 present() 直接引用的变量、除零或类型错误。"
            };
            diag.insert("hint".to_string(), json!(hint));
            fallback.insert("diagnostics".to_string(), serde_json::Value::Object(diag));
            return serde_json::to_string(&serde_json::Value::Object(fallback)).ok();
        }
    }

    // 回退: workflow 顶层 output(无 output_schema 或非 stock-analysis 工作流)
    //
    // P4 修复(2026-07-25):若 wf.output 是 results map(顶层含 stock-analysis
    // 已知节点 ID 之一,如 trigger/portfolio-mgr/trader 等),说明 portfolio-mgr
    // 节点未产出有效结果且 node_states 也无记录(可能是工作流异常终止或 rt-workflow
    // 未写入 node_states 的边界场景)。此时直接序列化 wf.output 会让前端
    // normalizeDecision 拿到 results map → 识别为 results map 后因 portfolio-mgr
    // 缺失而判为"全零空壳"返回 null → 前端日志噪音 + UI 闪烁。
    //
    // 修复策略:检测到 results map 时,走最小占位结构(与 V57 硬化同款),
    // 给前端一个明确的"决策缺失,已降级为观望"信号,而非含糊的 results map。
    if let Some(output) = wf.output.as_ref() {
        // 检测:顶层是 object 且含已知 workflow 节点 ID
        let is_results_map = output
            .as_object()
            .map(|obj| {
                obj.contains_key("portfolio-mgr")
                    || obj.contains_key("trigger")
                    || obj.contains_key("end-output")
                    || obj.contains_key("research-mgr")
                    || obj.contains_key("trader")
                    || obj.contains_key("value-investor")
                    || obj.contains_key("debate-convergence")
                    || obj.contains_key("raw-data")
                    || obj.contains_key("t-quote")
                    || obj.contains_key("t-kline")
            })
            .unwrap_or(false);
        if is_results_map {
            tracing::warn!(
                "[extract_decision_json] portfolio-mgr 缺位且 node_states 无记录,wf.output 是 results map,降级为最小占位决策"
            );
            let mut fallback = serde_json::Map::new();
            fallback.insert("action".to_string(), json!("观望"));
            fallback.insert("positionPct".to_string(), json!(0));
            fallback.insert("confidence".to_string(), json!(0));
            fallback.insert("riskLevel".to_string(), json!("未知"));
            fallback.insert("timeHorizon".to_string(), json!("短期"));
            fallback.insert(
                "reasoning".to_string(),
                json!("组合管理节点未产出有效决策,已降级为保守观望。portfolio-mgr 节点缺位且 node_states 无状态记录。"),
            );
            let mut diag = serde_json::Map::new();
            diag.insert("node".to_string(), json!("portfolio-mgr"));
            diag.insert("nodeStatus".to_string(), json!("Missing"));
            diag.insert(
                "nodeError".to_string(),
                json!("portfolio-mgr 节点在 results 和 node_states 中均缺位,可能工作流异常终止"),
            );
            diag.insert(
                "hint".to_string(),
                json!("检查工作流执行日志,确认 portfolio-mgr 节点是否被正确调度。若为工作流引擎 bug,需排查 rt-workflow engine 的节点写入逻辑。"),
            );
            fallback.insert("diagnostics".to_string(), serde_json::Value::Object(diag));
            return serde_json::to_string(&serde_json::Value::Object(fallback)).ok();
        }
        serde_json::to_string(output).ok()
    } else {
        None
    }
}

/// 从 Workflow 结果中提取 trader 节点的 LLM 决策 JSON。
///
/// trader 节点输出格式:
/// ```json
/// { "stance": "买入", "positionPct": 35, "confidence": 0.72,
///   "summary": "...", "key_points": [...], "scenarios": [...] }
/// ```
///
/// 用作"方案 D 双向并存"的 LLM 视角,与 portfolio-mgr 公式视角对比。
/// 优先取 `results["trader"]["result"]`（AgentNode 包装内的实际输出），
/// 回退到 `results["trader"]` 本身。
pub(crate) fn extract_llm_decision_json(wf: &Workflow) -> Option<String> {
    let trader = wf.results.get("trader")?;
    // V37 修复: trader 是 AgentNode，输出结构为 {role, content: <json_string>, ...}，
    // LLM 的业务字段（action/targetPrice/confidence）在 content JSON 字符串内部。
    // 旧代码取 .result（CodeNode 的字段），AgentNode 无此字段→永远 fallback 到包装对象，
    // 导致 compute_decision_agreement 拿不到 action 字段，一致性分数走兜底。
    // V41 修复: content 是 JSON 字符串，需解析为 JSON 对象再序列化后存储。
    // 旧代码直接 serialize Value::String(content)，导致 DB 中存储的是双重嵌套
    // 的 JSON 字符串（前端 JSON.parse 后仍是字符串而非对象）。
    match trader {
        serde_json::Value::Object(obj) => {
            if let Some(content_str) = obj.get("content").and_then(|v| v.as_str()) {
                // 解析 content 内层 JSON 字符串为 JSON 对象，再序列化
                if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(content_str) {
                    // V60 修复: 展开 report 字段的嵌套 JSON
                    // LLM 可能输出 {report: '{...}'} 格式，实际决策数据（verdict/currentPrice/confidence 等）
                    // 在 report 值的 JSON 字符串内部。将其展开到顶层，使前端 extractLlmField 能直接读取。
                    if let Some(report_str) = parsed.get("report").and_then(|v| v.as_str()) {
                        if let Ok(report_parsed) =
                            serde_json::from_str::<serde_json::Value>(report_str)
                        {
                            if let Some(report_obj) = report_parsed.as_object() {
                                let obj = parsed.as_object_mut().expect("parsed is already Object");
                                for (k, v) in report_obj {
                                    // 不覆盖顶层已有的同名字段
                                    obj.entry(k).or_insert_with(|| v.clone());
                                }
                            }
                        }
                    }
                    // V64 修复: report 展开后, 若顶层有 verdict 无 action,
                    // 将 verdict 映射为 action (trader 输出方向标签而非操作指令)
                    let needs_action = parsed.get("action").and_then(|v| v.as_str()).is_none();
                    if needs_action {
                        // 先 clone verdict 值, 避免与后续 mutable borrow 冲突
                        let verdict_clone =
                            parsed.get("verdict").and_then(|v| v.as_str()).map(|s| s.to_string());
                        if let Some(ref v) = verdict_clone {
                            let mapped: &str = match v.trim() {
                                "看多" | "看涨" => "买入",
                                "看空" | "看跌" => "卖出",
                                "中性" | "震荡" => "持有",
                                "不确定" | "无法判断" => "观望",
                                _ => v.trim(), // 兜底: 保留原值, 让 normalize_llm_action 处理
                            };
                            if let Some(obj) = parsed.as_object_mut() {
                                obj.insert(
                                    "action".into(),
                                    serde_json::Value::String(mapped.to_string()),
                                );
                            }
                        }
                    }
                    // V46 修复: 标准化 LLM 输出的 action 字段
                    // trader prompt 规定 action ∈ {买入,增持,持有,减持,卖出,观望},
                    // 但 LLM 可能输出"不确定""未知"等非标准值（尤其是当数据矛盾时
                    // LLM 选择输出"不确定"作为逃逸）。
                    // 通过白名单强制映射, 防止 DB 和 UI 出现非标准值。
                    // 注意: 不修改 targetPrice/stopLoss/confidence 等数值字段,
                    // 它们错误时 portfolio-mgr 的 sanity 预检会兜底。
                    normalize_llm_action(&mut parsed);
                    return serde_json::to_string(&parsed).ok();
                }
                // 解析失败时回退：返回原始 content 字符串
                return Some(content_str.to_string());
            }
            serde_json::to_string(trader).ok()
        },
        _ => serde_json::to_string(trader).ok(),
    }
}

/// 标准化 LLM 输出的 action 字段, 映射非标准值到标准值。
///
/// 标准值: 买入, 增持, 持有, 减持, 卖出, 观望
/// 非标准值映射规则:
///   "不确定" / "未知" / "? " / "" → "观望" (无判断 → 不操作)
///   "回避" / "远离" / "清仓" / "止损" → "卖出" (明确看空 → 卖出)
///   "卖" / "sell" → "卖出", "买" / "buy" → "买入"
///   "减" → "减持"
pub(crate) fn normalize_llm_action(parsed: &mut serde_json::Value) {
    let obj = match parsed.as_object_mut() {
        Some(o) => o,
        None => return,
    };
    let action = match obj.get("action").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => return,
    };
    let trimmed = action.trim();
    // 已在标准白名单中 → 不处理
    const STANDARD: &[&str] = &["买入", "增持", "持有", "减持", "卖出", "观望"];
    if STANDARD.contains(&trimmed) {
        return;
    }
    // V46 映射表: 把 LLM 可能输出的所有非标准值映射到标准值
    let normalized: &str = match trimmed {
        // 无判断 → 观望
        "不确定" | "未知" | "?" | "??" | "" | "无法判断" | "无法确定" => "观望",
        // 明确看空 → 卖出
        "回避" | "远离" | "清仓" | "止损" | "割肉" | "离场" => "卖出",
        // 近义词映射
        "卖" | "sell" | "做空" | "空" => "卖出",
        "买" | "buy" | "做多" | "多" => "买入",
        "减" => "减持",
        "增" | "加" => "增持",
        "持" => "持有",
        "观" => "观望",
        // 兜底: 其他未知值 → 观望（保守操作）
        _ => {
            tracing::warn!("[normalize_llm_action] 未知 action 值 {:?}, 兜底映射为观望", trimmed);
            "观望"
        },
    };
    obj.insert("action".to_string(), serde_json::Value::String(normalized.to_string()));
}

/// 双视角一致性诊断结果
///
/// V65 升级: 从 3 维度（action 50/positionPct 30/confidence 20）扩展为 6 维度
///   (action 30 + positionPct 20 + confidence 15 + riskLevel 15 + data_gaps 10 + evidence_cited 10)
/// 总分 100，低于 60 触发人工复核。
///
/// 上层可根据维度详情:
///   - 决定 confidence 调制幅度
///   - 生成分歧诊断 reasoning 文本
///   - 判断是否触发人工复核
///
/// P0 修复: 保留 f7 自指污染标记字段，标注公式决策中 trader 因子(f7)的参与程度，
/// 帮助识别"公式已含 trader 观点"导致一致性虚高或逻辑矛盾。
pub(crate) struct AgreementBreakdown {
    /// 总分 0-100（6 维度加权）
    pub total: i32,
    /// action 维度原始分 (满分 30)
    pub action_score: f64,
    /// action 是否基本一致 (>= 20 分)
    pub action_ok: bool,
    /// action 一致性说明 (exact_match / same_direction / opposite / ...)
    pub action_note: String,
    /// 公式视角的 action 原始值
    pub formula_action: String,
    /// LLM 视角的 action 原始值
    pub llm_action: String,
    /// positionPct 维度原始分 (满分 20)
    pub position_score: f64,
    /// 仓位差值绝对值
    pub position_gap: Option<f64>,
    /// confidence 维度原始分 (满分 15)
    pub confidence_score: f64,
    /// 置信度差值绝对值
    pub confidence_gap: Option<f64>,
    /// V65 新增: riskLevel 维度原始分 (满分 15)
    pub risk_level_score: f64,
    /// V65 新增: 公式 riskLevel 原始值
    pub formula_risk_level: String,
    /// V65 新增: LLM riskLevel 原始值
    pub llm_risk_level: String,
    /// V65 新增: data_gaps 维度原始分 (满分 10)
    pub data_gaps_score: f64,
    /// V65 新增: data_gaps Jaccard 相似度 (0-1)
    pub data_gaps_similarity: Option<f64>,
    /// V65 新增: evidence_cited 维度原始分 (满分 10)
    pub evidence_score: f64,
    /// V65 新增: LLM 引用上游论据数量
    pub evidence_count: i32,
    /// 冲突类型: all_agree / opposite_direction / action_divergence / position_gap / confidence_gap / risk_gap / data_gaps_diverge
    pub conflict_type: String,
    // ── P0: f7 自指污染标记（向后兼容保留）──
    /// 公式决策中 f7（trader 因子）权重占总权重百分比。None=无 f7 数据。
    pub f7_weight_pct: Option<f64>,
    /// 排除 f7 后的"纯净"后验值（0~1）。None=无 f7 数据。
    pub f7_free_posterior: Option<f64>,
    /// 排除 f7 后的"纯净"action。None=无 f7 数据。
    pub f7_free_action: Option<String>,
    /// 无 f7 版本的 action 一致性原始分 (满分 30，与主 action_score 相同语义)
    pub f7_free_action_score: Option<f64>,
}

/// 计算公式决策与 LLM 决策的一致性分数（0-100）。
///
/// V65 升级：6 维度对比，对应 trader.md 中"双视角对比说明"的权重分配：
///   - action: 30 分（精确匹配 30 / 同向 20 / 中性不同义 5-10 / 对立 0）
///   - positionPct: 20 分（≤10% 满分 20 / ≤20% 半分 10 / >20% 零分 0）
///   - confidence: 15 分（差值 ≤10 满分 15 / ≤20 半分 10 / 否则 5）
///   - riskLevel: 15 分（精确匹配 15 / 相邻 8 / 跨级 0）
///   - data_gaps: 10 分（Jaccard 相似度 × 10）
///   - evidence 引用密度: 10 分（≥3 条满分 10 / 2 条 5 / <2 条 0）
///
/// 归一化规则（与前端 normalizeAction 保持一致）:
/// - 移除空格/斜杠/下划线/全角空格
/// - 小写比较
/// - "买"和"增持"视为一致，"卖"和"减持"视为一致
pub(crate) fn compute_decision_agreement(
    formula_json: Option<&str>,
    llm_json: Option<&str>,
) -> Option<AgreementBreakdown> {
    let fj = serde_json::from_str::<serde_json::Value>(formula_json?).ok()?;
    let lj = serde_json::from_str::<serde_json::Value>(llm_json?).ok()?;

    // 归一化操作字符串
    let norm = |s: &str| s.trim().to_lowercase().replace([' ', '/', '_', '\u{3000}'], "");

    // ── 公式字段 ──
    let f_action = fj.get("action").and_then(|v| v.as_str().map(norm));
    let f_pos = fj.get("positionPct").and_then(|v| v.as_f64());
    let f_conf = fj.get("confidence").and_then(|v| v.as_f64());
    // V65: 公式 riskLevel / data_gaps
    let f_risk = fj.get("riskLevel").and_then(|v| v.as_str()).map(norm).unwrap_or_default();
    let f_gaps: std::collections::HashSet<String> = fj
        .get("data_gaps")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(norm)).collect())
        .unwrap_or_default();

    // ── LLM 字段（V65: trader 现在输出完整字段）──
    let l_action = lj.get("action").and_then(|v| v.as_str().map(norm));
    let l_pos = lj.get("positionPct").and_then(|v| v.as_f64());
    let l_conf = lj.get("confidence").and_then(|v| v.as_f64());
    let l_risk = lj.get("riskLevel").and_then(|v| v.as_str()).map(norm).unwrap_or_default();
    let l_gaps: std::collections::HashSet<String> = lj
        .get("data_gaps")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(norm)).collect())
        .unwrap_or_default();
    // V65: evidence_cited 数量
    let evidence_count: i32 = lj
        .get("evidence_cited")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len() as i32)
        .unwrap_or(0);

    // V50: 保存原始 action 值用于诊断展示
    let f_action_raw = fj.get("action").and_then(|v| v.as_str()).unwrap_or("?").to_string();
    let l_action_raw = lj.get("action").and_then(|v| v.as_str()).unwrap_or("?").to_string();
    // 预计算维度差值
    let pos_gap: Option<f64> = match (f_pos, l_pos) {
        (Some(a), Some(b)) => Some((a - b).abs()),
        _ => None,
    };
    let conf_gap: Option<f64> = match (f_conf, l_conf) {
        (Some(a), Some(b)) => Some((a - b).abs()),
        _ => None,
    };

    // ── V65: action 评分（满分 30，原满分 50 的 60%）──
    // 精确匹配 30 > 同向同类 20 > 中性不同义 5-10 > 对立 0
    let is_buy = |s: &str| s.contains("买") || s.contains("增持");
    let is_sell = |s: &str| s.contains("卖") || s.contains("减持");
    let is_hold = |s: &str| s == "持有";
    let is_watch = |s: &str| s == "观望";
    let is_uncertain = |s: &str| s.contains("不确定") || s.contains("未知");
    let action_score: f64 = match (f_action.clone(), l_action.clone()) {
        (Some(a), Some(b)) if a == b => 30.0,
        (Some(a), Some(b)) if is_buy(&a) && is_buy(&b) => 20.0,
        (Some(a), Some(b)) if is_sell(&a) && is_sell(&b) => 20.0,
        // 中性但不同义: 持有 vs 观望 = 10
        (Some(a), Some(b)) if (is_hold(&a) && is_watch(&b)) || (is_hold(&b) && is_watch(&a)) => {
            10.0
        },
        // 明确中性 vs 不确定: 持有/观望 vs 不确定 = 3
        (Some(a), Some(b)) if (is_hold(&a) || is_watch(&a)) && is_uncertain(&b) => 3.0,
        (Some(a), Some(b)) if (is_hold(&b) || is_watch(&b)) && is_uncertain(&a) => 3.0,
        // 观望 vs 不确定 = 6
        (Some(a), Some(b))
            if is_watch(&a) && is_uncertain(&b) || is_watch(&b) && is_uncertain(&a) =>
        {
            6.0
        },
        // 对立方向
        (Some(_), Some(_)) => 0.0,
        // 单侧缺失
        _ => 15.0,
    };

    // ── V65: positionPct 评分（满分 20，原满分 30 的 2/3）──
    let pos_score: f64 = match (f_pos, l_pos) {
        (Some(a), Some(b)) => {
            let diff = (a - b).abs();
            if diff <= 10.0 {
                20.0
            } else if diff <= 20.0 {
                10.0
            } else {
                0.0
            }
        },
        // V65: 单侧缺失不再给兜底分（避免虚高），记 0
        _ => 0.0,
    };

    // ── V65: confidence 评分（满分 15，原满分 20 的 75%）──
    // 注意：trader 的 confidence 是 0-100 整数，公式也是 0-100
    let conf_score: f64 = match (f_conf, l_conf) {
        (Some(a), Some(b)) => {
            let diff = (a - b).abs();
            if diff <= 10.0 {
                15.0
            } else if diff <= 20.0 {
                10.0
            } else if diff <= 40.0 {
                5.0
            } else {
                0.0
            }
        },
        _ => 0.0,
    };

    // ── V65: riskLevel 评分（满分 15）──
    // 公式与 LLM 都输出 4 档风险等级，比较等级距离
    let risk_rank = |s: &str| -> i32 {
        match s {
            s if s.contains("低") => 0,
            s if s.contains("中") => 1,
            s if s.contains("高") && !s.contains("极高") => 2,
            s if s.contains("极高") => 3,
            _ => 1, // 默认中风险
        }
    };
    let f_risk_rank = risk_rank(&f_risk);
    let l_risk_rank = risk_rank(&l_risk);
    let risk_diff = (f_risk_rank - l_risk_rank).abs();
    let risk_level_score: f64 = match risk_diff {
        0 => 15.0, // 精确匹配
        1 => 8.0,  // 相邻
        _ => 0.0,  // 跨级
    };
    let f_risk_raw = fj.get("riskLevel").and_then(|v| v.as_str()).unwrap_or("?").to_string();
    let l_risk_raw = lj.get("riskLevel").and_then(|v| v.as_str()).unwrap_or("?").to_string();

    // ── V65: data_gaps 评分（满分 10）──
    // Jaccard 相似度 = |A ∩ B| / |A ∪ B|
    let data_gaps_similarity: Option<f64> = if !f_gaps.is_empty() || !l_gaps.is_empty() {
        let intersection = f_gaps.intersection(&l_gaps).count() as f64;
        let union = f_gaps.union(&l_gaps).count() as f64;
        if union > 0.0 {
            Some(intersection / union)
        } else {
            Some(1.0) // 双方都为空视为完全一致
        }
    } else {
        // 双方都无 data_gaps 字段 → None（无法判断）
        None
    };
    let data_gaps_score: f64 = data_gaps_similarity.unwrap_or(0.0) * 10.0;

    // ── V65: evidence_cited 评分（满分 10）──
    // ≥3 条满分 10 / 2 条 5 / <2 条 0
    let evidence_score: f64 = match evidence_count {
        n if n >= 3 => 10.0,
        2 => 5.0,
        _ => 0.0,
    };

    // ── V65: 6 维度加权总分 ──
    let total = (action_score
        + pos_score
        + conf_score
        + risk_level_score
        + data_gaps_score
        + evidence_score)
        .round() as i32;

    // ── P0: 从公式决策中提取 f7_free 信息（消除自指悖论）──
    let f7_free_info = fj.get("f7_free").and_then(|v| {
        if v.is_object() {
            let obj = v.as_object()?;
            let f7_weight = obj.get("f7_weight").and_then(|w| w.as_f64())?;
            let total_weight = obj.get("total_weight").and_then(|w| w.as_f64())?;
            let f7_weight_pct = if total_weight > 0.0 {
                Some((f7_weight / total_weight * 100.0 * 10.0).round() / 10.0)
            } else {
                None
            };
            let posterior = obj.get("posterior").and_then(|p| p.as_f64());
            let action = obj.get("action").and_then(|a| a.as_str().map(|s| s.to_string()));
            Some((f7_weight_pct, posterior, action))
        } else {
            None
        }
    });
    let (f7_weight_pct, f7_free_posterior, f7_free_action) =
        f7_free_info.unwrap_or((None, None, None));

    // 计算无 f7 版本的 action 一致性评分（与主 action_score 同尺度，满分 30）
    let f7_compare_target = l_action.as_deref().or(f_action.as_deref());
    let f7_free_action_score = match (f7_free_action.as_deref().map(norm), f7_compare_target) {
        (Some(a), Some(b)) if a == b => Some(30.0),
        (Some(a), Some(b)) if is_buy(&a) && is_buy(&b) => Some(20.0),
        (Some(a), Some(b)) if is_sell(&a) && is_sell(&b) => Some(20.0),
        (Some(a), Some(b)) if (is_hold(&a) && is_watch(&b)) || (is_hold(&b) && is_watch(&a)) => {
            Some(10.0)
        },
        (Some(a), Some(b)) if (is_hold(&a) || is_watch(&a)) && is_uncertain(&b) => Some(3.0),
        (Some(a), Some(b)) if (is_hold(&b) || is_watch(&b)) && is_uncertain(&a) => Some(3.0),
        (Some(a), Some(b))
            if is_watch(&a) && is_uncertain(&b) || is_watch(&b) && is_uncertain(&a) =>
        {
            Some(6.0)
        },
        (Some(_), Some(_)) => Some(0.0),
        _ => None,
    };

    // ── V65: 冲突类型分类（6 维度版）──
    let conflict_type: &str = if l_action.is_none() && evidence_count == 0 {
        // 完全无 LLM 视角输入
        match f7_free_action_score {
            Some(s) if s >= 25.0 => "f7_low_influence",
            Some(s) if s >= 15.0 => "f7_moderate_influence",
            Some(s) if s >= 8.0 => "f7_high_influence",
            _ => "f7_dominant",
        }
    } else if action_score >= 25.0
        && pos_score >= 15.0
        && conf_score >= 12.0
        && risk_level_score >= 12.0
    {
        "all_agree"
    } else if action_score == 0.0 {
        "opposite_direction"
    } else if action_score <= 5.0 {
        "action_divergence"
    } else if pos_score == 0.0 && f_pos.is_some() && l_pos.is_some() {
        "position_gap"
    } else if risk_level_score == 0.0 && !f_risk.is_empty() && !l_risk.is_empty() {
        "risk_gap"
    } else if data_gaps_score < 5.0 && data_gaps_similarity.is_some() {
        "data_gaps_diverge"
    } else {
        "confidence_gap"
    };
    // action_note 分类
    let action_note: &str = if action_score >= 30.0 {
        "exact_match"
    } else if action_score >= 20.0 {
        "same_direction"
    } else if action_score >= 10.0 {
        "hold_vs_watch"
    } else if action_score >= 6.0 {
        "watch_vs_uncertain"
    } else if action_score >= 3.0 {
        "definite_vs_uncertain"
    } else if action_score == 0.0 {
        "opposite"
    } else {
        "missing_one_side"
    };

    Some(AgreementBreakdown {
        total,
        action_score,
        action_ok: action_score >= 20.0,
        action_note: action_note.to_string(),
        formula_action: f_action_raw,
        llm_action: l_action_raw,
        position_score: pos_score,
        position_gap: pos_gap,
        confidence_score: conf_score,
        confidence_gap: conf_gap,
        risk_level_score,
        formula_risk_level: f_risk_raw,
        llm_risk_level: l_risk_raw,
        data_gaps_score,
        data_gaps_similarity,
        evidence_score,
        evidence_count,
        conflict_type: conflict_type.to_string(),
        f7_weight_pct,
        f7_free_posterior,
        f7_free_action,
        f7_free_action_score,
    })
}

/// 解析 as_of_date 入参：None/空串 → None（live），Some(s) → 解析为 AsOfContext
/// 抽出供单测：未来日期 / 错误格式必须 4xx-style 错误
pub(crate) fn parse_asof_param(s: Option<String>) -> Result<Option<AsOfContext>, String> {
    AsOfContext::parse_optional(s.as_deref())
}

/// 默认值，与 stock-analysis 模板的 defaults 保持一致；
/// 改动这里请同步 `StockAnalysisConfigPanel.getDefaultVariables()`。
/// V39 修复: 从 300s 提升到 600s，适配 max_tool_rounds=3 的多轮工具节点
/// （trader/research-mgr 等节点 3 轮 LLM+工具调用总耗时约 200-400s）。
const DEFAULT_MAX_CONCURRENT: usize = 8;
const DEFAULT_STEP_TIMEOUT_SECS: u64 = 600;
/// 工作流整体超时（秒）。单步 step_timeout 只限单节点，多步累计可能很久；
/// 总超时兜底防止 LLM 卡死或 vendor 长时间无响应导致分析永久挂起。
/// 默认 30 分钟 = 1800s，覆盖典型 10+ 节点工作流（每步 600s × 并发 8 的最坏路径）。
const DEFAULT_TOTAL_TIMEOUT_SECS: u64 = 1800;

/// 从模板 variables 中解析 RunOptions 关键参数。
///
/// 用户在「股票分析设置 → 参数」中调整 `max_concurrent` /
/// `agent_timeout_secs` / `total_timeout_secs` 后，这里读到的就是新值；
/// 如果模板里没有这些 key（旧版本 / 用户清空）则用默认值。
///
/// 容错策略：
///   * 越界 / 非法类型 → 用默认值；
///   * max_concurrent ∈ [1, 32]，过小会让并发退化为串行，过大会拖垮 LLM 速率。
///   * step_timeout ∈ [10, 3600] 秒，避免 0 或极端大值。
///   * total_timeout ∈ [60, 7200] 秒，下限 1 分钟，上限 2 小时。
pub(crate) fn resolve_runtime_options(
    variables: Option<&[axagent_harness::workflow_types::Variable]>,
) -> (usize, std::time::Duration, std::time::Duration) {
    let lookup = |name: &str| -> Option<serde_json::Value> {
        variables.and_then(|vs| vs.iter().find(|v| v.name == name)).map(|v| v.value.clone())
    };

    let max_concurrent = lookup("max_concurrent")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(1, 32) as usize)
        .unwrap_or(DEFAULT_MAX_CONCURRENT);

    let step_timeout_secs = lookup("agent_timeout_secs")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(10, 3600))
        .unwrap_or(DEFAULT_STEP_TIMEOUT_SECS);

    let total_timeout_secs = lookup("total_timeout_secs")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(60, 7200))
        .unwrap_or(DEFAULT_TOTAL_TIMEOUT_SECS);

    (
        max_concurrent,
        std::time::Duration::from_secs(step_timeout_secs),
        std::time::Duration::from_secs(total_timeout_secs),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::workflow_types::Variable;
    use serde_json::json;

    #[test]
    fn resolve_runtime_options_uses_defaults_when_missing() {
        let (mc, to, total_to) = resolve_runtime_options(None);
        assert_eq!(mc, DEFAULT_MAX_CONCURRENT);
        assert_eq!(to.as_secs(), DEFAULT_STEP_TIMEOUT_SECS);
        assert_eq!(total_to.as_secs(), DEFAULT_TOTAL_TIMEOUT_SECS);
    }

    #[test]
    fn resolve_runtime_options_reads_template_vars() {
        let vars = vec![
            Variable {
                name: "max_concurrent".into(),
                var_type: "number".into(),
                value: json!(20),
                description: None,
                is_secret: false,
            },
            Variable {
                name: "agent_timeout_secs".into(),
                var_type: "number".into(),
                value: json!(120),
                description: None,
                is_secret: false,
            },
        ];
        let (mc, to, _total_to) = resolve_runtime_options(Some(&vars));
        assert_eq!(mc, 20);
        assert_eq!(to.as_secs(), 120);
    }

    #[test]
    fn resolve_runtime_options_clamps_extremes() {
        let vars = vec![
            Variable {
                name: "max_concurrent".into(),
                var_type: "number".into(),
                value: json!(0), // 0 → clamp 到 1
                description: None,
                is_secret: false,
            },
            Variable {
                name: "agent_timeout_secs".into(),
                var_type: "number".into(),
                value: json!(99999), // 过大 → clamp 到 3600
                description: None,
                is_secret: false,
            },
        ];
        let (mc, to, _total_to) = resolve_runtime_options(Some(&vars));
        assert_eq!(mc, 1);
        assert_eq!(to.as_secs(), 3600);
    }

    #[test]
    fn resolve_runtime_options_falls_back_on_bad_types() {
        let vars = vec![Variable {
            name: "max_concurrent".into(),
            var_type: "string".into(),
            value: json!("not a number"),
            description: None,
            is_secret: false,
        }];
        let (mc, _to, _total_to) = resolve_runtime_options(Some(&vars));
        assert_eq!(mc, DEFAULT_MAX_CONCURRENT);
    }

    // ── extract_decision_json(修复"决策信息缺失"误报)──

    /// 优先取 results["portfolio-mgr"]["result"](CodeNode 包装内 Rhai 实际输出)
    #[test]
    pub(crate) fn extract_decision_json_prefers_portfolio_mgr_result() {
        use std::collections::HashMap;
        let mut results = HashMap::new();
        results.insert(
            "portfolio-mgr".to_string(),
            json!({
                "status": "executed",
                "language": "rhai",
                "result": {
                    "action": "买入",
                    "positionPct": 50.0,
                    "confidence": 75.0,
                    "riskLevel": "中",
                    "reasoning": "技术面强势",
                    "timeHorizon": "mid",
                    "expectedHoldingDays": 28,
                },
                "input_params": { "totalScore": 70.0 },
                "node_id": "portfolio-mgr",
                "params": { "action": "买入" },
            }),
        );
        // 即使 wf.output 存在且被 output_schema 污染成整个 results map,
        // 优先从 portfolio-mgr 节点本身提取。
        results.insert("trigger".to_string(), json!({ "status": "ok" }));
        let wf = Workflow {
            id: "test".to_string(),
            name: "test".to_string(),
            nodes: vec![],
            edges: vec![],
            status: axagent_rt_workflow::workflow_engine::WorkflowStatus::Completed,
            created_at: 0,
            completed_at: None,
            results,
            node_states: HashMap::new(),
            output: Some(json!({
                "trigger": { "status": "ok" },
                "portfolio-mgr": { "status": "executed", "result": { "action": "买入" } },
                "end-output": { "status": "ok" },
            })),
            error_config: None,
            error_workflow_id: None,
        };
        let dj = extract_decision_json(&wf).expect("必须返回决策 JSON");
        let parsed: serde_json::Value = serde_json::from_str(&dj).expect("必须可解析");
        // 关键:从 portfolio-mgr.result 提取,action 是 "买入" 而非被 output 污染
        assert_eq!(parsed["action"], "买入");
        assert_eq!(parsed["confidence"], 75.0);
        assert_eq!(parsed["positionPct"], 50.0);
        assert_eq!(parsed["riskLevel"], "中");
    }

    /// portfolio-mgr 是 CodeNode 包装但 .result 字段缺失(异常路径)→ 降级用包装本身
    #[test]
    pub(crate) fn extract_decision_json_falls_back_to_pm_wrapper_when_result_missing() {
        use std::collections::HashMap;
        let mut results = HashMap::new();
        results.insert(
            "portfolio-mgr".to_string(),
            json!({
                "status": "executed",
                "language": "rhai",
                // 故意无 .result 字段(异常路径)
                "params": { "action": "HOLD", "confidence": 30.0 },
                "node_id": "portfolio-mgr",
            }),
        );
        let wf = Workflow {
            id: "test".to_string(),
            name: "test".to_string(),
            nodes: vec![],
            edges: vec![],
            status: axagent_rt_workflow::workflow_engine::WorkflowStatus::Completed,
            created_at: 0,
            completed_at: None,
            results,
            node_states: HashMap::new(),
            output: None,
            error_config: None,
            error_workflow_id: None,
        };
        let dj = extract_decision_json(&wf).expect("必须返回决策 JSON");
        let parsed: serde_json::Value = serde_json::from_str(&dj).expect("必须可解析");
        // 降级用 portfolio-mgr 本身(CodeNode 包装),有 params.action
        assert_eq!(parsed["params"]["action"], "HOLD");
    }

    /// portfolio-mgr 节点不存在时回退到 wf.output(兼容无 portfolio-mgr 工作流)
    #[test]
    pub(crate) fn extract_decision_json_falls_back_to_workflow_output() {
        use std::collections::HashMap;
        let mut results = HashMap::new();
        results.insert("trigger".to_string(), json!({ "status": "ok" }));
        let wf = Workflow {
            id: "test".to_string(),
            name: "test".to_string(),
            nodes: vec![],
            edges: vec![],
            status: axagent_rt_workflow::workflow_engine::WorkflowStatus::Completed,
            created_at: 0,
            completed_at: None,
            results,
            node_states: HashMap::new(),
            output: Some(json!({ "action": "BUY", "confidence": 60.0 })),
            error_config: None,
            error_workflow_id: None,
        };
        let dj = extract_decision_json(&wf).expect("必须返回决策 JSON");
        let parsed: serde_json::Value = serde_json::from_str(&dj).expect("必须可解析");
        assert_eq!(parsed["action"], "BUY");
    }

    /// V57 硬化：portfolio-mgr 节点 Failed（Rhai 运行时错误）且 results 缺位时，
    /// 必须返回最小有效决策 action:"观望" 并携带 diagnostics.nodeError，而非空壳。
    #[test]
    pub(crate) fn extract_decision_json_hardens_failed_portfolio_mgr() {
        use std::collections::HashMap;
        let results = HashMap::new(); // 无 portfolio-mgr（节点失败未写入结果）
        let mut node_states = HashMap::new();
        node_states.insert(
            "portfolio-mgr".to_string(),
            NodeRuntimeState {
                status: NodeStatus::Failed,
                attempts: 1,
                error: Some("Rhai 执行失败: Variable not found: foo".to_string()),
                started_at: None,
                completed_at: None,
            },
        );
        let wf = Workflow {
            id: "test".to_string(),
            name: "test".to_string(),
            nodes: vec![],
            edges: vec![],
            status: axagent_rt_workflow::WorkflowStatus::Failed,
            created_at: 0,
            completed_at: None,
            results,
            node_states,
            output: None,
            error_config: None,
            error_workflow_id: None,
        };
        let dj = extract_decision_json(&wf).expect("失败节点也必须返回最小有效决策");
        let parsed: serde_json::Value = serde_json::from_str(&dj).expect("必须可解析");
        assert_eq!(parsed["action"], "观望");
        assert_eq!(parsed["positionPct"], 0.0);
        assert_eq!(parsed["confidence"], 0.0);
        assert_eq!(parsed["diagnostics"]["node"], "portfolio-mgr");
        assert_eq!(parsed["diagnostics"]["nodeStatus"], "Failed");
        assert_eq!(parsed["diagnostics"]["nodeError"], "Rhai 执行失败: Variable not found: foo");
    }

    /// V57 硬化：portfolio-mgr 因上游失败被 Skipped 时同样兜底，
    /// 不再退化成 wf.output 空壳。
    #[test]
    pub(crate) fn extract_decision_json_hardens_skipped_portfolio_mgr() {
        use std::collections::HashMap;
        let results = HashMap::new();
        let mut node_states = HashMap::new();
        node_states.insert(
            "portfolio-mgr".to_string(),
            NodeRuntimeState {
                status: NodeStatus::Skipped,
                attempts: 0,
                error: None,
                started_at: None,
                completed_at: None,
            },
        );
        let wf = Workflow {
            id: "test".to_string(),
            name: "test".to_string(),
            nodes: vec![],
            edges: vec![],
            status: axagent_rt_workflow::WorkflowStatus::PartiallyCompleted,
            created_at: 0,
            completed_at: None,
            results,
            node_states,
            output: None,
            error_config: None,
            error_workflow_id: None,
        };
        let dj = extract_decision_json(&wf).expect("跳过节点也必须返回最小有效决策");
        let parsed: serde_json::Value = serde_json::from_str(&dj).expect("必须可解析");
        assert_eq!(parsed["action"], "观望");
        assert_eq!(parsed["diagnostics"]["nodeStatus"], "Skipped");
    }

    /// P4 修复(2026-07-25): portfolio-mgr 节点在 results 和 node_states 中均缺位,
    /// 但 wf.output 是整个 results map(顶层含 trigger/portfolio-mgr 等 workflow
    /// 节点 ID)。旧逻辑直接序列化 wf.output,前端 normalizeDecision 识别为
    /// results map 后因 portfolio-mgr 缺失而判为"全零空壳"返回 null。
    ///
    /// 修复后:检测到 wf.output 是 results map 时,降级为最小占位决策
    /// (action="观望" + diagnostics.nodeStatus="Missing"),前端不再误报"全零空壳"。
    #[test]
    pub(crate) fn extract_decision_json_hardens_workflow_output_results_map() {
        use std::collections::HashMap;
        let results = HashMap::new(); // results map 为空,模拟 portfolio-mgr 从未运行
        let node_states = HashMap::new(); // node_states 也无记录
        // wf.output 是整个 workflow 的 results map,顶层是节点 ID 而非决策字段
        let wf = Workflow {
            id: "test".to_string(),
            name: "test".to_string(),
            nodes: vec![],
            edges: vec![],
            status: axagent_rt_workflow::WorkflowStatus::Failed,
            created_at: 0,
            completed_at: None,
            results,
            node_states,
            output: Some(json!({
                "trigger": { "status": "Completed", "result": "ok" },
                "research-mgr": { "status": "Completed", "content": "..." },
                "trader": { "status": "Completed", "content": "{...}" },
                // 注意:portfolio-mgr 缺位(节点未运行)
            })),
            error_config: None,
            error_workflow_id: None,
        };
        let dj = extract_decision_json(&wf).expect("results map 必须降级为最小占位决策");
        let parsed: serde_json::Value = serde_json::from_str(&dj).expect("必须可解析");
        // 不应是 results map,应是最小占位决策
        assert_eq!(parsed["action"], "观望");
        assert_eq!(parsed["positionPct"], 0.0);
        assert_eq!(parsed["confidence"], 0.0);
        assert_eq!(parsed["diagnostics"]["node"], "portfolio-mgr");
        assert_eq!(parsed["diagnostics"]["nodeStatus"], "Missing");
        // 确认没有把 trigger/research-mgr/trader 这些 results map key 写入
        assert!(parsed.get("trigger").is_none());
        assert!(parsed.get("research-mgr").is_none());
        assert!(parsed.get("trader").is_none());
    }
}

/// 从 `<!-- VERDICT: {...} -->` 标签中提取并解析 VERDICT JSON。
/// 旧版 snapshot 中数据质量报告（如 data-quality）被存储为
/// `"report文本<!-- VERDICT: {...} -->"` 格式的纯文本字符串，
/// 此函数从其中提取 VERDICT JSON 供后续字段导航恢复。
pub(crate) fn extract_verdict_from_text(text: &str) -> Option<serde_json::Value> {
    let start_marker = "<!-- VERDICT: ";
    let end_marker = "-->";
    if let Some(start) = text.rfind(start_marker) {
        let json_start = start + start_marker.len();
        if let Some(end_offset) = text[json_start..].find(end_marker) {
            let verdict_str = text[json_start..json_start + end_offset].trim();
            if !verdict_str.is_empty() {
                return serde_json::from_str::<serde_json::Value>(verdict_str).ok();
            }
        }
    }
    None
}

/// 仅重跑决策（portfolio-mgr CodeNode），不复用上游节点。
///
/// 从已有分析的 `blackboard_snapshot` 中读取缓存的所有上游节点输出，
/// 注入 portfolio-mgr 的 Rhai 脚本中重新计算决策。
/// 适用于：修改 portfolio-mgr.rhai 公式后快速验证，无需等待完整 DAG。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "重运行股票决策计算")]
#[tauri::command]
pub async fn rerun_decision(
    state: State<'_, AppState>,
    analysis_id: String,
) -> Result<serde_json::Value, String> {
    use crate::commands::error::ErrorResponse;
    use rhai::{Engine, Scope};
    use std::collections::HashMap;

    let db = state.harness.db();

    // 1. 加载分析记录
    let analysis = stock_analyses::Entity::find_by_id(&analysis_id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询分析记录失败: {e}"))
        })?
        .ok_or_else(|| format!("分析记录不存在: {analysis_id}"))?;

    // 2. 解析 blackboard_snapshot → variables map
    let snapshot_str = analysis.blackboard_snapshot.unwrap_or_default();
    let mut snapshot: HashMap<String, serde_json::Value> = serde_json::from_str(&snapshot_str)
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("解析 blackboard_snapshot 失败: {e}"))
        })?;

    // 将 _raw.{nodeId} 条目提升到顶层（去除 _raw. 前缀），使 input_mapping
    // 中的原始 nodeId 路径（如 t-scoring.result.totalScore）能正确解析。
    // _raw.* 由 build_blackboard_snapshot 在 blackboard.rs 中写入。
    let raw_keys: Vec<String> =
        snapshot.keys().filter(|k| k.starts_with("_raw.")).cloned().collect();
    if !raw_keys.is_empty() {
        for raw_key in raw_keys {
            if let Some(key) = raw_key.strip_prefix("_raw.") {
                if let Some(val) = snapshot.remove(&raw_key) {
                    // 不覆盖已有 key（remapped key 优先）
                    snapshot.entry(key.to_string()).or_insert(val);
                }
            }
        }
    } else {
        // 旧版 snapshot（无 _raw.* 前缀）：反向推导 remapped key 的原始 nodeId
        let reverse_keys: Vec<(String, String)> = snapshot
            .keys()
            .filter_map(|k| {
                // ⚠️ 特定映射必须在通用 report.* 前缀匹配之前，
                // 否则 report.investment-plan 会被 strip_prefix("report.")
                // 截成 "investment-plan" 而非正确的 "trader"
                if *k == "report.investment-plan" {
                    Some(("trader".to_string(), k.clone()))
                } else if *k == "value.assessment" {
                    Some(("value-investor".to_string(), k.clone()))
                } else if *k == "rule_check.summary" {
                    Some(("rule-check".to_string(), k.clone()))
                } else if *k == "data_quality_summary" {
                    Some(("data-quality".to_string(), k.clone()))
                } else if *k == "raw.combined" {
                    Some(("raw-data".to_string(), k.clone()))
                } else {
                    k.strip_prefix("report.").map(|id| (id.to_string(), k.clone()))
                }
            })
            .collect();
        for (orig_id, remapped_key) in reverse_keys {
            if !snapshot.contains_key(&orig_id) {
                if let Some(val) = snapshot.get(&remapped_key) {
                    snapshot.insert(orig_id, val.clone());
                }
            }
        }
    }

    // 3. 加载工作流模板 → 提取 portfolio-mgr CodeNode
    let template = axagent_entities::workflow_template::Entity::find()
        .filter(axagent_entities::workflow_template::Column::Id.eq("stock-analysis"))
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询工作流模板失败: {e}"))
        })?
        .ok_or_else(|| "工作流模板不存在".to_string())?;

    let nodes: Vec<WorkflowNode> = serde_json::from_str(&template.nodes).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("解析模板节点失败: {e}"))
    })?;

    // 找到 portfolio-mgr 节点及其 code + input_mapping
    let (code, input_mapping) = nodes
        .iter()
        .find_map(|n| {
            if let WorkflowNode::Code(cn) = n {
                if cn.config.execute_directly && cn.base.id == "portfolio-mgr" {
                    Some((cn.config.code.clone(), cn.config.input_mapping.clone()))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .ok_or_else(|| "未找到 portfolio-mgr CodeNode".to_string())?;

    // 4. 执行 Rhai 脚本（复用 code_executor 的 register_common_functions，确保函数集一致）
    // 修复历史 bug：原手动注册漏掉 json_parse，导致 portfolio-mgr.rhai 的 safe_parse 在
    // rerun 路径下失败，进而使公告关键词检测 f3、资金面 f9、筹码面 f10、龙虎榜 f10、
    // PACE f11 等依赖 safe_parse 的下游逻辑全部失去数据。
    let mut engine = Engine::new();
    // SECURITY (C4): Rhai 沙箱限制 — 防 DoS
    engine.set_max_operations(200_000);
    engine.set_max_call_levels(32);
    engine.set_max_modules(0);
    engine.set_max_string_size(2_000_000);
    engine.set_max_array_size(50_000);
    // C4 补充: portfolio-mgr.rhai 因子多、表达式嵌套深（line 518 处超默认上限），
    // 必须放宽 max_expr_depths，否则 eval 直接抛 "Expression exceeds maximum complexity"
    // （编译期错误，脚本内 try/catch 无法捕获，会导致整个节点无输出）。
    // 256 为该脚本实测所需下限(48)的 ~5 倍余量；执行总操作数仍受 set_max_operations 约束。
    engine.set_max_expr_depths(256, 256);
    axagent_rt_workflow::work_engine::executors::register_common_functions(&mut engine);
    // ── P3-1: 注册 portfolio 公式函数（替代 Rhai 内联计算）──
    // 这些函数定义在 stock-analysis::portfolio_formula，纯数学、无副作用。
    use axagent_analysis_engine::portfolio_formula;
    engine.register_fn("pm_evidence_scale", |total_weight: f64, max_weight: f64| -> f64 {
        portfolio_formula::compute_evidence_scale(total_weight, max_weight)
    });
    engine.register_fn(
        "pm_kelly_position",
        |posterior: f64, odds: f64, cost_pct: f64, risk_level: &str| -> f64 {
            portfolio_formula::compute_kelly_position(posterior, odds, cost_pct, risk_level)
        },
    );
    engine.register_fn(
        "pm_classify_risk",
        |vol: rhai::Dynamic,
         sharpe: rhai::Dynamic,
         dd: rhai::Dynamic,
         roe: rhai::Dynamic,
         debt: rhai::Dynamic,
         growth: rhai::Dynamic|
         -> String {
            // P0 修复(2026-08-09): 原 6 个 Option<f64> 参数注册后不可调用（Rhai 1.25
            // 多 Option 参数闭包 Function not found），改为 Dynamic 参数内部转换。
            let f = |v: &rhai::Dynamic| -> Option<f64> {
                v.clone()
                    .try_cast::<f64>()
                    .or_else(|| v.clone().try_cast::<i64>().map(|x| x as f64))
            };
            portfolio_formula::classify_risk(
                f(&vol),
                f(&sharpe),
                f(&dd),
                f(&roe),
                f(&debt),
                f(&growth),
            )
        },
    );
    engine.register_fn("pm_risk_bias", |risk_level: &str| -> f64 {
        portfolio_formula::compute_risk_bias(risk_level)
    });
    engine.register_fn("pm_risk_veto", |action: &str, risk_level: &str| -> String {
        let (new_action, _, _) = portfolio_formula::apply_risk_veto(action, risk_level);
        new_action
    });
    engine.register_fn(
        "pm_covariance_decay",
        |f1_w: f64, f3_w: f64, f9_w: f64, f11_w: f64, decay_target: &str| -> f64 {
            let (f9, f11) = portfolio_formula::apply_covariance_decay(f1_w, f3_w, f9_w, f11_w);
            match decay_target {
                "f9" => f9,
                "f11" => f11,
                _ => 0.0,
            }
        },
    );
    // P0: 贝叶斯因子置信度（基于 prior→posterior 的证据强度）
    engine.register_fn("pm_compute_bayes_confidence", |prior: f64, posterior: f64| -> f64 {
        portfolio_formula::compute_bayes_confidence(prior, posterior)
    });
    // 因子数据完整度：供 data-quality.rhai 评估因子层数据完整度
    // P0 修复(2026-08-09): Rhai 1.25 的 register_fn 对含多个 Option<T> 参数的闭包
    // 注册后无法调用（全 Some/全 None/混合均报 Function not found，已实测确认），
    // 原 10 个 Option 参数的 pm_compute_factor_completeness 从未被成功调用过
    // （data-quality.rhai 此前因 count_chars 的 replace 崩溃未执行到此行）。
    // 改为 10 个 Dynamic 参数（万能类型，接受 f64/i64/&str/unit），闭包内转 Option。
    engine.register_fn(
        "pm_compute_factor_completeness",
        |total_score: rhai::Dynamic,
         consensus_score: rhai::Dynamic,
         catalyst_level: rhai::Dynamic,
         risk_volatility: rhai::Dynamic,
         valuation_dcf_upside: rhai::Dynamic,
         trader_direction: rhai::Dynamic,
         money_flow_main_net_inflow: rhai::Dynamic,
         lockup_shareholder_trades_len: rhai::Dynamic,
         announcements_len: rhai::Dynamic,
         pace_signal: rhai::Dynamic|
         -> f64 {
            // Rhai Dynamic 数值提取：f64/i64 均接受，unit/其他 → None
            let f = |v: &rhai::Dynamic| -> Option<f64> {
                v.clone()
                    .try_cast::<f64>()
                    .or_else(|| v.clone().try_cast::<i64>().map(|x| x as f64))
            };
            // into_string: ImmutableString/String → String，unit 报错 → None
            let s = |v: &rhai::Dynamic| v.clone().into_string().ok();
            let i = |v: &rhai::Dynamic| v.clone().try_cast::<i64>();
            portfolio_formula::compute_factor_completeness(
                f(&total_score),
                f(&consensus_score),
                s(&catalyst_level).as_deref(),
                f(&risk_volatility),
                f(&valuation_dcf_upside),
                s(&trader_direction).as_deref(),
                f(&money_flow_main_net_inflow),
                i(&lockup_shareholder_trades_len),
                i(&announcements_len),
                f(&pace_signal),
            )
        },
    );
    // V66 修复(2026-07-29): 补齐与 init/services.rs 主注册点的对称性。
    // 当前 portfolio-mgr.rhai 虽用本地词典未实际调用这两个函数，但保持注册
    // 对称可避免未来启用调用时 rerun 路径 panic。
    engine.register_fn("pm_compute_news_sentiment", |title: &str, summary: &str| -> f64 {
        axagent_astock_data::sentiment::compute_news_sentiment(title, summary).unwrap_or(0.0)
    });
    engine.register_fn("pm_compute_text_sentiment", |text: &str| -> f64 {
        axagent_astock_data::sentiment::compute_text_sentiment(text).unwrap_or(0.0)
    });
    let mut scope = Scope::new();

    // ── Gap 2: 注入近期 lessons（reflection_lessons 活跃规则）──
    // 让 portfolio-mgr.rhai 在决策时知道最近哪些股票犯过错。
    // lessons 按 confidence 降序取前 10 条，打包为 JSON 注入。
    {
        use axagent_entities::reflection_lessons;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
        let recent_lessons: Vec<String> = reflection_lessons::Entity::find()
            .filter(reflection_lessons::Column::Status.eq("active"))
            .filter(reflection_lessons::Column::Confidence.gt(0.5))
            .order_by(reflection_lessons::Column::Confidence, sea_orm::Order::Desc)
            .limit(10)
            .all(db)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|l| l.lesson_summary)
            .collect();
        let lessons_json = serde_json::to_string(&recent_lessons).unwrap_or_else(|_| "[]".into());
        scope.push_constant("recent_lessons", lessons_json);
        tracing::debug!("[rerun_decision] Gap2: 注入 {} 条 lessons 到 scope", recent_lessons.len());
    }

    // 简化版 resolve_var_path：导航 JSON 嵌套（支持 JSON 字符串自动解析）
    fn resolve_path(
        path: &str,
        vars: &HashMap<String, serde_json::Value>,
    ) -> Option<serde_json::Value> {
        if path.is_empty() {
            return None;
        }
        let parts: Vec<&str> = path.split('.').collect();
        if let Some(root) = vars.get(parts[0]) {
            let mut current = root.clone();
            for part in &parts[1..] {
                if let serde_json::Value::String(s) = &current {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                        current = parsed;
                    }
                }
                current = current.get(part)?.clone();
            }
            Some(current)
        } else {
            vars.get(path).cloned()
        }
    }

    // 注入 input_mapping 到 Rhai scope
    let has_raw = snapshot.keys().any(|k| k.starts_with("_raw."));
    // V37: 旧版 snapshot（无 _raw.*）中 ToolNode/AgentNode 的值已被 extract_node_text
    // 提取为纯文本，JSON 结构已丢失，resolve_path 无法下钻到内部字段。
    // 剥除 .result./.content. 前缀后，子字段导航仍会失败（纯文本不是 JSON）。
    // 此时大部分 input_mapping 解析为 None，Rhai 侧 weights_collapsed 兜底。
    // 建议用户重新运行完整工作流以生成新版 snapshot。
    if !has_raw {
        tracing::warn!(
            "[rerun_decision] 旧版 snapshot（无 _raw.*），JSON 结构已丢失，建议重新运行完整工作流。input_mapping 将尽力使用已有数据。"
        );
    }

    // V40 修复: 旧版 snapshot 的 remapped key → 原始 nodeId 反向映射
    // build_blackboard_snapshot 对某些节点做了 key 重命名，此处构建反向表
    // 以便 resolve_path 能找到正确的键。
    let remap_old: std::collections::HashMap<&str, &str> = [
        ("data_quality_summary", "data-quality"),
        ("report.investment-plan", "trader"),
        ("value.assessment", "value-investor"),
        ("rule_check.summary", "rule-check"),
        ("raw.combined", "raw-data"),
    ]
    .into_iter()
    .collect();

    for (target_key, source_key) in &input_mapping {
        // 对于旧版 snapshot（无 _raw.*），尝试剥除 result./content. 前缀：
        // 因为旧版 build_blackboard_snapshot 已经把 ToolNode 的 result 和 AgentNode
        // 的 content 提取为纯文本，外层包裹已丢失。剥除后路径直接从 JSON 内容开始。
        let mut used_key = if has_raw {
            source_key.clone()
        } else {
            // 尝试剥除 node_id.result. → node_id. 和 node_id.content. → node_id.
            source_key.replacen(".result.", ".", 1).replacen(".content.", ".", 1)
        };
        // V40 修复: 旧版 snapshot 中 remapped key 的查找
        // resolve_path 的第一步是 vars.get(parts[0])，如果 parts[0] 是
        // "data-quality" 但旧版 snapshot 的 key 是 "data_quality_summary"，
        // 查找会失败。此处尝试用 remap_old 转换 key。
        if !has_raw {
            let first_seg = used_key.split('.').next().unwrap_or("");
            if let Some(&mapped) = remap_old.get(first_seg) {
                used_key = used_key.replacen(first_seg, mapped, 1);
            }
        }
        let value = resolve_path(&used_key, &snapshot);
        match &value {
            None | Some(serde_json::Value::Null) => {
                // V40: 旧版 snapshot 中值可能是纯文本字符串（extract_node_text），
                // 此时 resolve_path 找不到子字段（如 .content.score），但整条记录
                // 可能以字符串形式存在。尝试以 used_key 的 root 部分直查整个值。
                if !has_raw {
                    let root = used_key.split('.').next().unwrap_or("");
                    if let Some(full_text) = snapshot.get(root).and_then(|v| v.as_str()) {
                        let trimmed_text = full_text.trim().to_string();

                        // V42 增强: 旧版 snapshot 的文本中可能包含
                        // <!-- VERDICT: {...} --> 标签。尝试提取标签内的 JSON 并
                        // 按 used_key 中的子字段路径导航，以恢复结构化数据。
                        let mut injected_from_verdict = false;
                        if let Some(verdict_json) = extract_verdict_from_text(&trimmed_text) {
                            // 从 used_key 中提取子字段路径（去掉 root 部分）
                            let used_parts: Vec<&str> = used_key.split('.').collect();
                            if used_parts.len() > 1 {
                                let mut cur = &verdict_json;
                                for part in &used_parts[1..] {
                                    cur = match cur.get(*part) {
                                        Some(v) => v,
                                        None => {
                                            cur = &serde_json::Value::Null;
                                            break;
                                        },
                                    };
                                }
                                if !cur.is_null() {
                                    match cur {
                                        serde_json::Value::Number(n) => {
                                            let val = n.as_f64().unwrap_or(0.0);
                                            let _ = scope.push_constant(target_key.as_str(), val);
                                            tracing::info!(
                                                "[rerun_decision] 旧版 snapshot VERDICT 恢复: {target_key} ← {root}<!--VERDICT-->#{part} = {val}",
                                                part = used_parts[1..].join(".")
                                            );
                                            injected_from_verdict = true;
                                        },
                                        serde_json::Value::String(s) => {
                                            let _ =
                                                scope.push_constant(target_key.as_str(), s.clone());
                                            tracing::info!(
                                                "[rerun_decision] 旧版 snapshot VERDICT 恢复: {target_key} ← {root}<!--VERDICT-->#{part} = {s}",
                                                part = used_parts[1..].join(".")
                                            );
                                            injected_from_verdict = true;
                                        },
                                        _ => {},
                                    }
                                }
                            }
                        }

                        if injected_from_verdict {
                            continue;
                        }

                        // 尝试解析为数字（如 "B" 等级文本虽然无法解析，但 score 字段
                        // 如 "85" 可以解析为数字）
                        if let Ok(num) = trimmed_text.parse::<f64>() {
                            let _ = scope.push_constant(target_key.as_str(), num);
                            tracing::warn!(
                                "[rerun_decision] 旧版 snapshot 回退: {target_key} ← {root} (解析为数字 {num})"
                            );
                        } else {
                            // V40: 纯文本字符串不能注入给预期为数字的 Rhai 变量
                            //（如 dqi_score 若为文本会导致 (dqi_score-50)/50 类型错误）。
                            // 只对已知文本字段注入字符串，其余推入 () 让
                            // Rhai 侧走 weights_collapsed 兜底。
                            if target_key == "stock_lessons" || target_key == "sanity_reason" {
                                let _ = scope.push_constant(target_key.as_str(), trimmed_text);
                                tracing::warn!(
                                    "[rerun_decision] 旧版 snapshot 回退: {target_key} ← {root} (纯文本)"
                                );
                            } else {
                                let _ = scope.push_constant(target_key.as_str(), ());
                                tracing::warn!(
                                    "[rerun_decision] 旧版 snapshot 回退: {target_key} ← {root} (纯文本无法用于数值计算，放弃)"
                                );
                            }
                        }
                        continue;
                    }
                }
                let _ = scope.push_constant(target_key.as_str(), ());
            },
            Some(serde_json::Value::Number(n)) => {
                let val = n.as_f64().unwrap_or(0.0);
                let _ = scope.push_constant(target_key.as_str(), val);
            },
            Some(serde_json::Value::String(s)) => {
                let _ = scope.push_constant(target_key.as_str(), s.clone());
            },
            Some(serde_json::Value::Bool(b)) => {
                let _ = scope.push_constant(target_key.as_str(), *b);
            },
            Some(serde_json::Value::Array(arr)) => {
                let items: rhai::Array = arr
                    .iter()
                    .map(|v| match v {
                        serde_json::Value::Number(n) => {
                            rhai::Dynamic::from(n.as_f64().unwrap_or(0.0))
                        },
                        serde_json::Value::String(s) => rhai::Dynamic::from(s.clone()),
                        serde_json::Value::Bool(b) => rhai::Dynamic::from(*b),
                        _ => rhai::Dynamic::UNIT,
                    })
                    .collect();
                scope.push_dynamic(target_key.as_str(), rhai::Dynamic::from(items));
            },
            Some(serde_json::Value::Object(obj)) => {
                let mut map = rhai::Map::new();
                for (k, v) in obj {
                    let val = match v {
                        serde_json::Value::Number(n) => {
                            rhai::Dynamic::from(n.as_f64().unwrap_or(0.0))
                        },
                        serde_json::Value::String(s) => rhai::Dynamic::from(s.clone()),
                        serde_json::Value::Bool(b) => rhai::Dynamic::from(*b),
                        _ => continue,
                    };
                    map.insert(k.clone().into(), val);
                }
                scope.push_dynamic(target_key.as_str(), rhai::Dynamic::from(map));
            },
        }
    }

    // 执行 Rhai 脚本
    // P1-D10: 通过全局 AST 缓存复用编译结果，避免 Rerun Decision 时重复编译。
    // 注意：code 来自数据库 workflow_template，可能与 include_str! 版本不同
    // （用户修改了 portfolio-mgr.rhai 后重新 seed），AST 缓存按 code hash 区分，
    // code 变化时自动产生新 key，不会命中旧缓存。
    let ast = axagent_harness::get_or_compile_ast("portfolio-mgr-rerun", &code, &engine).map_err(
        |e| ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("Rhai AST 编译失败: {e}")),
    )?;

    let result: rhai::Dynamic = engine.eval_ast_with_scope(&mut scope, &ast).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("Rhai 脚本执行失败: {e}"))
    })?;

    // 转换 Rhai 结果到 JSON
    fn to_json(v: &rhai::Dynamic) -> serde_json::Value {
        if v.is_unit() {
            return serde_json::Value::Null;
        }
        if v.is_bool() {
            return serde_json::Value::Bool(v.as_bool().unwrap_or(false));
        }
        if let Ok(s) = v.clone().into_string() {
            return serde_json::Value::String(s);
        }
        if let Ok(f) = v.as_float() {
            if let Some(n) = serde_json::Number::from_f64(f) {
                return serde_json::Value::Number(n);
            }
        }
        if let Some(arr) = v.clone().try_cast::<rhai::Array>() {
            return serde_json::Value::Array(arr.into_iter().map(|item| to_json(&item)).collect());
        }
        if let Some(map) = v.clone().try_cast::<rhai::Map>() {
            let mut obj = serde_json::Map::new();
            for (k, val) in &map {
                obj.insert(format!("{k}"), to_json(val));
            }
            return serde_json::Value::Object(obj);
        }
        serde_json::Value::String(format!("{v}"))
    }
    let decision_value = to_json(&result);

    // 5. 提取决策字段
    let action = decision_value.get("action").and_then(|v| v.as_str()).map(|s| s.to_string());
    let position_pct = decision_value.get("positionPct").and_then(|v| v.as_f64());
    let confidence = decision_value.get("confidence").and_then(|v| v.as_f64());
    let reasoning = decision_value.get("reasoning").and_then(|v| v.as_str()).map(|s| s.to_string());
    let time_horizon =
        decision_value.get("timeHorizon").and_then(|v| v.as_str()).map(|s| s.to_string());
    let holding_days = decision_value.get("expectedHoldingDays").and_then(|v| {
        if let Some(f) = v.as_f64() {
            Some(f as i64)
        } else {
            v.as_i64()
        }
    });

    let decision_json_str = serde_json::to_string(&decision_value).unwrap_or_default();

    // 6. 更新分析记录
    stock_analyses::Entity::update_many()
        .col_expr(stock_analyses::Column::DecisionAction, Expr::value(action))
        .col_expr(stock_analyses::Column::DecisionPositionPct, Expr::value(position_pct))
        .col_expr(stock_analyses::Column::DecisionReasoning, Expr::value(reasoning))
        .col_expr(stock_analyses::Column::DecisionJson, Expr::value(decision_json_str))
        .col_expr(stock_analyses::Column::DecisionTimeHorizon, Expr::value(time_horizon))
        .col_expr(stock_analyses::Column::DecisionExpectedHoldingDays, Expr::value(holding_days))
        .col_expr(
            stock_analyses::Column::UpdatedAt,
            Expr::value(chrono::Utc::now().timestamp_millis()),
        )
        .filter(stock_analyses::Column::Id.eq(&analysis_id))
        .exec(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("更新分析记录失败: {e}"))
        })?;

    tracing::warn!(
        "[rerun_decision] 决策重跑完成: analysis_id={analysis_id}, confidence={confidence:?}"
    );

    // 7. 构建 DashboardReport（借鉴 daily_stock_analysis 决策仪表盘格式）
    // 从 snapshot 提取评分节点输出和专家报告
    let score_json = snapshot
        .get("t-scoring")
        .or_else(|| snapshot.get("t-scoring.result"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let analyst_reports = extract_analyst_reports_from_snapshot(&snapshot);
    let stock_code = analysis.stock_code.clone();
    let stock_name = analysis.stock_name.clone();
    let analysis_date = analysis.analysis_date.clone();

    let dashboard_report =
        axagent_analysis_engine::dashboard_report::build_dashboard_report_from_workflow(
            &decision_value,
            &score_json,
            &stock_code,
            &stock_name,
            &analysis_date,
            &analyst_reports,
        );
    let dashboard_md =
        axagent_analysis_engine::dashboard_report::render_dashboard_md(&dashboard_report);

    tracing::info!(
        "[rerun_decision] DashboardReport 生成完成: integrity_passed={}, risk_alerts={}, catalysts={}",
        dashboard_report.integrity_passed,
        dashboard_report.risk_alerts.len(),
        dashboard_report.catalysts.len()
    );

    Ok(json!({
        "analysis_id": analysis_id,
        "decision": decision_value,
        "llm_decision_json": analysis.llm_decision_json,
        "dashboardReport": dashboard_report,
        "dashboardMd": dashboard_md,
    }))
}

/// 从 blackboard snapshot 提取分析师报告文本
///
/// snapshot 中的 key 格式为 `report.{expert_id}`（如 `report.a-fundamentals`），
/// 值为 JSON 字符串或对象。本函数把 `a-` 前缀去掉，映射到不带前缀的 expert_id
/// （如 `fundamentals-analyst`），供 `build_dashboard_report_from_workflow` 使用。
pub(crate) fn extract_analyst_reports_from_snapshot(
    snapshot: &std::collections::HashMap<String, serde_json::Value>,
) -> std::collections::HashMap<String, String> {
    let mut reports = std::collections::HashMap::new();

    // 专家 ID 映射：snapshot key 前缀 → build_dashboard_report_from_workflow 期望的 key
    let expert_mapping: &[(&str, &str)] = &[
        ("a-market-analyst", "market-analyst"),
        ("a-sentiment", "sentiment-analyst"),
        ("a-news", "news-analyst"),
        ("a-fundamentals", "fundamentals-analyst"),
        ("a-policy", "policy-analyst"),
        ("a-hot-money", "hot-money-tracker"),
        ("a-lockup", "lockup-watcher"),
    ];

    for (node_id, target_id) in expert_mapping {
        // 尝试两种 key 格式：report.{node_id} 和 {node_id}
        let report_key = format!("report.{node_id}");
        let value = snapshot.get(&report_key).or_else(|| snapshot.get(*node_id));
        if let Some(val) = value {
            let text = if let Some(s) = val.as_str() {
                s.to_string()
            } else if let Some(obj) = val.as_object() {
                // 对象类型：尝试取 content / report / text 字段
                obj.get("content")
                    .or_else(|| obj.get("report"))
                    .or_else(|| obj.get("text"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| val.to_string())
            } else {
                val.to_string()
            };
            if !text.is_empty() {
                reports.insert((*target_id).to_string(), text);
            }
        }
    }

    reports
}

/// 从 Workflow 执行结果构建 DashboardReport + Markdown 文本。
///
/// 复用 rerun_decision 中的构建逻辑，让正常完成的 run_stock_workflow
/// 也能在 workflow-completed 事件中携带 dashboard 数据，避免前端
/// dashboardReport 在工作流完成后仍为 null（概览/仪表板 tab 永远显示空态）。
///
/// 内部使用 extract_decision_json 提取 portfolio-mgr 决策，
/// 从 results["t-scoring"] 提取评分 JSON，从 results 提取分析师报告。
pub(crate) fn build_dashboard_from_workflow_result(
    wf: &Workflow,
    stock_code: &str,
    stock_name: &str,
    analysis_date: &str,
) -> Option<(axagent_harness::DashboardReport, String)> {
    // 1. 提取决策 JSON 字符串并 parse 为 Value
    let decision_str = extract_decision_json(wf)?;
    let decision_value: serde_json::Value = serde_json::from_str(&decision_str).unwrap_or(
        serde_json::json!({"action": "观望", "positionPct": 0, "confidence": 0.0, "reasoning": ""}),
    );

    // 2. 提取评分 JSON（与 rerun_decision 一致：优先 t-scoring，回退 t-scoring.result）
    let score_json = wf
        .results
        .get("t-scoring")
        .or_else(|| wf.results.get("t-scoring.result"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    // 3. 提取分析师报告
    let analyst_reports = extract_analyst_reports_from_snapshot(&wf.results);

    // 4. 构建 DashboardReport
    let dashboard_report =
        axagent_analysis_engine::dashboard_report::build_dashboard_report_from_workflow(
            &decision_value,
            &score_json,
            stock_code,
            stock_name,
            analysis_date,
            &analyst_reports,
        );
    let dashboard_md =
        axagent_analysis_engine::dashboard_report::render_dashboard_md(&dashboard_report);

    tracing::info!(
        "[build_dashboard_from_workflow_result] DashboardReport 构建完成: \
         integrity_passed={}, risk_alerts={}, catalysts={}",
        dashboard_report.integrity_passed,
        dashboard_report.risk_alerts.len(),
        dashboard_report.catalysts.len()
    );

    Some((dashboard_report, dashboard_md))
}
