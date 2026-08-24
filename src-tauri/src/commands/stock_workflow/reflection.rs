use super::core::fetch_stock_lessons;
use super::decision::{load_and_inject_template, resolve_runtime_options};
use super::serenity::extract_agent_output;
use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;
use axagent_astock_data::as_of::{self, AsOfContext};
use axagent_entities::stock_analyses;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use std::sync::Arc;
use tauri::State;

/// 反思复盘工作流：从原始分析的 blackboard_snapshot 记忆中反思。
/// 结果写入独立的 `stock_reflections` 表。
#[allow(clippy::too_many_arguments)]
pub async fn run_reflection_workflow(
    db: &DatabaseConnection,
    client: &axagent_astock_data::AStockClient,
    engine: &Arc<axagent_rt_workflow::work_engine::WorkEngine>,
    vector_store: &axagent_search::vector_store::VectorStore,
    master_key: &[u8; 32],
    stock_code: &str,
    stock_name: &str,
    original_analysis_id: &str,
    actual_outcome: &str,
    // v008 (C3 借鉴): 4 个结构化 outcome 变量
    raw_return: Option<f64>,
    alpha_return: Option<f64>,
    holding_days: Option<i32>,
    benchmark_name: Option<&str>,
    as_of_date: &str,
    hindsight_date: &str,
    min_confidence_threshold: u8,
    reflection_depth: &str,
    // [B2/B3 借鉴] 反思 row ID(B1 阶段落盘的 pending row)。
    // 传入则 UPDATE 现有 row;传 None 则按 v1 行为 INSERT 新 row,保持旧调用方兼容。
    reflection_id: Option<String>,
    // [方向3] 轨迹存储，用于持久化反思执行轨迹。
    // 传 None 则跳过 Trajectory 持久化（手动反思等不需要轨迹的场景）。
    trajectory_storage: Option<&std::sync::Arc<axagent_trajectory::TrajectoryStorage>>,
) -> Result<String, String> {
    use axagent_entities::stock_reflections;
    use sea_orm::sea_query::Expr;

    let now_ms = chrono::Utc::now().timestamp_millis();

    // ── [B2 借鉴] 幂等守卫: 如果 reflection_id 已 completed,直接返回 cached ──
    if let Some(ref rid) = reflection_id {
        if let Some(existing) =
            stock_reflections::Entity::find_by_id(rid.clone()).one(db).await.map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("B2 查询已存在反思失败: {e}"))
            })?
        {
            if existing.status == "completed" {
                tracing::info!(
                    "[B2 idempotency] reflection_id={rid} 已 completed,跳过重跑,直接返回 cached"
                );
                return Ok(rid.clone());
            }
        }
    }

    // ── [B3 借鉴] 原子写: reflection_id 存在则 UPDATE pending→running,否则 INSERT ──
    let analysis_id = reflection_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    if let Some(ref rid) = reflection_id {
        let _ = stock_reflections::Entity::update_many()
            .col_expr(stock_reflections::Column::Status, Expr::value("running"))
            .col_expr(stock_reflections::Column::UpdatedAt, Expr::value(now_ms))
            .filter(stock_reflections::Column::Id.eq(rid.clone()))
            .exec(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL)
                    .with_detail(format!("B3 UPDATE pending→running 失败: {e}"))
            })?;
        tracing::info!("[B3 atomic] reflection_id={rid} pending→running");
    } else {
        // 兼容旧调用方路径: INSERT 新 row
        stock_reflections::ActiveModel {
            id: Set(analysis_id.clone()),
            stock_code: Set(stock_code.to_string()),
            stock_name: Set(stock_name.to_string()),
            original_analysis_id: Set(original_analysis_id.to_string()),
            as_of_date: Set(as_of_date.to_string()),
            hindsight_date: Set(hindsight_date.to_string()),
            min_confidence_threshold: Set(min_confidence_threshold as i32),
            reflection_depth: Set(reflection_depth.to_string()),
            actual_outcome: Set(actual_outcome.to_string()),
            // v008 (C3 借鉴): 4 个结构化 outcome
            raw_return: Set(raw_return),
            alpha_return: Set(alpha_return),
            holding_days: Set(holding_days),
            benchmark_name: Set(benchmark_name.map(|s| s.to_string())),
            // v008 (C2 借鉴): 3 个输出 schema 字段
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
            status: Set("running".to_string()),
            created_at: Set(now_ms),
            updated_at: Set(now_ms),
        }
        .insert(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("DB 写入失败: {e}"))
        })?;
    }

    // 2. 加载反思复盘模板（stock-reflection，DAG 结构与 stock-analysis 一致）
    let loaded = load_and_inject_template(db, stock_code, stock_name, "stock-reflection").await?;

    // 注入 vendor 启用状态过滤器（与 stock-analysis 主工作流一致）
    super::decision::inject_vendor_state(client, loaded.variables.as_ref());

    let (max_concurrent, step_timeout, _total_timeout) =
        resolve_runtime_options(loaded.variables.as_deref());

    // 3. 创建嵌套工作流
    let wf_name = format!("stock-reflection-{stock_code}");
    let workflow =
        engine.create_workflow(&wf_name, loaded.nodes, loaded.edges).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建反思工作流失败: {e}"))
        })?;
    let wf_id = workflow.id.clone();

    // 4. 加载原始分析记录：时间维度 + blackboard_snapshot（分析工作流记忆）
    // [v2] 不再通过 sub-analysis SubWorkflowNode 重跑完整 stock-analysis DAG，
    //      而是从 stock_analyses.blackboard_snapshot 加载已保存的分析结果作为记忆，
    //      构造名为 "sub-analysis" 的变量注入工作流。
    //      context_sources / input_mapping 路径（如 sub-analysis.trader.content.action）
    //      保持不变，resolve_var_path 会从注入的变量中按路径下钻。
    //
    // 手动触发时 original_analysis_id="" → 无记忆可加载，注入空对象降级。
    // 但反思 prompt 模板 (reflection.md:17-18) hard-code 引用
    // {{original_time_horizon}} / {{original_holding_days}},所以必须注入占位值
    // (否则 work_engine 报 VARIABLE_NOT_FOUND,reflection-agent 节点 Failed,
    // 数据库 what_went_wrong 等字段全 null)。
    let original_analysis: Option<stock_analyses::Model> = if original_analysis_id.is_empty() {
        None
    } else {
        stock_analyses::Entity::find_by_id(original_analysis_id).one(db).await.ok().flatten()
    };

    let original_ctx: Option<(String, i64)> = original_analysis.as_ref().and_then(|a| {
        let t = a.decision_time_horizon.clone()?;
        let h = a.decision_expected_holding_days?;
        Some((t, h))
    });

    // 4a. 从 blackboard_snapshot 构造 sub-analysis 变量（分析工作流记忆）
    let sub_analysis_memory: serde_json::Value = match &original_analysis {
        Some(analysis) => {
            build_sub_analysis_from_snapshot(analysis.blackboard_snapshot.as_deref(), stock_code)
        },
        None => {
            tracing::warn!(
                "[reflection] {}: 无 original_analysis_id 或记录不存在,注入空 sub-analysis 记忆",
                stock_code
            );
            serde_json::json!({})
        },
    };

    // 5. 注入变量
    let mut variables = vec![
        // [v2] sub-analysis 变量：从 blackboard_snapshot 加载的分析工作流记忆。
        // 替代原 SubWorkflowNode 嵌套重放，避免重跑完整 stock-analysis DAG。
        // reflection-comparator 的 input_mapping (如 sub-analysis.trader.content.action)
        // 和 reflection-agent 的 context_sources 都引用此变量名。
        axagent_harness::workflow_types::Variable {
            name: "sub-analysis".into(),
            var_type: "object".into(),
            value: sub_analysis_memory,
            description: Some(
                "原始股票分析工作流的记忆（从 blackboard_snapshot._raw.* 恢复的节点输出）".into(),
            ),
            is_secret: false,
        },
        // 内联 system_prompt (stock_analysis_setup.rs:4538-4552) 引用了
        // {{stock_code}} / {{stock_name}} —— 必须在 variables 顶层,
        // input_mapping 的 source="trigger" 不会把它们提到顶层 (只会追加到
        // system_prompt 尾部的 "--- 输入上下文 ---" 块)。
        // 不注入会触发 reflection-agent 节点的 VARIABLE_NOT_FOUND。
        axagent_harness::workflow_types::Variable {
            name: "stock_code".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(stock_code.to_string()),
            description: Some("当前反思的股票代码".into()),
            is_secret: false,
        },
        axagent_harness::workflow_types::Variable {
            name: "stock_name".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(stock_name.to_string()),
            description: Some("当前反思的股票名称".into()),
            is_secret: false,
        },
        axagent_harness::workflow_types::Variable {
            name: "actual_outcome".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(actual_outcome.to_string()),
            description: Some("实际走势结果，格式如 '30天跌8% → 失败'".into()),
            is_secret: false,
        },
        axagent_harness::workflow_types::Variable {
            name: "reflection_depth".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(reflection_depth.to_string()),
            description: Some("反思深度：light(简要) / deep(详细推理链)".into()),
            is_secret: false,
        },
        // [时间旅行模式] 注入 hindsight_date 让 reflection-agent LLM 知道评估时点。
        // 工具调用也以此日期为 AS_OF 锚点，查看"截至此日期的实际走势"。
        axagent_harness::workflow_types::Variable {
            name: "hindsight_date".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(hindsight_date.to_string()),
            description: Some("反思评估时点（YYYY-MM-DD），工具调用和数据查看的时间锚点".into()),
            is_secret: false,
        },
        // [C3 借鉴] 4 个结构化 outcome 变量（reflection.md prompt 引用但原未注入）
        // 不注入会导致 VARIABLE_NOT_FOUND 或 LLM 看到空值，影响反思质量。
        axagent_harness::workflow_types::Variable {
            name: "raw_return_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(raw_return.unwrap_or(0.0)),
            description: Some("实际原始收益率百分比（如 -8.0 表示跌 8%）".into()),
            is_secret: false,
        },
        axagent_harness::workflow_types::Variable {
            name: "alpha_return_pct".into(),
            var_type: "number".into(),
            value: serde_json::json!(alpha_return.unwrap_or(0.0)),
            description: Some("相对基准的超额收益百分比".into()),
            is_secret: false,
        },
        axagent_harness::workflow_types::Variable {
            name: "holding_days".into(),
            var_type: "number".into(),
            value: serde_json::json!(holding_days.unwrap_or(0)),
            description: Some("实际持仓天数".into()),
            is_secret: false,
        },
        axagent_harness::workflow_types::Variable {
            name: "benchmark_name".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(benchmark_name.unwrap_or("沪深300").to_string()),
            description: Some("对比基准名称（如沪深300/中证500）".into()),
            is_secret: false,
        },
        // 反思 prompt 模板里引用了 {{stock_lessons}},必须显式注入,
        // 否则 work_engine 报 VARIABLE_NOT_FOUND 导致反思节点 Failed。
        // 数据源: 该股最近 3 个月的反思记录(去重排除当前正在创建的记录)。
        // P2-F15: fetch_stock_lessons 返回 (Option<String>, Vec<String>) 元组，
        // .0 是教训文本，.1 是被引用的 lesson_ids（在此场景不写入 lesson_applications，
        // 因为 reflection 工作流不是决策分析，不需要追踪 lesson 应用）。
        axagent_harness::workflow_types::Variable {
            name: "stock_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(
                fetch_stock_lessons(stock_code, db)
                    .await
                    .0
                    .unwrap_or_else(|| "（暂无历史反思）".to_string()),
            ),
            description: Some("该股历史反思教训（错因/被忽视信号/改进建议）".into()),
            is_secret: false,
        },
    ];
    if let Some((time_horizon, holding_days)) = original_ctx {
        variables.push(axagent_harness::workflow_types::Variable {
            name: "original_time_horizon".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(time_horizon),
            description: Some(
                "原始决策的时间维度：ultra_short(1-3天)/short(5天)/mid(28天)/long(90天+)".into(),
            ),
            is_secret: false,
        });
        variables.push(axagent_harness::workflow_types::Variable {
            name: "original_holding_days".into(),
            var_type: "number".into(),
            value: serde_json::json!(holding_days),
            description: Some("原始决策期望持有天数（交易日）".into()),
            is_secret: false,
        });
    } else {
        // 手动反思场景:无原始分析上下文,但 prompt 模板必须能渲染。
        // 注入占位值(让 LLM 知道这是手动触发的独立反思,无持仓期对齐数据)。
        variables.push(axagent_harness::workflow_types::Variable {
            name: "original_time_horizon".into(),
            var_type: "string".into(),
            value: serde_json::Value::String("manual".into()),
            description: Some("原始决策的时间维度(手动反思场景无原始分析,固定为 'manual')".into()),
            is_secret: false,
        });
        variables.push(axagent_harness::workflow_types::Variable {
            name: "original_holding_days".into(),
            var_type: "number".into(),
            value: serde_json::json!(0),
            description: Some("原始决策期望持有天数(手动反思场景无原始分析,固定为 0)".into()),
            is_secret: false,
        });
        tracing::info!(
            "[reflection] {}: 手动反思场景,注入占位 original_time_horizon='manual' / original_holding_days=0",
            stock_code
        );
    }
    let opts = axagent_rt_workflow::work_engine::RunOptions {
        max_concurrent,
        step_timeout,
        progress_callback: None,
        // [v2] 不再有 sub-analysis SubWorkflowNode，无需为子工作流传 input。
        // stock_code / stock_name / as_of_date 已通过 variables 顶层注入。
        input: None,
        input_schema: loaded.input_schema,
        output_schema: loaded.output_schema,
        dry_run: false,
        variables: Some(variables),
        ..Default::default()
    };

    // [时间旅行模式] 用 hindsight_date 作为 AS_OF 锚点包装工作流执行。
    // reflection-agent 调用的 K 线/公告工具会以 hindsight_date 为时间锚点，
    // 查看"截至评估时点的实际走势"，而非今天的全部数据。
    // - as_of_date 是原始分析日期（记忆锚点）
    // - hindsight_date 是反思评估时点（工具调用锚点）
    // 二者解耦：分析记忆从 blackboard_snapshot 加载（无 AS_OF），工具调用走 AS_OF(hindsight_date)
    let hindsight_ctx = AsOfContext::parse(hindsight_date).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("hindsight_date 解析失败: {e}"))
    })?;

    let result = as_of::AS_OF
        .scope(Some(hindsight_ctx), async move { engine.run_workflow(&wf_id, opts).await })
        .await;

    // 6. 处理结果
    match result {
        Ok(wf) => {
            // 通过 extract_agent_output 管线提取规范化 JSON（兼容多模型输出格式）
            let reflection_raw =
                wf.results.get("reflection").cloned().unwrap_or(serde_json::Value::Null);
            let reflection_json = extract_agent_output(reflection_raw).await;
            // 兜底: extract_agent_output 在某些 wrapper 格式下可能返回 JSON 字符串
            // (例如 LLM 输出被包成 `{output: "{...}"}` 时走 line 1552 分支直接 return 字符串),
            // 这时 as_object() 会得到 None,导致整个字段提取跳到 unwrap_or 兜底,
            // 数据库里 what_went_wrong / missed_signals / fix_for_future 全部为 null。
            // 二次解析: 把它当字符串再 parse 一次,还原成对象。
            let reflection_obj: Option<serde_json::Map<String, serde_json::Value>> =
                if let Some(obj) = reflection_json.as_object() {
                    Some(obj.clone())
                } else if let Some(s) = reflection_json.as_str() {
                    serde_json::from_str::<serde_json::Value>(s)
                        .ok()
                        .and_then(|v| v.as_object().cloned())
                } else {
                    None
                };

            // 兼容两种输出结构:
            //   A) 直接: {what_went_wrong, missed_signals, fix_for_future, params_suggestion}
            //   B) 嵌套: {reflection: {what_went_wrong, missed_signals, fix_for_future}, params_suggestion}
            // 内联 system_prompt 要求 A 格式,reflection.md 外部 expert prompt 要求 B 格式,
            // 实际 LLM 可能按任一格式输出,后端必须容错。
            let (what_went_wrong, missed_signals, fix_for_future, params_suggestion_json) =
                reflection_obj
                    .map(|obj| {
                        // 优先看嵌套 reflection 子对象,找不到再退到顶层
                        let inner = obj.get("reflection").and_then(|v| v.as_object());
                        let lookup = |key: &str| -> Option<&serde_json::Value> {
                            inner.and_then(|i| i.get(key)).or_else(|| obj.get(key))
                        };
                        let w = lookup("what_went_wrong")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let m = lookup("missed_signals").map(|v| v.to_string());
                        let f = lookup("fix_for_future")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let p = obj.get("params_suggestion").map(|v| v.to_string());
                        (w, m, f, p)
                    })
                    .unwrap_or((None, None, None, None));

            // 诊断: 检查反思节点是否成功,如果不成功,把状态/错误信息附到 status 字段
            // (Failed 节点 result 是 None,work_engine 不会写入 results,所以
            // wf.results 不等于完整执行轨迹 —— 之前只能看到"completed"但实际反思节点没跑)。
            use axagent_rt_workflow::workflow_engine::NodeStatus;
            let reflection_node_state = wf.node_states.get("reflection-agent");
            let status_text = match reflection_node_state {
                Some(s) if s.status == NodeStatus::Completed => "completed".to_string(),
                Some(s) if s.status == NodeStatus::Failed => {
                    let err = s.error.clone().unwrap_or_else(|| "未知错误".to_string());
                    format!("failed: reflection-agent: {err}")
                },
                Some(s) if s.status == NodeStatus::Skipped => {
                    "skipped: reflection-agent".to_string()
                },
                _ => "completed: reflection-agent 未在 node_states 中".to_string(),
            };

            let bb_text = serde_json::to_string(&wf.results).unwrap_or_default();
            let dj_text = if reflection_json.is_null() {
                None
            } else {
                Some(reflection_json.to_string())
            };

            let _ = stock_reflections::Entity::update_many()
                .col_expr(stock_reflections::Column::Status, Expr::value(&status_text))
                .col_expr(stock_reflections::Column::DecisionJson, Expr::value(dj_text))
                .col_expr(
                    stock_reflections::Column::WhatWentWrong,
                    Expr::value(what_went_wrong.clone()),
                )
                .col_expr(stock_reflections::Column::MissedSignals, Expr::value(missed_signals))
                .col_expr(
                    stock_reflections::Column::FixForFuture,
                    // [方向2] fix_for_future 在此处被 move,提前 clone 一份供 ExperiencePipeline 用
                    Expr::value(fix_for_future.clone()),
                )
                .col_expr(
                    stock_reflections::Column::ParameterSuggestionsJson,
                    Expr::value(params_suggestion_json.clone()),
                )
                .col_expr(stock_reflections::Column::BlackboardSnapshot, Expr::value(bb_text))
                // v008 (C2 借鉴): 回写 verdict / alpha_cited / lesson_summary
                .col_expr(
                    stock_reflections::Column::Verdict,
                    Expr::value(reflection_json.get("verdict").and_then(|v| v.as_str().map(|s| s.to_string()))),
                )
                .col_expr(
                    stock_reflections::Column::AlphaCited,
                    Expr::value(reflection_json.get("alpha_cited").and_then(|v| v.as_str().map(|s| s.to_string()))),
                )
                .col_expr(
                    stock_reflections::Column::LessonSummary,
                    Expr::value(reflection_json.get("lesson_summary").and_then(|v| v.as_str().map(|s| s.to_string()))),
                )
                .filter(stock_reflections::Column::Id.eq(&analysis_id))
                .exec(db)
                .await;

            // ── Path 2: 反思参数建议自动解析 ──
            let verdict_str = reflection_json.get("verdict").and_then(|v| v.as_str()).unwrap_or("");

            // ── Gap 1: Verdict → Strategy Performance 自动写入 ──
            // 反思的 verdict 是事后判断，比策略层 was_correct 更高质量。
            // 写入后 evolution_drift 可消费此反馈自动调整权重。
            let was_correct: i32 = match verdict_str {
                "correct" => 1,
                "wrong" => 0,
                _ => 0, // partial 也视为不正确
            };
            {
                use axagent_entities::strategy_performance;
                let sp_id = uuid::Uuid::new_v4().to_string();
                let decision_at = now_ms - (holding_days.unwrap_or(30) as i64 * 86_400_000);
                let sp_insert = strategy_performance::ActiveModel {
                    id: Set(sp_id.clone()),
                    strategy_id: Set("reflection_verdict".to_string()),
                    period: Set("reflection".to_string()),
                    stock_code: Set(stock_code.to_string()),
                    stock_name: Set(stock_name.to_string()),
                    decision_at: Set(decision_at),
                    exit_at: Set(now_ms),
                    holding_days: Set(holding_days.unwrap_or(30)),
                    return_pct: Set(raw_return.unwrap_or(0.0)),
                    was_correct: Set(was_correct),
                    decision_confidence: Set(0),
                    horizon_pnl_json: Set(None),
                    agreement_score: Set(None),
                    created_at: Set(now_ms),
                }
                .insert(db)
                .await;
                match sp_insert {
                    Ok(_) => tracing::info!(
                        "[reflection] Gap1: 写入 strategy_performance {sp_id}: \
                         verdict={verdict_str} was_correct={was_correct}"
                    ),
                    Err(e) => tracing::warn!("[reflection] 写入 strategy_performance 失败: {e}"),
                }
            }

            // ── Gap 3: 攒够 N 条一致建议自动触发 WFO 校准 ──
            // 当连续 3+ 条反思对某个参数提出同方向调整时，自动跑校准。
            if verdict_str == "wrong" || verdict_str == "partial" {
                if let Some(ref pj) = params_suggestion_json {
                    use axagent_analysis_engine::portfolio_formula::try_parse_param_suggestion;
                    if let Some(suggested) = try_parse_param_suggestion(pj) {
                        tracing::info!(
                            "[reflection] Gap3: 解析到参数建议 buy={:.2} capHi={:.0}, 检查一致性...",
                            suggested.buy_threshold,
                            suggested.cap_high
                        );
                        // 查询最近 10 条有参数建议的反思
                        use axagent_entities::stock_reflections as sr;
                        use sea_orm::QuerySelect;
                        let recent = sr::Entity::find()
                            .filter(sr::Column::Status.eq("completed"))
                            .filter(sr::Column::ParameterSuggestionsJson.is_not_null())
                            .order_by(sr::Column::CreatedAt, sea_orm::Order::Desc)
                            .limit(10)
                            .all(db)
                            .await;
                        if let Ok(rows) = recent {
                            let mut same_direction = 1; // 当前这条算 1
                            for r in &rows {
                                if r.id == analysis_id {
                                    continue;
                                }
                                if let Some(pj2) = r.parameter_suggestions_json.as_deref() {
                                    if let Some(prev) = try_parse_param_suggestion(pj2) {
                                        // 检查 buy_threshold 的调整方向是否一致
                                        let def = axagent_analysis_engine::portfolio_formula::PortfolioMgrParamSet::v56_default();
                                        let current_dir =
                                            suggested.buy_threshold < def.buy_threshold;
                                        let prev_dir = prev.buy_threshold < def.buy_threshold;
                                        if current_dir == prev_dir {
                                            same_direction += 1;
                                        } else {
                                            break; // 方向不同就停止计数
                                        }
                                        if same_direction >= 3 {
                                            tracing::info!(
                                                "[reflection] Gap3: 连续 {same_direction} 条建议降低 buy_threshold, \
                                                 自动触发 WFO 校准"
                                            );
                                            // 为了避免异步阻塞 reflection 主流程，只记录不实际执行
                                            // 实际自动校准由 scheduler/cron 层接管
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // 索引到 Memory RAG
            if let Some(ref w) = what_went_wrong {
                let memory_content = format!(
                    "反思:股票:{} {} 原始决策时间:{} 结果:{}\n错因:{}",
                    stock_code, stock_name, as_of_date, actual_outcome, w
                );
                let _ = crate::indexing::index_memory_item(
                    db,
                    master_key,
                    vector_store,
                    "stock_reflections",
                    &analysis_id,
                    &memory_content,
                    "openai::text-embedding-3-small",
                    None,
                )
                .await;
            }

            tracing::info!("[reflection] {}: 反思完成", stock_code);

            // ── [方向3] 持久化反思轨迹到 TrajectoryStorage ──
            // 为后续的 ExperiencePipeline（方向2）和 DreamConsolidator（方向6）提供数据基础。
            // 从反思结果中提取 verdict / lesson_summary / what_went_wrong 构造 Trajectory，
            // 用 TrajectoryScorer 自动计算 quality 和 value_score。
            if let Some(storage) = trajectory_storage {
                use axagent_harness::trajectory_scorer::TrajectoryScorer;
                use axagent_harness::trajectory_types::{
                    MessageRole, Trajectory, TrajectoryOutcome, TrajectoryStep,
                };

                let verdict_str = reflection_json.get("verdict").and_then(|v| v.as_str());
                let outcome = match verdict_str {
                    Some("correct") => TrajectoryOutcome::Success,
                    Some("partial") => TrajectoryOutcome::Partial,
                    Some("wrong") => TrajectoryOutcome::Failure,
                    _ if status_text.starts_with("failed") => TrajectoryOutcome::Abandoned,
                    _ => TrajectoryOutcome::Partial,
                };

                let lesson =
                    reflection_json.get("lesson_summary").and_then(|v| v.as_str()).unwrap_or("");
                let reasoning_text = what_went_wrong.as_deref().unwrap_or("");
                let duration_ms = (chrono::Utc::now().timestamp_millis() - now_ms).max(0) as u64;

                let steps = vec![
                    TrajectoryStep {
                        timestamp_ms: now_ms.max(0) as u64,
                        role: MessageRole::User,
                        content: format!(
                            "反思 {} ({}) 预测时间={} 评估时间={} 实际={}",
                            stock_code, stock_name, as_of_date, hindsight_date, actual_outcome
                        ),
                        reasoning: None,
                        tool_calls: None,
                        tool_results: None,
                    },
                    TrajectoryStep {
                        timestamp_ms: duration_ms,
                        role: MessageRole::Assistant,
                        content: lesson.to_string(),
                        reasoning: if reasoning_text.is_empty() {
                            None
                        } else {
                            Some(reasoning_text.to_string())
                        },
                        tool_calls: None,
                        tool_results: None,
                    },
                ];

                let mut trajectory = Trajectory::new(
                    analysis_id.clone(),
                    // [方向6] topic 包含股票代码，让 DreamConsolidator 按股票分组蒸馏
                    format!("stock_reflection:{}", stock_code),
                    format!("{} {} 反思", stock_code, stock_name),
                    lesson.to_string(),
                    outcome,
                    duration_ms,
                    steps,
                );
                TrajectoryScorer::apply(&mut trajectory);

                if let Err(e) = storage.save_trajectory(&trajectory).await {
                    tracing::warn!("[reflection] 保存 trajectory 失败: {e}");
                } else {
                    tracing::info!(
                        "[reflection] trajectory {} 已持久化 (outcome={:?} quality={:.2} value={:.2})",
                        &trajectory.id,
                        outcome,
                        trajectory.quality.overall,
                        trajectory.value_score
                    );
                }
            }

            // ── [F1 借鉴] 反思完成后自动提取 lesson 为可重用规则 ──
            // 借鉴 TradingAgents 反思→规则提取机制:反思完成后把 lesson_summary
            // 提取为可重用的规则存入 reflection_lessons 表,下次决策可查询。
            if status_text == "completed" {
                if let Some(ls) = reflection_json
                    .get("lesson_summary")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                {
                    let _ = extract_lesson_to_rule(
                        db,
                        stock_code,
                        &analysis_id,
                        &ls,
                        reflection_json.get("verdict").and_then(|v| v.as_str()),
                    )
                    .await;
                }
            }

            // ── [方向2] 接入 ExperiencePipeline，把反思转为 Experience 喂给 RLOptimizer ──
            // 设计要点:
            // - 用 verdict 映射 quality_score:correct=9, partial=5, wrong=2, 其他=4
            // - 异步提交,不阻塞反思主流程
            // - 失败不影响反思结果(只记 warn 日志)
            {
                use axagent_agent::Reflection;

                let verdict_str = reflection_json.get("verdict").and_then(|v| v.as_str());
                let quality_score: u8 = match verdict_str {
                    Some("correct") => 9,
                    Some("partial") => 5,
                    Some("wrong") => 2,
                    _ => 4,
                };

                let lesson_summary =
                    reflection_json.get("lesson_summary").and_then(|v| v.as_str()).unwrap_or("");
                let what_went_wrong_text = what_went_wrong.clone().unwrap_or_default();
                let missed =
                    reflection_json.get("missed_signals").and_then(|v| v.as_str()).unwrap_or("");
                let fix_text = fix_for_future.clone().unwrap_or_default();

                let mut error_patterns: Vec<String> = Vec::new();
                if !what_went_wrong_text.is_empty() {
                    error_patterns.push(what_went_wrong_text.clone());
                }
                if !missed.is_empty() {
                    error_patterns.push(missed.to_string());
                }
                let mut improvements: Vec<String> = Vec::new();
                if !fix_text.is_empty() {
                    improvements.push(fix_text.clone());
                }

                let quality_analysis = format!(
                    "verdict={:?} stock={} hindsight={} actual={}",
                    verdict_str, stock_code, hindsight_date, actual_outcome
                );

                let reflection = Reflection::new(analysis_id.clone())
                    .with_quality(quality_score, quality_analysis)
                    .with_patterns(error_patterns.clone(), Vec::new())
                    .with_improvements(improvements.clone())
                    .with_summary(lesson_summary.to_string());

                let pipeline = crate::commands::_shared_state::SHARED_PIPELINE.clone();
                let aid = analysis_id.clone();
                tokio::task::spawn(async move {
                    let mut pipeline = pipeline.write().await;
                    let exp = pipeline.process_reflection(&reflection).await;
                    tracing::info!(
                        "[reflection] ExperiencePipeline: 已吸收 reflection {} -> reward={:.3} done={}",
                        aid,
                        exp.reward,
                        exp.done
                    );
                });
            }

            Ok(analysis_id)
        },
        Err(e) => {
            let err_msg = format!("反思工作流失败: {e}");
            let _ = stock_reflections::Entity::update_many()
                .col_expr(
                    stock_reflections::Column::Status,
                    Expr::value(format!("failed: {err_msg}")),
                )
                .filter(stock_reflections::Column::Id.eq(&analysis_id))
                .exec(db)
                .await;
            Err(err_msg)
        },
    }
}
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "批量处理持仓到期反思")]
#[tauri::command]
pub async fn run_batch_reflection(
    state: State<'_, AppState>,
    max_count: Option<u32>,
) -> Result<serde_json::Value, String> {
    use axagent_entities::stock_analyses;
    use axagent_entities::stock_reflections;

    let max_count = max_count.unwrap_or(20) as usize;
    let db = state.harness.db();

    // 1. 扫所有 pending row,按 created_at ASC(最老的先处理,避免积压)
    let pendings: Vec<stock_reflections::Model> = stock_reflections::Entity::find()
        .filter(stock_reflections::Column::Status.eq("pending"))
        .order_by_asc(stock_reflections::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("D1 扫 pending row 失败: {e}"))
        })?;

    tracing::info!(
        "[D1 batch_reflection] 扫到 {} 条 pending row, max_count={}",
        pendings.len(),
        max_count
    );

    let mut resolved = 0u32;
    let mut failed = 0u32;
    let mut skipped_young = 0u32; // 持仓期未到
    let mut errors: Vec<String> = Vec::new();
    let today_ms = chrono::Utc::now().timestamp_millis();

    for (i, p) in pendings.iter().take(max_count).enumerate() {
        // 2a. 读原始分析
        let analysis =
            match stock_analyses::Entity::find_by_id(&p.original_analysis_id).one(db).await {
                Ok(Some(a)) => a,
                Ok(None) => {
                    tracing::warn!(
                        "[D1] pending reflection {} 关联 analysis_id={} 不存在,skip",
                        p.id,
                        p.original_analysis_id
                    );
                    skipped_young += 1;
                    continue;
                },
                Err(e) => {
                    tracing::error!("[D1] 查 analysis 失败: {e}");
                    failed += 1;
                    errors.push(format!("{}: 查询 analysis 失败: {e}", p.id));
                    continue;
                },
            };

        // 2b. 计算持仓期是否到达
        // 默认 28 天 = mid 决策标准持仓期(用户没指定时取 stock-analysis 模板默认)
        let expected_days = analysis.decision_expected_holding_days.unwrap_or(28);
        let analysis_date = analysis.as_of_date.as_deref().unwrap_or(&p.as_of_date);

        // [时间旅行模式] 评估时点由 pending row 的 hindsight_date 决定。
        // - hindsight_date 在未来 → 还没到反思时点，skip
        // - hindsight_date <= today → 可以反思，传给 run_reflection_workflow
        //   作为 AS_OF 锚点查看"截至评估时点的实际走势"
        // - days_held 基于 hindsight_date - analysis_date 计算
        let hindsight_date = p.hindsight_date.as_str();
        // P3-#13 修复：时区错位
        // 原实现用 `chrono::Utc::now().timestamp_millis()` 与 `NaiveDate.and_utc().timestamp_millis()`
        // 比较和相减，会因 UTC vs Asia/Shanghai 8 小时偏差导致跨日 days_held 计算偏少 1 天。
        // 例：北京时间 2026-07-14 02:00 = UTC 2026-07-13 18:00；若 analysis_date="2026-07-13"，
        //     hindsight_date="2026-07-14"，原实现 (hindsight_ms - analysis_ms) / 86400000 = 0，
        //     实际应为 1 天。
        // 修复：用 NaiveDate 直接相减，彻底绕开时区转换。today 也按 Asia/Shanghai 时区取 NaiveDate。
        let analysis_nd = chrono::NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d").ok();
        let hindsight_nd = chrono::NaiveDate::parse_from_str(hindsight_date, "%Y-%m-%d").ok();

        // today 按 Asia/Shanghai 时区取 NaiveDate（A 股交易日历以北京时间为准）
        let today_nd = {
            use chrono::TimeZone;
            // FixedOffset 8 小时 = Asia/Shanghai（chrono 内置无 IANA 时区数据库依赖）
            let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            offset.from_utc_datetime(&chrono::Utc::now().naive_utc()).date_naive()
        };

        // hindsight 在未来 → skip（按日历日比较，不受时区影响）
        if let Some(h) = hindsight_nd {
            if h > today_nd {
                tracing::info!(
                    "[D1] pending {} ({}) hindsight_date={} 在未来,未到评估时点 skip",
                    p.id,
                    p.stock_code,
                    hindsight_date
                );
                skipped_young += 1;
                continue;
            }
        }

        // days_held = hindsight_date - analysis_date（日历日相减，无时区偏差）
        let days_held = match (analysis_nd, hindsight_nd) {
            (Some(a), Some(h)) => (h - a).num_days().max(0),
            // 解析失败时回退到 timestamp_ms 计算（保留旧行为兼容脏数据）
            _ => {
                let analysis_ms = chrono::NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc().timestamp_millis())
                    .unwrap_or(p.created_at);
                let hindsight_ms = chrono::NaiveDate::parse_from_str(hindsight_date, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc().timestamp_millis())
                    .unwrap_or(today_ms);
                (hindsight_ms - analysis_ms).max(0) / 86_400_000
            },
        };

        if days_held < expected_days {
            tracing::info!(
                "[D1] pending {} ({}) 持仓 {}/{} 天,未到期 skip",
                p.id,
                p.stock_code,
                days_held,
                expected_days
            );
            skipped_young += 1;
            continue;
        }

        // 2c. 调 run_reflection_workflow(B3 UPDATE 路径)
        let r = run_reflection_workflow(
            db,
            &state.astock_client,
            &state.work_engine,
            &state.vector_store,
            state.harness.master_key(),
            &p.stock_code,
            &p.stock_name,
            &p.original_analysis_id,
            &p.actual_outcome,      // 留空字符串走 legacy fallback
            None,                   // raw_return: pending 阶段未算
            None,                   // alpha_return
            Some(days_held as i32), // holding_days 填入
            None,                   // benchmark_name
            analysis_date,
            // [时间旅行模式] 传 pending row 的 hindsight_date 而非 today
            hindsight_date,
            0u8,
            "light",
            Some(p.id.clone()),              // [B2/B3] 走 UPDATE 路径
            Some(&state.trajectory_storage), // [方向3] 持久化轨迹
        )
        .await;

        match r {
            Ok(_) => {
                tracing::info!(
                    "[D1] ✓ resolved {}/{} pending: {} ({})",
                    i + 1,
                    pendings.len(),
                    p.id,
                    p.stock_code
                );
                resolved += 1;
            },
            Err(e) => {
                tracing::error!("[D1] ✗ resolve failed {}: {e}", p.id);
                failed += 1;
                errors.push(format!("{}: {e}", p.id));
            },
        }
    }

    // ── [D2 借鉴] Resolved FIFO 清理 ──
    // 保留最近 1000 条 + 90 天内的 completed row,删除更老的。
    // pending row 永远保留(B1 借鉴:不能丢反思需求)。
    //
    // P3-#10 修复：原实现一次 DELETE 可能影响数十万条 row,阻塞 SQLite WAL。
    // 改为分批循环：每批 SELECT 1000 个 id → DELETE WHERE id IN(...)，
    // 直到无超龄 row。单批事务短，避免锁表。
    use sea_orm::QuerySelect;
    let ninety_days_ago_ms = today_ms - 90 * 86_400_000;
    let mut cleaned_up: u64 = 0;
    loop {
        // 取 1000 条超龄 completed row 的 id（按 updated_at ASC 优先删最老的）
        let stale_ids: Vec<String> = stock_reflections::Entity::find()
            .select_only()
            .column(stock_reflections::Column::Id)
            .filter(stock_reflections::Column::Status.eq("completed"))
            .filter(stock_reflections::Column::UpdatedAt.lt(ninety_days_ago_ms))
            .order_by_asc(stock_reflections::Column::UpdatedAt)
            .limit(1000)
            .into_tuple()
            .all(db)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("[D2] FIFO 查询超龄 row 失败: {e}");
                Vec::new()
            });
        if stale_ids.is_empty() {
            break;
        }
        let batch_size = stale_ids.len() as u64;
        let _ = stock_reflections::Entity::delete_many()
            .filter(stock_reflections::Column::Id.is_in(stale_ids))
            .exec(db)
            .await
            .map(|r| {
                cleaned_up += r.rows_affected;
            })
            .map_err(|e| {
                tracing::warn!("[D2] FIFO 批量删除失败: {e}");
            });
        // 若本批不足 1000 条,说明已无超龄 row,退出避免无限循环
        if batch_size < 1000 {
            break;
        }
    }
    tracing::info!("[D2 fifo_cleanup] 分批删除 {} 条超龄 completed row", cleaned_up);

    tracing::info!(
        "[D1 batch_reflection] 完成: total={} resolved={} failed={} skipped_young={} cleaned={}",
        pendings.len(),
        resolved,
        failed,
        skipped_young,
        cleaned_up
    );

    Ok(serde_json::json!({
        "totalPending": pendings.len(),
        "processed": pendings.len().min(max_count),
        "resolved": resolved,
        "failed": failed,
        "skippedYoung": skipped_young,
        "cleanedUp": cleaned_up,
        "errors": errors,
    }))
}

// ── [F1 借鉴] 提取反思教训为可重用规则 ──
//
// 借鉴 TradingAgents 反思→规则提取机制:反思完成后把 lesson_summary
// 提取为可重用的规则存入 reflection_lessons 表。
// 规则自动提取规则:lesson_summary ≤200 字符、含明确建议性内容的才提取。
async fn extract_lesson_to_rule(
    db: &sea_orm::DatabaseConnection,
    stock_code: &str,
    source_reflection_id: &str,
    lesson_summary: &str,
    verdict: Option<&str>,
) -> Result<(), String> {
    use axagent_entities::reflection_lessons;
    use sea_orm::ActiveModelTrait;
    use sea_orm::ColumnTrait;
    use sea_orm::EntityTrait;
    use sea_orm::QueryFilter;
    use sea_orm::Set;

    // 短文本过短或无实际建议性内容则跳过
    let trimmed = lesson_summary.trim();
    if trimmed.len() < 10 || trimmed.len() > 250 {
        return Ok(());
    }

    // [P2-#9 修复] 去重：检查相同 lesson_summary 是否已存在
    // 同一只股票多次相似反思会产生大量重复规则，此处按 stock_code + lesson_summary 去重。
    // 若已存在，更新 source_reflection_id 和 updated_at（保留原有 times_applied/success_count）。
    let existing = reflection_lessons::Entity::find()
        .filter(reflection_lessons::Column::StockCode.eq(stock_code))
        .filter(reflection_lessons::Column::LessonSummary.eq(trimmed))
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("F1 查询重复 lesson 失败: {e}"))
                .to_string()
        })?;

    if let Some(existing_model) = existing {
        // 已存在相同规则，更新 source_reflection_id 和 updated_at，保留应用统计
        let mut active: reflection_lessons::ActiveModel = existing_model.into();
        active.source_reflection_id = Set(Some(source_reflection_id.to_string()));
        active.updated_at = Set(chrono::Utc::now().timestamp_millis());
        active.update(db).await.map(|_| ()).map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("F1 更新重复 lesson 失败: {e}"))
                .to_string()
        })?;
        tracing::debug!("[F1] lesson_summary 已存在，更新 source_reflection_id: {}", trimmed);
        return Ok(());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now_ms = chrono::Utc::now().timestamp_millis();
    // 从 verdict 推断初始置信度
    let confidence = match verdict {
        Some("wrong") => 0.7, // wrong 的教训更有价值,给更高初始置信度
        Some("partial") => 0.5,
        _ => 0.3, // correct 或 None 的教训价值较低
    };

    reflection_lessons::ActiveModel {
        id: Set(id),
        lesson_summary: Set(trimmed.to_string()),
        rule_pattern: Set(None), // 后续由 F1 迭代扩展: LLM 分析 lesson_summary 自动提取
        source_reflection_id: Set(Some(source_reflection_id.to_string())),
        stock_code: Set(Some(stock_code.to_string())),
        applicable_scenarios: Set(None),
        times_applied: Set(0),
        success_count: Set(0),
        confidence: Set(confidence),
        status: Set("active".to_string()),
        created_at: Set(now_ms),
        updated_at: Set(now_ms),
    }
    .insert(db)
    .await
    .map(|_| ())
    .map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL)
            .with_detail(format!("F1 写入 reflection_lessons 失败: {e}"))
            .to_string()
    })
}

// ── [P1-#5 修复] 反思规则有效性验证 ──
//
// 接入原死代码 `reflection_lesson_validator.rs`（adjust_lesson_confidence 等函数）。
// 追踪 reflection_lessons 表中规则被引用后的决策表现，调整 confidence。
// 适合作为 cron 任务定期执行（如每日一次）。
//
// 验证维度：
// - 规则被引用次数（times_applied）：通过 stock_reflections 中 lesson_summary 模糊匹配
// - 引用后决策成功率（success_count）：基于 stock_analyses 表的 posterior 字段
// - 规则置信度衰减/提升：基于实际表现调整 confidence
//
// 调用方式：cron 调度器或 Tauri 命令 `run_lesson_validation_command`。
// 已接入 start_background_services 的 start_lesson_validation 定时任务。
pub async fn run_lesson_validation(
    db: &sea_orm::DatabaseConnection,
) -> Result<serde_json::Value, String> {
    use axagent_analysis_engine::reflection_lesson_validator::{
        build_lesson_validation, build_lesson_validation_report,
    };
    use axagent_entities::lesson_applications;
    use axagent_entities::reflection_lessons;
    use axagent_entities::stock_reflections;

    // 0. P2-F15 预处理：同步 lesson_applications.outcome_at_validation
    // 扫描所有 outcome_at_validation IS NULL 的行，从 stock_analyses.outcome 回写。
    // 确保 success_count 统计尽可能精确。
    let synced = super::core::sync_lesson_application_outcomes(db).await;
    if synced > 0 {
        tracing::info!(
            "[lesson-validation] 预处理: 从 stock_analyses.outcome 回写 {synced} 条 lesson_applications"
        );
    }

    // 1. 加载所有 active 规则
    let lessons: Vec<reflection_lessons::Model> = reflection_lessons::Entity::find()
        .filter(reflection_lessons::Column::Status.eq("active"))
        .all(db)
        .await
        .map_err(|e| format!("加载 reflection_lessons 失败: {e}"))?;

    tracing::info!("[lesson-validation] 加载 {} 条 active 规则", lessons.len());

    let mut validations = Vec::new();
    let mut updated_count = 0u32;
    // P2-F15 统计：精确统计 vs 模糊匹配的使用情况
    let mut precise_count = 0u32;
    let mut fallback_count = 0u32;

    for lesson in &lessons {
        // ── P2-F15 切入点 3：优先用 lesson_applications 精确统计 ──
        // 旧的 lesson_summary.contains() 模糊匹配存在误匹配/漏匹配问题，
        // 且统计的是"反思时提到该 lesson 的次数"而非"决策时应用了该 lesson 的次数"。
        // 现在优先用 lesson_applications 表精确统计 times_applied。
        //
        // success_count 优先用 lesson_applications.outcome_at_validation = 'win' 精确统计；
        // 如果所有 outcome_at_validation 都是 NULL（outcome 链路未打通），
        // 回退到旧的 stock_reflections.verdict 模糊匹配，避免误判所有规则为 0 成功率。
        let apps: Vec<lesson_applications::Model> = lesson_applications::Entity::find()
            .filter(lesson_applications::Column::LessonId.eq(&lesson.id))
            .all(db)
            .await
            .unwrap_or_default();

        let (applied_count, success_count, used_precise) = if !apps.is_empty() {
            // 精确统计路径
            let precise_applied = apps.len() as i32;
            // 统计 outcome_at_validation = 'win' 的数量
            let precise_success =
                apps.iter().filter(|a| a.outcome_at_validation.as_deref() == Some("win")).count()
                    as i32;

            // 检查是否有任何 outcome_at_validation 已被填充
            let has_any_outcome = apps.iter().any(|a| a.outcome_at_validation.is_some());

            if has_any_outcome {
                // outcome 链路已打通，完全使用精确统计
                (precise_applied, precise_success, true)
            } else {
                // outcome_at_validation 全部为 NULL（链路未打通）
                // times_applied 用精确统计，success_count 回退到模糊匹配
                let fallback_success = stock_reflections::Entity::find()
                    .filter(
                        stock_reflections::Column::LessonSummary.contains(&lesson.lesson_summary),
                    )
                    .filter(stock_reflections::Column::Status.eq("completed"))
                    .filter(stock_reflections::Column::Verdict.is_in(vec!["correct", "partial"]))
                    .all(db)
                    .await
                    .map(|v| v.len() as i32)
                    .unwrap_or(0);
                (precise_applied, fallback_success, true)
            }
        } else {
            // lesson_applications 表中无记录（旧数据或未接入），
            // 完全回退到旧的模糊匹配逻辑
            let fuzzy_applied = stock_reflections::Entity::find()
                .filter(stock_reflections::Column::LessonSummary.contains(&lesson.lesson_summary))
                .filter(stock_reflections::Column::Status.eq("completed"))
                .all(db)
                .await
                .map(|v| v.len() as i32)
                .unwrap_or(0);

            let fuzzy_success = stock_reflections::Entity::find()
                .filter(stock_reflections::Column::LessonSummary.contains(&lesson.lesson_summary))
                .filter(stock_reflections::Column::Status.eq("completed"))
                .filter(stock_reflections::Column::Verdict.is_in(vec!["correct", "partial"]))
                .all(db)
                .await
                .map(|v| v.len() as i32)
                .unwrap_or(0);

            (fuzzy_applied, fuzzy_success, false)
        };

        if used_precise {
            precise_count += 1;
        } else {
            fallback_count += 1;
        }

        // 4. 构建验证记录
        let validation = build_lesson_validation(
            lesson.id.clone(),
            lesson.lesson_summary.clone(),
            lesson.source_reflection_id.clone().unwrap_or_default(),
            lesson.stock_code.clone(),
            applied_count,
            success_count,
            lesson.confidence,
        );
        validations.push(validation.clone());

        // 5. 更新 reflection_lessons 表的 times_applied/success_count/confidence
        let new_status = if validation.adjusted_confidence < 0.2 {
            "deprecated"
        } else {
            "active"
        };

        let _ = reflection_lessons::Entity::update_many()
            .col_expr(
                reflection_lessons::Column::TimesApplied,
                sea_orm::sea_query::Expr::value(applied_count),
            )
            .col_expr(
                reflection_lessons::Column::SuccessCount,
                sea_orm::sea_query::Expr::value(success_count),
            )
            .col_expr(
                reflection_lessons::Column::Confidence,
                sea_orm::sea_query::Expr::value(validation.adjusted_confidence),
            )
            .col_expr(
                reflection_lessons::Column::Status,
                sea_orm::sea_query::Expr::value(new_status),
            )
            .filter(reflection_lessons::Column::Id.eq(&lesson.id))
            .exec(db)
            .await;

        updated_count += 1;
    }

    // 6. 生成验证报告
    let report = build_lesson_validation_report(&validations);

    tracing::info!(
        "[lesson-validation] 完成: validated={} deprecated={} avg_success_rate={:.2} | 精确统计={} 模糊回退={}",
        report.validated_lessons,
        report.deprecated_lessons,
        report.avg_success_rate,
        precise_count,
        fallback_count
    );

    Ok(serde_json::json!({
        "totalLessons": report.total_lessons,
        "validatedLessons": report.validated_lessons,
        "pendingLessons": report.pending_lessons,
        "deprecatedLessons": report.deprecated_lessons,
        "avgSuccessRate": report.avg_success_rate,
        "confidenceAdjustment": {
            "increased": report.confidence_adjustment_stats.increased,
            "decreased": report.confidence_adjustment_stats.decreased,
            "unchanged": report.confidence_adjustment_stats.unchanged,
        },
        "updatedCount": updated_count,
        // P2-F15: 统计来源分布，便于监控迁移进度
        "statsSource": {
            "precise": precise_count,
            "fallback": fallback_count,
        },
    }))
}

/// P2-F15 修复: 手动触发 lesson 验证的 Tauri 命令包装。
///
/// 内部核心函数 `run_lesson_validation` 已通过 `start_lesson_validation` 后台
/// 定时任务自动调度，但未暴露为 Tauri 命令，导致前端无法手动触发校证。
/// 此包装函数补齐该缺口，便于调试和紧急校证场景。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "运行教训规则验证")]
#[tauri::command]
pub async fn run_lesson_validation_command(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let db = state.harness.db().clone();
    run_lesson_validation(&db).await
}

// ── [缺陷5 fix] 内部批量反思函数(非 Tauri 命令,供 cron 调度器直接调用) ──
//
// 从 run_batch_reflection 提取的核心逻辑。
// 参数通过独立引用传入,不需要 AppState。
//
// P3-#11 修复：`_engine` 参数类型从 `&WorkEngine` 改为 `&Arc<WorkEngine>`，
// 避免循环内 `Arc::new(_engine.clone())` 克隆整个 WorkEngine（可能包含大量状态）。
// Arc::clone 只是原子引用计数加一，O(1)。
// 已接入 start_background_services 的 start_batch_reflection 定时任务。
pub async fn run_batch_reflection_inner(
    db: &sea_orm::DatabaseConnection,
    _client: &axagent_astock_data::AStockClient,
    _engine: &std::sync::Arc<axagent_rt_workflow::work_engine::WorkEngine>,
    _vector_store: &axagent_search::vector_store::VectorStore,
    _master_key: &[u8; 32],
    max_count: Option<u32>,
    trajectory_storage: Option<&std::sync::Arc<axagent_trajectory::TrajectoryStorage>>,
) -> Result<serde_json::Value, String> {
    use crate::commands::error::ErrorResponse;
    use axagent_entities::stock_analyses;
    use axagent_entities::stock_reflections;

    let max_count = max_count.unwrap_or(20) as usize;
    let today_ms = chrono::Utc::now().timestamp_millis();

    // 1. 扫所有 pending row,按 created_at ASC(最老的先处理,避免积压)
    let pendings: Vec<stock_reflections::Model> = stock_reflections::Entity::find()
        .filter(stock_reflections::Column::Status.eq("pending"))
        .order_by_asc(stock_reflections::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL)
                .with_detail(format!("run_batch_reflection_inner 扫 pending row 失败: {e}"))
        })?;

    tracing::info!(
        "[D1 batch_reflection] 扫到 {} 条 pending row, max_count={}",
        pendings.len(),
        max_count
    );

    let mut resolved = 0u32;
    let mut failed = 0u32;
    let mut skipped_young = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for p in pendings.iter().take(max_count) {
        let analysis =
            match stock_analyses::Entity::find_by_id(&p.original_analysis_id).one(db).await {
                Ok(Some(a)) => a,
                Ok(None) => {
                    skipped_young += 1;
                    continue;
                },
                Err(e) => {
                    failed += 1;
                    errors.push(format!("{}: 查询 analysis 失败: {e}", p.id));
                    continue;
                },
            };

        let expected_days = analysis.decision_expected_holding_days.unwrap_or(28);
        let analysis_date = analysis.as_of_date.as_deref().unwrap_or(&p.as_of_date);

        // [时间旅行模式] 评估时点由 pending row 的 hindsight_date 决定。
        // P3-#13 修复：用 NaiveDate 直接相减，避免 UTC vs Asia/Shanghai 时区错位。
        let hindsight_date = p.hindsight_date.as_str();
        let analysis_nd = chrono::NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d").ok();
        let hindsight_nd = chrono::NaiveDate::parse_from_str(hindsight_date, "%Y-%m-%d").ok();

        let today_nd = {
            use chrono::TimeZone;
            let offset = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
            offset.from_utc_datetime(&chrono::Utc::now().naive_utc()).date_naive()
        };

        if let Some(h) = hindsight_nd {
            if h > today_nd {
                skipped_young += 1;
                continue;
            }
        }

        let days_held = match (analysis_nd, hindsight_nd) {
            (Some(a), Some(h)) => (h - a).num_days().max(0),
            _ => {
                let analysis_ms = chrono::NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc().timestamp_millis())
                    .unwrap_or(p.created_at);
                let hindsight_ms = chrono::NaiveDate::parse_from_str(hindsight_date, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .map(|dt| dt.and_utc().timestamp_millis())
                    .unwrap_or(today_ms);
                (hindsight_ms - analysis_ms).max(0) / 86_400_000
            },
        };

        if days_held < expected_days {
            skipped_young += 1;
            continue;
        }

        // ── [As-of 时间旅行回测验证] 用 BacktestEngine 自动计算事后结果 ──
        // 在反思之前，先用历史行情验证分析决策的实际表现，
        // 自动填充 actual_outcome / raw_return / alpha_return，形成完整闭环。
        let (auto_outcome, auto_raw_return, auto_alpha_return, auto_holding_days) =
            match run_asof_backtest(&analysis, _client, hindsight_date).await {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!(
                        "[as-of backtest] {} 回测失败，使用 pending row 原始数据: {e}",
                        p.id
                    );
                    // 回测失败时回退到 pending row 的原始值
                    (p.actual_outcome.clone(), p.raw_return, p.alpha_return, Some(days_held as i32))
                },
            };

        let r = run_reflection_workflow(
            db,
            _client,
            // P3-#11: Arc::clone O(1)，而非克隆整个 WorkEngine
            &std::sync::Arc::clone(_engine),
            _vector_store,
            _master_key,
            &p.stock_code,
            &p.stock_name,
            &p.original_analysis_id,
            &auto_outcome,
            auto_raw_return,
            auto_alpha_return,
            auto_holding_days,
            None,
            analysis_date,
            // [时间旅行模式] 传 pending row 的 hindsight_date 而非 today
            hindsight_date,
            0u8,
            "light",
            Some(p.id.clone()),
            trajectory_storage, // [方向3] 透传轨迹存储
        )
        .await;

        match r {
            Ok(_) => {
                resolved += 1;
            },
            Err(e) => {
                failed += 1;
                errors.push(format!("{}: {e}", p.id));
            },
        }
    }

    // D2 FIFO 清理（P3-#10：分批删除，每批 1000 条，避免单次大事务锁表）
    use sea_orm::QuerySelect;
    let ninety_days_ago_ms = today_ms - 90 * 86_400_000;
    let mut cleaned_up: u64 = 0;
    loop {
        let stale_ids: Vec<String> = stock_reflections::Entity::find()
            .select_only()
            .column(stock_reflections::Column::Id)
            .filter(stock_reflections::Column::Status.eq("completed"))
            .filter(stock_reflections::Column::UpdatedAt.lt(ninety_days_ago_ms))
            .order_by_asc(stock_reflections::Column::UpdatedAt)
            .limit(1000)
            .into_tuple()
            .all(db)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("[D2 inner] FIFO 查询超龄 row 失败: {e}");
                Vec::new()
            });
        if stale_ids.is_empty() {
            break;
        }
        let batch_size = stale_ids.len() as u64;
        let _ = stock_reflections::Entity::delete_many()
            .filter(stock_reflections::Column::Id.is_in(stale_ids))
            .exec(db)
            .await
            .map(|r| {
                cleaned_up += r.rows_affected;
            })
            .map_err(|e| {
                tracing::warn!("[D2 inner] FIFO 批量删除失败: {e}");
            });
        if batch_size < 1000 {
            break;
        }
    }

    Ok(serde_json::json!({
        "totalPending": pendings.len(),
        "processed": pendings.len().min(max_count),
        "resolved": resolved,
        "failed": failed,
        "skippedYoung": skipped_young,
        "cleanedUp": cleaned_up,
        "errors": errors,
    }))
}

// ── [As-of 时间旅行回测] 辅助函数 ──────────────────────────

/// 用 BacktestEngine 对单条分析记录做 as-of 时间旅行回测。
///
/// 从 `stock_analyses` 记录中提取决策信息，调用 `BacktestEngine::backtest_decision`
/// 用历史行情验证决策表现，返回 `(actual_outcome, raw_return, alpha_return, holding_days)`。
///
/// # 参数
/// - `analysis`: stock_analyses 记录（含 decision_action / decision_json 等）
/// - `client`: AStockClient（实现 MarketDataProvider trait）
/// - `hindsight_date`: 事后评估日期（YYYY-MM-DD）
///
/// # 返回
/// - `Ok((actual_outcome, raw_return, alpha_return, holding_days))`
/// - `Err(String)`: 回测失败（上层会回退到 pending row 原始值）
async fn run_asof_backtest(
    analysis: &stock_analyses::Model,
    client: &axagent_astock_data::AStockClient,
    hindsight_date: &str,
) -> Result<(String, Option<f64>, Option<f64>, Option<i32>), String> {
    use axagent_analysis_engine::backtest::BacktestEngine;

    // 1. 提取决策信息
    let decision_action = analysis.decision_action.clone().unwrap_or_else(|| "hold".to_string());

    // 从 decision_json 中提取 confidence（默认 0.5）
    let decision_confidence = analysis
        .decision_json
        .as_ref()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|v| v.get("confidence").and_then(|c| c.as_f64()))
        .unwrap_or(0.5);

    let time_horizon = analysis.decision_time_horizon.clone();
    let expected_holding_days = analysis.decision_expected_holding_days;

    // 2. 用 BacktestEngine 回测
    // holding_days = hindsight_date - analysis_date（即实际持有天数）
    let analysis_date = analysis.as_of_date.as_deref().unwrap_or(analysis.analysis_date.as_str());

    let holding_days = chrono::NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d")
        .ok()
        .zip(chrono::NaiveDate::parse_from_str(hindsight_date, "%Y-%m-%d").ok())
        .map(|(a, h)| (h - a).num_days().max(0))
        .unwrap_or(expected_holding_days.unwrap_or(28));

    let result = BacktestEngine::backtest_decision(
        client,
        &analysis.stock_code,
        analysis_date,
        &decision_action,
        decision_confidence,
        holding_days,
        time_horizon.clone(),
        expected_holding_days,
    )
    .await
    .map_err(|e| format!("BacktestEngine 回测失败: {e}"))?;

    // 3. 构造 actual_outcome 字符串（供反思引擎使用）
    let actual_outcome = if result.was_correct {
        "correct"
    } else {
        "wrong"
    }
    .to_string();

    tracing::info!(
        "[as-of backtest] {} ({}) 决策={} 持有={}天 收益={:.2}% 正确={}",
        analysis.stock_code,
        analysis.stock_name,
        decision_action,
        result.holding_days,
        result.return_pct,
        result.was_correct
    );

    Ok((
        actual_outcome,
        Some(result.return_pct),
        None, // alpha 需独立计算，暂留空
        Some(result.holding_days as i32),
    ))
}

/// 从 `stock_analyses.blackboard_snapshot` 构造 `sub-analysis` 变量。
///
/// [v2] 替代原 SubWorkflowNode 嵌套重放：直接从已保存的分析结果记忆中恢复
/// 各节点输出，避免重跑完整 stock-analysis DAG。
///
/// ## snapshot 结构（由 `build_blackboard_snapshot` 写入）
/// - `_raw.<nodeId>` — 原始节点输出（含 content/params/result 字段）
/// - `report.<nodeId>` — 分析师报告（纯文本）
/// - `params.<nodeId>` — content 解析后的 JSON 对象
/// - `result.<nodeId>` — CodeNode 的 result 字段
///
/// ## content 字段预处理
/// AgentNode 的 `content` 字段通常是 JSON 字符串（如 `{"action":"买入",...}`）。
/// `resolve_var_path` 不会自动解析 JSON 字符串，路径如
/// `sub-analysis.trader.content.action` 会下钻失败。
/// 此处对每个节点的 `content` 字段做 JSON 解析，把字符串转为对象，
/// 确保 input_mapping 的点路径能正确下钻。
fn build_sub_analysis_from_snapshot(
    snapshot_json: Option<&str>,
    stock_code: &str,
) -> serde_json::Value {
    use serde_json::Map;

    let Some(json_str) = snapshot_json else {
        tracing::warn!(
            "[reflection] {}: blackboard_snapshot 为 None（原始分析可能未完成）,注入空记忆",
            stock_code
        );
        return serde_json::json!({});
    };

    let Ok(snapshot) = serde_json::from_str::<serde_json::Value>(json_str) else {
        tracing::error!(
            "[reflection] {}: blackboard_snapshot JSON 解析失败,注入空记忆",
            stock_code
        );
        return serde_json::json!({});
    };

    let Some(obj) = snapshot.as_object() else {
        tracing::warn!(
            "[reflection] {}: blackboard_snapshot 不是 JSON 对象,注入空记忆",
            stock_code
        );
        return serde_json::json!({});
    };

    // 检查是否有 _raw.* 条目（新版 snapshot）
    let has_raw = obj.keys().any(|k| k.starts_with("_raw."));

    if !has_raw {
        tracing::warn!(
            "[reflection] {}: 旧版 snapshot（无 _raw.*），JSON 结构已丢失,注入空记忆。建议重新运行完整分析工作流以生成新版 snapshot",
            stock_code
        );
        return serde_json::json!({});
    }

    let mut sub_analysis = Map::new();
    for (key, val) in obj {
        let Some(node_id) = key.strip_prefix("_raw.") else {
            continue;
        };

        // 克隆节点输出，对 content 字段做 JSON 解析预处理
        let mut node_output = val.clone();
        if let Some(node_obj) = node_output.as_object_mut() {
            if let Some(content) = node_obj.get("content").and_then(|v| v.as_str()) {
                // content 是 JSON 字符串 → 解析为对象，确保路径下钻可用
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) {
                    if parsed.is_object() {
                        node_obj.insert("content".into(), parsed);
                    }
                }
            }
        }

        sub_analysis.insert(node_id.to_string(), node_output);
    }

    if sub_analysis.is_empty() {
        tracing::warn!(
            "[reflection] {}: snapshot 中未找到任何 _raw.* 节点输出,注入空记忆",
            stock_code
        );
    }

    serde_json::Value::Object(sub_analysis)
}

/// [方向4/方向5] 提交反思结果的用户反馈（1-5 星评分）。
///
/// 接入 FeedbackOrchestrator + ExperiencePipeline 双轨：
/// - Pipeline：把反馈转为 Experience 写入 RLOptimizer 经验池（reward: 1→-1.0, 5→1.0）
/// - Orchestrator：计数正/负反馈，达到阈值触发 RLTraining / SkillEvolution
///
/// [方向5] 当 Orchestrator 返回 `TriggerSkillEvolution` 时，spawn 异步任务
/// 真正调用 SkillEvolutionEngine 对 lesson 做语义变异进化：
/// - 从 stock_reflections 表查出 lesson_summary
/// - 包装为单步 Skill（content = "1. {lesson}"）
/// - 用 try_lock 获取 engine（避免阻塞反馈返回）
/// - 拉取最近 30 条轨迹作为 test_trajectories
/// - 进化成功则更新 reflection_lessons.rule_pattern 字段
///
/// `analysis_id` 同时作为 trace_id，保证同一反思的多次评分会被 Orchestrator 去重。
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "提交反思反馈评分")]
#[tauri::command]
pub async fn submit_reflection_feedback(
    state: State<'_, AppState>,
    analysis_id: String,
    rating: u8,
    comment: Option<String>,
) -> Result<serde_json::Value, String> {
    use crate::commands::_shared_state::{SHARED_ORCHESTRATOR, SHARED_PIPELINE};

    if !(1..=5).contains(&rating) {
        return Err("评分必须在 1-5 之间".to_string());
    }
    if analysis_id.trim().is_empty() {
        return Err("analysis_id 不能为空".to_string());
    }

    tracing::info!(
        "[reflection_feedback] analysis_id={} rating={} comment={:?}",
        analysis_id,
        rating,
        comment
    );

    // 1. ExperiencePipeline：反馈 → Experience → 经验池
    let pipeline = SHARED_PIPELINE.clone();
    let trace = analysis_id.clone();
    let comment_clone = comment.clone();
    let pipeline_handle = tokio::task::spawn(async move {
        let mut pipeline = pipeline.write().await;
        pipeline.process_feedback(&trace, rating, comment_clone.as_deref()).await
    });

    // 2. FeedbackOrchestrator：计数 + 阈值触发动作
    let orchestrator = SHARED_ORCHESTRATOR.clone();
    let action_result = tokio::task::spawn_blocking(move || orchestrator.record_feedback(rating))
        .await
        .map_err(|e| format!("Orchestrator join 错误: {e}"))?;

    let action_str = match &action_result {
        axagent_agent::OrchestratorAction::None => "none",
        axagent_agent::OrchestratorAction::TriggerRLTraining { .. } => "trigger_rl_training",
        axagent_agent::OrchestratorAction::TriggerSkillEvolution { .. } => {
            // [方向5] 真正触发 SkillEvolutionEngine 进化（异步，不阻塞反馈返回）
            let evolution_state = state.clone_for_evolution();
            let ev_analysis_id = analysis_id.clone();
            tokio::task::spawn(async move {
                if let Err(e) = run_lesson_evolution(&evolution_state, &ev_analysis_id).await {
                    tracing::warn!("[reflection_feedback] SkillEvolution 失败: {e}");
                }
            });
            "trigger_skill_evolution"
        },
        axagent_agent::OrchestratorAction::TriggerPoolSizeCheck { .. } => "trigger_pool_size_check",
    };

    // 等待 Pipeline 完成（best-effort，失败不影响反馈提交）
    if let Err(e) = pipeline_handle.await {
        tracing::warn!("[reflection_feedback] Pipeline join 错误: {e}");
    }

    Ok(serde_json::json!({
        "analysisId": analysis_id,
        "rating": rating,
        "action": action_str,
        "orchestratorStats": {
            "totalFeedback": SHARED_ORCHESTRATOR.stats().total_feedback,
            "negativeCount": SHARED_ORCHESTRATOR.stats().negative_count,
            "positiveCount": SHARED_ORCHESTRATOR.stats().positive_count,
        }
    }))
}

/// [方向5] SkillEvolution 所需的最小状态快照（避免持有 AppState 引用）。
struct EvolutionStateSnapshot {
    db: sea_orm::DatabaseConnection,
    skill_engine: Arc<tokio::sync::Mutex<axagent_trajectory::SkillEvolutionEngine>>,
    trajectory_storage: Arc<axagent_trajectory::TrajectoryStorage>,
}

impl AppState {
    /// 克隆 SkillEvolution 所需的最小状态
    fn clone_for_evolution(&self) -> EvolutionStateSnapshot {
        EvolutionStateSnapshot {
            db: self.harness.db().clone(),
            skill_engine: self.skill_evolution_engine.clone(),
            trajectory_storage: self.trajectory_storage.clone(),
        }
    }
}

/// [方向5] 对指定反思的 lesson 执行 SkillEvolutionEngine 语义变异进化。
///
/// 流程：
/// 1. 从 stock_reflections 表查出 lesson_summary / verdict / stock_code
/// 2. 把 lesson 包装为单步 Skill（content = "1. {lesson}"）
/// 3. 用 try_lock 获取 engine（失败则跳过，不阻塞）
/// 4. 拉取最近 30 条轨迹作为 test_trajectories
/// 5. 调用 engine.run(&skill, &test_refs).await
/// 6. 进化成功则更新 reflection_lessons.rule_pattern 字段
async fn run_lesson_evolution(
    state: &EvolutionStateSnapshot,
    analysis_id: &str,
) -> Result<(), String> {
    use axagent_entities::{reflection_lessons, stock_reflections};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    // 1. 查询反思记录
    let reflection = stock_reflections::Entity::find_by_id(analysis_id.to_string())
        .one(&state.db)
        .await
        .map_err(|e| format!("查询 stock_reflections 失败: {e}"))?
        .ok_or_else(|| format!("反思记录 {analysis_id} 不存在"))?;

    let lesson_summary = reflection
        .lesson_summary
        .as_deref()
        .ok_or_else(|| "lesson_summary 为空，无法进化".to_string())?;

    let stock_code = reflection.stock_code.clone();

    // 2. 包装为单步 Skill（用 Skill::new 构造函数）
    let mut skill = axagent_trajectory::Skill::new(
        format!("反思教训:{}", stock_code),
        format!("股票 {} 反思教训", stock_code),
        format!("1. {lesson_summary}"),
        "reflection_lesson".to_string(),
    );
    skill.id = format!("lesson_{analysis_id}");

    // 3. try_lock 获取 engine（不阻塞）
    let mut engine = state
        .skill_engine
        .try_lock()
        .map_err(|_| "SkillEvolutionEngine 被占用（cron 正在运行），跳过本次进化".to_string())?;

    // 4. 拉取最近 30 条轨迹
    let trajectories = state
        .trajectory_storage
        .get_trajectories(Some(30))
        .await
        .map_err(|e| format!("拉取轨迹失败: {e}"))?;

    if trajectories.len() < 10 {
        return Err(format!("轨迹数量不足（{} < 10），无法进化", trajectories.len()));
    }

    let test_refs: Vec<&axagent_trajectory::Trajectory> = trajectories.iter().collect();

    // 5. 调用进化
    tracing::info!("[skill_evolution] 开始进化 lesson {} (stock={})", analysis_id, stock_code);
    let modification = engine.run(&skill, &test_refs).await;

    if let Some(modification) = &modification {
        tracing::info!(
            "[skill_evolution] 进化完成: confidence={:.3} reason={}",
            modification.confidence,
            modification.reason
        );

        // 6. 更新 reflection_lessons.rule_pattern 字段
        if !modification.new_content.is_empty() {
            let now_ms = chrono::Utc::now().timestamp_millis();
            let _ = reflection_lessons::Entity::update_many()
                .col_expr(
                    reflection_lessons::Column::RulePattern,
                    sea_orm::sea_query::Expr::value(modification.new_content.clone()),
                )
                .col_expr(
                    reflection_lessons::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::value(now_ms),
                )
                .filter(reflection_lessons::Column::SourceReflectionId.eq(analysis_id.to_string()))
                .exec(&state.db)
                .await;
            tracing::info!(
                "[skill_evolution] 已更新 reflection_lessons.rule_pattern (analysis_id={})",
                analysis_id
            );
        }
    } else {
        tracing::info!("[skill_evolution] 进化未产生改进（lesson={})", analysis_id);
    }

    Ok(())
}

// ── 单元测试：覆盖 LLM 输出 → IR → JSON 提取的全链路 ──
//
// 关键场景：
//   1) LLM 严格按新 prompt 输出 tool_json 块 → ToolUse 路径
//   2) LLM 偶发只输出普通 ```json 块（没有 name 字段） → 文本块 → 内部 JSON
//   3) LLM 输出截断的 JSON（用户日志里的"后 200 字符"场景） → 至少能拿到
//      一个有效前缀并解析出 candidates
//   4) Agent 节点输出顶层 params / output / candidates 字段 → 直返
//   5) extract_agent_output 顶层 params 优先于 content
