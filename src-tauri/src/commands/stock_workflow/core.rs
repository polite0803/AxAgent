use super::decision::QualityPrecheckResult;
use super::decision::{
    build_dashboard_from_workflow_result, compute_decision_agreement, data_quality_precheck,
    extract_decision_fields, extract_decision_json, extract_llm_decision_json,
    load_and_inject_template, parse_asof_param, resolve_runtime_options,
};
use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;
use axagent_analysis_engine::blackboard::build_blackboard_snapshot;
use axagent_analysis_engine::stock_reflection::{AnalysisStepResult, StockAnalysisOutcome};
use axagent_astock_data::as_of::{self, AsOfContext};
use axagent_entities::price_alerts;
use axagent_entities::stock_analyses;
use axagent_entities::stock_reflections;
use axagent_harness::workflow_types::Variable;
use axagent_rt_workflow::work_engine::{ProgressCallback, RunOptions, StepProgressEvent};
use sea_orm::DatabaseConnection;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;
use std::sync::Arc;
use tauri::{Emitter, State};

/// 启动股票分析工作流（DAG 模式）。
///
/// - 默认：生成新 UUID 并 INSERT 新 `stock_analyses` 行（fresh start）。
/// - 重跑分析场景：传入 `parent_analysis_id` 指向原始记录，新建独立行保留历史版本。
#[agent_command(domain = invest, safety = Caution, call_mode = StateInput, description = "启动股票分析工作流")]
#[tauri::command]
pub async fn run_stock_workflow(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    stock_code: String,
    dry_run: Option<bool>,
    as_of_date: Option<String>,
    // 版本化分析: 传入原始 analysisId 作为 parent，新建一条独立记录保留历史版本。
    // 不传则为首次分析（parent_analysis_id = NULL）。
    parent_analysis_id: Option<String>,
    // V53: 筛选来源标记 — "serenity" 表示来自瓶颈掘金候选
    screening_source: Option<String>,
    // P2-3.3: 报告输出语言 — "en" / "english" 英文报告，其他值或 None 默认中文
    language: Option<String>,
) -> Result<serde_json::Value, String> {
    // 解析 as_of_date；非法或未来日期直接 4xx-style 错误
    let as_of_ctx = parse_asof_param(as_of_date.clone())?;

    if let Some(ctx) = as_of_ctx {
        as_of::AS_OF
            .scope(Some(ctx), async {
                run_stock_workflow_inner(
                    app,
                    state.inner(),
                    stock_code,
                    dry_run,
                    as_of_date,
                    parent_analysis_id,
                    screening_source,
                    language,
                )
                .await
            })
            .await
    } else {
        run_stock_workflow_inner(
            app,
            state.inner(),
            stock_code,
            dry_run,
            None,
            parent_analysis_id,
            screening_source,
            language,
        )
        .await
    }
}

/// P1-1: T+0 异动重跑后端入口（由 RealtimeMonitor 的 t0_callback 调用）。
///
/// 与 `run_stock_workflow` 命令的区别：
/// - 不需要 Tauri 命令注入的 `State<'_, AppState>`，内部用 `app.state::<AppState>()` 获取
/// - 固定为 live 模式（as_of_date=None）—— T+0 重跑针对实时行情
/// - 不传 parent_analysis_id —— 每次新建独立版本（live 模式纯版本化策略）
/// - 不传 screening_source / language —— 走默认中文 + 非筛选来源
/// - 返回 analysisId 供 monitor 日志追踪
///
/// 设计要点：
/// - 使用 `tauri::Manager` trait 的 `app.state::<T>()` 获取共享状态
/// - 失败时返回 Err(String)，由调用方（monitor.rs）记录日志，不阻塞监控循环
///
/// P1-2: 并发控制
/// - 全局 `stock_workflow_t0_semaphore`（permits=5）：50+ 股票同时异动时限流
/// - per-stock `stock_workflow_t0_per_stock_locks`：同股票多次触发串行执行
/// - 三个 Tauri 事件让前端感知排队状态：
///   * `stock-t0-queue-entered`：进入队列等待（含队列深度）
///   * `stock-t0-rerun-started`：获取 permit + per-stock 锁，开始执行
///   * `stock-t0-rerun-completed`：执行结束（含 success / error 信息）
pub(crate) async fn trigger_t0_rerun(
    app: tauri::AppHandle,
    stock_code: String,
) -> Result<String, String> {
    use tauri::Manager;

    let state = app.state::<AppState>();
    let state_inner = state.inner();

    // P1-2: 1) 发出 queue-entered 事件（前端 toast 提示"排队中"）
    //       同时尝试获取全局 semaphore permit（阻塞等待）
    // active_count = 5 - available_permits 表示当前正在执行的 T+0 重跑数
    // （Semaphore 不直接暴露等待数，用 active_count 作为队列压力指标）
    let active_count =
        5_usize.saturating_sub(state_inner.stock_workflow_t0_semaphore.available_permits());
    let _ = app.emit(
        "stock-t0-queue-entered",
        json!({
            "stockCode": stock_code,
            "activeCount": active_count,
            "maxConcurrency": 5,
            "timestamp": chrono::Utc::now().timestamp_millis(),
        }),
    );

    // 2) 获取全局 permit（阻塞等待，但 callback 已在 tokio::spawn 内，不会阻塞 monitor 主循环）
    let _permit = state_inner
        .stock_workflow_t0_semaphore
        .acquire()
        .await
        .map_err(|e| format!("T+0 semaphore acquire 失败: {e}"))?;

    // 3) 获取 per-stock 锁（同股票串行，不同股票并行）
    let per_stock_lock = {
        let mut locks = state_inner.stock_workflow_t0_per_stock_locks.lock().await;
        locks
            .entry(stock_code.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    };
    let _stock_guard = per_stock_lock.lock().await;

    // 4) 发出 rerun-started 事件
    let started_at = chrono::Utc::now().timestamp_millis();
    let _ = app.emit(
        "stock-t0-rerun-started",
        json!({
            "stockCode": stock_code,
            "startedAt": started_at,
        }),
    );

    // 5) 执行工作流（permit + per-stock 锁同时持有）
    let app_for_inner = app.clone();
    let exec_result = run_stock_workflow_inner(
        app_for_inner,
        state_inner,
        stock_code.clone(),
        None, // dry_run
        None, // as_of_date — live 模式
        None, // parent_analysis_id — 新建独立版本
        None, // screening_source
        None, // language — 默认中文
    )
    .await;

    // 6) 发出 rerun-completed 事件（无论成功失败）
    let completed_at = chrono::Utc::now().timestamp_millis();
    let (success, analysis_id, error_msg) = match &exec_result {
        Ok(payload) => {
            let aid =
                payload.get("analysisId").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
            (true, aid, None::<String>)
        },
        Err(e) => (false, "unknown".to_string(), Some(e.clone())),
    };
    let _ = app.emit(
        "stock-t0-rerun-completed",
        json!({
            "stockCode": stock_code,
            "analysisId": analysis_id,
            "success": success,
            "error": error_msg,
            "startedAt": started_at,
            "completedAt": completed_at,
            "durationMs": completed_at - started_at,
        }),
    );

    // 7) 返回结果（permit + per-stock 锁在此自动释放）
    let result = exec_result?;
    let analysis_id =
        result.get("analysisId").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    Ok(analysis_id)
}

pub(crate) async fn run_stock_workflow_inner(
    app: tauri::AppHandle,
    state: &AppState,
    stock_code: String,
    dry_run: Option<bool>,
    as_of_date: Option<String>,
    // 版本化分析：重跑时指向原始记录 ID，新建独立行保留历史版本。
    parent_analysis_id: Option<String>,
    // V53: 筛选来源标记 — 告诉 stock-analysis 工作流当前股票来自哪里。
    // "serenity" 表示来自瓶颈掘金候选，允许风险分类器做评分修正。
    screening_source: Option<String>,
    // P2-3.3: 报告输出语言 — 传入 prompts::language_instruction 生成指示文本，
    // 追加到每个 AgentNode 的 system_prompt 末尾，让 LLM 用对应语言输出。
    language: Option<String>,
) -> Result<serde_json::Value, String> {
    let quote = state.astock_client.get_quote(&stock_code).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("行情获取失败: {e}"))
    })?;
    let now_ms = chrono::Utc::now().timestamp_millis();

    // ── 版本化策略：live 纯版本化，replay 同日覆盖 ──
    // - live 模式（as_of_date 为 None）：纯版本化，每次重跑都新建独立记录
    //   业务语义：日内行情实时变化（创业板最高 20% 涨跌幅），
    //   同日多次分析应各自保留为独立版本，以支持日内涨跌分析。
    // - replay 模式（as_of_date 指定）：以 as_of_date（YYYY-MM-DD）为版本边界
    //   - 重跑时 as_of_date 与原始记录相同 → 覆盖（UPDATE 现有记录，不新建行）
    //   - 重跑时 as_of_date 不同 → 新建版本（INSERT 新行，parent 指向原始记录）
    //   业务语义：历史数据已固定，重跑只是修正决策；跨交易日数据不同则保留版本。
    // - 首次分析 → INSERT 新行，parent=NULL
    let current_as_of =
        as_of_date.clone().unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
    let is_live_mode = as_of_date.is_none();

    let (analysis_id, parent_for_record, need_insert) = match &parent_analysis_id {
        Some(parent_id) => {
            match stock_analyses::Entity::find_by_id(parent_id.as_str())
                .one(state.harness.db())
                .await
            {
                Ok(Some(parent_record)) => {
                    if is_live_mode {
                        // live 模式：纯版本化，每次重跑都新建独立记录
                        tracing::info!(
                            "[run_stock_workflow] live 重跑(新建版本): parent={}",
                            parent_id
                        );
                        (uuid::Uuid::new_v4().to_string(), Some(parent_id.clone()), true)
                    } else {
                        // replay 模式：比较 as_of_date，相同则覆盖，不同则新建版本
                        let parent_as_of = parent_record.as_of_date.as_deref().unwrap_or("");
                        if parent_as_of == current_as_of {
                            // 同一 as_of_date：覆盖现有记录（重置为 running）
                            tracing::info!(
                                "[run_stock_workflow] replay 同日重跑(覆盖模式): parent={}, as_of={}",
                                parent_id,
                                current_as_of
                            );
                            stock_analyses::Entity::update_many()
                                .col_expr(stock_analyses::Column::Status, Expr::value("running"))
                                .col_expr(
                                    stock_analyses::Column::DecisionAction,
                                    Expr::value(None::<String>),
                                )
                                .col_expr(
                                    stock_analyses::Column::DecisionPositionPct,
                                    Expr::value(None::<f64>),
                                )
                                .col_expr(
                                    stock_analyses::Column::DecisionReasoning,
                                    Expr::value(None::<String>),
                                )
                                .col_expr(
                                    stock_analyses::Column::DecisionJson,
                                    Expr::value(None::<String>),
                                )
                                .col_expr(
                                    stock_analyses::Column::LlmDecisionJson,
                                    Expr::value(None::<String>),
                                )
                                .col_expr(stock_analyses::Column::UpdatedAt, Expr::value(now_ms))
                                .filter(stock_analyses::Column::Id.eq(parent_id.as_str()))
                                .exec(state.harness.db())
                                .await
                                .map_err(|e| {
                                    ErrorResponse::new(wf_err::INTERNAL)
                                        .with_detail(format!("覆盖更新失败: {e}"))
                                })?;
                            (parent_id.clone(), None, false)
                        } else {
                            // 跨 as_of_date：新建版本
                            tracing::info!(
                                "[run_stock_workflow] replay 跨日新建版本: parent={}, old_as_of={}, new_as_of={}",
                                parent_id,
                                parent_as_of,
                                current_as_of
                            );
                            (uuid::Uuid::new_v4().to_string(), Some(parent_id.clone()), true)
                        }
                    }
                },
                _ => {
                    tracing::warn!(
                        "[run_stock_workflow] parent={} 不存在,降级为 fresh start",
                        parent_id
                    );
                    (uuid::Uuid::new_v4().to_string(), None, true)
                },
            }
        },
        None => {
            // 首次分析
            (uuid::Uuid::new_v4().to_string(), None, true)
        },
    };

    if need_insert {
        stock_analyses::ActiveModel {
            id: Set(analysis_id.clone()),
            stock_code: Set(stock_code.clone()),
            stock_name: Set(quote.name.clone()),
            // B12: 在 as-of 模式下,analysis_date 必须是 as-of 截止日,而不是 today
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
            analysis_kind: Set(if as_of_date.is_some() {
                "replay".into()
            } else {
                "live".into()
            }),
            as_of_date: Set(Some(current_as_of.clone())),
            model_version: Set(None),
            data_snapshot_id: Set(None),
            outcome: Set(None),
            decision_time_horizon: Set(None),
            decision_expected_holding_days: Set(None),
            parent_analysis_id: Set(parent_for_record.clone()),
            trade_intent_status: Set("pending".into()),
            trade_intent_source: Set(None),
            trade_intent_source_ref_id: Set(None),
            trade_intent_reviewed_at: Set(None),
            trade_intent_reviewed_by: Set(None),
            trade_intent_review_notes: Set(None),
            trade_intent_actual_trade_id: Set(None),
            created_at: Set(now_ms),
            updated_at: Set(now_ms),
        }
        .insert(state.harness.db())
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("DB 写入失败: {e}"))
        })?;
    }

    // ── 数据质量预检：在发起 DAG 执行前检查关键数据是否完整 ──
    let stock_code_for_check = stock_code.clone();
    let quality_check =
        data_quality_precheck(&state.astock_client, &stock_code_for_check, &quote).await;
    match quality_check {
        QualityPrecheckResult::Insufficient { ref summary, ref missing_sources } => {
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
                "dataMissingReport": missing_report,
                "analysisId": analysis_id,
                "stockCode": stock_code,
                "stockName": quote.name,
                "dataQualityPrecheck": "insufficient",
            }));
        },
        QualityPrecheckResult::Pass => {
            // 数据充分，正常执行
        },
        QualityPrecheckResult::Partial(reason) => {
            tracing::info!("stock_workflow] 数据质量部分缺失，继续分析: {reason}");
        },
    }

    let mut loaded =
        load_and_inject_template(state.harness.db(), &stock_code, &quote.name, "stock-analysis")
            .await?;

    // P2-3.3: 报告语言切换 — 追加语言指示到 Agent 节点的 system_prompt 末尾
    if let Some(ref lang) = language {
        if let Some(instruction) = axagent_analysis_engine::prompts::language_instruction(lang) {
            for node in &mut loaded.nodes {
                if let axagent_harness::workflow_types::WorkflowNode::Agent(a) = node {
                    a.config.system_prompt = format!("{}\n{}", a.config.system_prompt, instruction);
                }
            }
            tracing::info!("[stock_workflow] 报告语言已切换为: {lang}");
        }
    }

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

    // 从模板变量解析 vendor_* 布尔开关，注入到 astock_client 的启用状态过滤器
    // 未启用的 vendor 会在 find_vendor 中被跳过，避免无效调用和超时重试
    super::decision::inject_vendor_state(&state.astock_client, loaded.variables.as_ref());

    let engine = Arc::clone(&state.work_engine);

    // ── 从模板变量中解析执行参数 ──
    // max_concurrent / step_timeout / total_timeout 通过模板变量让用户在设置面板调整。
    let (max_concurrent, step_timeout, total_timeout) =
        resolve_runtime_options(loaded.variables.as_deref());

    let wf_name = format!("stock-analysis-{stock_code}");
    let workflow =
        engine.create_workflow(&wf_name, loaded.nodes, loaded.edges).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建工作流失败: {e}"))
        })?;
    let wf_id = workflow.id.clone();
    let wf_id_ret = wf_id.clone();
    let app_h = app.clone();
    let db = state.harness.db().clone();
    let aid = analysis_id.clone();

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
                        // 修复: 透传 StepProgressEvent.error 真实错误，而非占位符 "Step failed"
                        "error": event.error.clone()
                            .unwrap_or_else(|| format!("Step {}", event.status)),
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
                    // 修复: 携带真实错误，前端 failedNodeErrors 才能显示具体失败原因
                    "error": event.error.clone(),
                    // 修复: 携带节点输出，前端 analystReports 能实时填充（一边进行一边显示）
                    "output": event.output,
                }),
            );
        })
    });

    let input_schema = loaded.input_schema;
    let output_schema = loaded.output_schema;
    let template_vars = loaded.variables;
    // 修复 E0382: 在 spawn 前（即 loaded.variables 被 move 到 template_vars 之后）从 template_vars 读取 tool_timeout
    let tool_timeout_secs = template_vars
        .as_ref()
        .and_then(|vars| {
            vars.iter().find(|v| v.name == "tool_timeout_secs").and_then(|v| v.value.as_u64())
        })
        .map(|s| std::cmp::max(s, 5))
        .unwrap_or(30u64);

    let sc_for_ret = stock_code.clone();
    let sc_name = quote.name.clone();
    let sc_name_for_spawn = sc_name.clone();
    let vector_store = state.vector_store.clone();
    let master_key = state.harness.master_key_owned();
    // 在 spawn 前拉取市场状态（沪深300判断牛/熊/震荡），捕获到闭包中
    let market_regime_json: Option<serde_json::Value> =
        state.astock_client.get_klines("000300", "daily", 60).await.ok().and_then(|klines| {
            if klines.is_empty() {
                return None;
            }
            let r = axagent_analysis_engine::market_regime::classify_regime(&klines);
            Some(serde_json::json!({
                "regime": r.regime,
                "confidence": r.confidence,
                "volatility": r.volatility,
                "description": r.description,
            }))
        });
    // 注入市场模拟指标（sim_stability/sim_liquidity/sim_impact）
    // 从个股 K 线计算轻量版，无需额外 API 调用
    let sim_metrics: serde_json::Value = state
        .astock_client
        .get_klines(&stock_code, "daily", 30)
        .await
        .ok()
        .and_then(|klines| {
            if klines.len() < 5 {
                return None;
            }
            // 计算日收益率序列 → 年化波动率 → sim_stability
            let mut returns = Vec::with_capacity(klines.len() - 1);
            let mut total_volume: f64 = 0.0;
            for pair in klines.windows(2) {
                let prev_close = pair[0].close;
                let cur = &pair[1];
                if prev_close > 0.0 {
                    returns.push((cur.close - prev_close) / prev_close);
                }
                total_volume += cur.volume;
            }
            let avg_price = klines.last()?.close;
            let n = returns.len() as f64;
            if n < 3.0 {
                return None;
            }
            let mean = returns.iter().sum::<f64>() / n;
            let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
            let annual_vol = variance.sqrt() * (252.0_f64).sqrt();
            // sim_stability: 0~1, 年化波动率 0%→1.0, 60%→0.5, 100%+→0.3
            let sim_stability = 1.0 / (1.0 + annual_vol * 3.0).clamp(0.3, 1.0);
            // sim_liquidity: 0~1, 从日均成交额估算
            let avg_daily_volume = total_volume / (klines.len() as f64);
            let daily_volume_val = avg_daily_volume * avg_price;
            // 日均成交额 ≥ 1亿 → 0.8, 1000万→0.5, 100万→0.2
            let sim_liquidity = (daily_volume_val / 100_000_000.0 * 0.7 + 0.1).clamp(0.1, 0.95);
            // sim_impact: bps, 从波动率和流动性估算
            let sim_impact =
                (annual_vol * 100.0 * (1.0 - sim_liquidity) * 50.0 + 5.0).clamp(1.0, 200.0);
            Some(serde_json::json!({
                "sim_stability": (sim_stability * 100.0).round() / 100.0,
                "sim_liquidity": (sim_liquidity * 100.0).round() / 100.0,
                "sim_impact": (sim_impact * 10.0).round() / 10.0,
                "sim_regime": if annual_vol > 0.4 { "high_vol" }
                    else if annual_vol > 0.2 { "normal" } else { "low_vol" },
            }))
        })
        .unwrap_or_else(|| {
            serde_json::json!({
                "sim_stability": serde_json::Value::Null,
                "sim_liquidity": serde_json::Value::Null,
                "sim_impact": serde_json::Value::Null,
                "sim_regime": serde_json::Value::Null,
            })
        });
    // 在 spawn 前捕获 as-of 上下文（tokio::task_local 不跨 tokio::spawn 传播）
    let captured_asof = as_of::current_as_of();
    // H4.2 修复：捕获 shutdown_token，使 spawn 任务在应用关闭时能协作取消，
    // 避免 AppState::drop 后后台任务仍阻塞 run_workflow 等待 LLM 响应。
    let shutdown_token = state.shutdown_token.clone();
    // P1-E13: 在 spawn 前克隆 astock_client，供组合风控门查询股票行业信息。
    // state 是借用引用，不能逃逸到 tokio::spawn 内部，必须在此克隆后 move 进去。
    let astock_for_sector = state.astock_client.clone();
    // P2-F15: 克隆 analysis_id 供 spawn 内部使用（record_lesson_applications），
    // 保留原始 analysis_id 供函数返回值使用。
    let analysis_id_for_spawn = analysis_id.clone();
    // P0: 克隆 RealtimeMonitor，供决策落库后自动创建价格告警
    let monitor_for_spawn = state.stock_monitor.get().cloned();
    // P2-1: 克隆 TriggerManager，供决策落库后发布 decision.completed 事件
    let trigger_mgr_for_spawn = state.work_engine.trigger_manager.clone();
    // 克隆自适应引擎，供工作流完成后触发自适应闭环
    let adaptive_engine_for_spawn = state.stock_adaptive_engine.clone();
    tokio::spawn(async move {
        // P3 修复: 在 spawn 内恢复 AS_OF + DEGRADATION_LOG 作用域
        as_of::with_optional_asof(captured_asof, async {
            as_of::with_degradation_log(async {
        // 按类型并发上限：tool/file 保持高位，llm/agent 对齐用户设定的 max_concurrent
        // 修复: 默认按类型上限 llm=3 会覆盖全局 max_concurrent，使设置面板的值失效
        let mut type_limits = std::collections::HashMap::new();
        type_limits.insert("tool".into(), 10usize);
        type_limits.insert("file".into(), 10usize);
        type_limits.insert("llm".into(), max_concurrent);
        type_limits.insert("agent".into(), max_concurrent);
        // 从模板变量读取工具节点超时（已在 spawn 前从 template_vars 算出，避免 move 冲突）
        let mut opts = RunOptions {
            max_concurrent,
            step_timeout,
            tool_timeout: std::time::Duration::from_secs(tool_timeout_secs),
            max_concurrent_by_type: Some(type_limits),
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
                if let Some(detail) = axagent_analysis_engine::recommender::get_serenity_candidate_detail(&stock_code) {
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
        // 注入市场模拟指标（DES 轻量版，从个股 K 线估算）
        if let Some(stab) = sim_metrics["sim_stability"].as_f64() {
            merged_vars.push(axagent_harness::workflow_types::Variable {
                name: "sim_stability".into(),
                var_type: "number".into(),
                value: serde_json::json!(stab),
                description: Some("市场模拟：价格稳定性(0~1, 越高越稳定)".into()),
                is_secret: false,
            });
        }
        if let Some(liq) = sim_metrics["sim_liquidity"].as_f64() {
            merged_vars.push(axagent_harness::workflow_types::Variable {
                name: "sim_liquidity".into(),
                var_type: "number".into(),
                value: serde_json::json!(liq),
                description: Some("市场模拟：流动性深度(0~1, 越高流动性越好)".into()),
                is_secret: false,
            });
        }
        if let Some(impact) = sim_metrics["sim_impact"].as_f64() {
            merged_vars.push(axagent_harness::workflow_types::Variable {
                name: "sim_impact".into(),
                var_type: "number".into(),
                value: serde_json::json!(impact),
                description: Some("市场模拟：大单冲击成本(bps)".into()),
                is_secret: false,
            });
        }
        // ── P1-E13: 注入组合风控门所需的持仓/现金/行业变量 ──
        // portfolio-risk-gate CodeNode 读取这些变量做组合层约束检查
        {
            use axagent_entities::portfolio_holdings;
            // 1. 查询当前持仓，构造 PositionSummary JSON
            //    用 avg_cost 估值 market_value（避免逐个调 get_quote 造成延迟）
            let holdings = portfolio_holdings::Entity::find()
                .all(&db)
                .await
                .unwrap_or_default();
            let holdings_json: Vec<serde_json::Value> = holdings
                .iter()
                .map(|h| {
                    let mv = h.shares * h.avg_cost;
                    serde_json::json!({
                        "stockCode": h.stock_code,
                        "stockName": h.stock_name,
                        "totalShares": h.shares as i32,
                        "avgCost": h.avg_cost,
                        "currentPrice": h.avg_cost,
                        "marketValue": mv,
                        "unrealizedPnl": 0.0,
                        "unrealizedPnlPct": 0.0,
                        "totalRealizedPnl": 0.0,
                        "sectorName": null,
                    })
                })
                .collect();
            let holdings_json_str =
                serde_json::to_string(&holdings_json).unwrap_or_else(|_| "[]".into());
            merged_vars.push(axagent_harness::workflow_types::Variable {
                name: "holdings_json".into(),
                var_type: "string".into(),
                value: serde_json::Value::String(holdings_json_str),
                description: Some("当前持仓 JSON 数组（供组合风控门检查仓位/行业暴露）".into()),
                is_secret: false,
            });
            // 2. portfolio_cash（暂时注入 0.0，后续可扩展为从账户设置读取）
            merged_vars.push(axagent_harness::workflow_types::Variable {
                name: "portfolio_cash".into(),
                var_type: "number".into(),
                value: serde_json::json!(0.0),
                description: Some("可用现金（供组合风控门计算组合总价值）".into()),
                is_secret: false,
            });
            // 3. stock_sector（当前股票的申万一级行业，供行业暴露检查）
            //    astock_for_sector 已在 spawn 前克隆，避免借用 state（生命周期安全）
            let stock_sector = astock_for_sector
                .get_sector_info(&stock_code)
                .await
                .ok()
                .flatten()
                .map(|s| s.sector_name)
                .unwrap_or_default();
            if !stock_sector.is_empty() {
                merged_vars.push(axagent_harness::workflow_types::Variable {
                    name: "stock_sector".into(),
                    var_type: "string".into(),
                    value: serde_json::Value::String(stock_sector.clone()),
                    description: Some(
                        "当前股票的申万一级行业（供组合风控门检查行业暴露）".into(),
                    ),
                    is_secret: false,
                });
            }
            tracing::info!(
                "[stock-analysis] P1-E13 注入: holdings={}条, sector={}",
                holdings.len(),
                if stock_sector.is_empty() { "(空)" } else { &stock_sector }
            );
        }
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
        //
        // P2-F15 切入点 3：fetch_stock_lessons 同时返回被引用的 lesson_ids，
        // 在此批量写入 lesson_applications 表，用于后续 run_lesson_validation
        // 精确统计 times_applied / success_count（替代旧的模糊匹配）。
        let (lessons_str, applied_lesson_ids) = fetch_stock_lessons(&stock_code, &db).await;
        let default_lessons = "（暂无历史反思）".to_string();
        let lessons_val = lessons_str.unwrap_or_else(|| default_lessons.clone());
        merged_vars.push(axagent_harness::workflow_types::Variable {
            name: "stock_lessons".into(),
            var_type: "string".into(),
            value: serde_json::Value::String(lessons_val.clone()),
            description: Some("该股历史反思教训（错因/被忽视信号/改进建议）".into()),
            is_secret: false,
        });
        // P2-F15: 批量写入 lesson_applications（失败不阻塞主流程）
        if !applied_lesson_ids.is_empty() {
            record_lesson_applications(
                &db,
                &applied_lesson_ids,
                &analysis_id_for_spawn,
                &stock_code,
            )
            .await;
        }
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

        // P0-T5: 整体超时兜底。step_timeout 只限单步，多步累计可能很久；
        // 超时后调 cancel_workflow 让 WorkEngine 协作取消，避免分析永久挂起。
        // H4.2 修复：同时监听 shutdown_token，应用关闭时主动取消工作流，
        // 避免 AppState::drop 后后台任务仍阻塞 run_workflow 等待 LLM 响应。
        // shutdown 与超时走相同分支（Err(Elapsed)），DB 状态更新为 "timeout" 语义等价。
        let timeout_future = tokio::time::timeout(
            total_timeout,
            engine.run_workflow(&wf_id, opts),
        );
        let workflow_result = tokio::select! {
            biased;
            _ = shutdown_token.cancelled() => {
                tracing::info!(%wf_id, "shutdown_token 触发，主动取消工作流");
                let _ = engine.cancel_workflow(&wf_id).await;
                // shutdown 等价于超时，用 zero-duration timeout 构造 Err(Elapsed)
                // 让下游 match 走 Err(_elapsed) 分支（更新 DB 状态为 timeout）
                tokio::time::timeout(
                    std::time::Duration::from_nanos(1),
                    std::future::pending::<
                        Result<
                            axagent_rt_workflow::workflow_engine::Workflow,
                            axagent_rt_workflow::workflow_engine::WorkflowError,
                        >,
                    >(),
                )
                .await
            },
            r = timeout_future => r,
        };

        match workflow_result {
            // 超时分支：主动取消工作流 + 更新 DB 状态为 timeout
            Err(_elapsed) => {
                tracing::warn!(%wf_id, "工作流总超时，主动取消");
                let _ = engine.cancel_workflow(&wf_id).await;
                let _ = app_h.emit(
                    "workflow-error",
                    serde_json::json!({
                        "workflowId": wf_id,
                        "error": format!("分析超时（超过 {} 秒）", total_timeout.as_secs())
                    }),
                );
                if let Err(db_e) = stock_analyses::Entity::update_many()
                    .col_expr(stock_analyses::Column::Status, Expr::value("timeout"))
                    .col_expr(
                        stock_analyses::Column::UpdatedAt,
                        Expr::value(chrono::Utc::now().timestamp_millis()),
                    )
                    .filter(stock_analyses::Column::Id.eq(&aid))
                    .exec(&db)
                    .await
                {
                    tracing::error!("[DB] timeout 状态更新失败: {db_e}");
                }
                // 版本化模式：保留失败记录供复盘，不删除
            },
            Ok(inner_result) => match inner_result {
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
                        // 版本化模式：保留取消记录供复盘，不删除
                    },
                    axagent_rt_workflow::workflow_engine::WorkflowStatus::Failed => {
                        tracing::warn!(%wf_id, status=?wf_status, "工作流以 Failed 状态结束，保存部分结果");
                        // 构建 DashboardReport（与 rerun_decision 复用同一逻辑），让前端
                        // 在工作流部分失败时也能展示已收集到的分析结果，避免 dashboard tab
                        // 永远显示空态。
                        let analysis_date_str = as_of::current_as_of()
                            .map(|c| c.as_string())
                            .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
                        let dashboard_payload = build_dashboard_from_workflow_result(
                            &result,
                            &stock_code,
                            &sc_name_for_spawn,
                            &analysis_date_str,
                        );
                        let (dashboard_report, dashboard_md) = match dashboard_payload {
                            Some((r, m)) => (json!(r), json!(m)),
                            None => (serde_json::Value::Null, serde_json::Value::Null),
                        };
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
                                Expr::value(expected_holding_days),
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
                        // 触发自适应闭环（降级模式，异步执行）
                        {
                            let engine = Arc::clone(&adaptive_engine_for_spawn);
                            let stock_code_clone = stock_code.clone();
                            let aid_clone = aid.clone();
                            let wf_id_clone = wf_id.clone();
                            let result_clone = result.clone();
                            tokio::spawn(async move {
                                trigger_adaptive_cycle(
                                    &engine,
                                    &stock_code_clone,
                                    &aid_clone,
                                    &wf_id_clone,
                                    &result_clone,
                                    "hold",
                                    0.5,
                                    "降级模式：部分分析步骤失败",
                                ).await;
                            });
                        }
                        // 版本化模式：不再删旧行/改 ID，直接用新行 ID emit
                        if let Err(e) = app_h.emit(
                            "workflow-completed",
                            serde_json::json!({
                                "workflowId": wf_id,
                                "results": result.results,
                                "output": result.output,
                                "degraded": true,
                                "degradationReason": "部分分析步骤失败，结果为部分数据",
                                "dashboardReport": dashboard_report,
                                "dashboardMd": dashboard_md,
                            }),
                        ) {
                            tracing::warn!("[emit] workflow-completed 发送失败: {e}");
                        }
                    },
                    _ => {
                        // 构建 DashboardReport（与 rerun_decision 复用同一逻辑），让前端
                        // dashboardReport 在工作流正常完成时立即填充，避免概览/仪表板 tab
                        // 显示空态("No dashboard report yet")。
                        let analysis_date_str = as_of::current_as_of()
                            .map(|c| c.as_string())
                            .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string());
                        let dashboard_payload = build_dashboard_from_workflow_result(
                            &result,
                            &stock_code,
                            &sc_name_for_spawn,
                            &analysis_date_str,
                        );
                        let (dashboard_report, dashboard_md) = match dashboard_payload {
                            Some((r, m)) => (json!(r), json!(m)),
                            None => (serde_json::Value::Null, serde_json::Value::Null),
                        };
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
                            // V65: 6 维度版分歧诊断
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
                                format!(
                                    "🤝双视角一致:{}分(维度:act={} pos={} conf={} risk={} gaps={} evid={}){}",
                                    ab.total, ab.action_score as i32, ab.position_score as i32,
                                    ab.confidence_score as i32, ab.risk_level_score as i32,
                                    ab.data_gaps_score as i32, ab.evidence_score as i32,
                                    f7_note
                                )
                            } else if ab.total >= 40 {
                                format!(
                                    "⚠️双视角部分一致:{}分(维度:act={} pos={} conf={} risk={} gaps={} evid={}){}",
                                    ab.total, ab.action_score as i32, ab.position_score as i32,
                                    ab.confidence_score as i32, ab.risk_level_score as i32,
                                    ab.data_gaps_score as i32, ab.evidence_score as i32,
                                    f7_note
                                )
                            } else {
                                // P0: f7 纯净版 action 一致性对比
                                let f7_free_note = match (ab.f7_free_action.as_deref(), ab.f7_free_action_score) {
                                    (Some(fa), Some(fs)) if *fa != ab.formula_action =>
                                        format!("(无f7={}/{})", fa, fs as i32),
                                    _ => String::new(),
                                };
                                format!(
                                    "🔴双视角分歧:{}分(公式{} vs LLM{},维度:act={} pos={} conf={} risk={} gaps={} evid={}){}{}",
                                    ab.total, ab.formula_action, ab.llm_action,
                                    ab.action_score as i32, ab.position_score as i32,
                                    ab.confidence_score as i32, ab.risk_level_score as i32,
                                    ab.data_gaps_score as i32, ab.evidence_score as i32,
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
                                        // V65: 完整 6 维度诊断结构体
                                        obj.insert("agreementBreakdown".into(), serde_json::json!({
                                            "total": ab.total,
                                            "actionOk": ab.action_ok,
                                            "actionNote": ab.action_note,
                                            "formulaAction": ab.formula_action,
                                            "llmAction": ab.llm_action,
                                            "actionScore": ab.action_score,
                                            "positionScore": ab.position_score,
                                            "positionGap": ab.position_gap,
                                            "confidenceScore": ab.confidence_score,
                                            "confidenceGap": ab.confidence_gap,
                                            // V65 新增维度
                                            "riskLevelScore": ab.risk_level_score,
                                            "formulaRiskLevel": ab.formula_risk_level,
                                            "llmRiskLevel": ab.llm_risk_level,
                                            "dataGapsScore": ab.data_gaps_score,
                                            "dataGapsSimilarity": ab.data_gaps_similarity,
                                            "evidenceScore": ab.evidence_score,
                                            "evidenceCount": ab.evidence_count,
                                            "conflictType": ab.conflict_type,
                                            // P0: f7 自指污染标记（向后兼容）
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
                        // 修复 M-RES-15: 原实现用 block_in_place + block_on 嵌套异步，
                        // 在 Tauri 异步上下文中可能导致死锁或性能问题。改为直接 .await。
                        let exit_urgency = {
                            use axagent_entities::portfolio_holdings;
                            let holding = portfolio_holdings::Entity::find()
                                .filter(portfolio_holdings::Column::StockCode.eq(&stock_code))
                                .one(&db).await.ok().flatten();
                            // 提取当前分析决策
                            let action_str = decision_json.as_deref()
                                .and_then(|dj| serde_json::from_str::<serde_json::Value>(dj).ok())
                                .and_then(|v| v.get("action").and_then(|a| a.as_str()).map(String::from));
                            match action_str.as_deref() {
                                Some("卖出") => Some(90.0),   // 高紧迫卖出
                                Some("减持") => Some(60.0),   // 中紧迫减持
                                Some("观望") if holding.as_ref().map(|h| h.shares > 0.0).unwrap_or(false) => Some(30.0), // 低紧迫（不增持）
                                _ => None,                     // 持有/买入 → 不触发退出
                            }
                        };
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
                                Expr::value(expected_holding_days),
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

                        // P0: 决策落库后自动创建价格告警（targetPrice/stopLoss → price_alerts 表）
                        // 用户做完分析后无需手动设告警，决策结论自动变成可执行的价格触发器。
                        // 仅对含方向性动作（买入/增持/持有/减持/卖出）的决策创建告警；
                        // 观望/skip 不创建。targetPrice → above 告警，stopLoss → below 告警。
                        if let Some(ref dj_str) = mem_dj {
                            if let Ok(dj_val) = serde_json::from_str::<serde_json::Value>(dj_str) {
                                let dj_action = dj_val
                                    .get("action")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let should_create_alert = matches!(
                                    dj_action,
                                    "买入" | "增持" | "持有" | "减持" | "卖出"
                                );
                                if should_create_alert {
                                    let target_price = dj_val
                                        .get("targetPrice")
                                        .and_then(|v| v.as_f64());
                                    let stop_loss = dj_val
                                        .get("stopLoss")
                                        .and_then(|v| v.as_f64());
                                    let now_ms = chrono::Utc::now().timestamp_millis();

                                    // 创建止盈告警（above targetPrice → alert_type=take_profit）
                                    if let Some(tp) = target_price {
                                        if tp > 0.0 {
                                            let alert_id = uuid::Uuid::new_v4().to_string();
                                            let alert_model = price_alerts::ActiveModel {
                                                id: Set(alert_id),
                                                stock_code: Set(stock_code.clone()),
                                                stock_name: Set(sc_name_for_spawn.clone()),
                                                condition: Set("above".into()),
                                                target_price: Set(tp),
                                                alert_type: Set(Some(
                                                    "take_profit".into(),
                                                )),
                                                condition_type: Set(Some("price".into())),
                                                threshold: Set(Some(tp)),
                                                is_triggered: Set(0),
                                                triggered_at: Set(None),
                                                created_at: Set(now_ms),
                                                updated_at: Set(now_ms),
                                            };
                                            if let Ok(inserted) =
                                                alert_model.insert(&db).await
                                            {
                                                // 同步加入 RealtimeMonitor
                                                if let Some(ref monitor) = monitor_for_spawn {
                                                    use axagent_analysis_engine::monitor::MonitorConfig;
                                                    let config = MonitorConfig {
                                                        stock_code: stock_code.clone(),
                                                        stock_name: sc_name_for_spawn.clone(),
                                                        stop_loss: None,
                                                        take_profit: Some(tp),
                                                        resistance_break: None,
                                                        support_break: None,
                                                        change_pct_alert: None,
                                                        turnover_rate_alert: None,
                                                        enabled: true,
                                                    };
                                                    monitor.add_config(config).await;
                                                }
                                                tracing::info!(
                                                    "[auto_price_alert] 已创建止盈告警: {} {} above {:.2} (id={})",
                                                    stock_code, sc_name_for_spawn, tp, inserted.id
                                                );
                                            }
                                        }
                                    }

                                    // 创建止损告警（below stopLoss → alert_type=stop_loss）
                                    if let Some(sl) = stop_loss {
                                        if sl > 0.0 {
                                            let alert_id = uuid::Uuid::new_v4().to_string();
                                            let alert_model = price_alerts::ActiveModel {
                                                id: Set(alert_id),
                                                stock_code: Set(stock_code.clone()),
                                                stock_name: Set(sc_name_for_spawn.clone()),
                                                condition: Set("below".into()),
                                                target_price: Set(sl),
                                                alert_type: Set(Some(
                                                    "stop_loss".into(),
                                                )),
                                                condition_type: Set(Some("price".into())),
                                                threshold: Set(Some(sl)),
                                                is_triggered: Set(0),
                                                triggered_at: Set(None),
                                                created_at: Set(now_ms),
                                                updated_at: Set(now_ms),
                                            };
                                            if let Ok(inserted) =
                                                alert_model.insert(&db).await
                                            {
                                                // 同步加入 RealtimeMonitor
                                                if let Some(ref monitor) = monitor_for_spawn {
                                                    use axagent_analysis_engine::monitor::MonitorConfig;
                                                    let config = MonitorConfig {
                                                        stock_code: stock_code.clone(),
                                                        stock_name: sc_name_for_spawn.clone(),
                                                        stop_loss: Some(sl),
                                                        take_profit: None,
                                                        resistance_break: None,
                                                        support_break: None,
                                                        change_pct_alert: None,
                                                        turnover_rate_alert: None,
                                                        enabled: true,
                                                    };
                                                    monitor.add_config(config).await;
                                                }
                                                tracing::info!(
                                                    "[auto_price_alert] 已创建止损告警: {} {} below {:.2} (id={})",
                                                    stock_code, sc_name_for_spawn, sl, inserted.id
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // P2-1: 决策事件总线 — 发布 decision.completed 事件
                        // 订阅该事件的工作流（如 auto-position-plan / auto-stop-loss-review）会被自动触发。
                        // publish_event 内部会调用 engine.run_workflow，失败仅 warn 不阻塞主流程。
                        // 事件 payload 设计：包含完整决策上下文，订阅方可按需取用。
                        {
                            let event_payload = serde_json::json!({
                                "analysisId": aid,
                                "stockCode": stock_code,
                                "stockName": sc_name_for_spawn,
                                "action": mem_action,
                                "decisionJson": mem_dj.as_ref().and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()).unwrap_or(serde_json::Value::Null),
                                "asOfDate": as_of::current_as_of().map(|c| c.as_string()),
                                "parentAnalysisId": parent_for_record,
                                "timestamp": chrono::Utc::now().timestamp_millis(),
                            });
                            // spawn 避免阻塞主流程：publish_event 内部可能触发多个工作流
                            let mgr = trigger_mgr_for_spawn.clone();
                            let payload_clone = event_payload.clone();
                            tokio::spawn(async move {
                                let triggered = mgr.publish_event("decision.completed", payload_clone).await;
                                if !triggered.is_empty() {
                                    tracing::info!(
                                        "[decision_bus] decision.completed 已触发 {} 个工作流: {:?}",
                                        triggered.len(), triggered
                                    );
                                }
                            });
                            // 同时 emit 前端事件，让 UI 也能感知决策事件总线活动（可选）
                            let _ = app_h.emit("decision-completed", event_payload);
                        }

                        // 索引决策到 Memory RAG（best-effort，失败不阻塞）
                        // 版本化模式：直接使用 aid（新行 ID 稳定，不会变更）
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


                        // 触发自适应闭环（异步，不阻塞主流程）
                        // 工作流完成后自动执行反思→诊断→进化→验证→应用
                        {
                            let decision_text = mem_action.as_deref().unwrap_or("hold").to_string();
                            let confidence_val = mem_dj
                                .as_ref()
                                .and_then(|dj| serde_json::from_str::<serde_json::Value>(dj).ok())
                                .and_then(|v| v.get("confidence").cloned())
                                .and_then(|c| c.as_f64())
                                .unwrap_or(0.7) as f32;
                            let rationale_text = mem_reasoning.as_deref().unwrap_or("").to_string();
                            let engine = Arc::clone(&adaptive_engine_for_spawn);
                            let stock_code_clone = stock_code.clone();
                            let aid_clone = aid.clone();
                            let wf_id_clone = wf_id.clone();
                            let result_clone = result.clone();
                            tokio::spawn(async move {
                                trigger_adaptive_cycle(
                                    &engine,
                                    &stock_code_clone,
                                    &aid_clone,
                                    &wf_id_clone,
                                    &result_clone,
                                    &decision_text,
                                    confidence_val,
                                    &rationale_text,
                                ).await;
                            });
                        }
                        // DB 写入完成后再 emit，避免前端 extract_evidence_citations 读到空数据
                        if let Err(e) = app_h.emit(
                            "workflow-completed",
                            serde_json::json!({
                                "workflowId": wf_id,
                                "results": result.results,
                                "output": result.output,
                                "dashboardReport": dashboard_report,
                                "dashboardMd": dashboard_md,
                            }),
                        ) {
                            tracing::warn!("[emit] workflow-completed 发送失败: {e}");
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
                // 版本化模式：保留错误记录供复盘，不删除
            },
        } // end inner match (Ok(result) / Err(e))
        }}).await  // end outer match + async block + with_degradation_log
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
#[agent_command(domain = invest, safety = Caution, call_mode = StateInput, description = "取消正在运行的股票分析工作流")]
#[tauri::command]
pub async fn cancel_stock_workflow(
    state: State<'_, AppState>,
    workflow_id: String,
) -> Result<(), String> {
    state.work_engine.cancel_workflow(&workflow_id).await.map(|_| ()).map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("取消工作流失败: {e}")).to_string()
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
#[allow(dead_code)]
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
        parent_analysis_id: Set(None),
        trade_intent_status: Set("pending".into()),
        trade_intent_source: Set(None),
        trade_intent_source_ref_id: Set(None),
        trade_intent_reviewed_at: Set(None),
        trade_intent_reviewed_by: Set(None),
        trade_intent_review_notes: Set(None),
        trade_intent_actual_trade_id: Set(None),
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
        QualityPrecheckResult::Insufficient { summary, missing_sources } => {
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
    let (max_concurrent, step_timeout, _total_timeout) =
        resolve_runtime_options(loaded.variables.as_deref());

    // 5.5 [A1 借鉴] 注入历史反思教训(TradingAgents past_context 机制):
    //   批量/定时分析场景下,trader/research-mgr/value-investor 节点能看到
    //   该股最近 90 天的反思教训(lesson_summary),避免重蹈覆辙。前端触发场景下
    //   run_stock_workflow_inner 同样会注入,这里是补齐 cron / batch 入口。
    //   必须始终注入,即使为空（否则 VARIABLE_NOT_FOUND）。
    //
    //   P2-F15 切入点 3：同时收集被引用的 lesson_ids，写入 lesson_applications 表。
    let (lessons_str, applied_lesson_ids) = fetch_stock_lessons(stock_code, db).await;
    let default_lessons = "（暂无历史反思）".to_string();
    let lessons_val = lessons_str.unwrap_or_else(|| default_lessons.clone());
    // P2-F15: 批量写入 lesson_applications（失败不阻塞主流程）
    if !applied_lesson_ids.is_empty() {
        record_lesson_applications(db, &applied_lesson_ids, &analysis_id, stock_code).await;
    }
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
    let workflow =
        engine.create_workflow(&wf_name, loaded.nodes, loaded.edges).await.map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("创建工作流失败: {e}"))
        })?;
    let wf_id = workflow.id.clone();

    let opts = RunOptions {
        max_concurrent,
        step_timeout,
        // 按类型并发上限对齐全局 max_concurrent，修复默认 llm=3 覆盖用户设定
        max_concurrent_by_type: Some({
            let mut m = std::collections::HashMap::new();
            m.insert("tool".into(), 10usize);
            m.insert("file".into(), 10usize);
            m.insert("llm".into(), max_concurrent);
            m.insert("agent".into(), max_concurrent);
            m
        }),
        // 从模板变量读取工具节点超时
        tool_timeout: std::time::Duration::from_secs(
            loaded
                .variables
                .as_ref()
                .and_then(|vars| {
                    vars.iter()
                        .find(|v| v.name == "tool_timeout_secs")
                        .and_then(|v| v.value.as_u64())
                })
                .map(|s| std::cmp::max(s, 5))
                .unwrap_or(30),
        ),
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
            // 提取 expected_holding_days（在 decision_json_str 被 move 之前）
            let (_, _, _, _, expected_holding_days) = extract_decision_fields(&decision_json_str);
            let decision_output = decision_json_str
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

            let decision_action = decision_output
                .as_ref()
                .and_then(|d| d.get("action").and_then(|a| a.as_str().map(|s| s.to_string())));

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
            //
            // [时间旅行模式] hindsight_date = analysis_date + expected_holding_days
            //   反思评估时点由决策的期望持有期决定，而非固定"今天"。
            //   批量反思任务在 hindsight_date 到达时才执行，并以 hindsight_date
            //   作为 AS_OF 锚点查看"截至评估时点的实际走势"。
            //   - expected_holding_days 缺失时默认 28 天（与批量任务默认值一致）
            //   - 若计算出的 hindsight_date 在未来，批量任务会 skip 直到日期到达
            let pending_id = uuid::Uuid::new_v4().to_string();
            let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let hindsight_date_str = {
                let hold_days = expected_holding_days.unwrap_or(28) as i64;
                let analysis_naive = chrono::NaiveDate::parse_from_str(&today_str, "%Y-%m-%d")
                    .unwrap_or_else(|_| chrono::Local::now().date_naive());
                let h = analysis_naive + chrono::Duration::days(hold_days);
                h.format("%Y-%m-%d").to_string()
            };
            let _ = stock_reflections::ActiveModel {
                id: Set(pending_id.clone()),
                stock_code: Set(stock_code.to_string()),
                stock_name: Set(stock_name.to_string()),
                original_analysis_id: Set(analysis_id.clone()),
                as_of_date: Set(today_str.clone()),
                hindsight_date: Set(hindsight_date_str),
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
    let three_months_ago =
        (chrono::Utc::now() - chrono::Duration::days(90)).format("%Y-%m-%d").to_string();
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
) -> (Option<String>, Vec<String>) {
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
        // 仍需查询规则化教训（可能存在）
        return fetch_rule_lessons(stock_code, db).await;
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

    // 追加规则化教训 + 收集被引用的 lesson_ids
    let (rule_lines, lesson_ids) = fetch_rule_lessons(stock_code, db).await;
    if let Some(rule_text) = rule_lines {
        lines.push(rule_text);
    }

    let text = if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    };
    (text, lesson_ids)
}

/// 查询规则化教训（reflection_lessons 表）
///
/// 返回 `(Option<String>, Vec<String>)`：
/// - `Option<String>`：拼好的规则化教训文本（无则 None）
/// - `Vec<String>`：被引用的 lesson_id 列表（供调用方写入 lesson_applications）
///
/// P2-F15 切入点 3：被引用的 lesson_ids 会写入 lesson_applications 表，
/// 用于后续 run_lesson_validation 精确统计 times_applied / success_count。
async fn fetch_rule_lessons(
    stock_code: &str,
    db: &sea_orm::DatabaseConnection,
) -> (Option<String>, Vec<String>) {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

    // ── [F1 闭环] 规则化教训：从 reflection_lessons 表查询 ──
    // 修复首轮分析发现的"reflection_lessons 闭环断裂"问题：
    // extract_lesson_to_rule 会把高质量 lesson_summary 写入 reflection_lessons,
    // 但原 fetch_stock_lessons 只查 stock_reflections,规则化教训永远不被消费。
    // 现补充查询 reflection_lessons 表的规则化教训（按 confidence 降序取前 5 条）。
    use axagent_entities::reflection_lessons;
    let rule_lessons: Vec<reflection_lessons::Model> = reflection_lessons::Entity::find()
        .filter(reflection_lessons::Column::StockCode.eq(stock_code))
        .filter(reflection_lessons::Column::Confidence.gte(0.3)) // 过滤低质量/已废弃规则
        .order_by_desc(reflection_lessons::Column::Confidence)
        .all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .take(5)
        .collect();

    if rule_lessons.is_empty() {
        return (None, Vec::new());
    }

    // P2-F15: 收集被引用的 lesson_ids 供调用方写入 lesson_applications
    let lesson_ids: Vec<String> = rule_lessons.iter().map(|l| l.id.clone()).collect();

    let mut lines: Vec<String> = Vec::new();
    lines.push(String::new());
    lines.push(format!("【规则化教训 {} 条(按置信度降序)】", rule_lessons.len()));
    for (i, l) in rule_lessons.iter().enumerate() {
        lines.push(format!(
            "#{} (confidence={:.2}, 应用{}次/成功{}次): {}",
            i + 1,
            l.confidence,
            l.times_applied,
            l.success_count,
            l.lesson_summary
        ));
        if let Some(ref p) = l.rule_pattern {
            lines.push(format!("  - 触发条件：{}", p));
        }
    }

    (Some(lines.join("\n")), lesson_ids)
}

/// P2-F15 切入点 3：批量写入 lesson_applications 关联表
///
/// 在 `fetch_stock_lessons` 注入规则化教训到分析上下文后调用，记录"本次决策分析
/// 引用了哪些 lesson"。后续 `run_lesson_validation` 据此精确统计
/// `times_applied` / `success_count`，替代旧的 `lesson_summary.contains()`
/// 模糊匹配。
///
/// ## 幂等性
/// 按 `(lesson_id, analysis_id)` 复合唯一性去重：同一条 lesson 在同一个
/// analysis 中只记录一次。实现上用 `INSERT OR IGNORE` 语义（先 SELECT
/// 再 INSERT，存在则跳过），避免重复分析场景下产生重复行。
///
/// ## 错误处理
/// 单条插入失败不阻塞主流程，仅记录 warn 日志。理由：lesson_applications
/// 是追踪表，写入失败只影响后续验证精度，不应让股票分析主流程失败。
///
/// ## 参数
/// - `db`: 数据库连接
/// - `lesson_ids`: 被引用的 reflection_lessons.id 列表
/// - `analysis_id`: 本次决策分析的 stock_analyses.id
/// - `stock_code`: 股票代码（冗余字段，便于按股票维度查询）
pub(crate) async fn record_lesson_applications(
    db: &sea_orm::DatabaseConnection,
    lesson_ids: &[String],
    analysis_id: &str,
    stock_code: &str,
) {
    use axagent_entities::lesson_applications;
    use axagent_entities::reflection_lessons;
    use sea_orm::ExprTrait;
    use sea_orm::sea_query::Expr;

    let now_rfc3339 = chrono::Utc::now().to_rfc3339();
    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut inserted = 0usize;
    let mut skipped = 0usize;

    for lesson_id in lesson_ids {
        // 幂等：先查 (lesson_id, analysis_id) 是否已存在
        let existing = lesson_applications::Entity::find()
            .filter(lesson_applications::Column::LessonId.eq(lesson_id.as_str()))
            .filter(lesson_applications::Column::AnalysisId.eq(analysis_id))
            .one(db)
            .await;

        match existing {
            Ok(Some(_)) => {
                skipped += 1;
                continue;
            },
            Ok(None) => {
                // 不存在，继续插入
            },
            Err(e) => {
                tracing::warn!(
                    "[lesson_applications] 查询 (lesson_id={lesson_id}, analysis_id={analysis_id}) 失败: {e}, 跳过"
                );
                continue;
            },
        }

        let active = lesson_applications::ActiveModel {
            id: Set(uuid::Uuid::new_v4().to_string()),
            lesson_id: Set(lesson_id.clone()),
            analysis_id: Set(analysis_id.to_string()),
            stock_code: Set(stock_code.to_string()),
            applied_at: Set(now_rfc3339.clone()),
            outcome_at_validation: Set(None),
            validation_source: Set(None),
            created_at: Set(now_ms),
        };

        match lesson_applications::Entity::insert(active).exec(db).await {
            Ok(_) => inserted += 1,
            Err(e) => {
                tracing::warn!(
                    "[lesson_applications] 插入 (lesson_id={lesson_id}, analysis_id={analysis_id}) 失败: {e}"
                );
            },
        }
    }

    // 顺带同步 reflection_lessons.times_applied（+1）
    // 理由：times_applied 是冗余字段，lesson_applications 才是权威来源。
    //       但 fetch_rule_lessons 按 confidence 排序时仍需读 times_applied，
    //       保持同步避免显示陈旧数据。
    for lesson_id in lesson_ids {
        let _ = reflection_lessons::Entity::update_many()
            .col_expr(
                reflection_lessons::Column::TimesApplied,
                Expr::col(reflection_lessons::Column::TimesApplied).add(1),
            )
            .col_expr(reflection_lessons::Column::UpdatedAt, Expr::value(now_ms))
            .filter(reflection_lessons::Column::Id.eq(lesson_id.as_str()))
            .exec(db)
            .await;
    }

    tracing::info!(
        "[lesson_applications] analysis_id={analysis_id} stock={stock_code}: 插入 {inserted} 条, 跳过 {skipped} 条"
    );
}

/// P2-F15 切入点 3：T+N 验证完成后回写 lesson_applications.outcome_at_validation
///
/// 当某条决策分析（`stock_analyses.id`）的 outcome 被确定后调用，更新所有
/// 引用了该 analysis 的 `lesson_applications` 行的 `outcome_at_validation`
/// 和 `validation_source` 字段。后续 `run_lesson_validation` 据此精确统计
/// `success_count`。
///
/// ## 调用时机
/// - `run_decision_backtest` 完成 T+N 验证后（通过 stock_code + 日期反推 analysis_id）
/// - 手动标注 outcome 后（validation_source = "manual"）
/// - 未来 outcome 链路打通后的任何更新点
///
/// ## 参数
/// - `db`: 数据库连接
/// - `analysis_id`: 决策分析 ID（stock_analyses.id）
/// - `outcome`: "win" / "loss"
/// - `validation_source`: "t_plus_5" / "t_plus_20" / "t_plus_60" / "manual"
///
/// ## 返回
/// 更新的行数（0 表示无匹配行，即该 analysis 没有引用任何 lesson）
pub(crate) async fn update_lesson_application_outcome(
    db: &sea_orm::DatabaseConnection,
    analysis_id: &str,
    outcome: &str,
    validation_source: &str,
) -> u64 {
    use axagent_entities::lesson_applications;
    use sea_orm::sea_query::Expr;

    let result = lesson_applications::Entity::update_many()
        .col_expr(
            lesson_applications::Column::OutcomeAtValidation,
            Expr::value(outcome),
        )
        .col_expr(
            lesson_applications::Column::ValidationSource,
            Expr::value(validation_source),
        )
        .filter(lesson_applications::Column::AnalysisId.eq(analysis_id))
        // 只更新尚未回写的行，避免覆盖已验证结果
        .filter(lesson_applications::Column::OutcomeAtValidation.is_null())
        .exec(db)
        .await;

    match result {
        Ok(r) => {
            tracing::info!(
                "[lesson_applications] analysis_id={analysis_id} outcome={outcome} source={validation_source}: 更新 {} 行",
                r.rows_affected
            );
            r.rows_affected
        },
        Err(e) => {
            tracing::warn!(
                "[lesson_applications] 更新 outcome 失败 analysis_id={analysis_id}: {e}"
            );
            0
        },
    }
}

/// P2-F15 切入点 3：同步 lesson_applications.outcome_at_validation
///
/// 扫描所有 `outcome_at_validation IS NULL` 的 `lesson_applications` 行，
/// 根据其 `analysis_id` 查 `stock_analyses.outcome`，如果 outcome 已被设置
/// （win/loss），则回写 `lesson_applications.outcome_at_validation`。
///
/// ## 调用时机
/// 在 `run_lesson_validation` 开始前调用，确保尽可能多的 lesson_applications
/// 行有 outcome 数据，提高 success_count 统计精度。
///
/// ## 返回
/// 回写的行数
pub(crate) async fn sync_lesson_application_outcomes(db: &sea_orm::DatabaseConnection) -> u64 {
    use axagent_entities::lesson_applications;
    use axagent_entities::stock_analyses;

    // 1. 查所有 outcome_at_validation IS NULL 的行
    let pending_apps: Vec<lesson_applications::Model> = lesson_applications::Entity::find()
        .filter(lesson_applications::Column::OutcomeAtValidation.is_null())
        .all(db)
        .await
        .unwrap_or_default();

    if pending_apps.is_empty() {
        return 0;
    }

    // 2. 收集所有 analysis_id，批量查 stock_analyses.outcome
    let analysis_ids: Vec<String> = pending_apps
        .iter()
        .map(|a| a.analysis_id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let analyses: Vec<stock_analyses::Model> = stock_analyses::Entity::find()
        .filter(stock_analyses::Column::Id.is_in(analysis_ids))
        .filter(stock_analyses::Column::Outcome.is_not_null())
        .all(db)
        .await
        .unwrap_or_default();

    if analyses.is_empty() {
        return 0;
    }

    // 3. 构建 analysis_id → outcome 映射
    let outcome_map: std::collections::HashMap<String, String> =
        analyses.iter().filter_map(|a| a.outcome.clone().map(|o| (a.id.clone(), o))).collect();

    // 4. 逐行回写（只接受 win/loss，忽略其他值如 pending）
    let mut updated = 0u64;
    for (analysis_id, outcome) in &outcome_map {
        if outcome != "win" && outcome != "loss" {
            continue;
        }
        updated +=
            update_lesson_application_outcome(db, analysis_id, outcome, "stock_analyses_outcome")
                .await;
    }

    if updated > 0 {
        tracing::info!(
            "[lesson_applications] sync_outcomes: 从 stock_analyses.outcome 回写 {updated} 行"
        );
    }
    updated
}

// ── 自适应闭环集成 ──────────────────────────────────────

/// 从工作流结果构建 StockAnalysisOutcome
///
/// 将 Workflow 的执行结果映射到反思引擎所需的输入结构
fn build_outcome_from_workflow(
    workflow: &axagent_harness::workflow_types::Workflow,
    stock_code: &str,
    analysis_id: &str,
    execution_id: &str,
    decision: &str,
    confidence: f32,
    decision_rationale: &str,
) -> StockAnalysisOutcome {
    let step_results: Vec<AnalysisStepResult> = workflow
        .results
        .iter()
        .map(|(node_id, result)| {
            let node_state = workflow.node_states.get(node_id);
            let status = node_state
                .map(|s| format!("{:?}", s.status).to_lowercase())
                .unwrap_or_else(|| "unknown".to_string());
            let duration_ms = node_state
                .and_then(|s| {
                    let start = s.started_at?;
                    let end = s.completed_at?;
                    Some((end - start) as u64)
                })
                .unwrap_or(0);
            let error = if status == "failed" || status == "timeout" {
                node_state.and_then(|s| s.error.clone())
            } else {
                None
            };
            let output_summary = if result.is_string() {
                Some(result.as_str().unwrap_or("").to_string())
            } else {
                Some(serde_json::to_string(result).unwrap_or_default())
            };
            let attempts = node_state.map(|s| s.attempts).unwrap_or(1);

            AnalysisStepResult {
                step_id: node_id.clone(),
                step_name: node_id.clone(),
                node_type: "workflow_node".to_string(),
                status,
                duration_ms,
                attempts,
                error,
                output_summary,
            }
        })
        .collect();

    let success =
        matches!(workflow.status, axagent_harness::workflow_types::WorkflowStatus::Completed);

    let duration_ms =
        workflow.completed_at.zip(Some(workflow.created_at)).map(|(c, s)| c - s).unwrap_or(0);

    StockAnalysisOutcome {
        analysis_id: analysis_id.to_string(),
        stock_code: stock_code.to_string(),
        execution_id: execution_id.to_string(),
        step_results,
        decision: decision.to_string(),
        confidence,
        decision_rationale: decision_rationale.to_string(),
        signals: Vec::new(),
        success,
        error: if success {
            None
        } else {
            Some("workflow failed".to_string())
        },
        duration_ms,
    }
}

/// 异步触发自适应闭环
///
/// 在工作流完成后异步执行反思→诊断→进化→验证→应用的完整闭环，
/// 不阻塞主流程，失败仅记录日志
pub(crate) async fn trigger_adaptive_cycle(
    adaptive_engine: &Arc<axagent_analysis_engine::stock_adaptive_engine::StockAdaptiveEngine>,
    stock_code: &str,
    analysis_id: &str,
    execution_id: &str,
    workflow: &axagent_harness::workflow_types::Workflow,
    decision: &str,
    confidence: f32,
    decision_rationale: &str,
) {
    let outcome = build_outcome_from_workflow(
        workflow,
        stock_code,
        analysis_id,
        execution_id,
        decision,
        confidence,
        decision_rationale,
    );

    let engine = adaptive_engine.clone();
    let stock_code = stock_code.to_string();
    let analysis_id = analysis_id.to_string();

    // 异步执行自适应闭环，不阻塞主流程
    let result = engine.run_adaptive_cycle(&outcome).await;

    // 根据自适应状态记录日志
    let status_str = format!("{:?}", result.adaptation_status);
    tracing::info!(
        "[adaptive] 自适应闭环完成: stock={}, analysis={}, status={}, 改进={}",
        stock_code,
        analysis_id,
        status_str,
        result.improvement_summary
    );

    // 如果触发了进化，记录关键信息
    if result.evolution_result.is_some() {
        if let Some(config) = &result.applied_config {
            tracing::info!(
                "[adaptive] 新配置已应用: ewma_alpha={:.4}, lookback_days={}",
                config.ewma_alpha,
                config.lookback_days
            );
        }
    }
}
