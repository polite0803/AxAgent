use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_astock_data::as_of::AsOfContext;
use axagent_core::entity::stock_analyses;
use axagent_harness::workflow_types::{JsonSchema, Variable, WorkflowEdge, WorkflowNode};
use axagent_rt_workflow::Workflow;
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
    Insufficient {
        summary: String,
        missing_sources: Vec<DataMissingItem>,
    },
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
        QualityPrecheckResult::Insufficient {
            summary,
            missing_sources,
        }
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
pub(crate) async fn data_quality_precheck(
    client: &axagent_astock_data::AStockClient,
    stock_code: &str,
    quote: &axagent_astock_data::StockQuote,
) -> QualityPrecheckResult {
    // 1. quote — 已在参数中传入, 直接检查
    let quote_check = if quote.price <= 0.0 && quote.name.is_empty() {
        SourceCheck::Failed("价格为空、股票代码不存在或未上市".into())
    } else {
        SourceCheck::Ok
    };

    // 2. financials
    let fin_check = match client.get_financials(stock_code).await {
        Ok(financials) => {
            let has_revenue = financials.iter().any(|f| f.revenue.unwrap_or(0.0) > 0.0);
            let has_profit = financials.iter().any(|f| f.net_profit.unwrap_or(0.0) > 0.0);
            if !has_revenue && !has_profit {
                SourceCheck::Partial("营收/利润缺失".into())
            } else {
                SourceCheck::Ok
            }
        },
        Err(e) => SourceCheck::Failed(format!("全部数据源获取失败: {e}")),
    };

    // V38 修复: K 线至少需要 60 日才能计算 MA(20)+MACD(26) 等关键技术指标。
    // 不足 60 日但 ≥30 日时仅降级为 Partial（可继续但技术分析受限）。
    let kline_check = match client.get_klines(stock_code, "daily", 500).await {
        Ok(klines) if klines.len() >= 60 => SourceCheck::Ok,
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
    let news_check = match client.get_news(stock_code, 10).await {
        Ok(news) if !news.is_empty() => SourceCheck::Ok,
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
    ])
}

pub(crate) struct LoadedTemplate {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub input_schema: Option<JsonSchema>,
    pub output_schema: Option<JsonSchema>,
    pub variables: Option<Vec<Variable>>,
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
    use axagent_core::entity::workflow_template;

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

    let input_schema: Option<JsonSchema> = template
        .input_schema
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());
    let output_schema: Option<JsonSchema> = template
        .output_schema
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok());
    let variables: Option<Vec<Variable>> = template
        .variables
        .as_ref()
        .and_then(|v| serde_json::from_str(v).ok());

    Ok(LoadedTemplate {
        nodes,
        edges,
        input_schema,
        output_schema,
        variables,
    })
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
    let action = parsed
        .get("action")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let position_pct = parsed
        .get("positionPct")
        .or_else(|| parsed.get("position_pct"))
        .and_then(|v| v.as_f64());
    let reasoning = parsed
        .get("reasoning")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
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
        let actual = match pm {
            serde_json::Value::Object(obj) => {
                obj.get("result").cloned().unwrap_or_else(|| pm.clone())
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
            // quality-fallback 输出格式: {"action":"持有/减持/卖出","positionPct":0-20,"reasoning":"..."}
            if serde_json::from_str::<serde_json::Value>(content_str).is_ok() {
                return Some(content_str.to_string());
            }
        }
    }
    // 回退: workflow 顶层 output(无 output_schema 或非 stock-analysis 工作流)
    wf.output
        .as_ref()
        .and_then(|v| serde_json::to_string(v).ok())
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
/// V50 升级: compute_decision_agreement 不再只返回 0-100 总分,
/// 而是返回分维度诊断结构体。上层可根据维度详情:
///   - 决定 confidence 调制幅度
///   - 生成分歧诊断 reasoning 文本
///   - 判断是否触发人工复核
///
/// P0 修复: 新增 f7 自指污染标记字段，标注公式决策中 trader 因子(f7)的参与程度，
/// 帮助识别"公式已含 trader 观点"导致一致性虚高或逻辑矛盾。
pub(crate) struct AgreementBreakdown {
    /// 总分 0-100
    pub total: i32,
    /// action 维度原始分 (满分 50)
    pub action_score: f64,
    /// action 是否基本一致 (>= 35 分)
    pub action_ok: bool,
    /// action 一致性说明 (exact_match / same_direction / opposite / ...)
    pub action_note: String,
    /// 公式视角的 action 原始值
    pub formula_action: String,
    /// LLM 视角的 action 原始值
    pub llm_action: String,
    /// positionPct 维度原始分 (满分 30)
    pub position_score: f64,
    /// 仓位差值绝对值
    pub position_gap: Option<f64>,
    /// confidence 维度原始分 (满分 20)
    pub confidence_score: f64,
    /// 置信度差值绝对值
    pub confidence_gap: Option<f64>,
    /// 冲突类型: all_agree / opposite_direction / action_divergence / position_gap / confidence_gap
    pub conflict_type: String,
    // ── P0: f7 自指污染标记 ──
    /// 公式决策中 f7（trader 因子）权重占总权重百分比。None=无 f7 数据。
    pub f7_weight_pct: Option<f64>,
    /// 排除 f7 后的"纯净"后验值（0~1）。None=无 f7 数据。
    pub f7_free_posterior: Option<f64>,
    /// 排除 f7 后的"纯净"action。None=无 f7 数据。
    pub f7_free_action: Option<String>,
    /// 无 f7 版本的 action 一致性原始分 (满分 50，与主 action_score 相同语义)
    pub f7_free_action_score: Option<f64>,
}

/// 计算公式决策与 LLM 决策的一致性分数（0-100）。
///
/// 借鉴 TradingAgents 的冗余校验机制：
/// 对比 action（操作方向）、positionPct（仓位百分比）、confidence（置信度）
/// 三个维度，权重分别为 50/30/20。
///
/// V40 修复：
/// - 从 trader 输出取 action 而非 stance（trader prompt 输出字段为 action）
/// - trader 无 positionPct 字段，故 LLM 的 positionPct 视为缺失→pos_score 走兜底 15
/// - 移除 #[allow(dead_code)]，在 stock_workflow 完成时调用并写入决策元数据
///
/// 归一化规则（与前端 normalizeAction 保持一致）:
/// - 移除空格/斜杠/下划线/全角空格
/// - 小写比较
/// - "买"和"增持"视为一致，"卖"和"减持"视为一致
///
/// V50 升级: 返回 AgreementBreakdown 而非 Option<i32>，包含分维度诊断
pub(crate) fn compute_decision_agreement(
    formula_json: Option<&str>,
    llm_json: Option<&str>,
) -> Option<AgreementBreakdown> {
    let fj = serde_json::from_str::<serde_json::Value>(formula_json?).ok()?;
    let lj = serde_json::from_str::<serde_json::Value>(llm_json?).ok()?;

    // 归一化操作字符串
    let norm = |s: &str| {
        s.trim()
            .to_lowercase()
            .replace([' ', '/', '_', '\u{3000}'], "")
    };

    // 公式字段: action / positionPct / confidence
    let f_action = fj.get("action").and_then(|v| v.as_str().map(norm));
    let f_pos = fj.get("positionPct").and_then(|v| v.as_f64());
    let f_conf = fj.get("confidence").and_then(|v| v.as_f64());

    // V40: LLM 字段也取 action（trader prompt 输出格式中的字段名是 action 而非 stance）
    let l_action = lj.get("action").and_then(|v| v.as_str().map(norm));
    let l_pos = lj.get("positionPct").and_then(|v| v.as_f64());
    let l_conf = lj.get("confidence").and_then(|v| v.as_f64());

    // V50: 保存原始 action 值用于诊断展示
    let f_action_raw = fj
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_string();
    let l_action_raw = lj
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("?")
        .to_string();
    // V50: 预计算维度差值
    let pos_gap: Option<f64> = match (f_pos, l_pos) {
        (Some(a), Some(b)) => Some((a - b).abs()),
        _ => None,
    };
    let conf_gap: Option<f64> = match (f_conf, l_conf) {
        (Some(a), Some(b)) => Some((a - b).abs()),
        _ => None,
    };

    // V45 修复: action 一致性评分精细化（纠正"中性桶"虚高问题）
    //
    // 旧逻辑缺陷: 所有"非买卖"的 action 归入同一个"中性桶", 给 40/50 分。
    //   导致 "持有"(明确持仓决策) vs "不确定"(无判断) 得到 80% 一致性,
    //   与 "买入 vs 增持"(同向微差) 同分, 语义上完全不合理。
    //
    // 新逻辑 — 四级评分:
    //   精确匹配(50) > 同向同类(35) > 中性不同义(5~15) > 对立方向(0)
    //
    // 中性内部细分:
    //   "持有" vs "观望" = 15 (都是明确操作建议, 只是激进度不同)
    //   "持有/观望" vs "不确定" = 5 (一个明确, 一个无判断, 差距极大)
    //   "观望" vs "不确定" = 10 (观望至少排除了买卖, 不确定连这个都没排除)
    let is_buy = |s: &str| s.contains("买") || s.contains("增持");
    let is_sell = |s: &str| s.contains("卖") || s.contains("减持");
    let is_hold = |s: &str| s == "持有";
    let is_watch = |s: &str| s == "观望";
    let is_uncertain = |s: &str| s.contains("不确定") || s.contains("未知");
    let action_score: f64 = match (f_action.clone(), l_action.clone()) {
        (Some(a), Some(b)) if a == b => 50.0,
        (Some(a), Some(b)) if is_buy(&a) && is_buy(&b) => 35.0,
        (Some(a), Some(b)) if is_sell(&a) && is_sell(&b) => 35.0,
        // 中性但不同义: 持有 vs 观望 = 15
        (Some(a), Some(b)) if (is_hold(&a) && is_watch(&b)) || (is_hold(&b) && is_watch(&a)) => {
            15.0
        },
        // 明确中性 vs 不确定: 持有/观望 vs 不确定 = 5
        (Some(a), Some(b)) if (is_hold(&a) || is_watch(&a)) && is_uncertain(&b) => 5.0,
        (Some(a), Some(b)) if (is_hold(&b) || is_watch(&b)) && is_uncertain(&a) => 5.0,
        // 观望 vs 不确定 = 10 (观望比持有弱一点, 所以惩罚轻一些)
        (Some(a), Some(b))
            if is_watch(&a) && is_uncertain(&b) || is_watch(&b) && is_uncertain(&a) =>
        {
            10.0
        },
        // 对立方向
        (Some(_), Some(_)) => 0.0,
        // 单侧缺失
        _ => 25.0,
    };

    // positionPct 一致性 (权重 30%)
    let pos_score: f64 = match (f_pos, l_pos) {
        (Some(a), Some(b)) => {
            let diff = (a - b).abs();
            if diff <= 5.0 {
                30.0
            } else if diff <= 15.0 {
                20.0
            } else if diff <= 30.0 {
                10.0
            } else {
                0.0
            }
        },
        _ => 15.0,
    };

    // confidence 一致性 (权重 20%)
    let conf_score: f64 = match (f_conf, l_conf) {
        (Some(a), Some(b)) => {
            let diff = (a - b).abs();
            if diff <= 0.1 {
                20.0
            } else if diff <= 0.2 {
                15.0
            } else if diff <= 0.4 {
                8.0
            } else {
                0.0
            }
        },
        _ => 10.0,
    };

    let total = (action_score + pos_score + conf_score).round() as i32;

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
            let action = obj
                .get("action")
                .and_then(|a| a.as_str().map(|s| s.to_string()));
            Some((f7_weight_pct, posterior, action))
        } else {
            None
        }
    });
    let (f7_weight_pct, f7_free_posterior, f7_free_action) =
        f7_free_info.unwrap_or((None, None, None));

    // 计算无 f7 版本的 action 一致性评分
    // P0: 比较 formula(no-f7) vs LLM action（当 LLM 有 action 时）
    // P3: 当 LLM 无 action 时，回退到 formula(no-f7) vs formula(full) — trader 影响度
    let f7_compare_target = l_action.as_deref().or(f_action.as_deref());
    let f7_free_action_score =
        match (f7_free_action.as_deref().map(norm), f7_compare_target.map(norm)) {
            (Some(a), Some(b)) if a == b => Some(50.0),
            (Some(a), Some(b)) if is_buy(&a) && is_buy(&b) => Some(35.0),
            (Some(a), Some(b)) if is_sell(&a) && is_sell(&b) => Some(35.0),
            (Some(a), Some(b))
                if (is_hold(&a) && is_watch(&b)) || (is_hold(&b) && is_watch(&a)) =>
            {
                Some(15.0)
            },
            (Some(a), Some(b)) if (is_hold(&a) || is_watch(&a)) && is_uncertain(&b) => Some(5.0),
            (Some(a), Some(b)) if (is_hold(&b) || is_watch(&b)) && is_uncertain(&a) => Some(5.0),
            (Some(a), Some(b))
                if is_watch(&a) && is_uncertain(&b) || is_watch(&b) && is_uncertain(&a) =>
            {
                Some(10.0)
            },
            (Some(_), Some(_)) => Some(0.0),
            _ => None,
        };

    // ── V50: 冲突类型分类 ──
    // P3: 新增 f7_influence 类型 — trader 不输出 action，用 formula(no-f7) vs formula(full)
    //     衡量 trader 信息对公式的影响度
    let conflict_type: &str = if l_action.is_none() {
        // P3: 无 LLM action, 用 f7_free_action_score 区间判断 influence 程度
        match f7_free_action_score {
            Some(s) if s >= 45.0 => "f7_low_influence", // trader 信息对公式影响小
            Some(s) if s >= 35.0 => "f7_moderate_influence", // trader 信息有中等影响
            Some(s) if s >= 20.0 => "f7_high_influence", // trader 信息大幅改变公式输出
            _ => "f7_dominant",                         // trader 信息主导公式决策
        }
    } else if action_score >= 45.0 && pos_score >= 25.0 && conf_score >= 18.0 {
        "all_agree"
    } else if action_score == 0.0 {
        "opposite_direction"
    } else if action_score <= 5.0 {
        "action_divergence"
    } else if pos_score <= 10.0 {
        "position_gap"
    } else {
        "confidence_gap"
    };
    // ── V50: action_note 分类 ──
    let action_note: &str = if action_score >= 50.0 {
        "exact_match"
    } else if action_score >= 35.0 {
        "same_direction"
    } else if action_score >= 15.0 {
        "hold_vs_watch"
    } else if action_score >= 10.0 {
        "watch_vs_uncertain"
    } else if action_score >= 5.0 {
        "definite_vs_uncertain"
    } else if action_score == 0.0 {
        "opposite"
    } else {
        "missing_one_side"
    };

    Some(AgreementBreakdown {
        total,
        action_score,
        action_ok: action_score >= 35.0,
        action_note: action_note.to_string(),
        formula_action: f_action_raw,
        llm_action: l_action_raw,
        position_score: pos_score,
        position_gap: pos_gap,
        confidence_score: conf_score,
        confidence_gap: conf_gap,
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

/// 从模板 variables 中解析 RunOptions 关键参数。
///
/// 用户在「股票分析设置 → 参数」中调整 `max_concurrent` /
/// `agent_timeout_secs` 后，这里读到的就是新值；如果模板里没有这两个
/// key（旧版本 / 用户清空）则用默认值。
///
/// 容错策略：
///   * 越界 / 非法类型 → 用默认值；
///   * max_concurrent ∈ [1, 32]，过小会让并发退化为串行，过大会拖垮 LLM 速率。
///   * step_timeout ∈ [10, 3600] 秒，避免 0 或极端大值。
pub(crate) fn resolve_runtime_options(
    variables: Option<&[axagent_harness::workflow_types::Variable]>,
) -> (usize, std::time::Duration) {
    let lookup = |name: &str| -> Option<serde_json::Value> {
        variables
            .and_then(|vs| vs.iter().find(|v| v.name == name))
            .map(|v| v.value.clone())
    };

    let max_concurrent = lookup("max_concurrent")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(1, 32) as usize)
        .unwrap_or(DEFAULT_MAX_CONCURRENT);

    let step_timeout_secs = lookup("agent_timeout_secs")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(10, 3600))
        .unwrap_or(DEFAULT_STEP_TIMEOUT_SECS);

    (max_concurrent, std::time::Duration::from_secs(step_timeout_secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::workflow_types::Variable;
    use serde_json::json;

    #[test]
    fn resolve_runtime_options_uses_defaults_when_missing() {
        let (mc, to) = resolve_runtime_options(None);
        assert_eq!(mc, DEFAULT_MAX_CONCURRENT);
        assert_eq!(to.as_secs(), DEFAULT_STEP_TIMEOUT_SECS);
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
        let (mc, to) = resolve_runtime_options(Some(&vars));
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
        let (mc, to) = resolve_runtime_options(Some(&vars));
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
        let (mc, _) = resolve_runtime_options(Some(&vars));
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
    let raw_keys: Vec<String> = snapshot
        .keys()
        .filter(|k| k.starts_with("_raw."))
        .cloned()
        .collect();
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
                    k.strip_prefix("report.")
                        .map(|id| (id.to_string(), k.clone()))
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
    let template = axagent_core::entity::workflow_template::Entity::find()
        .filter(axagent_core::entity::workflow_template::Column::Id.eq("stock-analysis"))
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

    // 4. 执行 Rhai 脚本（与 code_executor::execute_rhai_directly 相同逻辑）
    let mut engine = Engine::new();
    // SECURITY (C4): Rhai 沙箱限制 — 防 DoS
    engine.set_max_operations(200_000);
    engine.set_max_call_levels(32);
    engine.set_max_modules(0);
    engine.set_max_string_size(2_000_000);
    engine.set_max_array_size(50_000);
    engine.register_fn("clamp", |value: f64, min: f64, max: f64| -> f64 {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    });
    engine.register_fn("join", |arr: rhai::Array, sep: &str| -> String {
        arr.iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join(sep)
    });
    let mut scope = Scope::new();

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
            source_key
                .replacen(".result.", ".", 1)
                .replacen(".content.", ".", 1)
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
    let result: rhai::Dynamic = engine.eval_with_scope(&mut scope, &code).map_err(|e| {
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
    let action = decision_value
        .get("action")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let position_pct = decision_value.get("positionPct").and_then(|v| v.as_f64());
    let confidence = decision_value.get("confidence").and_then(|v| v.as_f64());
    let reasoning = decision_value
        .get("reasoning")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let time_horizon = decision_value
        .get("timeHorizon")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
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

    Ok(json!({
        "analysis_id": analysis_id,
        "decision": decision_value,
    }))
}
