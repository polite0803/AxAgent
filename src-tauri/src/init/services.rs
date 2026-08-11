// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::recommendation_cron::run_recommendation_cron;
use crate::index_queue::IndexJobService;
use chrono;
use notify::{Event, RecursiveMode, Watcher};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Emitter;

pub fn start_background_services(
    app: &tauri::AppHandle,
    state: &AppState,
    app_dir: std::path::PathBuf,
    _tray_language: String,
) {
    // P1-D10 收尾: 注册 portfolio-mgr.rhai 依赖的 pm_* 函数到共享 Rhai Engine。
    // 必须在任何工作流执行（cron / pipeline / batch_reflection 等）之前完成，
    // 否则 shared_rhai_engine() 已初始化后注册会被 OnceLock 拒绝（仅记 warn）。
    // 修复前: DAG 主路径未注册 pm_*, portfolio-mgr.rhai 的 pm_* 调用失败被
    // try/catch 吞掉, 决策永远走保守兜底（action="观望", confidence=0）。
    register_portfolio_mgr_rhai_functions();
    init_mcp_oauth(state);
    start_auto_backup(app, state, app_dir.clone());
    start_webdav_sync(app, state, app_dir.clone());
    #[cfg(not(mobile))]
    start_tray(app, &_tray_language);
    start_closed_loop_service(app, state);
    start_insight_generation(state);
    start_pattern_learning(state);
    start_cross_session_learning(state);
    start_rl_reward_computation(state, app_dir);
    start_batch_processing(state);
    start_user_profile_persistence(state);
    start_skill_evolution(state);
    start_dream_consolidation(state);
    start_dream_task_executor(state);
    start_coevolution_task_executor(state);
    start_pattern_analyzer_task_executor(state);
    start_insight_generator_task_executor(state);
    start_auto_tool_observation(state);
    start_text_grad_analysis(state);
    start_cron_scheduler(state);
    start_trigger_recovery(state);
    start_persistent_runner(state);
    start_platform_adapters(state);
    start_skill_watcher(app, state);
    start_memory_decay_tick(state);
    start_memory_maintenance_tick(state);
    start_retrieval_feedback_tick(state);
    start_obsidian_vaults_registration(state);
    start_knowledge_consolidation_tick(state);
    start_trajectory_cleanup(state);
    start_index_job_service(app, state);
    start_plugins(state);
    // 反思工作流定时任务（方向1：接入上游反思基础设施）
    start_batch_reflection(state);
    start_lesson_validation(state);
    // [方向6] DreamConsolidator：每日 18:00 跨股票蒸馏反思轨迹
    start_dream_consolidation(state);
    // 股票全业务管道：每日 18:00 自动发现+分析+持仓再评估
    start_stock_pipeline(app, state);
    // P3-B5(G): vendor 健康后台周期探测，加速 Degraded vendor 恢复
    start_vendor_health_prober(state);
    // P0: 启动实时监控引擎（价格告警 + T+0 异动重跑）
    start_realtime_monitor(app, state);
    // P1-2: 启动实时行情推送（替代前端 15s 轮询，2s Active / 10s Background 自适应）
    start_realtime_quote_watcher(app, state);
    // P1-3: 启动风控自动巡检（条件单评估 + 交易意图过期处理）
    start_risk_inspection(app, state);

    // G14: 注册 DojoSdkExecutor
    register_dojo_sdk_executor(state);

    // PTY 事件转发器
    #[cfg(not(mobile))]
    start_pty_event_forwarder(app, state);
}

/// 启动时批量注册 ConnectedVault 类型 KB 到全局 VaultRegistry。
///
/// 此前 `register_vault` 仅在创建/转换 KB 时调用，应用重启后
/// VaultRegistry 为空，导致 9 个 `obsidian_*` 工具全部报 `NotBound` 错误。
/// 本函数在启动后异步查询所有 `kind = connected_vault` 且 `enabled = true` 的 KB，
/// 重新注册到 VaultRegistry，修复 Obsidian 集成链路断裂问题。
fn start_obsidian_vaults_registration(state: &AppState) {
    let harness_state = state.harness.clone();
    tauri::async_runtime::spawn(async move {
        // 延迟 2 秒，确保数据库初始化完成
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let db = harness_state.db();
        match axagent_dao::repo::knowledge::list_knowledge_bases(db).await {
            Ok(all_kbs) => {
                // 用 filter_map 同时过滤 + 提取 vault_path，避免后续 unwrap() panic
                let vault_kbs: Vec<_> = all_kbs
                    .iter()
                    .filter_map(|kb| {
                        if kb.enabled && matches!(kb.kind, axagent_harness::KbKind::ConnectedVault)
                        {
                            kb.vault_path.as_ref().map(|path| (kb, path.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();

                if vault_kbs.is_empty() {
                    tracing::info!("[obsidian] 启动时未发现 ConnectedVault KB，跳过注册");
                    return;
                }

                let mut registered = 0usize;
                let mut failed = 0usize;
                for (kb, vault_path) in &vault_kbs {
                    let root = std::path::PathBuf::from(vault_path);
                    match axagent_tools::tools::obsidian::register_vault(&kb.id, root) {
                        Ok(()) => {
                            tracing::info!(
                                "[obsidian] 启动注册 ConnectedVault KB: id={} name={} vault={}",
                                kb.id,
                                kb.name,
                                vault_path
                            );
                            registered += 1;
                        },
                        Err(e) => {
                            tracing::warn!(
                                "[obsidian] 启动注册 ConnectedVault KB 失败: id={} name={} error={}",
                                kb.id,
                                kb.name,
                                e
                            );
                            failed += 1;
                        },
                    }
                }
                tracing::info!(
                    "[obsidian] 启动批量注册完成：成功 {} 个，失败 {} 个，总计 {} 个",
                    registered,
                    failed,
                    vault_kbs.len()
                );
            },
            Err(e) => {
                tracing::warn!("[obsidian] 启动时查询 knowledge_bases 失败: {}", e);
            },
        }
    });
}

/// 知识转换定时任务：定期将 Wiki/Memory 中的实体回流到知识图谱。
///
/// 解决"三套实体系统（Wiki 笔记、Memory 记忆、Knowledge 实体）各自为政"的问题：
/// 1. Wiki → Knowledge：查询所有 ConnectedVault KB，触发实体抽取（已存在的 extract_entities_from_wiki）
/// 2. Memory → Knowledge：将 Memory 中的高重要性条目转换为知识图谱实体
/// 3. 跨源实体合并：调用 merge_duplicate_entities_across_all 去重
///
/// 每 6 小时执行一次，避免频繁 LLM 调用。
/// 失败时仅记录警告，不影响主流程。
fn start_knowledge_consolidation_tick(state: &AppState) {
    let harness_state = state.harness.clone();
    tauri::async_runtime::spawn(async move {
        // 延迟 30 秒，确保所有启动任务完成
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        let interval = std::time::Duration::from_secs(6 * 3600); // 6 小时
        loop {
            tokio::time::sleep(interval).await;

            tracing::info!("[knowledge_consolidation] 开始知识转换周期");
            let started = std::time::Instant::now();

            // ── 步骤 1：跨源实体合并（轻量，纯数据库操作） ──
            match axagent_dao::repo::knowledge_graph::merge_duplicate_entities_across_all(
                harness_state.db(),
            )
            .await
            {
                Ok(result) => {
                    if result.groups_found > 0 {
                        tracing::info!(
                            "[knowledge_consolidation] 跨源实体合并：{} 个分组，{} 个实体合并，{} 个关系更新",
                            result.groups_found,
                            result.entities_merged,
                            result.relations_updated
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!("[knowledge_consolidation] 跨源实体合并失败: {}", e);
                },
            }

            // ── 步骤 2：Wiki → Knowledge 实体抽取（重量级，需要 LLM） ──
            // 查询所有 ConnectedVault KB，逐个触发实体抽取
            match axagent_dao::repo::knowledge::list_knowledge_bases(harness_state.db()).await {
                Ok(all_kbs) => {
                    let vault_kbs: Vec<_> = all_kbs
                        .iter()
                        .filter(|kb| {
                            kb.enabled
                                && matches!(kb.kind, axagent_harness::KbKind::ConnectedVault)
                                && kb.vault_path.is_some()
                        })
                        .collect();

                    for kb in &vault_kbs {
                        tracing::info!(
                            "[knowledge_consolidation] 处理 ConnectedVault KB: id={} name={}",
                            kb.id,
                            kb.name
                        );
                        // 使用已有的 extract_entities_from_wiki 逻辑
                        // 通过 index_job 机制异步执行（避免阻塞定时任务）
                        let metadata = serde_json::json!({
                            "auto_extract": true,
                            "triggered_by": "consolidation_tick",
                        });
                        let input = axagent_dao::repo::index_jobs::CreateIndexJobInput {
                            job_type: axagent_dao::repo::index_jobs::JOB_TYPE_EXTRACT_ENTITIES
                                .to_string(),
                            container_type: "Wiki".to_string(),
                            container_id: kb.id.clone(),
                            item_id: kb.id.clone(),
                            max_retries: Some(1),
                            priority: Some(5),
                            metadata: Some(serde_json::to_string(&metadata).unwrap_or_default()),
                        };
                        let _ =
                            axagent_dao::repo::index_jobs::enqueue_job(harness_state.db(), input)
                                .await
                                .map_err(|e| {
                                    tracing::warn!(
                                        "[knowledge_consolidation] 队列实体抽取任务失败 kb={}: {}",
                                        kb.id,
                                        e
                                    );
                                    e
                                });
                    }
                },
                Err(e) => {
                    tracing::warn!("[knowledge_consolidation] 查询 ConnectedVault KB 失败: {}", e);
                },
            }

            // ── 步骤 3：Memory → Knowledge 实体回流 ──
            // 查询高重要性 Memory 条目，写入知识图谱
            match axagent_dao::repo::memory::list_high_importance_items(
                harness_state.db(),
                Some(0.7), // importance >= 0.7
                Some(100), // 最多 100 条
            )
            .await
            {
                Ok(items) if !items.is_empty() => {
                    tracing::info!(
                        "[knowledge_consolidation] 发现 {} 条高重要性 Memory 条目，开始回流",
                        items.len()
                    );
                    let mut converted = 0usize;
                    for item in &items {
                        // 将 Memory 条目转换为知识图谱实体
                        let kb_id = if item.namespace_id.is_empty() {
                            "memory_default".to_string()
                        } else {
                            item.namespace_id.clone()
                        };
                        let name: String = item.content.chars().take(100).collect();
                        let confidence = (item.importance).min(1.0);
                        match axagent_dao::repo::knowledge_graph::upsert_entity(
                            harness_state.db(),
                            &kb_id,
                            &name,
                            "memory_item",
                            "[]", // empty aliases JSON
                            confidence,
                            None,
                            None,
                        )
                        .await
                        {
                            Ok(_) => converted += 1,
                            Err(e) => {
                                tracing::debug!(
                                    "[knowledge_consolidation] Memory→Entity 转换失败 item={}: {}",
                                    item.id,
                                    e
                                );
                            },
                        }
                    }
                    tracing::info!(
                        "[knowledge_consolidation] Memory→Knowledge 回流完成：{} 条成功",
                        converted
                    );
                },
                _ => {
                    // 无高重要性条目或查询失败，静默跳过
                },
            }

            // ── 步骤 4：Agent 工具调用结果 → Memory 沉淀 ──
            // 扫描最近 24 小时的对话，将工具结果（WebSearch/CodeInterpreter 等）
            // 自动沉淀为 Memory 条目，让 Agent 的执行结果可被后续 RAG 检索使用
            match axagent_dao::repo::memory::deposit_tool_results_from_recent_messages(
                harness_state.db(),
                Some(24),
            )
            .await
            {
                Ok(count) if count > 0 => {
                    tracing::info!(
                        "[knowledge_consolidation] 工具结果沉积：{} 条新 Memory 条目",
                        count
                    );
                },
                Ok(_) => {
                    // 无新条目，静默跳过
                },
                Err(e) => {
                    tracing::warn!("[knowledge_consolidation] 工具结果沉积失败: {}", e);
                },
            }

            // ── 步骤 5：KB 文档 → Wiki 自动同步 ──
            // 对 ConnectedVault KB 中新增的文档，自动在 Wiki 中创建对应笔记
            // 形成"KB↔Wiki"双向同步闭环
            if let Ok(all_kbs) =
                axagent_dao::repo::knowledge::list_knowledge_bases(harness_state.db()).await
            {
                use axagent_harness::note_dtos::CreateNoteInput;

                let vault_kbs: Vec<_> = all_kbs
                    .iter()
                    .filter(|kb| {
                        kb.enabled
                            && matches!(kb.kind, axagent_harness::KbKind::ConnectedVault)
                            && kb.vault_path.is_some()
                    })
                    .collect();

                for kb in &vault_kbs {
                    if let Ok(docs) =
                        axagent_dao::repo::knowledge::list_documents(harness_state.db(), &kb.id)
                            .await
                    {
                        let vault_id = kb.vault_path.as_deref().unwrap_or("");
                        let mut synced = 0usize;
                        for doc in &docs {
                            // 检查文档是否已同步到 Wiki
                            let already_synced = axagent_dao::repo::wiki::note_exists_for_document(
                                harness_state.db(),
                                vault_id,
                                &doc.id,
                            )
                            .await
                            .unwrap_or(false);

                            if !already_synced {
                                // 创建 Wiki 笔记
                                let input = CreateNoteInput {
                                    vault_id: vault_id.to_string(),
                                    title: doc.title.clone(),
                                    file_path: doc.source_path.clone(),
                                    content: String::new(),
                                    author: "system".to_string(),
                                    page_type: Some("knowledge_document".to_string()),
                                    source_refs: Some(vec![format!("kb:{}:doc:{}", kb.id, doc.id)]),
                                };
                                match axagent_dao::repo::note::create_note(
                                    harness_state.db(),
                                    input,
                                )
                                .await
                                {
                                    Ok(_) => synced += 1,
                                    Err(e) => {
                                        tracing::debug!(
                                            "[knowledge_consolidation] Wiki 同步失败 doc={}: {}",
                                            doc.id,
                                            e
                                        );
                                    },
                                }
                            }
                        }
                        if synced > 0 {
                            tracing::info!(
                                "[knowledge_consolidation] KB→Wiki 同步：kb={} 新增 {} 篇笔记",
                                kb.id,
                                synced
                            );
                        }
                    }
                }
            }

            tracing::info!(
                "[knowledge_consolidation] 知识转换周期完成，耗时 {}ms",
                started.elapsed().as_millis()
            );
        }
    });
    tracing::info!("[knowledge_consolidation] 知识转换定时任务已启动（每 6 小时）");
}

/// PTY 事件转发器：从 PtyManager 的 mpsc 通道消费输出/退出事件，
/// 通过 Tauri 事件总线 emit 到前端（事件名 `pty_output` / `pty_exit`）。
#[cfg(not(mobile))]
fn start_pty_event_forwarder(app: &tauri::AppHandle, state: &AppState) {
    use axagent_runtime::pty::{PtyExitEvent, PtyOutputEvent};

    let app_handle_output = app.clone();
    let pty_manager_output = state.pty_manager.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let event: Option<PtyOutputEvent> = pty_manager_output.recv_output().await;
            let Some(event) = event else {
                break;
            };
            if let Err(e) = app_handle_output.emit("pty_output", event) {
                tracing::warn!("pty_output emit failed: {}", e);
            }
        }
    });

    let app_handle_exit = app.clone();
    let pty_manager_exit = state.pty_manager.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            let event: Option<PtyExitEvent> = pty_manager_exit.recv_exit().await;
            let Some(event) = event else {
                break;
            };
            if let Err(e) = app_handle_exit.emit("pty_exit", event) {
                tracing::warn!("pty_exit emit failed: {}", e);
            }
        }
    });
}

/// P1-D10: 注册 portfolio-mgr.rhai 依赖的 pm_* 函数到共享 Rhai Engine。
///
/// rt-workflow（hybrid 层）不能依赖 AxInvest 专属 crate `axagent-stock-analysis`，
/// 但主 crate（wiring 层）可以同时依赖两者。此函数在应用启动时调用
/// `register_shared_engine_initializer`，把 pm_* 函数注入到
/// `code_executor::shared_rhai_engine()` 的初始化流程中。
///
/// 注册的函数（与 `stock_workflow/decision.rs` Rerun Decision 路径保持对称）：
/// - `pm_evidence_scale`: 非线性证据缩放（sqrt 曲线）
/// - `pm_kelly_position`: 凯利仓位计算（半凯利 + 成本扣减 + 风险上限）
/// - `pm_classify_risk`: 基于量化指标的算法风险分类
/// - `pm_risk_bias`: 风险等级对应的行为阈值偏移
/// - `pm_risk_veto`: 风控否决（高风险禁止加仓 / 极高风险禁止持仓）
/// - `pm_covariance_decay`: 因子协方差衰减（减少信号重复计数）
/// - `pm_portfolio_risk_gate`: 组合风控门（P1-E13）
/// - `pm_compute_news_sentiment`: 统一新闻情感分（P2-B4，[-1.0, 1.0]）
/// - `pm_compute_text_sentiment`: 单文本情感分（P2-B4，[-1.0, 1.0]）
/// - `pm_compute_bayes_confidence`: 贝叶斯因子置信度（P0，基于 prior→posterior 证据强度）
/// - `pm_compute_factor_completeness`: 因子数据完整度（供 data-quality.rhai 使用）
///
/// 必须在 `shared_rhai_engine()` 首次调用前注册（即任何工作流执行前）。
/// 后续注册不会生效（`OnceLock::set` 在已初始化后返回 Err，仅记 warn）。
fn register_portfolio_mgr_rhai_functions() {
    use axagent_analysis_engine::portfolio_formula;
    use axagent_rt_workflow::work_engine::executors::register_shared_engine_initializer;

    register_shared_engine_initializer(Box::new(|engine| {
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
                // P0 修复(2026-08-09): 与 decision.rs 对称——原 6 个 Option<f64> 参数
                // 注册后不可调用（Rhai 1.25 多 Option 参数闭包 Function not found）。
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
        // P1-E13: 组合风控门 — 在 portfolio-mgr 之后运行，做组合层约束检查
        // P0 修复(2026-08-09): 原含 2 个 Option 参数（target_price/stock_sector），Rhai 1.25
        // register_fn 对多 Option 参数闭包注册后不可调用 → portfolio-risk-gate.rhai:114
        // 调用必 Function not found → 被 try/catch 吞掉 → 风控门从未真正执行（一直走 catch
        // 保守兜底）。改为 9 个 Dynamic 参数，闭包内转 Option。
        engine.register_fn(
            "pm_portfolio_risk_gate",
            |pm_action: rhai::Dynamic,
             pm_position_pct: rhai::Dynamic,
             pm_risk_level: rhai::Dynamic,
             current_price: rhai::Dynamic,
             target_price: rhai::Dynamic,
             stock_code: rhai::Dynamic,
             stock_sector: rhai::Dynamic,
             holdings_json: rhai::Dynamic,
             portfolio_cash: rhai::Dynamic|
             -> String {
                // Rhai Dynamic 数值提取：f64/i64 均接受
                let f = |v: &rhai::Dynamic| -> Option<f64> {
                    v.clone()
                        .try_cast::<f64>()
                        .or_else(|| v.clone().try_cast::<i64>().map(|x| x as f64))
                };
                let s = |v: &rhai::Dynamic| v.clone().into_string().ok();
                let pm_action_s = s(&pm_action).unwrap_or_default();
                let pm_risk_level_s = s(&pm_risk_level).unwrap_or_default();
                let stock_code_s = s(&stock_code).unwrap_or_default();
                let holdings_json_s = s(&holdings_json).unwrap_or_default();
                let stock_sector_s = s(&stock_sector);
                portfolio_formula::portfolio_risk_gate(
                    &pm_action_s,
                    f(&pm_position_pct).unwrap_or(0.0),
                    &pm_risk_level_s,
                    f(&current_price).unwrap_or(0.0),
                    f(&target_price),
                    &stock_code_s,
                    stock_sector_s.as_deref(),
                    &holdings_json_s,
                    f(&portfolio_cash).unwrap_or(0.0),
                )
            },
        );
        // P2-B4: 统一新闻情感分词典 — 供 portfolio-mgr.rhai 公告关键词检测复用
        // 输入: 新闻/公告标题 + 摘要, 返回 [-1.0, 1.0] 区间的 sentiment_score
        // 无任何关键词命中时返回 0.0(Rhai 不支持 Option<f64>, 用 0.0 表示 None)
        // 内部含否定词检测(避免"不存在退市风险"被误判为风险信号)
        engine.register_fn("pm_compute_news_sentiment", |title: &str, summary: &str| -> f64 {
            axagent_astock_data::sentiment::compute_news_sentiment(title, summary).unwrap_or(0.0)
        });
        // P2-B4: 单文本版本(只传标题或合并后的文本)
        engine.register_fn("pm_compute_text_sentiment", |text: &str| -> f64 {
            axagent_astock_data::sentiment::compute_text_sentiment(text).unwrap_or(0.0)
        });
        // P0: 贝叶斯因子置信度（基于 prior→posterior 的证据强度）
        // 必须与 decision.rs 本地 Engine 注册保持对称，否则 DAG 工作流执行
        // portfolio-mgr.rhai 第 996 行调用会触发 "Function not found" 错误，
        // 整个 portfolio-mgr 被 catch 块降级为保守决策。
        engine.register_fn("pm_compute_bayes_confidence", |prior: f64, posterior: f64| -> f64 {
            portfolio_formula::compute_bayes_confidence(prior, posterior)
        });
        // 因子数据完整度：供 data-quality.rhai 评估因子层数据完整度
        // P0 修复(2026-08-09): 与 decision.rs 对称——Rhai 1.25 register_fn 对含多个
        // Option<T> 参数的闭包注册后无法调用（实测确认），改为 10 个 Dynamic 参数。
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
    }));
    tracing::info!(
        "[P1-D10/E13 + P2-B4] portfolio-mgr pm_* + risk-gate + sentiment 函数已注册到共享 Rhai Engine 初始化器"
    );
}

/// 初始化 MCP OAuth 凭据存储的全局单例。
///
/// 必须在任何 MCP 工具调用前完成，否则 `McpOAuthStore::try_global()` 返回 `None`，
/// 受保护服务器将按匿名方式连接（很可能 401）。
fn init_mcp_oauth(state: &AppState) {
    let master_key = state.harness.master_key_owned();
    let crypto = std::sync::Arc::new(
        axagent_crypto::platform_adapter_impl::DefaultCryptoService::new(master_key),
    );
    let store = std::sync::Arc::new(axagent_mcp::mcp_oauth::McpOAuthStore::new(crypto));
    axagent_mcp::mcp_oauth::McpOAuthStore::init_global(store);
    tracing::info!("[McpOAuth] 全局 OAuth 凭据存储已初始化");
}

fn start_plugins(state: &AppState) {
    let plugin_manager = state.plugin_manager.clone();
    let dashboard_registry = state.dashboard_registry.clone();

    tauri::async_runtime::spawn(async move {
        tracing::info!("Initializing plugin system...");

        let mut manager = plugin_manager.write().await;
        let _started = match manager.start_enabled_plugins() {
            Ok(started) => {
                if !started.is_empty() {
                    tracing::info!(
                        "Started {} enabled plugin(s): {}",
                        started.len(),
                        started.join(", ")
                    );
                } else {
                    tracing::info!("No enabled plugins to start");
                }
                started
            },
            Err(e) => {
                tracing::error!("Failed to start enabled plugins: {e}");
                Vec::new()
            },
        };

        drop(manager);

        if let Some(registry) = dashboard_registry {
            if let Err(e) = registry.reload().await {
                tracing::warn!("Failed to reload dashboard plugins: {e}");
            } else {
                let count = registry.list_plugins().await.len();
                tracing::info!("Loaded {count} dashboard plugin(s)");
            }
        }

        tracing::info!("Plugin system initialization complete");
    });
}

fn start_auto_backup(app: &tauri::AppHandle, state: &AppState, app_dir: std::path::PathBuf) {
    let db = state.harness.db().clone();
    let app_data = app_dir.clone();
    let handle = state.auto_backup_handle.clone();
    let shutdown_token = state.shutdown_token.clone();
    let app_for_emit = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_dao::repo::settings::get_settings(&db).await {
            if settings.auto_backup_enabled && settings.auto_backup_interval_hours > 0 {
                let backup_dir_setting =
                    axagent_storage::path_vars::decode_path_opt(&settings.backup_dir);
                let interval = settings.auto_backup_interval_hours;
                let max_count = settings.auto_backup_max_count;
                let interval_secs = interval as u64 * 3600;
                let db2 = db.clone();
                let app_dir2 = app_data.clone();
                let shutdown_token = shutdown_token.clone();
                let app_for_backup = app_for_emit.clone();

                let initial_delay_secs = match axagent_dao::repo::backup::list_backups(
                    &db,
                    &axagent_storage::DefaultPathEncoder,
                )
                .await
                {
                    Ok(backups) if !backups.is_empty() => {
                        let last_ts = &backups[0].created_at;
                        if let Ok(last_time) =
                            chrono::NaiveDateTime::parse_from_str(last_ts, "%Y-%m-%d %H:%M:%S")
                        {
                            let elapsed = chrono::Utc::now()
                                .naive_utc()
                                .signed_duration_since(last_time)
                                .num_seconds()
                                .max(0) as u64;
                            interval_secs.saturating_sub(elapsed)
                        } else {
                            interval_secs
                        }
                    },
                    _ => interval_secs,
                };

                let task = tokio::spawn(async move {
                    let dur = std::time::Duration::from_secs(interval_secs);
                    tokio::time::sleep(std::time::Duration::from_secs(initial_delay_secs)).await;
                    loop {
                        tokio::select! {
                            _ = shutdown_token.cancelled() => {
                                tracing::info!("[auto_backup] 收到关闭信号，停止自动备份");
                                break;
                            }
                            _ = tokio::time::sleep(dur) => {
                                let backup_dir = axagent_dao::repo::backup::resolve_backup_dir(
                                    backup_dir_setting.as_deref(),
                                    &app_dir2,
                                );
                                if let Err(e) =
                                    axagent_dao::repo::backup::create_backup(&db2, "sqlite", &backup_dir, &axagent_storage::DefaultPathEncoder)
                                        .await
                                {
                                    tracing::warn!("Auto-backup failed: {}", e);
                                    let _ = app_for_backup.emit("auto-backup-completed", serde_json::json!({
                                        "success": false,
                                        "error": e.to_string(),
                                    }));
                                } else {
                                    tracing::info!("Auto-backup created");
                                    let _ = app_for_backup.emit("auto-backup-completed", serde_json::json!({
                                        "success": true,
                                        "message": "Auto-backup created successfully",
                                    }));
                                    let _ =
                                        axagent_dao::repo::backup::cleanup_old_backups(&db2, max_count, &axagent_storage::DefaultPathEncoder)
                                            .await;
                                }
                            }
                        }
                    }
                });
                *handle.lock().await = Some(task);
            }
        }
    });
}

fn start_memory_maintenance_tick(state: &AppState) {
    let memory_service = state.memory_service.clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("memory_maintenance", async move {
        let interval = std::time::Duration::from_secs(7200);
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[memory_maintenance] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    let ms = memory_service.read().await;
                    let disambiguation = ms.disambiguate_entities().await;
                    drop(ms);
                    if disambiguation.merged > 0 {
                        tracing::info!(
                            "[memory_maintenance] Disambiguated entities: merged {} of {}",
                            disambiguation.merged,
                            disambiguation.total
                        );
                    }
                    let ms = memory_service.read().await;
                    let clusters = ms.find_similar_clusters(0.75).await;
                    drop(ms);
                    if !clusters.is_empty() {
                        tracing::info!(
                            "[memory_maintenance] Found {} similar memory clusters (potential duplicates)",
                            clusters.len()
                        );
                    }
                }
            }
        }
    });
}

/// 反思工作流定时任务：定期扫描 pending row 并执行反思。
///
/// 每 6 小时运行一次，首次延迟 60 秒（避免启动时抢资源）。
/// 调用 `run_batch_reflection_inner` 处理最多 20 条 pending row。
/// 监听 `shutdown_token` 支持优雅关闭。
fn start_batch_reflection(state: &AppState) {
    let db = state.harness.db().clone();
    let client = state.astock_client.clone();
    let engine = state.work_engine.clone();
    let vector_store = state.vector_store.clone();
    let master_key = state.harness.master_key_owned();
    // [方向3] 透传 TrajectoryStorage，让批量反思也持久化执行轨迹
    let trajectory_storage = state.trajectory_storage.clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("batch_reflection", async move {
        let initial_delay = std::time::Duration::from_secs(60);
        let interval = std::time::Duration::from_secs(6 * 3600); // 6 小时
        tokio::time::sleep(initial_delay).await;
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[batch_reflection] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    tracing::info!("[batch_reflection] 开始执行批量反思");
                    match crate::commands::stock_workflow::run_batch_reflection_inner(
                        &db,
                        &client,
                        &engine,
                        &vector_store,
                        &master_key,
                        None,
                        Some(&trajectory_storage),
                    ).await {
                        Ok(result) => {
                            tracing::info!(
                                "[batch_reflection] 完成: {}",
                                serde_json::to_string(&result).unwrap_or_default()
                            );
                        }
                        Err(e) => {
                            tracing::error!("[batch_reflection] 失败: {e}");
                        }
                    }
                }
            }
        }
    });
}

/// 反思规则有效性验证定时任务：定期校证 `reflection_lessons` 表的 confidence。
///
/// 每 24 小时运行一次，首次延迟 120 秒。
/// 调用 `run_lesson_validation` 基于 `times_applied` 和 `actual_success_rate`
/// 自动调整每条 lesson 的 confidence（×1.1 提升 / ×0.8 衰减 / 0.1 强废弃）。
/// 监听 `shutdown_token` 支持优雅关闭。
fn start_lesson_validation(state: &AppState) {
    let db = state.harness.db().clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("lesson_validation", async move {
        let initial_delay = std::time::Duration::from_secs(120);
        let interval = std::time::Duration::from_secs(24 * 3600); // 24 小时
        tokio::time::sleep(initial_delay).await;
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[lesson_validation] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    tracing::info!("[lesson_validation] 开始校证反思规则置信度");
                    match crate::commands::stock_workflow::run_lesson_validation(&db).await {
                        Ok(result) => {
                            tracing::info!(
                                "[lesson_validation] 完成: validated={} deprecated={} avg_success_rate={:.2}",
                                result.get("validatedLessons").and_then(|v| v.as_i64()).unwrap_or(0),
                                result.get("deprecatedLessons").and_then(|v| v.as_i64()).unwrap_or(0),
                                result.get("avgSuccessRate").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            );
                        }
                        Err(e) => {
                            tracing::error!("[lesson_validation] 失败: {e}");
                        }
                    }
                }
            }
        }
    });
}

/// P3-B5(G): vendor 健康后台周期探测任务
///
/// 每 90 秒扫描所有 vendor，对 Degraded 状态的 vendor 主动调用
/// `check_vendor_health` 探针方法。探测成功 → `record_success` 加速恢复；
/// 探测失败 → 更新 `last_failure_at`，延长恢复时间。
///
/// 设计动机：原架构为被动恢复——Degraded vendor 必须等 `try_vendors_retry`
/// 调用尝试才能恢复。若该 vendor 不在主路径上（如 ths/cninfo 在非热门股
/// 分析场景），可能长期无法自动恢复，影响后续分析质量。
///
/// Disabled 状态的 vendor 不参与探测（用户手动禁用，需手动恢复）。
/// 监听 `shutdown_token` 支持优雅关闭。
fn start_vendor_health_prober(state: &AppState) {
    let client = state.astock_client.clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("vendor_health_prober", async move {
        let initial_delay = std::time::Duration::from_secs(90);
        let interval = std::time::Duration::from_secs(90);
        tokio::time::sleep(initial_delay).await;
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[vendor_health_prober] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    // 获取所有 vendor 的当前健康状态
                    let health_states = client.health_tracker.get_all_health().await;
                    // 筛选 Degraded 状态的 vendor（排除 Disabled 和 Healthy）
                    let degraded_vendors: Vec<String> = health_states
                        .iter()
                        .filter(|h| h.status == axagent_astock_data::vendor_health::VendorStatus::Degraded)
                        .map(|h| h.name.clone())
                        .collect();

                    if degraded_vendors.is_empty() {
                        continue;
                    }

                    tracing::info!(
                        "[vendor_health_prober] 发现 {} 个 Degraded vendor，开始探测: {:?}",
                        degraded_vendors.len(),
                        degraded_vendors
                    );

                    for vendor_name in &degraded_vendors {
                        // 探测单个 vendor
                        match client.check_vendor_health(vendor_name).await {
                            Ok(()) => {
                                client.health_tracker.record_success(vendor_name).await;
                                tracing::info!(
                                    "[vendor_health_prober] {} 探测成功，触发自动恢复",
                                    vendor_name
                                );
                            },
                            Err(e) => {
                                client
                                    .health_tracker
                                    .record_failure(vendor_name, &e.to_string())
                                    .await;
                                tracing::debug!(
                                    "[vendor_health_prober] {} 探测仍失败: {}",
                                    vendor_name,
                                    e
                                );
                            },
                        }
                    }
                }
            }
        }
    });
}

fn start_platform_adapters(state: &AppState) {
    let platform_manager = state.platform_manager.clone();
    let db = state.harness.db().clone();

    tauri::async_runtime::spawn(async move {
        let config = axagent_dao::repo::platform_config::get_platform_config(&db).await;
        match platform_manager.reconcile(&config).await {
            Ok(report) => {
                if !report.started.is_empty() {
                    tracing::info!(
                        "[PlatformManager] boot reconcile: started {:?}",
                        report.started
                    );
                }
                if !report.errors.is_empty() {
                    for (name, err) in &report.errors {
                        tracing::error!(
                            "[PlatformManager] boot reconcile: {} error: {}",
                            name,
                            err
                        );
                    }
                }
            },
            Err(e) => {
                tracing::error!("[PlatformManager] boot reconcile failed: {}", e);
            },
        }
    });
}

fn start_webdav_sync(app: &tauri::AppHandle, state: &AppState, app_dir: std::path::PathBuf) {
    let db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    let app_data_dir = app_dir.clone();
    let handle = state.webdav_sync_handle.clone();
    let shutdown_token = state.shutdown_token.clone();
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_dao::repo::settings::get_settings(&db).await {
            if settings.webdav_sync_enabled && settings.webdav_sync_interval_minutes > 0 {
                let db2 = db.clone();
                let interval = settings.webdav_sync_interval_minutes;
                let interval_secs = interval as u64 * 60;

                let initial_delay_secs =
                    match axagent_dao::repo::settings::get_setting(&db, "webdav_last_sync_time")
                        .await
                    {
                        Ok(Some(ts)) => {
                            if let Ok(last_time) = chrono::DateTime::parse_from_rfc3339(&ts) {
                                let elapsed = chrono::Utc::now()
                                    .signed_duration_since(last_time)
                                    .num_seconds()
                                    .max(0) as u64;
                                interval_secs.saturating_sub(elapsed)
                            } else {
                                interval_secs
                            }
                        },
                        _ => interval_secs,
                    };

                let task = crate::commands::webdav::spawn_webdav_sync_task(
                    app_clone,
                    db2,
                    master_key,
                    app_data_dir,
                    interval,
                    initial_delay_secs,
                    shutdown_token,
                );
                *handle.lock().await = Some(task);
            }
        }
    });
}

#[cfg(not(mobile))]
fn start_tray(app: &tauri::AppHandle, tray_language: &str) {
    if let Err(e) = crate::tray::create_tray(app, tray_language) {
        tracing::warn!("Failed to create system tray: {}", e);
    }
}

fn start_closed_loop_service(_app: &tauri::AppHandle, state: &AppState) {
    let db = state.harness.db().clone();
    let closed_loop = state.closed_loop_service.clone();
    let nudge_service = state.nudge_service.clone();
    tauri::async_runtime::spawn(async move {
        if let Ok(settings) = axagent_dao::repo::settings::get_settings(&db).await {
            if settings.closed_loop_enabled {
                closed_loop.start();
                let interval_minutes = settings.closed_loop_interval_minutes.max(1);
                let interval = std::time::Duration::from_secs(interval_minutes as u64 * 60);
                loop {
                    tokio::time::sleep(interval).await;
                    let new_nudges: Vec<axagent_trajectory::PeriodicNudge> =
                        closed_loop.tick().await;
                    if !new_nudges.is_empty() {
                        tracing::info!(
                            "[closed_loop] Generated {} periodic nudges",
                            new_nudges.len()
                        );
                        let candidates: Vec<axagent_trajectory::NudgeCandidate> = new_nudges
                            .iter()
                            .map(|pn| axagent_trajectory::NudgeCandidate {
                                entity: axagent_trajectory::NudgeEntity {
                                    id: pn.id.clone(),
                                    name: pn.title.clone(),
                                    entity_type: format!("{:?}", pn.nudge_type),
                                    confidence: if pn.urgency == "high" {
                                        0.9
                                    } else if pn.urgency == "medium" {
                                        0.7
                                    } else {
                                        0.5
                                    },
                                },
                                reason: pn.description.clone(),
                                urgency: match pn.urgency.as_str() {
                                    "high" => axagent_trajectory::Urgency::High,
                                    "medium" => axagent_trajectory::Urgency::Medium,
                                    _ => axagent_trajectory::Urgency::Low,
                                },
                                suggested_action: Some(pn.suggested_action.clone()),
                            })
                            .collect();
                        let mut ns: tokio::sync::MutexGuard<'_, axagent_trajectory::NudgeService> =
                            nudge_service.lock().await;
                        let ctx = axagent_trajectory::NudgeContext {
                            current_task: None,
                            recent_entities: None,
                            session_id: "closed_loop_bg".to_string(),
                        };
                        ns.generate_nudges(ctx, candidates);
                    }
                }
            }
        }
    });
}

fn start_insight_generation(state: &AppState) {
    let realtime_learning = state.realtime_learning.clone();
    let insight_system = state.insight_system.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(10 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let new_insights = {
                let rl: tokio::sync::MutexGuard<'_, axagent_trajectory::RealTimeLearning> =
                    realtime_learning.lock().await;
                rl.generate_insights()
            };
            if !new_insights.is_empty() {
                tracing::info!(
                    "[insight] Generated {} learning insights from feedback",
                    new_insights.len()
                );
                let mut is = insight_system.write().await;
                for insight in new_insights {
                    is.add_insight(insight);
                }
            }
        }
    });
}

fn start_pattern_learning(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let pattern_learner = state.pattern_learner.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(15 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(20)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[pattern] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };
            if trajectories.is_empty() {
                continue;
            }
            let mut pl = pattern_learner.write().await;
            let new_patterns = pl.update_from_batch(&trajectories);
            drop(pl);
            if !new_patterns.is_empty() {
                tracing::info!(
                    "[pattern] Learned {} new patterns from {} trajectories",
                    new_patterns.len(),
                    trajectories.len()
                );
                for pattern in &new_patterns {
                    if let Err(e) = trajectory_storage.save_pattern(pattern).await {
                        tracing::warn!("[pattern] Failed to persist pattern: {}", e);
                    }
                }
            }
        }
    });
}

fn start_cross_session_learning(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let cross_session_learner = state.cross_session_learner.clone();
    let insight_system = state.insight_system.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(30 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(50)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[cross_session] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };
            if trajectories.len() < 3 {
                continue;
            }
            let mut by_session: std::collections::HashMap<
                String,
                Vec<axagent_trajectory::Trajectory>,
            > = std::collections::HashMap::new();
            for t in trajectories {
                by_session.entry(t.session_id.clone()).or_default().push(t);
            }
            if by_session.len() < 2 {
                continue;
            }
            let mut csl = cross_session_learner.write().await;
            let new_patterns = csl.learn_from_sessions(by_session);
            drop(csl);
            if !new_patterns.is_empty() {
                tracing::info!(
                    "[cross_session] Discovered {} cross-session patterns",
                    new_patterns.len()
                );
                let mut is = insight_system.write().await;
                for pattern in &new_patterns {
                    if let Err(e) = trajectory_storage.save_pattern(pattern).await {
                        tracing::warn!("[cross_session] Failed to persist pattern: {}", e);
                    }
                    if pattern.success_rate >= 0.7 && pattern.frequency >= 3 {
                        is.add_insight(axagent_trajectory::LearningInsight {
                            id: format!("cs_{}", pattern.id),
                            category: axagent_trajectory::InsightCategory::Pattern,
                            title: format!("Cross-session pattern: {}", pattern.name),
                            description: pattern.description.clone(),
                            confidence: pattern.success_rate,
                            evidence: pattern.trajectory_ids.iter().take(3).cloned().collect(),
                            suggested_action: Some(
                                "Consider creating a skill for this recurring pattern".to_string(),
                            ),
                            created_at: chrono::Utc::now().timestamp_millis(),
                        });
                    }
                }
            }
        }
    });
}

fn start_rl_reward_computation(state: &AppState, app_dir: std::path::PathBuf) {
    let trajectory_storage = state.trajectory_storage.clone();
    let rl_engine = state.rl_engine.clone();
    let insight_system = state.insight_system.clone();
    let process_reward_model = state.process_reward_model.clone();
    let _db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    tauri::async_runtime::spawn(async move {
        if let Some(bridge) =
            axagent_runtime::llm_bridge::build_llm_bridge_from_db(&master_key).await
        {
            {
                let mut rl = rl_engine.write().await;
                rl.set_llm_judge(Box::new(bridge.clone()));
            }
            tracing::info!("[rl] LLM judge injected into RLEngine");

            {
                let mut prm = process_reward_model.lock().await;
                prm.set_provider(Box::new(bridge));
            }
            tracing::info!("[rl] LLM PRM provider injected into ProcessRewardModel");
        }

        let interval = std::time::Duration::from_secs(20 * 60);
        let mut reward_normalizer = axagent_trajectory::RewardNormalizer::new();
        loop {
            tokio::time::sleep(interval).await;
            let mut trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(15)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[rl] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };
            if trajectories.is_empty() {
                continue;
            }
            let rl = rl_engine.read().await;
            let mut total_rewards = 0;
            let mut total_advantages = 0;
            let mut total_prm_rewards = 0;
            for trajectory in &mut trajectories {
                {
                    let mut rewards = rl.compute_rewards(trajectory).await;
                    total_rewards += rewards.len();
                    rl.shape_rewards(&mut rewards);
                    reward_normalizer.normalize(&mut rewards);
                    trajectory.rewards = rewards;

                    {
                        let prm = process_reward_model.lock().await;
                        let prm_result = prm.compute_trajectory_rewards(trajectory).await;
                        if !prm_result.step_rewards.is_empty() {
                            total_prm_rewards += prm_result.step_rewards.len();
                            let combined_value =
                                trajectory.value_score * 0.5 + prm_result.weighted_reward * 0.5;
                            trajectory.value_score = combined_value;
                            tracing::debug!(
                                "[rl] PRM for trajectory {}: aggregate={:.3}, outcome={:.3}, weighted={:.3}",
                                &trajectory.id[..trajectory.id.len().min(8)],
                                prm_result.aggregate_reward,
                                prm_result.outcome_reward,
                                prm_result.weighted_reward
                            );
                        }
                    }

                    let values = rl.estimate_value_function(trajectory);
                    if !values.is_empty() {
                        let advantages = rl.compute_advantages(&trajectory.rewards, &values);
                        total_advantages += advantages.len();
                        let avg_advantage: f64 = if !advantages.is_empty() {
                            advantages.iter().sum::<f64>() / advantages.len() as f64
                        } else {
                            0.0
                        };
                        if avg_advantage > 0.3 {
                            let gradients = rl.compute_policy_gradient(trajectory, &advantages);
                            tracing::debug!(
                                "[rl] High-advantage trajectory {}: avg_adv={:.3}, gradients={:?}",
                                &trajectory.id[..trajectory.id.len().min(8)],
                                avg_advantage,
                                gradients
                            );
                            // M7-C: 桥接 compute_policy_gradient → RLOptimizer 权重更新
                            if !gradients.is_empty() {
                                let mut opt =
                                    crate::commands::_shared_state::SHARED_OPTIMIZER.write().await;
                                opt.apply_gradients(&gradients);
                            }
                        }
                    }
                    let total_reward: f64 = trajectory.rewards.iter().map(|r| r.value).sum();
                    trajectory.value_score = (trajectory.value_score + total_reward) / 2.0;
                    if let Err(e) = trajectory_storage.save_trajectory(trajectory).await {
                        tracing::warn!("[rl] Failed to update trajectory: {}", e);
                    }
                }
            }
            drop(rl);
            if total_rewards > 0 {
                tracing::info!(
                    "[rl] Computed {} rewards, {} advantages, {} PRM step-evals across {} trajectories",
                    total_rewards,
                    total_advantages,
                    total_prm_rewards,
                    trajectories.len()
                );
                let reward_trajectories: Vec<_> =
                    trajectories.iter().filter(|t| !t.rewards.is_empty()).collect();
                if reward_trajectories.len() >= 3 {
                    let avg_reward: f64 = reward_trajectories
                        .iter()
                        .map(|t| t.rewards.iter().map(|r| r.value).sum::<f64>())
                        .sum::<f64>()
                        / reward_trajectories.len() as f64;
                    let high_reward_count = reward_trajectories
                        .iter()
                        .filter(|t| t.rewards.iter().map(|r| r.value).sum::<f64>() > avg_reward)
                        .count();
                    let mut is = insight_system.write().await;
                    is.add_insight(axagent_trajectory::LearningInsight {
                        id: format!("rl_{}", chrono::Utc::now().timestamp_millis()),
                        category: if avg_reward > 0.0 { axagent_trajectory::InsightCategory::Pattern } else { axagent_trajectory::InsightCategory::Warning },
                        title: format!("RL reward analysis: avg={:.2}", avg_reward),
                        description: format!("{} trajectories analyzed, {} above average reward. Average reward: {:.3}",
                            reward_trajectories.len(), high_reward_count, avg_reward),
                        confidence: (avg_reward.abs() * 2.0).min(0.9),
                        evidence: vec![],
                        suggested_action: if avg_reward < 0.0 {
                            Some("Recent interactions have negative reward signals. Consider adjusting tool usage patterns.".to_string())
                        } else { None },
                        created_at: chrono::Utc::now().timestamp_millis(),
                    });
                }
            }
            // M7-E: 每轮遍历结束保存 RLOptimizer 状态
            let save_dir = app_dir.clone();
            let _ = tokio::task::spawn_blocking(move || {
                crate::commands::_shared_state::save_rl_optimizer(&save_dir);
            })
            .await;
        }
    });
}

fn start_batch_processing(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let batch_processor = state.batch_processor.clone();
    let insight_system = state.insight_system.clone();
    let token = state.shutdown_token.clone();
    state.task_manager.spawn("batch_processing", async move {
        let interval = std::time::Duration::from_secs(60 * 60);
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[batch_processing] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
            let bp = &*batch_processor;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(50)).await {
                    Ok(t) => t,
                    Err(_) => continue,
                };
            if trajectories.len() < 5 {
                continue;
            }
            let quality_filtered = bp.filter_by_quality(&trajectories, 0.3);
            if quality_filtered.is_empty() {
                continue;
            }
            let analysis = bp.analyze_batch(&quality_filtered);
            let mut is = insight_system.write().await;
            is.add_insight(axagent_trajectory::LearningInsight {
                id: format!("batch_{}", chrono::Utc::now().timestamp_millis()),
                category: axagent_trajectory::InsightCategory::Improvement,
                title: format!("Batch analysis: {} trajectories, {:.0}% success",
                    analysis.total,
                    if analysis.total > 0 { analysis.outcome_counts.values().sum::<usize>() as f64 / analysis.total as f64 * 100.0 } else { 0.0 }),
                description: format!("Quality: avg={:.2}, value={:.2}. Patterns: {}.",
                    analysis.avg_quality, analysis.avg_value, analysis.top_patterns.len().min(5)),
                confidence: (analysis.avg_quality * 1.5).min(0.9),
                evidence: vec![],
                suggested_action: if analysis.avg_quality < 0.4 {
                    Some("Batch quality is low. Consider reviewing recent interactions for improvement opportunities.".to_string())
                } else { None },
                created_at: chrono::Utc::now().timestamp_millis(),
            });
                }
            }
        }
    });
}

fn start_user_profile_persistence(state: &AppState) {
    let user_profile = state.user_profile.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(10 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let profile = user_profile.read().await;
            let md_content = profile.to_user_md();
            drop(profile);
            if let Some(home) = {
                #[cfg(mobile)]
                {
                    dirs::data_dir().or_else(dirs::home_dir)
                }
                #[cfg(not(mobile))]
                {
                    dirs::home_dir()
                }
            } {
                let user_md_path = home.join(".axagent").join("USER.md");
                if let Some(parent) = user_md_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::write(&user_md_path, &md_content) {
                    tracing::warn!("[user-profile] Failed to write USER.md: {}", e);
                }
            }
        }
    });
}

fn start_skill_evolution(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let skill_evolution_engine = state.skill_evolution_engine.clone();
    let insight_system = state.insight_system.clone();
    let constitution = state.constitution.clone();
    let intrinsic_motivation = state.intrinsic_motivation.clone();
    let coevolution_env = state.coevolution_env.clone();
    let _db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    tauri::async_runtime::spawn(async move {
        if let Some(bridge) =
            axagent_runtime::llm_bridge::build_llm_bridge_from_db(&master_key).await
        {
            let engine = skill_evolution_engine.lock().await;
            engine.set_llm_provider(std::sync::Arc::new(bridge)).await;
            drop(engine);
            tracing::info!("[evolution] LLM provider injected into SkillEvolutionEngine");
        }

        let interval = std::time::Duration::from_secs(45 * 60);
        let success_threshold = 0.5;
        let min_usages = 3;
        // P1: 连续失败次数阈值，达到即触发自动进化
        let auto_trigger_consecutive_failures: u32 = 3;
        loop {
            tokio::time::sleep(interval).await;
            let skills: Vec<axagent_trajectory::Skill> = match trajectory_storage.get_skills().await
            {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("[evolution] Failed to fetch skills: {}", e);
                    continue;
                },
            };
            let weak_skills: Vec<_> = skills
                .into_iter()
                .filter(|s| {
                    s.consecutive_failures >= auto_trigger_consecutive_failures
                        || (s.total_usages >= min_usages && s.success_rate < success_threshold)
                })
                .collect();
            if weak_skills.is_empty() {
                continue;
            }
            tracing::info!(
                "[evolution] Found {} skills to evolve (consecutive_failures>={} or success<{:.0}%)",
                weak_skills.len(),
                auto_trigger_consecutive_failures,
                success_threshold * 100.0
            );
            let test_trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(30)).await {
                    Ok(t) => t,
                    Err(_) => continue,
                };
            let test_refs: Vec<&axagent_trajectory::Trajectory> =
                test_trajectories.iter().collect();
            for skill in weak_skills.iter().take(2) {
                let mut engine: tokio::sync::MutexGuard<
                    '_,
                    axagent_trajectory::SkillEvolutionEngine,
                > = skill_evolution_engine.lock().await;
                let result = engine.run(skill, &test_refs).await;
                if let Some(modification) = result {
                    if let Err(violations) = constitution.validate_evolution(&modification) {
                        let has_fatal = violations
                            .iter()
                            .any(|v| v.severity == axagent_trajectory::ViolationSeverity::Fatal);
                        let has_critical = violations
                            .iter()
                            .any(|v| v.severity == axagent_trajectory::ViolationSeverity::Critical);
                        if has_fatal || has_critical {
                            tracing::warn!(
                                "[evolution] Constitution blocked skill '{}' evolution: {} violations (fatal={}, critical={})",
                                skill.name,
                                violations.len(),
                                has_fatal,
                                has_critical
                            );
                            continue;
                        }
                        tracing::info!(
                            "[evolution] Constitution warnings for skill '{}' evolution: {:?}",
                            skill.name,
                            violations.iter().map(|v| &v.description).collect::<Vec<_>>()
                        );
                    }
                    if modification.validation_result.as_ref().is_some_and(|v| v.success) {
                        tracing::info!(
                            "[evolution] Skill '{}' evolved: {} (confidence={:.3})",
                            skill.name,
                            modification.reason,
                            modification.confidence
                        );
                        let mut updated_skill = skill.clone();
                        updated_skill.content = modification.new_content.clone();
                        updated_skill.quality_score = modification.confidence;
                        updated_skill.version = format!(
                            "{}.e{}",
                            updated_skill
                                .version
                                .trim_end_matches(|c: char| c == '.' || c.is_ascii_digit()),
                            chrono::Utc::now().timestamp_millis() % 10000
                        );
                        if let Err(e) = trajectory_storage.save_skill(&updated_skill).await {
                            tracing::warn!("[evolution] Failed to save evolved skill: {}", e);
                        }

                        {
                            let mut im = intrinsic_motivation.lock().await;
                            for traj in &test_trajectories {
                                let _intrinsic_reward = im.compute_intrinsic_reward(traj);
                            }
                        }

                        {
                            let mut env = coevolution_env.lock().await;
                            env.update_performance(modification.confidence);
                        }

                        let mut is = insight_system.write().await;
                        is.add_insight(axagent_trajectory::LearningInsight {
                            id: format!("evo_{}", chrono::Utc::now().timestamp_millis()),
                            category: axagent_trajectory::InsightCategory::Improvement,
                            title: format!("Skill '{}' auto-evolved", skill.name),
                            description: modification.reason.clone(),
                            confidence: modification.confidence,
                            evidence: vec![],
                            suggested_action: Some(format!(
                                "Review evolved skill '{}' for correctness",
                                skill.name
                            )),
                            created_at: chrono::Utc::now().timestamp_millis(),
                        });
                    } else {
                        tracing::info!(
                            "[evolution] Skill '{}' evolution did not improve fitness",
                            skill.name
                        );
                    }
                }
            }
        }
    });
}

fn start_skill_watcher(app: &tauri::AppHandle, state: &AppState) {
    let home = {
        #[cfg(mobile)]
        {
            dirs::data_dir().or_else(dirs::home_dir).unwrap_or_default()
        }
        #[cfg(not(mobile))]
        {
            dirs::home_dir().unwrap_or_default()
        }
    };
    let skill_dirs: Vec<std::path::PathBuf> = vec![
        home.join(".axagent").join("skills"),
        home.join(".claude").join("skills"),
        home.join(".trae").join("skills"),
        home.join(".codebuddy").join("skills"),
        home.join(".workbuddy").join("skills"),
        home.join(".agents").join("skills"),
    ];

    let app_handle = app.clone();
    let shutdown = Arc::new(AtomicBool::new(false));
    let _ = state.skill_watcher_shutdown.set(shutdown.clone());
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher =
            match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("Failed to create skill watcher: {}", e);
                    return;
                },
            };

        for dir in &skill_dirs {
            if dir.exists() {
                if let Err(e) = watcher.watch(dir, RecursiveMode::NonRecursive) {
                    tracing::warn!("Failed to watch skill dir {:?}: {}", dir, e);
                }
            }
        }

        tracing::info!("Skill file watcher started");

        let mut pending: std::collections::HashMap<String, std::time::Instant> =
            std::collections::HashMap::new();
        let debounce = std::time::Duration::from_secs(2);

        loop {
            if shutdown.load(Ordering::Relaxed) {
                tracing::info!("Skill file watcher 收到关闭信号");
                return;
            }
            match rx.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(event) => {
                    if !event.kind.is_modify() {
                        continue;
                    }
                    for path in &event.paths {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        let is_skill_file = matches!(
                            name,
                            "SKILL.md" | "manifest.json" | "skill-manifest.json" | "frontend.json"
                        );
                        if !is_skill_file {
                            continue;
                        }

                        if let Some(parent) = path.parent() {
                            if let Some(skill_name) = parent.file_name().and_then(|n| n.to_str()) {
                                pending
                                    .entry(skill_name.to_string())
                                    .or_insert(std::time::Instant::now());
                            }
                        }
                    }
                },
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // 检查是否有到期的事件需要发送
                    let now = std::time::Instant::now();
                    let mut ready: Vec<String> = vec![];
                    pending.retain(|name, ts| {
                        if now.duration_since(*ts) >= debounce {
                            ready.push(name.clone());
                            false
                        } else {
                            true
                        }
                    });

                    if ready.is_empty() {
                        continue;
                    }

                    let app = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        for name in ready {
                            let _ = app.emit("skill:file-changed", name);
                        }
                    });
                },
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    tracing::info!("Skill file watcher stopped");
                    return;
                },
            }
        }
    });
}

/// Dream 巩固定时任务
///
/// 每 30 分钟检查一次：
/// 1. 通过 trajectory 数量增量检测新会话，调用 record_new_session 累加计数
/// 2. 检查 should_consolidate 门控（启用/未运行/间隔/会话数/锁）
/// 3. 满足门控则执行 consolidate（经验回放→知识蒸馏→对比学习→建议生成）
fn start_dream_consolidation(state: &AppState) {
    let consolidator = state.dream_consolidator.clone();
    let trajectory_storage = state.trajectory_storage.clone();
    let last_trajectory_count = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(u64::MAX));
    let last_count = last_trajectory_count.clone();
    tauri::async_runtime::spawn(async move {
        // 初始化 trajectory 基线计数
        if let Ok(trajs) = trajectory_storage.get_trajectories(Some(10000)).await {
            last_count.store(trajs.len() as u64, std::sync::atomic::Ordering::Relaxed);
        }

        let interval = std::time::Duration::from_secs(30 * 60);
        loop {
            tokio::time::sleep(interval).await;

            // 检测新会话：trajectory 数量增量即为新会话数
            let current_count = match trajectory_storage.get_trajectories(Some(10000)).await {
                Ok(trajs) => trajs.len() as u64,
                Err(e) => {
                    tracing::warn!("[dream] 获取 trajectory 失败: {}", e);
                    continue;
                },
            };
            let prev_count = last_count.swap(current_count, std::sync::atomic::Ordering::Relaxed);
            // 首次循环 prev_count == u64::MAX（基线），跳过计数
            if prev_count != u64::MAX && current_count > prev_count {
                let new_sessions = (current_count - prev_count) as usize;
                for _ in 0..new_sessions {
                    consolidator.record_new_session().await;
                }
                tracing::info!("[dream] 记录 {} 个新会话", new_sessions);
            }

            // 检查门控条件
            if !consolidator.should_consolidate().await {
                continue;
            }

            tracing::info!("[dream] 开始执行巩固...");
            let result = consolidator
                .consolidate(
                    Some(&|n| tracing::info!("[dream] 提取 {} 条记忆", n)),
                    Some(&|n| tracing::info!("[dream] 发现 {} 个模式", n)),
                    Some(&|n| tracing::info!("[dream] 生成 {} 个建议", n)),
                )
                .await;

            if result.executed {
                tracing::info!(
                    "[dream] 巩固完成: {} 条记忆, {} 个模式, {} 个建议, 耗时 {} 秒",
                    result.memories_extracted,
                    result.patterns_discovered,
                    result.suggestions_generated,
                    result.duration_secs
                );

                // Dream↔Evolution 联动：发现新模式时提示可能需要触发技能进化
                // 注意：不直接调用 SkillEvolutionEngine（避免循环依赖），
                // 仅记录日志，由独立的 start_skill_evolution 定时任务在下一轮自动检测弱技能并进化
                if result.patterns_discovered > 0 {
                    tracing::info!(
                        "[dream] 发现 {} 个新模式，下一轮 skill evolution 将评估是否需要进化相关技能",
                        result.patterns_discovered
                    );
                }
            } else {
                tracing::warn!(
                    "[dream] 巩固未执行: {}",
                    result.error.as_deref().unwrap_or("未知原因")
                );
            }
        }
    });
}

fn start_memory_decay_tick(state: &AppState) {
    let memory_service = state.memory_service.clone();
    let harness_state = state.harness.clone();
    tauri::async_runtime::spawn(async move {
        let interval = std::time::Duration::from_secs(3600);
        loop {
            tokio::time::sleep(interval).await;
            // 1. trajectory working memory 衰减（sentinel namespace）
            let ms = memory_service.read().await;
            let evicted = ms.apply_decay_tick().await;
            drop(ms);
            if evicted > 0 {
                tracing::info!("[memory_decay] trajectory evicted {} entries", evicted);
            }
            // 2. 用户 namespace 全表衰减（三层记忆系统）
            match axagent_dao::repo::memory::apply_decay_tick(harness_state.db()).await {
                Ok((expired, low_score, capacity)) => {
                    if expired + low_score + capacity > 0 {
                        tracing::info!(
                            "[memory_decay] user ns: expired={}, low_score={}, capacity={}",
                            expired,
                            low_score,
                            capacity
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!("[memory_decay] user ns tick failed: {}", e);
                },
            }
        }
    });
}

/// DreamTask 全量清理定时任务
///
/// 每 60 分钟执行一次 FullCleanup，涵盖：
/// - 轨迹整合（consolidator，与 start_dream_consolidation 共享实例，内部有门控）
/// - 记忆压缩（auto_memory_extractor + FTS5 optimize）
/// - 技能更新（skill_evolution_engine，与 start_skill_evolution 共享实例）
/// - 僵尸 SubAgent 清理（sub_agent_registry）
/// - FTS5 索引优化（optimize + vacuum）
///
/// 与 start_dream_consolidation 的关系：
/// - start_dream_consolidation 30 分钟一次，仅做轨迹巩固（轻量）
/// - start_dream_task_executor 60 分钟一次，做全量清理（重量）
///
/// 两者共享 DreamConsolidator 实例，consolidator 内部 should_consolidate 门控
/// 会避免重复执行实际的巩固操作。
fn start_dream_task_executor(state: &AppState) {
    // 组装 DreamTaskContext（所有依赖均为 AppState 中的 Arc 克隆）
    let ctx = axagent_runtime::tasks::dream_task::DreamTaskContext {
        consolidator: Some(state.dream_consolidator.clone()),
        trajectory_storage: Some(state.trajectory_storage.clone()),
        skill_evolution_engine: Some(state.skill_evolution_engine.clone()),
        auto_memory_extractor: Some(state.auto_memory_extractor.clone()),
        sub_agent_registry: Some(state.sub_agent_registry.clone()),
    };

    tauri::async_runtime::spawn(async move {
        // 启动后延迟 10 分钟首次执行，避免与启动期间的其它密集任务冲突
        let initial_delay = std::time::Duration::from_secs(10 * 60);
        tokio::time::sleep(initial_delay).await;

        let interval = std::time::Duration::from_secs(60 * 60);
        loop {
            let task = axagent_runtime::tasks::dream_task::DreamTask::on_session_end();
            tracing::info!("[dream_task_executor] 触发全量清理 (scope={:?})", task.scope);
            let result =
                axagent_runtime::tasks::dream_task::DreamTaskExecutor::execute(&task, &ctx).await;
            if !result.errors.is_empty() {
                tracing::warn!(
                    "[dream_task_executor] 本次执行有 {} 个子任务跳过/失败: {:?}",
                    result.errors.len(),
                    result.errors
                );
            }
            tokio::time::sleep(interval).await;
        }
    });
}

/// CoevolutionTask 协同进化定时任务
///
/// 每 30 分钟执行一次，根据近期轨迹成功率驱动难度调整 + 生成针对薄弱类别的新任务。
/// 与 `start_skill_evolution` 共享同一 `CoevolutionEnvironment` 实例：
/// - `start_skill_evolution` 在技能进化成功后被动更新性能
/// - 本任务主动周期性地用整体成功率驱动协同进化
///
/// 依赖：`coevolution_env` / `trajectory_storage` / `insight_system`
/// 任一缺失会跳过对应子功能并在 `result.errors` 中记录。
fn start_coevolution_task_executor(state: &AppState) {
    let ctx = axagent_runtime::tasks::coevolution_task::CoevolutionTaskContext {
        coevolution_env: Some(state.coevolution_env.clone()),
        trajectory_storage: Some(state.trajectory_storage.clone()),
        insight_system: Some(state.insight_system.clone()),
    };

    tauri::async_runtime::spawn(async move {
        // 启动后延迟 10 分钟首次执行，避免与启动期间的其它密集任务冲突
        let initial_delay = std::time::Duration::from_secs(10 * 60);
        tokio::time::sleep(initial_delay).await;

        let interval = std::time::Duration::from_secs(30 * 60);
        loop {
            tracing::info!("[coevolution_task_executor] 触发协同进化周期任务");
            let result =
                axagent_runtime::tasks::coevolution_task::CoevolutionTaskExecutor::execute(&ctx)
                    .await;
            if !result.errors.is_empty() {
                tracing::warn!(
                    "[coevolution_task_executor] 本次执行有 {} 个子任务跳过/失败: {:?}",
                    result.errors.len(),
                    result.errors
                );
            }
            tokio::time::sleep(interval).await;
        }
    });
}

/// PatternAnalyzerTask 跨会话模式分析定时任务
///
/// 每 2 小时执行一次，从近期轨迹提取代码风格 / 工具偏好 / 时间分布模式，
/// 把关键发现作为 `LearningInsight` 写入 `insight_system`。
///
/// 与 `start_pattern_learning` 互补：
/// - `start_pattern_learning` 学习任务级 `TrajectoryPattern`（含 success_rate）
/// - 本任务提取更细粒度的用户行为模式，用于丰富用户画像与行为洞察
///
/// 依赖：`trajectory_storage` / `insight_system`
fn start_pattern_analyzer_task_executor(state: &AppState) {
    let ctx = axagent_runtime::tasks::pattern_task::PatternAnalyzerTaskContext {
        trajectory_storage: Some(state.trajectory_storage.clone()),
        insight_system: Some(state.insight_system.clone()),
    };

    tauri::async_runtime::spawn(async move {
        // 启动后延迟 15 分钟首次执行，比 coevolution 稍晚以错峰
        let initial_delay = std::time::Duration::from_secs(15 * 60);
        tokio::time::sleep(initial_delay).await;

        let interval = std::time::Duration::from_secs(2 * 60 * 60);
        loop {
            tracing::info!("[pattern_analyzer_task_executor] 触发模式分析周期任务");
            let result =
                axagent_runtime::tasks::pattern_task::PatternAnalyzerTaskExecutor::execute(&ctx)
                    .await;
            if !result.errors.is_empty() {
                tracing::warn!(
                    "[pattern_analyzer_task_executor] 本次执行有 {} 个子任务跳过/失败: {:?}",
                    result.errors.len(),
                    result.errors
                );
            }
            tokio::time::sleep(interval).await;
        }
    });
}

/// InsightGeneratorTask 学习洞察生成定时任务
///
/// 每 6 小时执行一次，从近期轨迹分析整体趋势（成功率 / 质量分布），
/// 生成趋势洞察 + 日报。与 `start_insight_generation` 互补：
/// - `start_insight_generation` 从实时反馈生成洞察（10 分钟一次，关注实时）
/// - 本任务从轨迹存储的整体趋势生成洞察 + 日报（周期更长，关注长期）
///
/// 依赖：`trajectory_storage` / `insight_system`
fn start_insight_generator_task_executor(state: &AppState) {
    let ctx = axagent_runtime::tasks::insight_task::InsightGeneratorTaskContext {
        trajectory_storage: Some(state.trajectory_storage.clone()),
        insight_system: Some(state.insight_system.clone()),
    };

    tauri::async_runtime::spawn(async move {
        // 启动后延迟 20 分钟首次执行，与其它任务错峰
        let initial_delay = std::time::Duration::from_secs(20 * 60);
        tokio::time::sleep(initial_delay).await;

        let interval = std::time::Duration::from_secs(6 * 60 * 60);
        loop {
            tracing::info!("[insight_generator_task_executor] 触发洞察生成周期任务");
            let result =
                axagent_runtime::tasks::insight_task::InsightGeneratorTaskExecutor::execute(&ctx)
                    .await;
            if !result.errors.is_empty() {
                tracing::warn!(
                    "[insight_generator_task_executor] 本次执行有 {} 个子任务跳过/失败: {:?}",
                    result.errors.len(),
                    result.errors
                );
            }
            tokio::time::sleep(interval).await;
        }
    });
}

fn start_auto_tool_observation(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let auto_tool_creator = state.auto_tool_creator.clone();
    let _db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    tauri::async_runtime::spawn(async move {
        if let Some(bridge) =
            axagent_runtime::llm_bridge::build_llm_bridge_from_db(&master_key).await
        {
            let mut atc = auto_tool_creator.lock().await;
            atc.set_llm_provider(Box::new(bridge));
            drop(atc);
            tracing::info!("[auto_tool] LLM provider injected into AutoToolCreator");
        }

        let interval = std::time::Duration::from_secs(60 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(30)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[auto_tool] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };

            let mut atc = auto_tool_creator.lock().await;
            for trajectory in &trajectories {
                atc.observe_trajectory(trajectory);
            }

            let frequent = atc.get_frequent_patterns(3);
            if !frequent.is_empty() {
                tracing::info!(
                    "[auto_tool] Observed {} frequent tool patterns (top: {:?})",
                    frequent.len(),
                    &frequent[..frequent.len().min(5)]
                );

                for (pattern, count) in frequent.iter().take(2) {
                    if atc.get_tool(&axagent_trajectory::slugify(pattern)).is_none() {
                        match atc
                            .create_tool_from_pattern(
                                pattern,
                                &format!("Auto-observed pattern ({} occurrences)", count),
                                vec![],
                            )
                            .await
                        {
                            Ok(tool) => {
                                tracing::info!(
                                    "[auto_tool] Created tool '{}' from pattern '{}' (freq={})",
                                    tool.name,
                                    pattern,
                                    count
                                );
                            },
                            Err(e) => {
                                tracing::debug!(
                                    "[auto_tool] Could not create tool from '{}': {}",
                                    pattern,
                                    e
                                );
                            },
                        }
                    }
                }
            }
        }
    });
}

fn start_text_grad_analysis(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let text_grad_engine = state.text_grad_engine.clone();
    let _db = state.harness.db().clone();
    let master_key = state.harness.master_key_owned();
    tauri::async_runtime::spawn(async move {
        if let Some(bridge) =
            axagent_runtime::llm_bridge::build_llm_bridge_from_db(&master_key).await
        {
            let mut engine = text_grad_engine.lock().await;
            engine.set_provider(bridge);
            drop(engine);
            tracing::info!("[text_grad] LLM provider injected into TextGradEngine");
        }

        let interval = std::time::Duration::from_secs(2 * 60 * 60);
        loop {
            tokio::time::sleep(interval).await;
            let trajectories: Vec<axagent_trajectory::Trajectory> =
                match trajectory_storage.get_trajectories(Some(10)).await {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[text_grad] Failed to fetch trajectories: {}", e);
                        continue;
                    },
                };

            let mut engine = text_grad_engine.lock().await;
            for trajectory in &trajectories {
                if trajectory.steps.len() < 3 {
                    continue;
                }
                let session_id = &trajectory.session_id;
                let topic = &trajectory.topic;

                for (i, step) in trajectory.steps.iter().enumerate() {
                    let content_summary: String = step.content.chars().take(200).collect();
                    let node_id = format!("{}_{}", &session_id[..session_id.len().min(8)], i);
                    engine.add_node(node_id.clone(), content_summary, Some(format!("step_{}", i)));
                    if i > 0 {
                        let prev_id =
                            format!("{}_{}", &session_id[..session_id.len().min(8)], i - 1);
                        engine.add_edge(prev_id, node_id, 1.0);
                    }
                }

                if !trajectory.steps.is_empty() {
                    if let Some(last_step) = trajectory.steps.last() {
                        let feedback = match trajectory.outcome {
                            axagent_trajectory::TrajectoryOutcome::Success => {
                                format!("Task succeeded: {}", topic)
                            },
                            axagent_trajectory::TrajectoryOutcome::Failure => {
                                format!(
                                    "Task failed: {} - last step: {}",
                                    topic,
                                    last_step.content.chars().take(100).collect::<String>()
                                )
                            },
                            axagent_trajectory::TrajectoryOutcome::Partial => {
                                format!("Task partially completed: {}", topic)
                            },
                            axagent_trajectory::TrajectoryOutcome::Abandoned => {
                                format!("Task abandoned: {}", topic)
                            },
                        };
                        let last_id = format!(
                            "{}_{}",
                            &session_id[..session_id.len().min(8)],
                            trajectory.steps.len() - 1
                        );
                        let _ = engine.forward();
                        let _ = engine.backward(&last_id, &feedback).await;
                    }
                }
            }

            let stats = engine.stats();
            tracing::info!(
                "[text_grad] Graph stats: {} nodes, {} edges, {} gradients computed",
                stats.node_count,
                stats.edge_count,
                stats.gradient_count
            );
        }
    });
}

fn start_cron_scheduler(state: &AppState) {
    use axagent_runtime::cron::{CronExecutor, CronScheduler};
    use std::sync::Arc;

    let store = state.cron_job_store.clone();

    // 注入共享存储到 tools crate，使 CronCreateTool 等可用
    axagent_tools::tools::cron::init_cron_store(store.clone());

    // 设置工具解析器（从全局 registry 按需自动注册工作流中引用的工具）
    {
        let registry = state.local_tool_registry.clone();
        let work_engine = state.work_engine.clone();
        // P0-1: 注入 astock_client 用于接通 32 个 stock_mcp_tools 到工作流执行路径
        let astock_client = state.astock_client.clone();
        // P0-3: 注入数据库连接，用于在执行 compute_valuation 前加载估值参数配置
        let db = state.harness.db().clone();
        // P0-1: 缓存 stock_mcp_tools 工具名集合，避免每次 resolve 都重新生成 Vec。
        // P2-8: 合并 G3 产业链工具（来自 axagent_analysis_engine::mcp_tools）。
        static STOCK_TOOL_NAMES: std::sync::OnceLock<std::collections::HashSet<String>> =
            std::sync::OnceLock::new();
        let stock_tools = STOCK_TOOL_NAMES.get_or_init(|| {
            let mut set: std::collections::HashSet<String> =
                axagent_astock_data::mcp_tools::stock_mcp_tools()
                    .into_iter()
                    .filter_map(|t| t.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .collect();
            // G3 产业链工具（P2-8 从 astock-data 迁回 stock-analysis）
            for tool in axagent_analysis_engine::mcp_tools::industry_chain_mcp_tools() {
                if let Some(name) = tool.get("name").and_then(|v| v.as_str()) {
                    set.insert(name.to_string());
                }
            }
            set
        });
        let resolver: axagent_runtime::work_engine::ToolResolver = std::sync::Arc::new(
            move |tool_name: String| {
                let registry = registry.clone();
                let work_engine = work_engine.clone();
                let astock_client = astock_client.clone();
                let db = db.clone();
                let in_stock_tools = stock_tools.contains(&tool_name);
                let in_industry_chain =
                    axagent_analysis_engine::mcp_tools::is_industry_chain_tool(&tool_name);
                tracing::info!(
                    "[ToolResolver] 被调用: tool_name={}, in_stock_tools={}, in_industry_chain={}",
                    tool_name,
                    in_stock_tools,
                    in_industry_chain
                );
                Box::pin(async move {
                    let reg = registry.lock().await;
                    let known = reg.list_all_tool_names().contains(&tool_name)
                        || reg.mcp.mcp_tools.contains_key(&tool_name);
                    tracing::info!(
                        "[ToolResolver] 解析 tool_name={}, known={}, in_stock_tools={}, in_industry_chain={}",
                        tool_name,
                        known,
                        in_stock_tools,
                        in_industry_chain
                    );
                    if known {
                        let registry = registry.clone();
                        let cb: axagent_runtime::work_engine::ToolCallback =
                            std::sync::Arc::new(move |tn: String, args: serde_json::Value| {
                                let registry = registry.clone();
                                Box::pin(async move {
                                    let reg = registry.lock().await;
                                    let input_str = serde_json::to_string(&args)
                                        .unwrap_or_else(|_| "{}".to_string());
                                    match reg.execute(&tn, &input_str).await {
                                        Ok(output) => {
                                            Ok(serde_json::json!({"content": output.content}))
                                        },
                                        Err(e) => Err(format!("Tool execution error: {}", e)),
                                    }
                                })
                            });
                        Some(cb)
                    } else if let Some(template_id) = tool_name.strip_prefix("workflow::") {
                        // 工作流注册为工具：workflow::<template_id>
                        let engine = work_engine.clone();
                        let template_id = template_id.to_string();
                        let cb: axagent_runtime::work_engine::ToolCallback =
                            std::sync::Arc::new(move |_tn: String, args: serde_json::Value| {
                                let engine = engine.clone();
                                let tid = template_id.clone();
                                Box::pin(async move {
                                    let mut opts =
                                        axagent_runtime::work_engine::RunOptions::default();
                                    if let Some(input) = args.get("input") {
                                        opts.input = Some(input.clone());
                                    }
                                    match engine.run_workflow(&tid, opts).await {
                                        Ok(wf) => Ok(serde_json::json!({
                                            "content": serde_json::json!({
                                                "status": format!("{:?}", wf.status),
                                                "results": wf.results,
                                            }).to_string()
                                        })),
                                        Err(e) => {
                                            Err(format!("Workflow tool '{}' failed: {:?}", tid, e))
                                        },
                                    }
                                })
                            });
                        Some(cb)
                    } else if in_industry_chain {
                        // P2-8: G3 产业链工具由 axagent_analysis_engine::mcp_tools 提供，
                        // 不依赖 astock_client，直接同步执行（纯计算，无网络/DB 调用）。
                        let cb: axagent_runtime::work_engine::ToolCallback = std::sync::Arc::new(
                            move |tn: String, args: serde_json::Value| {
                                Box::pin(async move {
                                    match axagent_analysis_engine::mcp_tools::execute_industry_chain_tool(&tn, &args)
                                    {
                                        Ok(content) => {
                                            Ok(serde_json::json!({ "content": content }))
                                        },
                                        Err(e) => Err(format!(
                                            "industry chain tool '{}' failed: {}",
                                            tn, e
                                        )),
                                    }
                                })
                            },
                        );
                        Some(cb)
                    } else if in_stock_tools {
                        // P0-1: stock_mcp_tools 接通——32 个股票工具之前从未接通工作流执行路径，
                        // ToolResolver 返回 None 导致所有 ToolNode 失败，错误被 core.rs Failed 分支
                        // emit degraded: true 吞掉。现在通过 astock_client.execute_mcp_tool 真正执行。
                        let client = astock_client.clone();
                        let db = db.clone();
                        let cb: axagent_runtime::work_engine::ToolCallback = std::sync::Arc::new(
                            move |tn: String, args: serde_json::Value| {
                                let client = client.clone();
                                let db = db.clone();
                                Box::pin(async move {
                                    // P0-3: 为 compute_valuation 工具注入用户配置的估值参数
                                    let args = crate::commands::stock_analysis::inject_valuation_config_for_tool(
                                        &tn,
                                        &db,
                                        args,
                                    )
                                    .await;
                                    match axagent_astock_data::mcp_tools::execute_mcp_tool(
                                        &client, &tn, &args,
                                    )
                                    .await
                                    {
                                        Ok(content) => {
                                            // P0-2 修复(2026-07-22): 检测空结果(静默降级)。
                                            // get_money_flow 等方法在所有 vendor 失败时返回
                                            // Ok(None/vec![]) 而非 Err,导致节点标记 "completed"
                                            // 但实际数据为空,下游 LLM 误以为数据可用。
                                            // 这里添加 warn 日志让空结果可观测。
                                            let trimmed = content.trim();
                                            if trimmed.is_empty()
                                                || trimmed == "null"
                                                || trimmed == "[]"
                                                || trimmed == "{}"
                                            {
                                                tracing::warn!(
                                                    "[ToolCallback] 工具 '{}' 返回空结果(len={}), \
                                                     可能 vendor 全部降级或缓存未命中",
                                                    tn,
                                                    content.len()
                                                );
                                            }
                                            Ok(serde_json::json!({ "content": content }))
                                        },
                                        Err(e) => Err(format!("stock tool '{}' failed: {}", tn, e)),
                                    }
                                })
                            },
                        );
                        Some(cb)
                    } else {
                        tracing::warn!(
                            "[ToolResolver] 工具 '{}' 未匹配任何解析路径 (known=false, not workflow::, not stock_tools)",
                            tool_name
                        );
                        None
                    }
                })
            },
        );
        // 复用 Tauri 全局 runtime，避免一次性创建/销毁 runtime 的开销。
        tracing::info!("[start_cron_scheduler] 即将调用 set_tool_resolver");
        tauri::async_runtime::block_on(state.work_engine.set_tool_resolver(resolver));
        tracing::info!("[start_cron_scheduler] set_tool_resolver 已完成");
    }

    // 设置 RAG 知识源检索回调（供工作流 Agent 节点从知识库/记忆/Wiki 检索上下文）
    {
        let db = state.harness.db().clone();
        let master_key = state.harness.master_key_owned();
        let vector_store = state.vector_store.clone();
        let rag_callback: axagent_rt_workflow::work_engine::RagCallback = std::sync::Arc::new(
            move |kb_ids: Vec<String>,
                  mem_ids: Vec<String>,
                  wiki_ids: Vec<String>,
                  query: String| {
                let db = db.clone();
                let vector_store = vector_store.clone();
                Box::pin(async move {
                    let embed_fn = crate::indexing::ProviderEmbedFn;
                    let result = axagent_search::rag::collect_rag_context(
                        &db,
                        &master_key,
                        &vector_store,
                        &kb_ids,
                        &mem_ids,
                        &wiki_ids,
                        &query,
                        5,
                        embed_fn,
                    )
                    .await;
                    Ok(result)
                })
            },
        );
        // 复用 Tauri 全局 runtime，避免一次性创建/销毁 runtime 的开销。
        tauri::async_runtime::block_on(state.work_engine.set_rag_callback(rag_callback));
    }

    let work_engine = state.work_engine.clone();
    let cron_store = state.cron_job_store.clone();
    let astock_client = state.astock_client.clone();
    let notification_dispatcher = state.notification_dispatcher.clone();
    let sync_db = state.harness.db().clone();
    let mut executor = CronExecutor::new();
    executor.set_handler(move |job| {
        // 知识源定时刷新：task_type = knowledge_source_fetch_all
        if job.task_type.as_deref() == Some("knowledge_source_fetch_all") {
            let store = cron_store.clone();
            let db = sync_db.clone();
            let job_id = job.id.clone();
            let job_name = job.name.clone();
            let recurring = job.recurring;
            tokio::task::spawn(async move {
                let started = axagent_runtime_core::cron_job::now_millis();
                let results =
                    crate::commands::knowledge_source::run_knowledge_source_sync(&db).await;
                let errors = results.iter().filter(|r| r.action == "error").count();
                let ok = results.len().saturating_sub(errors);
                let result = axagent_runtime_core::TaskRunResult {
                    success: errors == 0,
                    output: Some(format!(
                        "知识源同步完成: {} 成功, {} 失败, 共 {} 源",
                        ok,
                        errors,
                        results.len()
                    )),
                    error: (errors > 0).then(|| format!("{errors} 个知识源抓取失败")).or(None),
                    duration_ms: (axagent_runtime_core::cron_job::now_millis() - started) as u64,
                    executed_at: started,
                };
                tracing::info!(
                    "[CronScheduler] 知识源刷新任务 '{}' 完成: {:?}",
                    job_name,
                    result.output
                );
                store.record_run(&job_id, result).await;
                if !recurring {
                    let _ = store
                        .set_status(&job_id, axagent_runtime_core::CronJobStatus::Disabled)
                        .await;
                }
            });
            return;
        }
        // 荐股定时任务（task_type = stock-recommendation，无 workflow_id）
        if job.workflow_id.is_none() && job.task_type.as_deref() == Some("stock-recommendation") {
            let store = cron_store.clone();
            let client = astock_client.clone();
            let dispatcher = notification_dispatcher.clone();
            let job_id = job.id.clone();
            let job_name = job.name.clone();
            let prompt = job.prompt.clone();
            let recurring = job.recurring;
            tokio::task::spawn(async move {
                let started = axagent_runtime_core::cron_job::now_millis();
                let result = run_recommendation_cron(&client, &dispatcher, &prompt, &job_name)
                    .await
                    .map(|summary| axagent_runtime_core::TaskRunResult {
                        success: true,
                        output: Some(serde_json::to_string(&summary).unwrap_or_default()),
                        error: None,
                        duration_ms: (axagent_runtime_core::cron_job::now_millis() - started)
                            as u64,
                        executed_at: started,
                    })
                    .unwrap_or_else(|e| {
                        tracing::error!("[CronScheduler] 荐股任务 '{}' 失败: {e}", job_name);
                        axagent_runtime_core::TaskRunResult {
                            success: false,
                            output: None,
                            error: Some(e),
                            duration_ms: (axagent_runtime_core::cron_job::now_millis() - started)
                                as u64,
                            executed_at: started,
                        }
                    });
                store.record_run(&job_id, result).await;
                if !recurring {
                    let _ = store
                        .set_status(&job_id, axagent_runtime_core::CronJobStatus::Disabled)
                        .await;
                }
            });
            return;
        }
        if let Some(ref wf_id) = job.workflow_id {
            let engine = work_engine.clone();
            let store = cron_store.clone();
            let wf_id = wf_id.clone();
            let job_id = job.id.clone();
            let job_name = job.name.clone();
            let recurring = job.recurring;
            tokio::task::spawn(async move {
                let started = axagent_runtime_core::cron_job::now_millis();
                let opts = axagent_runtime::work_engine::RunOptions::default();
                let result = match engine.run_workflow(&wf_id, opts).await {
                    Ok(workflow) => {
                        tracing::info!(
                            "[CronScheduler] 工作流任务 '{}' 完成: {:?}",
                            job_name,
                            workflow.status
                        );
                        axagent_runtime_core::TaskRunResult {
                            success: true,
                            output: Some(format!("{:?}", workflow.status)),
                            error: None,
                            duration_ms: (axagent_runtime_core::cron_job::now_millis() - started)
                                as u64,
                            executed_at: started,
                        }
                    },
                    Err(e) => {
                        let err_msg = format!("{:?}", e);
                        tracing::error!(
                            "[CronScheduler] 工作流任务 '{}' 失败: {err_msg}",
                            job_name
                        );
                        axagent_runtime_core::TaskRunResult {
                            success: false,
                            output: None,
                            error: Some(err_msg),
                            duration_ms: (axagent_runtime_core::cron_job::now_millis() - started)
                                as u64,
                            executed_at: started,
                        }
                    },
                };
                store.record_run(&job_id, result).await;
                // 非循环任务执行后禁用
                if !recurring {
                    let _ = store
                        .set_status(&job_id, axagent_runtime_core::CronJobStatus::Disabled)
                        .await;
                }
            });
        } else {
            tracing::info!(
                "[CronScheduler] 触发任务 '{}': {}",
                job.name,
                &job.prompt[..std::cmp::min(job.prompt.len(), 200)]
            );
        }
    });

    let scheduler = Arc::new(CronScheduler::new(store, Arc::new(executor)));

    // 保存到 AppState 以便外部控制（停止/重启）
    {
        let mut state_scheduler = tauri::async_runtime::block_on(state.cron_scheduler.write());
        *state_scheduler = Some(scheduler.clone());
    }

    tauri::async_runtime::spawn(async move {
        scheduler.start().await;
    });

    tracing::info!("[CronScheduler] 已启动（统一 Cron + ScheduledTask），每30秒轮询一次");
}

/// 2.7 P1:启动时从 DB 恢复工作流触发器到运行时 `TriggerManager`。
///
/// 在 `start_cron_scheduler` 之后调用 — `init_trigger_manager` 已在
/// `create_app_state` 中执行,这里只需扫描 `workflow_templates.trigger_config`
/// 字段,对非 Manual 类型触发器批量调用 `register_*`。
///
/// 失败仅 warn 日志,不阻断启动 — 即使所有触发器恢复失败,工作流模板
/// 本身仍然可用,用户可手动触发或通过 update 命令重新激活。
fn start_trigger_recovery(state: &AppState) {
    let db = state.harness.db().clone();
    let trigger_manager = state.work_engine.trigger_manager.clone();
    tauri::async_runtime::spawn(async move {
        let (sched, webhook, event) =
            crate::init::trigger_recovery::recover_workflow_triggers(&db, &trigger_manager).await;
        tracing::info!(
            "[start_trigger_recovery] 触发器恢复完成: {} schedule, {} webhook, {} event",
            sched,
            webhook,
            event
        );
    });
}

/// 3.3 P2:启动 PersistentRunner 后台守护线程。
///
/// 守护线程每 60 秒检查一次 pending session。默认 `enabled: false` 时
/// 守护线程空转 sleep,不会有任何调度行为。
///
/// **注意**:当前 executor 闭包为占位实现,返回 `Err("not implemented")`。
/// 真正的 SessionManager 适配器需后续实现 — 实现后即可通过配置启用持久化重试。
fn start_persistent_runner(state: &AppState) {
    let Some(runner) = state.persistent_runner.clone() else {
        tracing::debug!("[start_persistent_runner] PersistentRunner 未构造,跳过");
        return;
    };

    // 占位 executor — 真正的 SessionManager 适配器需后续实现。
    // 当前返回 Err,让 PersistentRunner 记录 warn 日志但不 panic。
    let executor: axagent_runtime::persistent_runner::SessionExecutor = Arc::new(|_session| {
        Box::pin(async {
            tracing::warn!("[PersistentRunner] SessionExecutor 适配器尚未实现,session 执行被跳过");
            Err("SessionExecutor adapter not yet implemented".to_string())
        })
    });

    // 修复：spawn_daemon 内部调用 tokio::spawn,必须在 tokio runtime 上下文中执行。
    // start_background_services 是同步函数,在 Tauri setup 闭包中直接调用时不在 runtime 上下文,
    // 直接调用 spawn_daemon 会 panic "there is no reactor running"。
    // 用 tauri::async_runtime::spawn 包裹,确保进入 runtime 上下文后再调用 spawn_daemon。
    tauri::async_runtime::spawn(async move {
        let handle = runner.spawn_daemon(60, executor);
        tracing::info!(
            "[start_persistent_runner] 守护线程已启动(默认 enabled=false,空转等待配置启用)"
        );

        // JoinHandle 被 drop 时 tokio 不会取消任务(detach),守护线程继续运行。
        // 若需要优雅关闭,可后续把 handle 挂到 task_manager。
        drop(handle);
    });
}

fn start_trajectory_cleanup(state: &AppState) {
    let trajectory_storage = state.trajectory_storage.clone();
    let handle = state.trajectory_cleanup_handle.clone();
    let shutdown_token = state.shutdown_token.clone();
    let config = axagent_trajectory::TrajectoryCleanupConfig::default();
    let config_for_log = config.clone();
    let interval = std::time::Duration::from_secs(24 * 3600);

    tauri::async_runtime::spawn(async move {
        let task = tokio::spawn(async move {
            let mut tick = tokio::time::interval(interval);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        match trajectory_storage.cleanup(&config).await {
                            Ok(count) if count > 0 => {
                                tracing::info!(
                                    "[trajectory_cleanup] Cleaned up {} old trajectories",
                                    count
                                );
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(
                                    "[trajectory_cleanup] cleanup failed: {}",
                                    e
                                );
                            }
                        }
                    }
                    _ = shutdown_token.cancelled() => {
                        tracing::info!(
                            "[trajectory_cleanup] Received shutdown signal, stopping"
                        );
                        break;
                    }
                }
            }
        });
        *handle.lock().await = Some(task);
    });
    tracing::info!(
        "[trajectory_cleanup] Started with max_age_days={:?}, max_trajectories={:?}, interval=24h",
        config_for_log.max_age_days,
        config_for_log.max_trajectories
    );
}

fn start_index_job_service(app: &tauri::AppHandle, state: &AppState) {
    let db = state.harness.db().clone();
    let vector_store = state.vector_store.clone();
    let master_key = state.harness.master_key_owned();
    let shutdown_token = state.shutdown_token.clone();
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        let service = std::sync::Arc::new(IndexJobService::new(
            db,
            vector_store,
            master_key,
            shutdown_token,
            app_handle,
        ));
        service.start().await;
    });
    tracing::info!("[index_queue] 已启动持久化索引队列 worker");
}

/// 股票全业务管道定时任务：每日 18:00 Asia/Shanghai 自动运行
///
/// 首次延迟 60 秒（等待其他服务初始化），之后计算下一个 18:00 北京时间。
/// 反思阶段由现有 6h cron 接力，此处只管发现+分析+持仓再评估。
fn start_stock_pipeline(app: &tauri::AppHandle, state: &AppState) {
    let db = state.harness.db().clone();
    let client = state.astock_client.clone();
    let engine = state.work_engine.clone();
    let token = state.shutdown_token.clone();
    let app_handle = app.clone();

    state.task_manager.spawn("stock_pipeline", async move {
        // 首次延迟 60 秒，等待其他服务初始化
        let initial_delay = std::time::Duration::from_secs(60);
        tokio::time::sleep(initial_delay).await;

        loop {
            // 计算下一个 18:00 Asia/Shanghai（转为 UTC）
            let next_run = next_18_00_shanghai();
            let wait_duration = next_run.signed_duration_since(chrono::Utc::now());
            let wait_secs = wait_duration.num_seconds().max(60) as u64;

            tracing::info!(
                "[stock_pipeline] 距离下次执行还有 {} 秒（约 {} 小时）",
                wait_secs,
                wait_secs / 3600
            );

            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("[stock_pipeline] 收到关闭信号");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(wait_secs)) => {
                    tracing::info!("[stock_pipeline] 开始执行股票管道");

                    let app_for_progress = app_handle.clone();
                    let progress_callback = std::sync::Arc::new(move |step: &str, detail: &str| {
                        let _ = app_for_progress.emit(
                            "pipeline-step",
                            serde_json::json!({
                                "step": step,
                                "detail": detail,
                                "timestamp": chrono::Utc::now().timestamp_millis(),
                            }),
                        );
                    });

                    let config = crate::commands::stock_pipeline::core::PipelineConfig::default();
                    match crate::commands::stock_pipeline::core::run_stock_pipeline_inner(
                        &db,
                        &client,
                        &engine,
                        &config,
                        None,
                        Some(progress_callback),
                    )
                    .await
                    {
                        Ok(result) => {
                            tracing::info!(
                                "[stock_pipeline] 管道执行完成: {} 候选, {} 新分析, {} 持仓再评估",
                                result.candidates.len(),
                                result.new_analyses.len(),
                                result.reassessed.len()
                            );
                        }
                        Err(e) => {
                            tracing::error!("[stock_pipeline] 管道执行失败: {e}");
                        }
                    }
                }
            }
        }
    });
}

/// 计算下一个 18:00 Asia/Shanghai 时间点（转为 UTC 返回）
fn next_18_00_shanghai() -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    let cst = chrono::FixedOffset::east_opt(8 * 3600).unwrap();
    let now_shanghai = chrono::Local::now().with_timezone(&cst);
    let today_18 = now_shanghai.date_naive().and_hms_opt(18, 0, 0).unwrap();
    let today_18_shanghai = cst.from_local_datetime(&today_18).unwrap();

    if now_shanghai < today_18_shanghai {
        today_18_shanghai.with_timezone(&chrono::Utc)
    } else {
        (today_18_shanghai + chrono::Duration::days(1)).with_timezone(&chrono::Utc)
    }
}

/// P0: 启动 RealtimeMonitor（实时监控引擎）
///
/// 之前 `app_state.stock_monitor` 一直是 `None`，PriceAlertPanel 创建的告警永远
/// 不会触发。此处：
///   1. 构造 `RealtimeMonitor`（注入 AStockClient 作为 MarketDataProvider）
///   2. 注入 `TauriMonitorEmitter`（emit 前端事件 + 写 DB + 推送通知）
///   3. 从 `price_alerts` 表加载未触发告警到 monitor 配置
///   4. 启动 30s 轮询循环（非交易时段自动跳过）
///
/// 启动后 `app_state.stock_monitor` 由 `Some(Arc<RealtimeMonitor>)` 占位，
/// `create_price_alert` 等命令可调用 `monitor.add_config()` 即时加入监控。
fn start_realtime_monitor(app: &tauri::AppHandle, state: &AppState) {
    use crate::commands::stock_workflow::core::trigger_t0_rerun;
    use crate::init::monitor_emitter::TauriMonitorEmitter;
    use axagent_analysis_engine::cross_stock_aggregator::{
        AggregatorConfig, CrossStockSignalAggregator,
    };
    use axagent_analysis_engine::monitor::{MonitorConfig, RealtimeMonitor, TZeroCallback};
    use axagent_entities::price_alerts;
    use axagent_harness::market_data::MarketDataProvider;
    use sea_orm::ColumnTrait;
    use sea_orm::EntityTrait;
    use sea_orm::QueryFilter;

    // 将 Arc<AStockClient> 转为 Arc<dyn MarketDataProvider>
    let provider: Arc<dyn MarketDataProvider> = state.astock_client.clone();

    let monitor = Arc::new(RealtimeMonitor::new(provider));

    // P3-3: 实例化跨股票信号聚合器并注入 monitor
    // 默认配置：5 分钟窗口内 ≥3 只同向信号触发组合级告警，10 分钟冷却
    let aggregator = Arc::new(CrossStockSignalAggregator::new(AggregatorConfig::default()));
    let monitor_for_agg = monitor.clone();
    let agg_for_monitor = aggregator.clone();
    tauri::async_runtime::block_on(async move {
        monitor_for_agg.set_aggregator(agg_for_monitor).await;
    });
    tracing::info!(
        "[realtime_monitor] 跨股票信号聚合器已注入（5min 窗口 / 3 只同向触发 / 10min 冷却）"
    );

    // 注入 TauriMonitorEmitter
    let emitter = Arc::new(TauriMonitorEmitter::new(
        app.clone(),
        state.harness.db().clone(),
        state.notification_dispatcher.clone(),
    ));
    let monitor_clone = monitor.clone();
    tauri::async_runtime::block_on(async move {
        monitor_clone.set_event_emitter(emitter).await;
    });

    // P1-1: 注入 T+0 重跑 callback —— 后端直接调 run_stock_workflow_inner，
    //        不依赖前端 UI 在线。失败仅记录日志，不阻塞监控循环。
    //        spawn 是必要的：callback 内部调用 async fn，且要释放 monitor 的内部锁。
    let app_for_t0 = app.clone();
    let t0_callback: TZeroCallback = Arc::new(move |stock_code: String| {
        let app = app_for_t0.clone();
        Box::pin(async move {
            // 立即 spawn 一个独立任务执行重跑，避免阻塞 monitor 主循环
            // （run_stock_workflow_inner 是长耗时操作，包含 LLM 调用）
            let app_clone = app.clone();
            let stock_clone = stock_code.clone();
            tokio::spawn(async move { trigger_t0_rerun(app_clone, stock_clone).await })
                .await
                .map_err(|e| format!("T+0 重跑任务 panic: {e}"))?
                .map_err(|e| format!("T+0 重跑失败: {e}"))
        })
    });
    let monitor_for_t0 = monitor.clone();
    tauri::async_runtime::block_on(async move {
        monitor_for_t0.set_t0_callback(t0_callback).await;
    });
    tracing::info!("[realtime_monitor] T+0 callback 已注入（后端自动重跑）");

    // 从 price_alerts 表加载未触发的告警到 monitor 配置
    let db = state.harness.db().clone();
    let monitor_for_load = monitor.clone();
    tauri::async_runtime::spawn(async move {
        match price_alerts::Entity::find()
            .filter(price_alerts::Column::IsTriggered.eq(0))
            .all(&db)
            .await
        {
            Ok(alerts) => {
                let count = alerts.len();
                for a in &alerts {
                    // 将 price_alerts 表的 condition 映射到 MonitorConfig
                    // above → take_profit；below → stop_loss
                    let config = MonitorConfig {
                        stock_code: a.stock_code.clone(),
                        stock_name: a.stock_name.clone(),
                        stop_loss: if a.condition == "below" {
                            Some(a.target_price)
                        } else {
                            None
                        },
                        take_profit: if a.condition == "above" {
                            Some(a.target_price)
                        } else {
                            None
                        },
                        resistance_break: None,
                        support_break: None,
                        change_pct_alert: None,
                        turnover_rate_alert: None,
                        enabled: true,
                    };
                    monitor_for_load.add_config(config).await;
                }
                tracing::info!("[realtime_monitor] 已加载 {} 个未触发告警到监控配置", count);
            },
            Err(e) => {
                tracing::warn!("[realtime_monitor] 加载 price_alerts 失败: {e}");
            },
        }
    });

    // 启动 30s 轮询循环（非交易时段自动跳过）
    // P1-1: 从 stock-analysis 工作流模板的 variables 读取用户配置的
    // monitor_poll_interval_secs / monitor_alert_cooldown_secs（若模板/变量缺失则用默认值）
    let monitor_for_start = monitor.clone();
    let db_for_start = state.harness.db().clone();
    tauri::async_runtime::spawn(async move {
        let (poll_secs, cooldown_secs) = load_monitor_runtime_config(&db_for_start).await;
        monitor_for_start.start_with_config(poll_secs, cooldown_secs).await;
    });

    // 写入 AppState，供 create_price_alert 等命令使用
    // OnceLock::set 仅在首次调用成功；后续调用返回 Err 但不影响已启动的 monitor
    if state.stock_monitor.set(monitor).is_err() {
        tracing::warn!("[realtime_monitor] stock_monitor 已初始化，跳过重复 set");
    }
    // P3-3: 聚合器写入 AppState，供 Tauri 命令访问
    if state.cross_stock_aggregator.set(aggregator).is_err() {
        tracing::warn!("[realtime_monitor] cross_stock_aggregator 已初始化，跳过重复 set");
    }

    tracing::info!(
        "[realtime_monitor] 已启动（轮询 + 告警冷却配置由 stock-analysis 模板 variables 决定）"
    );
}

/// P1-1: 从 stock-analysis 工作流模板的 variables JSON 字段读取 monitor 运行时配置。
///
/// 读取变量：
/// - `monitor_poll_interval_secs` (u64, 默认 30)
/// - `monitor_alert_cooldown_secs` (i64, 默认 300)
///
/// 模板或变量缺失时返回默认值，不阻塞 monitor 启动。
async fn load_monitor_runtime_config(db: &sea_orm::DatabaseConnection) -> (u64, i64) {
    use axagent_entities::workflow_template;
    use sea_orm::EntityTrait;

    let default_poll: u64 = 30;
    let default_cooldown: i64 = 300;

    let Ok(Some(tpl)) = workflow_template::Entity::find_by_id("stock-analysis").one(db).await
    else {
        return (default_poll, default_cooldown);
    };

    let Some(vars_json) = tpl.variables.as_ref() else {
        return (default_poll, default_cooldown);
    };

    let Ok(vars) = serde_json::from_str::<serde_json::Value>(vars_json) else {
        return (default_poll, default_cooldown);
    };

    let Some(arr) = vars.as_array() else {
        return (default_poll, default_cooldown);
    };

    let mut poll = default_poll;
    let mut cooldown = default_cooldown;
    for v in arr {
        let Some(name) = v.get("name").and_then(|n| n.as_str()) else {
            continue;
        };
        let Some(value) = v.get("value") else {
            continue;
        };
        match name {
            "monitor_poll_interval_secs" => {
                if let Some(s) = value.as_u64() {
                    poll = s;
                }
            },
            "monitor_alert_cooldown_secs" => {
                if let Some(s) = value.as_i64() {
                    cooldown = s;
                }
            },
            _ => {},
        }
    }

    (poll, cooldown)
}

/// P1-2: 启动实时行情推送引擎（替代前端 15s 轮询）。
///
/// 设计要点：
/// - 构造 `RealTimeQuoteWatcher`，注入 `QuoteCallback` 通过 `app.emit("stock-quote-update", ...)`
///   将行情变更事件推送到前端，前端监听该事件实时更新 UI（延迟从 15s 降到 2s）
/// - 前端通过 `watch_stock_quotes` 命令加入监控列表（Active 优先级 2s，Background 10s）
/// - 启动时无监控股票，等前端加入后才轮询（空列表时 5s 空转 sleep）
/// - 写入 `app_state.quote_watcher` 供命令端使用
fn start_realtime_quote_watcher(app: &tauri::AppHandle, state: &AppState) {
    use axagent_astock_data::realtime_quote::{QuoteChangeEvent, RealTimeQuoteWatcher};
    use futures::future::BoxFuture;
    use tauri::Emitter;

    let db = state.harness.db().clone();

    // 构造 callback（不依赖 tokio runtime）
    let app_for_callback = app.clone();
    let callback: axagent_astock_data::realtime_quote::QuoteCallback =
        Arc::new(move |event: QuoteChangeEvent| {
            let app = app_for_callback.clone();
            let db = db.clone();
            Box::pin(async move {
                // payload 结构与前端 stockAnalysisStore 的 StockQuoteUpdate 类型对齐
                let payload = serde_json::json!({
                    "stockCode": event.stock_code,
                    "current": event.current,
                    "changePct": event.change_pct,
                    "trigger": event.trigger,
                    // 上一帧行情（首次为 null）
                    "previous": event.previous,
                });
                if let Err(e) = app.emit("stock-quote-update", payload) {
                    tracing::trace!("[quote_watcher] emit stock-quote-update 失败: {e}");
                }

                // ── 风控闭环：行情变动 → 条件单评估 → 交易意图写入 ──
                if let Err(e) =
                    axagent_analysis_engine::risk_inspection::evaluate_quote_against_conditions(
                        &db, &event,
                    )
                    .await
                {
                    tracing::warn!(
                        "[quote_watcher] 条件单评估失败: stock={} err={}",
                        event.stock_code,
                        e
                    );
                }
            }) as BoxFuture<'static, ()>
        });

    let watcher = Arc::new(RealTimeQuoteWatcher::new(state.astock_client.clone(), Some(callback)));

    // start() 内部调用 tokio::spawn，必须确保在 runtime 上下文中。
    // start_background_services 是同步函数，Tauri setup 闭包直接调用时不在 runtime 上下文，
    // 用 tauri::async_runtime::spawn 包裹进入 runtime 上下文。
    {
        let w = watcher.clone();
        tauri::async_runtime::spawn(async move {
            let _join_handle = w.start();
        });
    }

    // 写入 AppState，供 watch_stock_quotes 等命令使用
    if state.quote_watcher.set(watcher).is_err() {
        tracing::warn!("[quote_watcher] quote_watcher 已初始化，跳过重复 set");
    }

    tracing::info!("[quote_watcher] 已启动（2s Active / 10s Background 自适应轮询）");
}

/// 启动风控自动巡检服务
///
/// 负责：
/// 1. 从数据库加载条件单到内存引擎
/// 2. 定时过期处理 pending 交易意图（72h 超时 → expired）
/// 3. 定时热加载条件单配置（5min 刷新）
/// 4. 行情回调中的条件单评估已在 `start_realtime_quote_watcher` 中串联
fn start_risk_inspection(_app: &tauri::AppHandle, state: &AppState) {
    let db = state.harness.db().clone();

    tauri::async_runtime::spawn(async move {
        // 延迟 3 秒等数据库完全就绪
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        match axagent_analysis_engine::risk_inspection::start_risk_inspection_service(db).await {
            Ok(()) => {
                tracing::info!("[risk_inspection] 风控自动巡检已启动");
            },
            Err(e) => {
                tracing::warn!("[risk_inspection] 风控自动巡检启动失败: {e}");
            },
        }
    });

    tracing::info!("[risk_inspection] 已触发启动流程（异步等待 DB 就绪）");
}

/// G14: 注册 DojoSdkExecutor — 让 MCP 协议中的 dojo_* / sector_precomputed_* 工具
/// 能路由到 quant / stock-analysis / tools 等具体 crate。
///
/// 执行器实例持有 astock_client 与可选 db 连接，db 通过 `set_db` 在异步任务中注入。
/// 注册后所有通过 MCP 协议调用的 DojoSDK 工具都会走 `DojoSdkExecutorImpl::execute`。
fn register_dojo_sdk_executor(state: &AppState) {
    use crate::commands::dojo_sdk::DojoSdkExecutorImpl;

    let executor = DojoSdkExecutorImpl::new(state.astock_client.clone());
    // 同步注入数据库连接（set_db 是同步函数，不会阻塞）
    executor.set_db(state.harness.db().clone());

    let executor_box: Box<dyn axagent_astock_data::mcp_tools::DojoSdkExecutor> = Box::new(executor);
    axagent_astock_data::mcp_tools::register_dojo_sdk_executor(executor_box);
    tracing::info!("[DojoSDK] DojoSdkExecutor 已注册（6 个 DojoSDK 工具可用）");

    // P2-9: 启动 G19 PLANS_REGISTRY 的 TTL 清理后台任务
    // 默认 TTL=24h，每 1h 清理一次过期 plan，避免长期运行内存膨胀
    // 接受 shutdown_token 以便应用关闭时优雅退出
    crate::commands::dojo_sdk::spawn_plan_ttl_cleanup(state.shutdown_token.clone());
    tracing::info!("[G19 TTL] PLANS_REGISTRY TTL 清理后台任务已启动（TTL=24h，间隔=1h）");
}

/// G17: 创建 CronDeliverySink 适配器，把 MessageGateway 包装成 CronDeliverySink。
///
/// 此函数在 `start_background_services` 中调用，让 cron 调度器通过 sink 投递
/// 执行结果到配置的渠道（Gateway / Webhook / Notification / File）。
///
/// 使用独立的 MessageGateway 实例（与 AppState.platform_manager 解耦）：
/// - Cron 投递是 fire-and-forget，gateway.send_message 失败仅记录 error 不阻塞调度
/// - Gateway 投递需要 endpoint 已注册（前端 IM 平台启动后注册），未注册时该次投递失败
/// - Webhook / File 渠道不依赖 gateway，始终可用
pub fn create_cron_delivery_sink(
    _state: &AppState,
) -> Arc<dyn axagent_harness::cron_delivery::CronDeliverySink> {
    let gateway = Arc::new(axagent_rt_messaging::message_gateway::MessageGateway::new());
    Arc::new(crate::init::cron_delivery_sink::GatewayDeliverySink::new(gateway))
}

/// 检索命中反馈应用定时任务。
///
/// 此前 `retrieval_hits` 表只写不读，形成数据沼泽。本任务每小时：
/// 1. 聚合各 KB 的正/负/无关反馈计数（最近 24 小时窗口）
/// 2. 查询全局反馈统计
/// 3. 记录到日志，作为后续 RAG 自适应优化（RL 检索/embedder 微调）的输入信号
///
/// 第一阶段仅做数据采集与日志记录；真正的权重调整需要后续接入 RL 引擎。
fn start_retrieval_feedback_tick(state: &AppState) {
    let harness_state = state.harness.clone();
    tauri::async_runtime::spawn(async move {
        // 首次延迟 5 分钟启动，避免与启动初始化竞争
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
        let interval = std::time::Duration::from_secs(3600);
        loop {
            tokio::time::sleep(interval).await;

            // 24 小时滑动窗口
            let now = chrono::Utc::now().timestamp();
            let since = now - 86400;

            // 1. 按 KB 聚合反馈
            match axagent_dao::repo::retrieval_hit::aggregate_feedback_by_kb(
                harness_state.db(),
                Some(since),
            )
            .await
            {
                Ok(stats) => {
                    if !stats.is_empty() {
                        tracing::info!(
                            "[retrieval_feedback] 最近 24h KB 反馈聚合：{} 个 KB 有反馈数据",
                            stats.len()
                        );
                        for (kb_id, pos, neg, irr) in &stats {
                            tracing::info!(
                                "[retrieval_feedback] kb={} positive={} negative={} irrelevant={}",
                                kb_id,
                                pos,
                                neg,
                                irr
                            );
                        }
                    }
                },
                Err(e) => {
                    tracing::warn!("[retrieval_feedback] KB 聚合失败: {}", e);
                },
            }

            // 2. 全局反馈统计
            match axagent_dao::repo::retrieval_hit::get_feedback_stats(
                harness_state.db(),
                None,
                Some(since),
            )
            .await
            {
                Ok(stats) => {
                    if stats.total_hits > 0 {
                        tracing::info!(
                            "[retrieval_feedback] 最近 24h 全局统计：total={} positive={} negative={} irrelevant={} no_feedback={} used_in_response={} positive_rate={:.3}",
                            stats.total_hits,
                            stats.positive,
                            stats.negative,
                            stats.irrelevant,
                            stats.no_feedback,
                            stats.used_in_response,
                            stats.positive_rate
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!("[retrieval_feedback] 全局统计失败: {}", e);
                },
            }
        }
    });
    tracing::info!("[retrieval_feedback] 反馈应用定时任务已启动（每小时）");
}
