use super::decision::QualityPrecheckResult;
use super::decision::{
    compute_decision_agreement, data_quality_precheck, extract_decision_fields,
    extract_decision_json, extract_llm_decision_json, load_and_inject_template, parse_asof_param,
    resolve_runtime_options,
};
use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_astock_data::as_of::{self, AsOfContext};
use axagent_entities::stock_analyses;
use axagent_entities::stock_reflections;
use axagent_harness::workflow_types::Variable;
use axagent_rt_workflow::work_engine::{ProgressCallback, RunOptions, StepProgressEvent};
use axagent_stock_analysis::blackboard::build_blackboard_snapshot;
use sea_orm::DatabaseConnection;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;
use std::sync::Arc;
use tauri::{Emitter, State};

/// 启动股票分析工作流（DAG 模式）。
///
/// - 默认：生成新 UUID 并 INSERT 新 `stock_analyses` 行（fresh start）。
/// - 重跑分析场景：传入 `analysis_id` 让后端先 DELETE 同 id 旧行再 INSERT,
///   保留 id 稳定,前端 store 引用不会断。
#[tauri::command]
pub async fn run_stock_workflow(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    stock_code: String,
    dry_run: Option<bool>,
    as_of_date: Option<String>,
    // 可选: 传入已存在的 analysisId 即可"覆盖"该记录（用于重跑分析场景）。
    // 不传则生成新 UUID 并 INSERT 新行(fresh start)。
    analysis_id: Option<String>,
    // V53: 筛选来源标记 — "serenity" 表示来自瓶颈掘金候选
    screening_source: Option<String>,
) -> Result<serde_json::Value, String> {
    // 解析 as_of_date；非法或未来日期直接 4xx-style 错误
    let as_of_ctx = parse_asof_param(as_of_date.clone())?;

    if let Some(ctx) = as_of_ctx {
        as_of::AS_OF
            .scope(Some(ctx), async {
                run_stock_workflow_inner(
                    app,
                    state,
                    stock_code,
                    dry_run,
                    as_of_date,
                    analysis_id,
                    screening_source,
                )
                .await
            })
            .await
    } else {
        run_stock_workflow_inner(
            app,
            state,
            stock_code,
            dry_run,
            None,
            analysis_id,
            screening_source,
        )
        .await
    }
}

async fn run_stock_workflow_inner(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    stock_code: String,
    dry_run: Option<bool>,
    as_of_date: Option<String>,
    analysis_id_override: Option<String>,
    // V53: 筛选来源标记 — 告诉 stock-analysis 工作流当前股票来自哪里。
    // "serenity" 表示来自瓶颈掘金候选，允许风险分类器做评分修正。
    screening_source: Option<String>,
) -> Result<serde_json::Value, String> {
    let quote = state
        .astock_client
        .get_quote(&stock_code)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("行情获取失败: {e}"))
        })?;
    let now_ms = chrono::Utc::now().timestamp_millis();

    // 重跑分析（override 模式）：不删旧行，先用临时 ID INSERT 新行。
    // 工作流成功后再删旧行 + 更新临时行 ID 为 override id。
    // 失败则删除临时行，旧数据完好无损。
    let override_target = if let Some(ref provided) = analysis_id_override {
        // 验证旧行存在再记录，避免 Delete Nonexistent 后 ID 丢失
        match stock_analyses::Entity::find_by_id(provided.as_str())
            .one(state.harness.db())
            .await
        {
            Ok(Some(_)) => Some(provided.clone()),
            _ => {
                tracing::warn!(
                    "[run_stock_workflow] override_id={provided} 对应的旧行不存在,跳过覆盖"
                );
                None
            },
        }
    } else {
        None
    };
    let analysis_id = uuid::Uuid::new_v4().to_string();

    stock_analyses::ActiveModel {
        id: Set(analysis_id.clone()),
        stock_code: Set(stock_code.clone()),
        stock_name: Set(quote.name.clone()),
        // B12: 在 as-of 模式下,analysis_date 必须是 as-of 截止日,而不是 today
        // —— spec §4.1 闭世界假设要求工作流产物日期 = 截断日,否则回放历史会串味
        analysis_date: Set(as_of::current_as_of()
            .map(|c| c.as_string())
            .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string())),
        provider_id: Set("workflow".into()),
        conversation_id: Set(uuid::Uuid::new_v4().to_string()),
        status: Set("running".into()),
        decision_action: Set(None),
        decision_position_pct: Set(None),
        decision_reasoning: Set(None),
        decision_json: Set(None),
        llm_decision_json: Set(None),
        blackboard_snapshot: Set(None),
        config_id: Set(None),
        // Time-travel metadata: 标记该 analysis 为 replay 模式 + 截止日
        analysis_kind: Set(if as_of_date.is_some() {
            "replay".into()
        } else {
            "live".into()
        }),
        // 始终保存 as_of_date：live 模式用分析当日，replay 模式用用户指定日期
        as_of_date: Set(Some(
            as_of_date
                .clone()
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
        )),
        model_version: Set(None),
        data_snapshot_id: Set(None),
        outcome: Set(None),
        decision_time_horizon: Set(None),
        decision_expected_holding_days: Set(None),
        created_at: Set(now_ms),
        updated_at: Set(now_ms),
    }
    .insert(state.harness.db())
    .await
    .map_err(|e| ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("DB 写入失败: {e}")))?;

    // ── 数据质量预检：在发起 DAG 执行前检查关键数据是否完整 ──
    let stock_code_for_check = stock_code.clone();
    let quality_check =
        data_quality_precheck(&state.astock_client, &stock_code_for_check, &quote).await;
    match quality_check {
        QualityPrecheckResult::Insufficient {
            ref summary,
            ref missing_sources,
        } => {
            tracing::warn!(
                "[stock_workflow] 数据质量不足，跳过 DAG 执行: {summary} ({})",
                stock_code_for_check
            );
            // 构建结构化缺失报告
            let missing_report: Vec<serde_json::Value> = missing_sources
                .iter()
                .map(|item| {
                    json!({
                        "source": item.source,
                        "status": item.status,
                        "detail": item.detail,
                    })
                })
                .collect();
            // 更新 stock_analyses 状态
            if let Err(e) = stock_analyses::Entity::update(stock_analyses::ActiveModel {
                id: Set(analysis_id.clone()),
                status: Set("failed".into()),
                decision_json: Set(Some(
                    json!({
                        "action": "skip",
                        "reasoning": format!("数据不足，跳过分析: {summary}"),
                        "data_missing_report": missing_report,
                    })
                    .to_string(),
                )),
                updated_at: Set(chrono::Utc::now().timestamp_millis()),
                ..Default::default()
            })
            .exec(state.harness.db())
            .await
            {
                tracing::error!("[DB] 预检不足状态更新失败: {e}");
            }
            return Ok(json!({
                "status": "skipped",
                "reason": summary,
                "data_missing_report": missing_report,
                "analysis_id": analysis_id,
                "stock_code": stock_code,
                "stock_name": quote.name,
                "data_quality_precheck": "insufficient",
            }));
        },
        QualityPrecheckResult::Pass => {
            // 数据充分，正常执行
        },
        QualityPrecheckResult::Partial(reason) => {
            tracing::info!("stock_workflow] 数据质量部分缺失，继续分析: {reason}");
        },
    }

    let loaded =
        load_and_inject_template(state.harness.db(), &stock_code, &quote.name, "stock-analysis")
            .await?;

    if let Some(ref vars) = loaded.variables {
        for v in vars {
            if v.name == "vendor_iwencai_key" {
                if let serde_json::Value::String(ref key) = v.value {
                    if !key.is_empty() {
                        *state.astock_client.iwencai_key.write().await = key.clone();
                    }
                }
            }
            if v.name == "vendor_xueqiu_token" {
                if let serde_json::Value::String(ref token) = v.value {
                    if !token.is_empty() {
                        if let Some(ref xq) = state.astock_client.xq_token {
                            *xq.write().await = token.clone();
                        }
                    }
                }
            }
            if v.name == "vendor_neodata_token" {
                if let serde_json::Value::String(ref token) = v.value {
                    if !token.is_empty() {
                        if let Some(ref nd) = state.astock_client.neodata_token {
                            *nd.write().await = token.clone();
                        }
                    }
                }
            }
        }
    }

    let engine = Arc::clone(&state.work_engine);

    // ── 从模板变量中解析执行参数 ──
    // max_concurrent / step_timeout 之前在 RunOptions 中硬编码为 9/300，
    // 现在通过模板变量 `max_concurrent` / `agent_timeout_secs` 让用户在设置面板调整。
    let (max_concurrent, step_timeout) = resolve_runtime_options(loaded.variables.as_deref());

    let wf_name = format!("stock-analysis-{stock_code}");
    let workflow = engine
        .create_workflow(&wf_name, loaded.nodes, loaded.edges)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建工作流失败: {e}"))
        })?;
    let wf_id = workflow.id.clone();
    let wf_id_ret = wf_id.clone();
    let app_h = app.clone();
    let db = state.harness.db().clone();
    let aid = analysis_id.clone();
    let override_target_for_spawn = override_target.clone();

    let progress_app = app.clone();
    let progress_wf_id = wf_id.clone();
    let progress_cb: ProgressCallback = Arc::new(move |event: StepProgressEvent| {
        let app = progress_app.clone();
        let wf_id = progress_wf_id.clone();
        Box::pin(async move {
            // 根据步骤状态分发到对应的前端事件（与 executionStore 监听器匹配）
            let (event_name, payload) = match event.status.as_str() {
                "running" => (
                    "workflow-step-start",
                    serde_json::json!({
                        "conversationId": format!("wf-{}", wf_id),
                        "stepId": event.node_id,
                        "stepGoal": event.node_id,
                        "agentRole": "workflow",
                    }),
                ),
                "completed" => (
                    "workflow-step-complete",
                    serde_json::json!({
                        "conversationId": format!("wf-{}", wf_id),
                        "stepId": event.node_id,
                        "stepGoal": event.node_id,
                    }),
                ),
                s if s == "failed" || s == "timeout" => (
                    "workflow-step-error",
                    serde_json::json!({
                        "conversationId": format!("wf-{}", wf_id),
                        "stepId": event.node_id,
                        "error": format!("Step {}", event.status),
                    }),
                ),
                _ => return, // 未知状态，忽略
            };
            let _ = app.emit(event_name, payload);
            // 向后兼容：同时发送旧事件 workflow-step-done
            let _ = app.emit(
                "workflow-step-done",
                serde_json::json!({
                    "workflowId": wf_id,
                    "nodeId": event.node_id,
                    "status": event.status,
                    "totalNodes": event.total_nodes,
                    "completedNodes": event.completed_nodes,
                    "executionId": event.execution_id,
                }),
            );
        })
    });

    let input_schema = loaded.input_schema;
    let output_schema = loaded.output_schema;
    let template_vars = loaded.variables;

    let sc_for_ret = stock_code.clone();
    let sc_name = quote.name.clone();
    let sc_name_for_spawn = sc_name.clone();
    let vector_store = state.vector_store.clone();
    let master_key = state.harness.master_key_owned();
    // 在 spawn 前拉取市场状态（沪深300判断牛/熊/震荡），捕获到闭包中
    let market_regime_json: Option<serde_json::Value> = state
        .astock_client
        .get_klines("000300", "daily", 60)
        .await
        .ok()
        .and_then(|klines| {
            if klines.is_empty() {
                return None;
            }
            let r = axagent_stock_analysis::market_regime::classify_regime(&klines);
            Some(serde_json::json!({
                "regime": r.regime,
                "confidence": r.confidence,
                "volatility": r.volatility,
                "description": r.description,
            }))
        });
    // 在 spawn 前捕获 as-of 上下文（tokio::task_local 不跨 tokio::spawn 传播）
    let captured_asof = as_of::current_as_of();
    tokio::spawn(async move {
        // P3 修复: 在 spawn 内恢复 AS_OF + DEGRADATION_LOG 作用域
        as_of::with_optional_asof(captured_asof, async {
            as_of::with_degradation_log(async {
        let mut opts = RunOptions {
            max_concurrent,
            step_timeout,
            progress_callback: Some(progress_cb),
            input: Some(json!({"stock_code": &stock_code})),
            input_schema: input_schema.clone(),
            output_schema: output_schema.clone(),
            dry_run: dry_run.unwrap_or(false),
            ..Default::default()
        };
        let mut merged_vars: Vec<axagent_harness::workflow_types::Variable> = vec![
            axagent_harness::workflow_types::Variable {
                name: "stock_code".into(),
                var_type: "string".into(),
                value: serde_json::Value::String(stock_code.clone()),
                description: Some("当前分析的股票代码".into()),
                is_secret: false,
            },
            axagent_harness::workflow_types::Variable {
                name: "stock_name".into(),
                var_type: "string".into(),
                value: serde_json::Value::String(sc_name_for_spawn.clone()),
                description: Some("当前分析的股票名称".into()),
                is_secret: false,
            },
        ];
        if let Some(d) = as_of_date.as_deref() {
            merged_vars.push(axagent_harness::workflow_types::Variable {
                name: "as_of_date".into(),
                var_type: "string".into(),
                value: serde_json::Value::String(d.to_string()),
                description: Some("时间旅行模式截止日 (YYYY-MM-DD)；live 模式为空".into()),
                is_secret: false,
            });
        }
        if let Some(v) = template_vars {
            for tv in v {
                if !merged_vars.iter().any(|mv| mv.name == tv.name) {
                    merged_vars.push(tv);
                }
            }
        }
        // V53: 调用方指定 screening_source 时覆盖模板默认值
        // 使瓶颈掘金→股票分析的上下文可传递到 portfolio-mgr
        if let Some(ref source) = screening_source {
            if !source.is_empty() {
                if let Some(existing) = merged_vars.iter_mut().find(|mv| mv.name == "screening_source") {
                    existing.value = serde_json::Value::String(source.clone());
                } else {
                    merged_vars.push(axagent_harness::workflow_types::Variable {
                        name: "screening_source".into(),
                        var_type: "string".into(),
                        value: serde_json::Value::String(source.clone()),
                        description: Some("筛选来源标记".into()),
                        is_secret: false,
                    });
                }
            }
        }
        // X1 修复: 当 screening_source = serenity 时，从候选缓存注入瓶颈分析数据
        // 使 portfolio-mgr.rhai 能感知 Serenity 瓶颈分析结果，增加因子 6: 瓶颈置信度
        if let Some(ref source) = screening_source {
            if source == "serenity" {
                if let Some(detail) = axagent_stock_analysis::recommender::get_serenity_candidate_detail(&stock_code) {
                    merged_vars.push(axagent_harness::workflow_types::Variable {
                        name: "serenity_context".into(),
                        var_type: "object".into(),
                        value: detail.clone(),
                        description: Some("Serenity 瓶颈分析上下文（serenity_score / bottleneck_product / catalysts 等）".into()),
                        is_secret: false,
                    });
                    tracing::info!("[stock-analysis] 注入 serenity_context: score={}, bottleneck={}",
                        detail["serenity_score"].as_f64().unwrap_or(0.0),
                        detail["bottleneck_product"].as_str().unwrap_or(""));
                } else {
                    tracing::warn!("[stock-analysis] screening_source=serenity 但候选缓存为空: {}", stock_code);
                }
            }
        }
        // 注入相似历史决策案例（失败案例优先，最多 5 条）
        let similar_cases_str = fetch_similar_cases(&stock_code, &db).await;
        if let Some(ref cases) = similar_cases_str {
            merged_vars.push(axagent_harness::workflow_types::Variable {
                name: "similar_cases".into(),
                var_type: "string".into(),
                value: serde_json::Value::String(cases.clone()),
                description: Some("相似历史决策（失败案例，供避免重复错误）".into()),
                is_secret: false,
            });
        }
        // 注入市场状态（沪深300判断牛/熊/震荡），兜底防止模板变量缺失
        let regime_value = market_regime_json.unwrap_or_else(|| {
            serde_json::json!({
                "regime": "unknown",
                "confidence": null,
                "volatility": null,
                "description": "⚠️ 市场状态数据暂不可用（沪深300 K线拉取失败），请勿据此做多空判断，基于个股自身数据完成分析"
            })
        });
        merged_vars.push(axagent_harness::workflow_types::Variable {
            name: "market_regime".into(),
            var_type: "object".into(),
            value: regime_value.clone(),
            description: Some("当前市场状态(bull/bear/sideways)+波动率+描述".into()),
            is_secret: false,
        });
        // 从 market_regime 派生 prompt 偏向 + 触发规则
        let regime_str = regime_value["regime"].as_str().unwrap_or("unknown");
        let vol_str = regime_value["volatility"].as_str().unwrap_or("low");
        let (regime_prompt_bias, regime_triggered_rules) = match (regime_str, vol_str) {
            ("bull", "high") => (
                "顺势偏多但高波动环境：关注业绩超预期+资金流入，同时警惕短期大幅回撤",
                "1. 侧重成长性指标（营收增速、ROE趋势）；2. 估值容忍度可适当放宽；3. 关注大单资金流向；4. 高波动环境需关注最大回撤",
            ),
            ("bull", _) => (
                "顺势偏多：关注业绩超预期+资金流入，警惕追高",
                "1. 侧重成长性指标（营收增速、ROE趋势）；2. 估值容忍度可适当放宽；3. 关注大单资金流向",
            ),
            ("bear", "high") => (
                "防御为主+高波动环境：严格关注低估值+稳健现金流，警惕杀估值+踩踏风险",
                "1. 侧重防御性指标（现金流、负债率）；2. 估值要求更严格；3. 关注避险资金流向；4. 高波动环境建议降低仓位",
            ),
            ("bear", _) => (
                "防御为主：关注低估值+稳健现金流，警惕杀估值",
                "1. 侧重防御性指标（现金流、负债率）；2. 估值要求更严格；3. 关注避险资金流向",
            ),
            ("sideways", _) => (
                "精选个股：关注催化剂+预期差，警惕无主线行情",
                "1. 侧重个股α；2. 关注催化剂事件；3. 估值锚定历史中枢",
            ),
            _ => (
                "市场状态未知，不预设多空偏向，仅基于个股自身基本面完成分析",
                "无触发规则，全维度中性分析",
            ),
        };
        merged_vars.push(axagent_harness::workflow_types::Variable {
            name: "regime_prompt_bias".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(regime_prompt_bias.to_string()),
            description: Some("按当前市场状态(regime)匹配的分析偏向指令".into()),
            is_secret: false,
        });
        merged_vars.push(axagent_harness::workflow_types::Variable {
            name: "regime_triggered_rules".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(regime_triggered_rules.to_string()),
            description: Some("当前市场状态触发的分析规则清单".into()),
            is_secret: false,
        });
        // 注入历史反思教训（从 stock_reflections 表取最近的结构化反思结果）
        // 必须始终注入，即使为空，否则 value-investor/research-mgr/trader 等节点
        // 的 input_mapping 引用 {{stock_lessons}} 会报 VARIABLE_NOT_FOUND。
        let lessons_str = fetch_stock_lessons(&stock_code, &db).await;
        let default_lessons = "（暂无历史反思）".to_string();
        let lessons_val = lessons_str.unwrap_or_else(|| default_lessons.clone());
        merged_vars.push(axagent_harness::workflow_types::Variable {
            name: "stock_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(lessons_val.clone()),
            description: Some("该股历史反思教训（错因/被忽视信号/改进建议）".into()),
            is_secret: false,
        });
        // P1: 注入 per-role 经验和教训到辩论角色 prompt
        merged_vars.push(axagent_harness::workflow_types::Variable {
            name: "bull_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(format!(
                "你作为多方研究员的过往经验教训：{}",
                lessons_val
            )),
            description: Some("该股多方视角的历史反思教训".into()),
            is_secret: false,
        });
        merged_vars.push(axagent_harness::workflow_types::Variable {
            name: "bear_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(format!(
                "你作为空方研究员的过往经验教训：{}",
                lessons_val
            )),
            description: Some("该股空方视角的历史反思教训".into()),
            is_secret: false,
        });
        opts.variables = Some(merged_vars);

        match engine.run_workflow(&wf_id, opts).await {
            Ok(result) => {
                let wf_status = result.status;
                match wf_status {
                    axagent_rt_workflow::workflow_engine::WorkflowStatus::Cancelled => {
                        if let Err(e) = app_h.emit(
                            "workflow-error",
                            serde_json::json!({ "workflowId": wf_id, "error": "分析已被取消" }),
                        ) {
                            tracing::warn!("[emit] workflow-error 发送失败: {e}");
                        }
                        if let Err(e) = stock_analyses::Entity::update_many()
                            .col_expr(stock_analyses::Column::Status, Expr::value("cancelled"))
                            .col_expr(
                                stock_analyses::Column::UpdatedAt,
                                Expr::value(chrono::Utc::now().timestamp_millis()),
                            )
                            .filter(stock_analyses::Column::Id.eq(&aid))
                            .exec(&db)
                            .await
                        {
                            tracing::error!("[DB] Cancelled 状态更新失败: {e}");
                        }
                        // 重跑取消：清理临时行，旧数据不受影响
                        if override_target_for_spawn.is_some() {
                            let _ = stock_analyses::Entity::delete_by_id(aid.as_str())
                                .exec(&db)
                                .await;
                        }
                    },
                    axagent_rt_workflow::workflow_engine::WorkflowStatus::Failed => {
                        tracing::warn!(%wf_id, status=?wf_status, "工作流以 Failed 状态结束，保存部分结果");
                        if let Err(e) = app_h.emit(
                            "workflow-completed",
                            serde_json::json!({
                                "workflowId": wf_id,
                                "results": result.results,
                                "output": result.output,
                                "degraded": true,
                                "degradationReason": "部分分析步骤失败，结果为部分数据",
                            }),
                        ) {
                            tracing::warn!("[emit] workflow-completed 发送失败: {e}");
                        }
                        // 即使有节点失败，仍然保存已有结果
                        // 修复"决策信息缺失"误报:优先从 portfolio-mgr 节点本身
                        // 提取决策(见 extract_decision_json 注释),回退到 wf.output。
                        let decision_json = extract_decision_json(&result);
                        let (action, position_pct, reasoning, time_horizon, expected_holding_days) =
                            extract_decision_fields(&decision_json);
                        let degradation_report = as_of::take_asof_degradation_report();
                        let llm_dj_partial = extract_llm_decision_json(&result);
                        let as_of_for_meta: Option<AsOfContext> = as_of::current_as_of();
                        let bb_snapshot = serde_json::to_string(&build_blackboard_snapshot(
                            &result.results,
                            as_of_for_meta.as_ref(),
                            &degradation_report,
                        ))
                        .unwrap_or_else(|_| "{}".to_string());
                        if let Err(e) = stock_analyses::Entity::update_many()
                            .col_expr(stock_analyses::Column::Status, Expr::value("completed"))
                            .col_expr(stock_analyses::Column::DecisionAction, Expr::value(action))
                            .col_expr(
                                stock_analyses::Column::DecisionPositionPct,
                                Expr::value(position_pct),
                            )
                            .col_expr(
                                stock_analyses::Column::DecisionReasoning,
                                Expr::value(reasoning),
                            )
                            .col_expr(
                                stock_analyses::Column::DecisionJson,
                                Expr::value(decision_json),
                            )
                            .col_expr(
                                stock_analyses::Column::BlackboardSnapshot,
                                Expr::value(bb_snapshot),
                            )
                            .col_expr(
                                stock_analyses::Column::DecisionTimeHorizon,
                                Expr::value(time_horizon),
                            )
                            .col_expr(
                                stock_analyses::Column::DecisionExpectedHoldingDays,
                                Expr::value(expected_holding_days.map(|d| d as i64)),
                            )
                            .col_expr(
                                stock_analyses::Column::LlmDecisionJson,
                                Expr::value(llm_dj_partial),
                            )
                            .col_expr(
                                stock_analyses::Column::UpdatedAt,
                                Expr::value(chrono::Utc::now().timestamp_millis()),
                            )
                            .filter(stock_analyses::Column::Id.eq(&aid))
                            .exec(&db)
                            .await
                        {
                            tracing::error!("[DB] Failed 状态下保存分析结果失败: {e}");
                        }
                        // 重跑（Failed 有部分结果）：删旧行，更新临时行 ID 到 override id
                        if let Some(ref old_id) = override_target_for_spawn {
                            let _ = stock_analyses::Entity::delete_by_id(old_id.as_str())
                                .exec(&db)
                                .await;
                            let _ = stock_analyses::Entity::update_many()
                                .col_expr(stock_analyses::Column::Id, Expr::value(old_id.as_str()))
                                .filter(stock_analyses::Column::Id.eq(&aid))
                                .exec(&db)
                                .await;
                        }
                    },
                    _ => {
                        if let Err(e) = app_h.emit(
                            "workflow-completed",
                            serde_json::json!({
                                "workflowId": wf_id,
                                "results": result.results,
                                "output": result.output,
                            }),
                        ) {
                            tracing::warn!("[emit] workflow-completed 发送失败: {e}");
                        }
                        // 修复"决策信息缺失"误报:优先从 portfolio-mgr 节点本身
                        // 提取决策(见 extract_decision_json 注释),回退到 wf.output。
                        let decision_json = extract_decision_json(&result);
                        // V40 修复:计算 LLM 决策(trader)与公式决策(portfolio-mgr)的一致性分数
                        // V50 升级: 返回 AgreementBreakdown，包含分维度诊断
                        let llm_dj_agr = extract_llm_decision_json(&result);
                        let agreement_breakdown = compute_decision_agreement(
                            decision_json.as_deref(),
                            llm_dj_agr.as_deref(),
                        );
                        // V50: 预计算分歧诊断文本（供 reasoning 追加和 UI 展示）
                        let disagreement_note = agreement_breakdown.as_ref().map(|ab| {
                            // P0: 存在 f7 自指时标注污染程度
                            let f7_note = ab.f7_weight_pct.map(|pct|
                                format!(" [f7污染{}%]", pct)
                            ).unwrap_or_default();
                            // P3: trader 不输出 action, 改用 trader 影响度评分
                            if ab.conflict_type.starts_with("f7_") {
                                let inf_level = match ab.conflict_type.as_str() {
                                    "f7_low_influence" => "低",
                                    "f7_moderate_influence" => "中",
                                    "f7_high_influence" => "高",
                                    "f7_dominant" => "主导",
                                    _ => "?",
                                };
                                format!(
                                    "📊trader影响:{} (公式{} vs 无f7{},分={}){}",
                                    inf_level, ab.formula_action,
                                    ab.f7_free_action.as_deref().unwrap_or("?"),
                                    ab.f7_free_action_score.unwrap_or(0.0) as i32,
                                    f7_note,
                                )
                            } else if ab.total >= 60 {
                                format!("🤝双视角一致:{}分{}", ab.total, f7_note)
                            } else if ab.total >= 40 {
                                format!("⚠️双视角部分一致:{}分{}", ab.total, f7_note)
                            } else {
                                // P0: f7 纯净版 action 一致性对比
                                let f7_free_note = match (ab.f7_free_action.as_deref(), ab.f7_free_action_score) {
                                    (Some(fa), Some(fs)) if *fa != ab.formula_action =>
                                        format!("(无f7={}/{})", fa, fs as i32),
                                    _ => String::new(),
                                };
                                format!(
                                    "🔴双视角分歧:{}分(公式{} vs LLM{},维度:act={} pos={} conf={}){}{}",
                                    ab.total, ab.formula_action, ab.llm_action,
                                    ab.action_score as i32, ab.position_score as i32,
                                    ab.confidence_score as i32,
                                    f7_note, f7_free_note
                                )
                            }
                        });
                        // V50: 将一致性诊断 + 调整后置信度嵌入 decision_json
                        let decision_json = decision_json.map(|dj| {
                            if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&dj) {
                                if let Some(obj) = v.as_object_mut() {
                                    if let Some(ref ab) = agreement_breakdown {
                                        // 向后兼容: formulaLlmAgreement = 总分
                                        obj.insert(
                                            "formulaLlmAgreement".into(),
                                            serde_json::json!(ab.total),
                                        );
                                        // V50: 完整诊断结构体
                                        obj.insert("agreementBreakdown".into(), serde_json::json!({
                                            "total": ab.total,
                                            "actionOk": ab.action_ok,
                                            "actionNote": ab.action_note,
                                            "formulaAction": ab.formula_action,
                                            "llmAction": ab.llm_action,
                                            "positionGap": ab.position_gap,
                                            "confidenceGap": ab.confidence_gap,
                                            "conflictType": ab.conflict_type,
                                            // P0: f7 自指污染标记
                                            "f7WeightPct": ab.f7_weight_pct,
                                            "f7FreePosterior": ab.f7_free_posterior,
                                            "f7FreeAction": ab.f7_free_action,
                                            "f7FreeActionScore": ab.f7_free_action_score,
                                        }));
                                        // V50: 置信度调制 — 一致时 boost, 分歧时 penalty
                                        let formula_conf = obj.get("confidence")
                                            .and_then(|c| c.as_f64())
                                            .unwrap_or(50.0);
                                        let factor = 1.0 + (ab.total as f64 - 50.0) / 100.0;
                                        let adj = (formula_conf * factor).clamp(0.0, 100.0);
                                        obj.insert(
                                            "adjustedConfidence".into(),
                                            serde_json::json!((adj * 10.0).round() / 10.0),
                                        );
                                    }
                                }
                                v.to_string()
                            } else {
                                dj
                            }
                        });
                        // ── P1-1: 如果当前股票在持仓中，计算退出紧迫度 ──
                        // 读取 portfolio_holdings 表，判断分析结果是否触发退出建议
                        let exit_urgency = (|| -> Option<f64> {
                            let holding = db.blocking_find(
                                |db| async {
                                    use axagent_entities::portfolio_holdings;
                                    portfolio_holdings::Entity::find()
                                        .filter(portfolio_holdings::Column::StockCode.eq(&stock_code))
                                        .one(db).await.ok().flatten()
                                }
                            ).ok()??;
                            // 提取当前分析决策
                            let action_str = decision_json.as_deref()
                                .and_then(|dj| serde_json::from_str::<serde_json::Value>(dj).ok())
                                .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(String::from));
                            match action_str.as_deref() {
                                Some("卖出") => Some(90.0),   // 高紧迫卖出
                                Some("减持") => Some(60.0),   // 中紧迫减持
                                Some("观望") if holding.shares > 0 => Some(30.0), // 低紧迫（不增持）
                                _ => None,                     // 持有/买入 → 不触发退出
                            }
                        })();
                        // 将退出紧迫度注入 decision_json
                        let decision_json = if exit_urgency.is_some() {
                            decision_json.map(|dj| {
                                if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&dj) {
                                    if let Some(obj) = v.as_object_mut() {
                                        obj.insert("_exitUrgency".into(), serde_json::json!(exit_urgency));
                                    }
                                    v.to_string()
                                } else { dj }
                            })
                        } else { decision_json };
                        let (
                            action,
                            position_pct,
                            reasoning,
                            time_horizon,
                            expected_holding_days,
                        ) = extract_decision_fields(&decision_json);
                        // V50: reasoning 末尾追加双视角分歧诊断
                        let reasoning = match (reasoning, disagreement_note) {
                            (Some(r), Some(note)) => Some(format!("{} | {}", r, note)),
                            (r, _) => r,
                        };
                        // 克隆决策字段供 Memory RAG 索引（原值将被 DB 写入消费）
                        let mem_action = action.clone();
                        let mem_reasoning = reasoning.clone();
                        let mem_dj = decision_json.clone();
                        // 持久化工作流结果到 blackboard_snapshot，供历史回放/报告
                        // 生成/跨日 key_levels 聚合使用。修复 Defect #2。
                        // B7: 消费 take_asof_degradation_report() 写入 `degraded` 块
                        // (spec §4.1: vendor 降级报告)
                        let as_of_for_meta: Option<AsOfContext> = as_of::current_as_of();
                        let degradation_report = as_of::take_asof_degradation_report();
                        let bb_snapshot = serde_json::to_string(&build_blackboard_snapshot(
                            &result.results,
                            as_of_for_meta.as_ref(),
                            &degradation_report,
                        ))
                        .unwrap_or_else(|_| "{}".to_string());
                        let llm_dj = extract_llm_decision_json(&result);
                        if let Err(e) = stock_analyses::Entity::update_many()
                            .col_expr(stock_analyses::Column::Status, Expr::value("completed"))
                            .col_expr(stock_analyses::Column::DecisionAction, Expr::value(action))
                            .col_expr(
                                stock_analyses::Column::DecisionPositionPct,
                                Expr::value(position_pct),
                            )
                            .col_expr(
                                stock_analyses::Column::DecisionReasoning,
                                Expr::value(reasoning),
                            )
                            .col_expr(
                                stock_analyses::Column::DecisionJson,
                                Expr::value(decision_json),
                            )
                            .col_expr(
                                stock_analyses::Column::BlackboardSnapshot,
                                Expr::value(bb_snapshot),
                            )
                            .col_expr(
                                stock_analyses::Column::DecisionTimeHorizon,
                                Expr::value(time_horizon),
                            )
                            .col_expr(
                                stock_analyses::Column::DecisionExpectedHoldingDays,
                                Expr::value(expected_holding_days.map(|d| d as i64)),
                            )
                            .col_expr(
                                stock_analyses::Column::LlmDecisionJson,
                                Expr::value(llm_dj),
                            )
                            .col_expr(
                                stock_analyses::Column::UpdatedAt,
                                Expr::value(chrono::Utc::now().timestamp_millis()),
                            )
                            .filter(stock_analyses::Column::Id.eq(&aid))
                            .exec(&db)
                            .await
                        {
                            tracing::error!("[DB] 保存分析结果失败: {e}");
                        }

                        // 索引决策到 Memory RAG（best-effort，失败不阻塞）
                        if let Some(ref dj) = mem_dj {
                            if !dj.is_empty() {
                                let confidence_str = serde_json::from_str::<serde_json::Value>(dj)
                                    .ok()
                                    .and_then(|v| v.get("confidence").and_then(|c| c.as_f64()))
                                    .map(|c| format!("{:.0}", c))
                                    .unwrap_or_else(|| "?".to_string());
                                let memory_content = format!(
                                    "股票:{} {} 决策:{} 置信度:{} 日期:{}\n{}",
                                    stock_code,
                                    sc_name_for_spawn,
                                    mem_action.as_deref().unwrap_or(""),
                                    confidence_str,
                                    chrono::Utc::now().format("%Y-%m-%d"),
                                    mem_reasoning.as_deref().unwrap_or(""),
                                );
                                let _ = crate::indexing::index_memory_item(
                                    &db,
                                    &master_key,
                                    &vector_store,
                                    "stock_decisions",
                                    &aid,
                                    &memory_content,
                                    "openai::text-embedding-3-small",
                                    None,
                                )
                                .await;
                            }
                        }
                        // 重跑成功：删旧行，更新临时行 ID 到 override id（保持前端 URL 稳定）
                        if let Some(ref old_id) = override_target_for_spawn {
                            let _ = stock_analyses::Entity::delete_by_id(old_id.as_str())
                                .exec(&db)
                                .await;
                            let _ = stock_analyses::Entity::update_many()
                                .col_expr(stock_analyses::Column::Id, Expr::value(old_id.as_str()))
                                .filter(stock_analyses::Column::Id.eq(&aid))
                                .exec(&db)
                                .await;
                        }
                    },
                }
            },
            Err(e) => {
                let _ = app_h.emit(
                    "workflow-error",
                    serde_json::json!({ "workflowId": wf_id, "error": e.to_string() }),
                );
                if let Err(db_e) = stock_analyses::Entity::update_many()
                    .col_expr(stock_analyses::Column::Status, Expr::value(format!("failed: {e}")))
                    .col_expr(
                        stock_analyses::Column::UpdatedAt,
                        Expr::value(chrono::Utc::now().timestamp_millis()),
                    )
                    .filter(stock_analyses::Column::Id.eq(&aid))
                    .exec(&db)
                    .await
                {
                    tracing::error!("[DB] run_workflow Err 状态更新失败: {db_e}");
                }
                // 重跑工作流引擎错误：清理临时行，旧数据不受影响
                if override_target_for_spawn.is_some() {
                    let _ = stock_analyses::Entity::delete_by_id(aid.as_str())
                        .exec(&db)
                        .await;
                }
            },
        }}).await  // with_degradation_log
    }).await // with_optional_asof
    });

    Ok(serde_json::json!({
        "analysisId": analysis_id,
        "workflowId": wf_id_ret,
        "stockCode": sc_for_ret,
        "stockName": sc_name,
    }))
}

/// 取消正在运行的股票分析工作流
#[tauri::command]
pub async fn cancel_stock_workflow(
    state: State<'_, AppState>,
    workflow_id: String,
) -> Result<(), String> {
    state
        .work_engine
        .cancel_workflow(&workflow_id)
        .await
        .map(|_| ())
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("取消工作流失败: {e}"))
                .to_string()
        })
}

// ── 批量/定时分析入口（无 Tauri State 依赖，供 CronExecutor 调用）──

/// 对单只股票执行完整分析（无 Tauri 事件发射，适合批量定时扫描）
///
/// 与 `run_stock_workflow_inner` 逻辑相同但：
/// - 不发射 `workflow-step-done` 事件（无前端监听）
/// - 不需要 `as_of_date` 参数（使用当前时间，非回放模式）
/// - 不需要 `dry_run`（总是完整执行）
/// - 参数是独立引用而非 Tauri State
pub async fn run_single_stock_analysis(
    db: &DatabaseConnection,
    client: &axagent_astock_data::AStockClient,
    engine: &Arc<axagent_rt_workflow::work_engine::WorkEngine>,
    stock_code: &str,
    stock_name: &str,
) -> Result<String, String> {
    // 1. 创建 stock_analyses 记录
    let now_ms = chrono::Utc::now().timestamp_millis();
    let analysis_id = uuid::Uuid::new_v4().to_string();

    stock_analyses::ActiveModel {
        id: Set(analysis_id.clone()),
        stock_code: Set(stock_code.to_string()),
        stock_name: Set(stock_name.to_string()),
        analysis_date: Set(chrono::Utc::now().format("%Y-%m-%d").to_string()),
        provider_id: Set("workflow".into()),
        conversation_id: Set(uuid::Uuid::new_v4().to_string()),
        status: Set("running".into()),
        decision_action: Set(None),
        decision_position_pct: Set(None),
        decision_reasoning: Set(None),
        decision_json: Set(None),
        llm_decision_json: Set(None),
        blackboard_snapshot: Set(None),
        config_id: Set(None),
        analysis_kind: Set("live".into()),
        as_of_date: Set(Some(chrono::Utc::now().format("%Y-%m-%d").to_string())),
        model_version: Set(None),
        data_snapshot_id: Set(None),
        outcome: Set(None),
        decision_time_horizon: Set(None),
        decision_expected_holding_days: Set(None),
        created_at: Set(now_ms),
        updated_at: Set(now_ms),
    }
    .insert(db)
    .await
    .map_err(|e| ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("DB 写入失败: {e}")))?;

    // 2. 获取行情（用于数据预检和 stock name）
    let quote = client.get_quote(stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("行情获取失败: {e}"))
    })?;

    // 3. 数据质量预检
    match data_quality_precheck(client, stock_code, &quote).await {
        QualityPrecheckResult::Insufficient {
            summary,
            missing_sources,
        } => {
            let missing_report: Vec<serde_json::Value> = missing_sources
                .iter()
                .map(|item| {
                    json!({
                        "source": item.source,
                        "status": item.status,
                        "detail": item.detail,
                    })
                })
                .collect();
            let _ = stock_analyses::Entity::update(stock_analyses::ActiveModel {
                id: Set(analysis_id.clone()),
                status: Set("failed".into()),
                decision_json: Set(Some(
                    json!({
                        "action": "skip",
                        "reasoning": format!("数据不足，跳过分析: {summary}"),
                        "data_missing_report": missing_report,
                    })
                    .to_string(),
                )),
                updated_at: Set(chrono::Utc::now().timestamp_millis()),
                ..Default::default()
            })
            .exec(db)
            .await;
            return Err(summary);
        },
        QualityPrecheckResult::Pass | QualityPrecheckResult::Partial(_) => {
            // 继续执行
        },
    }

    // 4. 加载模板并注入 stock_code
    let loaded = load_and_inject_template(db, stock_code, stock_name, "stock-analysis").await?;

    // 5. 解析运行时参数
    let (max_concurrent, step_timeout) = resolve_runtime_options(loaded.variables.as_deref());

    // 5.5 [A1 借鉴] 注入历史反思教训(TradingAgents past_context 机制):
    //   批量/定时分析场景下,trader/research-mgr/value-investor 节点能看到
    //   该股最近 90 天的反思教训(lesson_summary),避免重蹈覆辙。前端触发场景下
    //   run_stock_workflow_inner 同样会注入,这里是补齐 cron / batch 入口。
    //   必须始终注入,即使为空（否则 VARIABLE_NOT_FOUND）。
    let lessons_str = fetch_stock_lessons(stock_code, db).await;
    let default_lessons = "（暂无历史反思）".to_string();
    let lessons_val = lessons_str.unwrap_or_else(|| default_lessons.clone());
    let variables = vec![
        Variable {
            name: "stock_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(lessons_val.clone()),
            description: Some("A1: 该股最近 90 天的反思教训".into()),
            is_secret: false,
        },
        Variable {
            name: "bull_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(format!(
                "你作为多方研究员的过往经验教训：{}",
                lessons_val
            )),
            description: Some("该股多方视角的历史反思教训".into()),
            is_secret: false,
        },
        Variable {
            name: "bear_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(format!(
                "你作为空方研究员的过往经验教训：{}",
                lessons_val
            )),
            description: Some("该股空方视角的历史反思教训".into()),
            is_secret: false,
        },
    ];

    // 6. 创建并运行工作流
    let wf_name = format!("stock-analysis-{stock_code}-batch");
    let workflow = engine
        .create_workflow(&wf_name, loaded.nodes, loaded.edges)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建工作流失败: {e}"))
        })?;
    let wf_id = workflow.id.clone();

    let opts = RunOptions {
        max_concurrent,
        step_timeout,
        progress_callback: None,
        input: Some(json!({"stock_code": stock_code})),
        input_schema: loaded.input_schema.clone(),
        output_schema: loaded.output_schema.clone(),
        dry_run: false,
        variables: if variables.is_empty() {
            None
        } else {
            Some(variables)
        },
        ..Default::default()
    };

    let result = engine.run_workflow(&wf_id, opts).await;

    match result {
        Ok(wf) => {
            // 更新为完成状态
            // 修复"决策信息缺失"误报:用 extract_decision_json 从 portfolio-mgr
            // 节点 .result 提取决策(而非 CodeNode 包装顶层,后者无 action 字段)。
            let decision_json_str = extract_decision_json(&wf);
            let decision_output = decision_json_str
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

            let decision_action = decision_output.as_ref().and_then(|d| {
                d.get("action")
                    .and_then(|a| a.as_str().map(|s| s.to_string()))
            });

            let _ = stock_analyses::Entity::update(stock_analyses::ActiveModel {
                id: Set(analysis_id.clone()),
                status: Set("completed".into()),
                decision_action: Set(decision_action),
                decision_json: Set(decision_json_str),
                updated_at: Set(chrono::Utc::now().timestamp_millis()),
                ..Default::default()
            })
            .exec(db)
            .await;

            // ── [B1 借鉴] 两阶段协议: 落盘时同步写 stock_reflections pending row ──
            // TradingAgents 反思模式: 先占位(pending)再异步 resolve。这样:
            //   1) 系统重启/进程崩溃后,D1 批量反思能扫到所有 pending,不会丢失
            //   2) 持仓期到时,D1 知道哪些 row 该被 resolve(避免重复 INSERT 触发冲突)
            //   3) fetch_stock_lessons 可基于 status='resolved' 过滤,只注入真正可用的教训
            // 字段: as_of_date = analysis_date, raw_return/alpha_return/holding_days
            //   全部 None(预测不到),status='pending',后续由 D1 批量补全。
            let pending_id = uuid::Uuid::new_v4().to_string();
            let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let _ = stock_reflections::ActiveModel {
                id: Set(pending_id.clone()),
                stock_code: Set(stock_code.to_string()),
                stock_name: Set(stock_name.to_string()),
                original_analysis_id: Set(analysis_id.clone()),
                as_of_date: Set(today_str.clone()),
                hindsight_date: Set(today_str),
                min_confidence_threshold: Set(70),
                reflection_depth: Set("light".to_string()),
                actual_outcome: Set(String::new()),
                // v008 (C3 借鉴): 结构化 outcome,pending 阶段全 None
                raw_return: Set(None),
                alpha_return: Set(None),
                holding_days: Set(None),
                benchmark_name: Set(None),
                // v008 (C2 借鉴): 输出 schema,pending 阶段全 None
                verdict: Set(None),
                alpha_cited: Set(None),
                lesson_summary: Set(None),
                what_went_wrong: Set(None),
                missed_signals: Set(None),
                fix_for_future: Set(None),
                parameter_suggestions_json: Set(None),
                decision_json: Set(None),
                blackboard_snapshot: Set(None),
                model_version: Set(None),
                status: Set("pending".to_string()),
                created_at: Set(chrono::Utc::now().timestamp_millis()),
                updated_at: Set(chrono::Utc::now().timestamp_millis()),
            }
            .insert(db)
            .await;
            tracing::info!(
                "[B1 batch_analysis] {stock_code} ({stock_name}) 已落盘 pending reflection {pending_id},等 D1 持仓期到达 resolve"
            );

            tracing::info!(
                "[batch_analysis] {stock_code} ({stock_name}) 完成, status={:?}",
                wf.status
            );
            Ok(analysis_id)
        },
        Err(e) => {
            let err_msg = format!("{:?}", e);
            let _ = stock_analyses::Entity::update(stock_analyses::ActiveModel {
                id: Set(analysis_id.clone()),
                status: Set("failed".into()),
                decision_json: Set(Some(
                    json!({
                        "action": "error",
                        "reasoning": err_msg.clone(),
                    })
                    .to_string(),
                )),
                updated_at: Set(chrono::Utc::now().timestamp_millis()),
                ..Default::default()
            })
            .exec(db)
            .await;

            tracing::error!("[batch_analysis] {stock_code} 失败: {err_msg}");
            Err(err_msg)
        },
    }
}

/// 从 stock_analyses 表查询同股票过去 3 个月的失败案例，返回格式化文本。
pub(crate) async fn fetch_similar_cases(
    stock_code: &str,
    db: &sea_orm::DatabaseConnection,
) -> Option<String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
    let three_months_ago = (chrono::Utc::now() - chrono::Duration::days(90))
        .format("%Y-%m-%d")
        .to_string();
    let all = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::StockCode.eq(stock_code))
        .filter(stock_analyses::Column::Outcome.eq("loss"))
        .filter(stock_analyses::Column::AnalysisDate.gte(&three_months_ago))
        .order_by(stock_analyses::Column::AnalysisDate, sea_orm::Order::Desc)
        .all(db)
        .await
        .unwrap_or_default();
    let similar: Vec<_> = all.into_iter().take(5).collect();
    if similar.is_empty() {
        return None;
    }
    let mut lines: Vec<String> = Vec::new();
    for s in similar {
        let conf = s
            .decision_json
            .as_deref()
            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
            .and_then(|v| v.get("confidence").and_then(|c| c.as_f64()))
            .map(|c| format!("{}", c as u8))
            .unwrap_or_else(|| "?".to_string());
        let action = s.decision_action.as_deref().unwrap_or("?");
        let reasoning = s.decision_reasoning.as_deref().unwrap_or("");
        let abbr = if reasoning.len() > 60 {
            &reasoning[..60]
        } else {
            reasoning
        };
        lines.push(format!(
            "- 日期:{} 决策:{} 置信度:{} → 失败。要点:{}",
            s.analysis_date, action, conf, abbr
        ));
    }
    Some(lines.join("\n"))
}
/// 从 stock_reflections 表查询该股最近的结构化反思教训（错因/被忽视信号/改进建议），返回格式化文本。
///
/// ## v008 + E1 升级（借鉴 TradingAgents past_context 机制）
///
/// 借鉴 TradingAgents 反思机制的多范围教训注入:
/// - **same_ticker**(3 条): 同 ticker 最近 90 天的反思,直接可借鉴
/// - **all_recent**(2 条): 所有 ticker 最近 7 天的反思,捕捉市场级教训
///   (如"近期白马股普遍杀估值""科技股 Q3 业绩雷高发")
/// - 跨 sector 范围需要 stock_analyses.sector 字段(v009 之后再做)
///
/// ## v008 字段选择
///
/// 输出 lesson_summary (≤200 字符) + verdict(判定标签) + alpha_cited(关键 alpha)
/// 替代之前的 what_went_wrong/missed_signals/fix_for_future 三件套
/// (后三个字段在新反思中可能为空,因为 prompt 现在只强制 short 文本)。
pub(crate) async fn fetch_stock_lessons(
    stock_code: &str,
    db: &sea_orm::DatabaseConnection,
) -> Option<String> {
    use chrono::Utc;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    // ── same_ticker: 3 条同 ticker 近 90 天已完成反思 ──
    let three_months_ago = Utc::now() - chrono::Duration::days(90);
    let same_ticker: Vec<stock_reflections::Model> = stock_reflections::Entity::find()
        .filter(stock_reflections::Column::StockCode.eq(stock_code))
        .filter(stock_reflections::Column::Status.eq("completed")) // 只注入已 resolve 的教训
        .filter(stock_reflections::Column::CreatedAt.gte(three_months_ago.timestamp_millis()))
        .order_by_desc(stock_reflections::Column::CreatedAt)
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .take(3)
        .collect();

    // ── all_recent: 2 条所有 ticker 近 7 天(跨 ticker 市场级教训)──
    let seven_days_ago = Utc::now() - chrono::Duration::days(7);
    let all_recent: Vec<stock_reflections::Model> = stock_reflections::Entity::find()
        .filter(stock_reflections::Column::CreatedAt.gte(seven_days_ago.timestamp_millis()))
        .filter(stock_reflections::Column::Status.eq("completed")) // 只看已 resolve 的
        .order_by_desc(stock_reflections::Column::CreatedAt)
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.stock_code != stock_code) // 排除 same_ticker 已经包含的
        .take(2)
        .collect();

    if same_ticker.is_empty() && all_recent.is_empty() {
        return None;
    }

    let mut lines: Vec<String> = Vec::new();

    if !same_ticker.is_empty() {
        lines.push(format!("【同股近 90 天反思 {} 条】", same_ticker.len()));
        for (i, l) in same_ticker.iter().enumerate() {
            lines.push(format!("#{} ({}, 反思于 {})", i + 1, l.stock_code, l.hindsight_date));
            if let Some(ref ls) = l.lesson_summary {
                lines.push(format!("  - 总结：{}", ls));
            }
            if let Some(ref v) = l.verdict {
                lines.push(format!("  - 判定：{}", v));
            }
            if let Some(ref ac) = l.alpha_cited {
                lines.push(format!("  - 关键 alpha：{}", ac));
            }
            // 兼容旧反思(无 v008 字段)
            if let Some(ref w) = l.what_went_wrong {
                lines.push(format!("  - 错因：{}", w));
            }
            if let Some(ref f) = l.fix_for_future {
                lines.push(format!("  - 改进建议：{}", f));
            }
        }
    }

    if !all_recent.is_empty() {
        lines.push(String::new());
        lines.push(format!("【近期市场级反思 {} 条(跨 ticker 近 7 天)】", all_recent.len()));
        for (i, l) in all_recent.iter().enumerate() {
            lines.push(format!("#{} {} ({}):", i + 1, l.stock_code, l.stock_name));
            if let Some(ref ls) = l.lesson_summary {
                lines.push(format!("  - {}", ls));
            } else if let Some(ref w) = l.what_went_wrong {
                lines.push(format!("  - 错因：{}", w));
            }
        }
    }

    Some(lines.join("\n"))
}
