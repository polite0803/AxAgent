use super::core::fetch_stock_lessons;
use super::decision::{load_and_inject_template, resolve_runtime_options};
use super::serenity::extract_agent_output;
use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_astock_data::as_of::AsOfContext;
use axagent_core::entity::stock_analyses;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::json;
use std::sync::Arc;
use tauri::State;

/// 反思复盘工作流：嵌套原股票分析工作流的 as-of，取后见信息对比，反思。
///
/// 加载与 [run_single_stock_analysis] 相同的 stock-analysis DAG，
/// 设置 as_of_date 回到原始分析日期（数据与原分析一致），
/// 注入 `actual_outcome` 变量让 portfolio-manager 产生反思。
///
/// ## v008 升级（借鉴 TradingAgents 反思机制）
///
/// 新增 4 个结构化 outcome 参数（`raw_return` / `alpha_return` /
/// `holding_days` / `benchmark_name`）作为 C3 借鉴；`actual_outcome`
/// 保留为 legacy/fallback 自然语言描述。C1 + C2 强约束在 reflection-agent
/// system_prompt 体现（≤200 字符 lesson_summary + verdict 标签 + alpha_cited）。
///
/// ## v009 升级（B1+B2+B3 借鉴）
///
/// - B1 落盘协议:调用方(批量分析)已写入 `stock_reflections` row with `status="pending"`。
/// - B2 幂等守卫:当 `reflection_id` 已存在且 `status="completed"`,直接返回
///   cached row 的 `lesson_summary` / `verdict` / `decision_json`,避免重跑 LLM。
/// - B3 原子写:传入 `reflection_id` 时,UPDATE 现有 row 而非 INSERT 新的,
///   避免重复 INSERT 触发冲突。
///
/// 结果写入独立的 `stock_reflections` 表。
#[allow(clippy::too_many_arguments)]
pub async fn run_reflection_workflow(
    db: &DatabaseConnection,
    _client: &axagent_astock_data::AStockClient,
    engine: &Arc<axagent_rt_workflow::work_engine::WorkEngine>,
    vector_store: &axagent_core::vector_store::VectorStore,
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
) -> Result<String, String> {
    use axagent_astock_data::as_of;
    use axagent_core::entity::stock_reflections;
    use sea_orm::sea_query::Expr;

    let now_ms = chrono::Utc::now().timestamp_millis();

    // ── [B2 借鉴] 幂等守卫: 如果 reflection_id 已 completed,直接返回 cached ──
    if let Some(ref rid) = reflection_id {
        if let Some(existing) = stock_reflections::Entity::find_by_id(rid.clone())
            .one(db)
            .await
            .map_err(|e| {
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
    let analysis_id = reflection_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

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
    let (max_concurrent, step_timeout) = resolve_runtime_options(loaded.variables.as_deref());

    // 3. 创建嵌套工作流
    let wf_name = format!("stock-reflection-{stock_code}");
    let workflow = engine
        .create_workflow(&wf_name, loaded.nodes, loaded.edges)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建反思工作流失败: {e}"))
        })?;
    let wf_id = workflow.id.clone();

    // 4. 加载原始决策的时间维度信息
    // 手动触发时 original_analysis_id="" → original_ctx=None。
    // 但反思 prompt 模板 (reflection.md:17-18) hard-code 引用
    // {{original_time_horizon}} / {{original_holding_days}},所以必须注入占位值
    // (否则 work_engine 报 VARIABLE_NOT_FOUND,reflection-agent 节点 Failed,
    // 数据库 what_went_wrong 等字段全 null)。
    // 之前的注释说"让工作流模板自己决定怎么处理"——实际模板没有兜底处理。
    let original_ctx: Option<(String, i64)> = if original_analysis_id.is_empty() {
        None
    } else {
        let time_horizon = stock_analyses::Entity::find_by_id(original_analysis_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .and_then(|a| a.decision_time_horizon);
        let holding_days = stock_analyses::Entity::find_by_id(original_analysis_id)
            .one(db)
            .await
            .ok()
            .flatten()
            .and_then(|a| a.decision_expected_holding_days.map(|d| d as i64));
        match (time_horizon, holding_days) {
            (Some(t), Some(h)) => Some((t, h)),
            _ => None,
        }
    };

    // 5. 注入变量
    let mut variables = vec![
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
        // 反思 prompt 模板里引用了 {{stock_lessons}},必须显式注入,
        // 否则 work_engine 报 VARIABLE_NOT_FOUND 导致反思节点 Failed。
        // 数据源: 该股最近 3 个月的反思记录(去重排除当前正在创建的记录)。
        axagent_harness::workflow_types::Variable {
            name: "stock_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(
                fetch_stock_lessons(stock_code, db)
                    .await
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
        // [BUGFIX] 之前只传 stock_code,缺 stock_name / as_of_date。
        // 反思工作流内的 sub-analysis 节点 (嵌套 stock-analysis 子工作流) 的
        // input_mapping 把这 3 个变量映射到子工作流的 input,缺任何一个都会
        // 导致子工作流报 "参数 X 应为 string 类型" 或 "VARIABLE_NOT_FOUND: X"。
        input: Some(json!({
            "stock_code": stock_code,
            "stock_name": stock_name,
            "as_of_date": as_of_date,
        })),
        input_schema: loaded.input_schema,
        output_schema: loaded.output_schema,
        dry_run: false,
        variables: Some(variables),
        ..Default::default()
    };

    // 5. as-of 范围执行
    let ctx = AsOfContext::parse(as_of_date).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("as_of 解析失败: {e}"))
    })?;

    let result = as_of::AS_OF
        .scope(Some(ctx), async move { engine.run_workflow(&wf_id, opts).await })
        .await;

    // 6. 处理结果
    match result {
        Ok(wf) => {
            // 通过 extract_agent_output 管线提取规范化 JSON（兼容多模型输出格式）
            let reflection_raw = wf
                .results
                .get("reflection")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
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
                .col_expr(stock_reflections::Column::FixForFuture, Expr::value(fix_for_future))
                .col_expr(
                    stock_reflections::Column::ParameterSuggestionsJson,
                    Expr::value(params_suggestion_json),
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
#[tauri::command]
pub async fn run_batch_reflection(
    state: State<'_, AppState>,
    max_count: Option<u32>,
) -> Result<serde_json::Value, String> {
    use axagent_core::entity::stock_analyses;
    use axagent_core::entity::stock_reflections;

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
        let analysis = match stock_analyses::Entity::find_by_id(&p.original_analysis_id)
            .one(db)
            .await
        {
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
        let expected_days = analysis
            .decision_expected_holding_days
            .map(|d| d as i64)
            .unwrap_or(28);
        let analysis_date = analysis.as_of_date.as_deref().unwrap_or(&p.as_of_date);
        let analysis_ms = chrono::NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc().timestamp_millis())
            .unwrap_or(p.created_at);
        let days_held = (today_ms - analysis_ms).max(0) / 86_400_000; // ms → days

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
            &chrono::Utc::now().format("%Y-%m-%d").to_string(),
            0u8,
            "light",
            Some(p.id.clone()), // [B2/B3] 走 UPDATE 路径
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
    let ninety_days_ago_ms = today_ms - 90 * 86_400_000;
    let cleaned_up = stock_reflections::Entity::delete_many()
        .filter(stock_reflections::Column::Status.eq("completed"))
        .filter(stock_reflections::Column::UpdatedAt.lt(ninety_days_ago_ms))
        .exec(db)
        .await
        .map(|r| r.rows_affected)
        .unwrap_or_else(|e| {
            tracing::warn!("[D2] FIFO 清理失败: {e}");
            0
        });
    tracing::info!("[D2 fifo_cleanup] 删除 {} 条超龄 completed row", cleaned_up);

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
    use axagent_core::entity::reflection_lessons;
    use sea_orm::ActiveModelTrait;
    use sea_orm::Set;

    // 短文本过短或无实际建议性内容则跳过
    let trimmed = lesson_summary.trim();
    if trimmed.len() < 10 || trimmed.len() > 250 {
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

// ── [缺陷5 fix] 内部批量反思函数(非 Tauri 命令,供 cron 调度器直接调用) ──
//
// 从 run_batch_reflection 提取的核心逻辑。
// 参数通过独立引用传入,不需要 AppState。
pub async fn run_batch_reflection_inner(
    db: &sea_orm::DatabaseConnection,
    _client: &axagent_astock_data::AStockClient,
    _engine: &axagent_rt_workflow::work_engine::WorkEngine,
    _vector_store: &axagent_core::vector_store::VectorStore,
    _master_key: &[u8; 32],
    max_count: Option<u32>,
) -> Result<serde_json::Value, String> {
    use crate::commands::error::ErrorResponse;
    use axagent_core::entity::stock_analyses;
    use axagent_core::entity::stock_reflections;

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
        let analysis = match stock_analyses::Entity::find_by_id(&p.original_analysis_id)
            .one(db)
            .await
        {
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

        let expected_days = analysis
            .decision_expected_holding_days
            .map(|d| d as i64)
            .unwrap_or(28);
        let analysis_date = analysis.as_of_date.as_deref().unwrap_or(&p.as_of_date);
        let analysis_ms = chrono::NaiveDate::parse_from_str(analysis_date, "%Y-%m-%d")
            .ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc().timestamp_millis())
            .unwrap_or(p.created_at);
        let days_held = (today_ms - analysis_ms).max(0) / 86_400_000;

        if days_held < expected_days {
            skipped_young += 1;
            continue;
        }

        let r = run_reflection_workflow(
            db,
            _client,
            &std::sync::Arc::new(_engine.clone()),
            _vector_store,
            _master_key,
            &p.stock_code,
            &p.stock_name,
            &p.original_analysis_id,
            &p.actual_outcome,
            None,
            None,
            Some(days_held as i32),
            None,
            analysis_date,
            &chrono::Utc::now().format("%Y-%m-%d").to_string(),
            0u8,
            "light",
            Some(p.id.clone()),
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

    // D2 FIFO 清理
    let ninety_days_ago_ms = today_ms - 90 * 86_400_000;
    let cleaned_up = stock_reflections::Entity::delete_many()
        .filter(stock_reflections::Column::Status.eq("completed"))
        .filter(stock_reflections::Column::UpdatedAt.lt(ninety_days_ago_ms))
        .exec(db)
        .await
        .map(|r| r.rows_affected)
        .unwrap_or(0);

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

// ── 单元测试：覆盖 LLM 输出 → IR → JSON 提取的全链路 ──
//
// 关键场景：
//   1) LLM 严格按新 prompt 输出 tool_json 块 → ToolUse 路径
//   2) LLM 偶发只输出普通 ```json 块（没有 name 字段） → 文本块 → 内部 JSON
//   3) LLM 输出截断的 JSON（用户日志里的"后 200 字符"场景） → 至少能拿到
//      一个有效前缀并解析出 candidates
//   4) Agent 节点输出顶层 params / output / candidates 字段 → 直返
//   5) extract_agent_output 顶层 params 优先于 content
