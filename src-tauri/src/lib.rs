// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::needless_borrow)]

// ── Windows: lib 单元测试 manifest 处理（跟随上游 build.rs 方案） ──
// 上游 build.rs 用 `cargo:rustc-link-arg=/MANIFESTINPUT` 对所有 target
// （含 lib unit test harness）统一嵌入 Common Controls v6 manifest，
// lib.rs 无需再声明 #[link(test-manifest)]（本地旧方案已删除）。

mod android_utils;
mod commands;
mod context_manager;
mod index_queue;
mod indexing;
mod indexing_triggers;
pub mod init;
pub use init::{create_app_state, init_database_with_dir};
mod knowledge_integration;
mod memory_extract;

#[macro_use]
mod register_commands;

pub mod paths;
pub use paths::axagent_home;
mod semantic_cache;
mod smart_router;
pub mod state;

#[macro_use]
mod util;

#[cfg(not(mobile))]
mod tray;
#[cfg(not(mobile))]
mod window_state;

#[cfg(mobile)]
mod tray {
    #[tauri::command]
    #[allow(dead_code)]
    pub fn set_tray_labels(_app: tauri::AppHandle, _show_label: String, _quit_label: String) {}
}

#[cfg(target_os = "windows")]
mod windows_utils;

#[allow(clippy::disallowed_types)]
mod app_state;
mod config_validator;

use tauri::{Emitter, Manager};

pub use app_state::AppState;

/// 在独立线程中创建 current_thread tokio runtime 并执行 async 任务。
///
/// 消除 setup 阶段 7 处重复的 `Builder::new_current_thread().enable_all().build()` 模式。
/// 不能在 Tauri 的 tokio runtime 内直接 block_on，需要在独立线程+独立 runtime 中执行。
fn spawn_block_on<F, T>(task_name: &'static str, f: F) -> std::thread::Result<T>
where
    F: Send + 'static + std::future::Future<Output = T>,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                let msg = format!("Fatal: tokio runtime creation failed for {task_name}: {e}");
                // m6: 非 Android 平台 report_fatal_error 可能为空实现，
                // 额外写入 crash 日志文件确保崩溃原因可追溯。
                android_utils::report_fatal_error(&msg);
                #[cfg(not(target_os = "android"))]
                {
                    eprintln!("{msg}");
                    if let Ok(mut log_dir) =
                        std::env::var("APPDATA").or_else(|_| std::env::var("HOME"))
                    {
                        log_dir.push_str("/axagent-crash.log");
                        let _ = std::fs::write(&log_dir, &msg);
                    }
                }
                panic!("{msg}");
            },
        };
        rt.block_on(f)
    })
    .join()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ── 日志 / tracing（必须在 panic hook 之前初始化） ─
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("AxAgent"),
        );
        if let Err(e) = tracing_log::LogTracer::init() {
            // LogTracer 失败非致命：android_logger 仍可捕获直接 log:: 调用，
            // 只是 tracing 事件不会被转发到 logcat。
            log::error!("Failed to init LogTracer: {} — tracing->log bridge unavailable", e);
        }

        // ── 最早阶段的崩溃诊断标记 ──
        // 此标记在 `android_utils::mark_startup_phase` 可用之前写入，
        // 直接写入外部可访问路径（用户可通过文件管理器读取）。
        tracing::info!("=== AxAgent Android START ===");
        // 注意：使用 append 而非 overwrite，防止跨启动丢失日志
        let boot_msg = "[BOOT] run() entered\n";
        let boot_paths = [
            "/storage/emulated/0/Download/axinvest-crash.log",
            "/storage/emulated/0/Android/data/top.axinvest.desktop/files/axinvest-crash.log",
        ];
        for bp in &boot_paths {
            // 追加而非覆盖
            let existing = std::fs::read_to_string(bp).unwrap_or_default();
            let _ = std::fs::write(bp, existing + &*boot_msg);
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                "%Y-%m-%dT%H:%M:%S%.3f%z".into(),
            ))
            .init();
    }

    // ── 全局 panic hook ──
    std::panic::set_hook(Box::new(|info| {
        let msg = match (
            info.payload().downcast_ref::<&str>(),
            info.payload().downcast_ref::<String>(),
        ) {
            (Some(s), _) => s.to_string(),
            (_, Some(s)) => s.clone(),
            _ => "unknown panic".to_string(),
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        tracing::error!(
            panic.message = %msg,
            panic.location = %location,
            "FATAL: process panicked"
        );
        // 给日志一点时间刷新到 logcat/stderr
        std::thread::sleep(std::time::Duration::from_millis(100));
        android_utils::report_fatal_error(&format!("Panic: {} at {}", msg, location));
    }));

    #[cfg(target_os = "android")]
    {
        tracing::info!("AxAgent starting on Android (tracing -> log -> logcat)");
        android_utils::mark_startup_phase("run_start");
    }

    // ── TLS crypto provider ──
    if rustls::crypto::aws_lc_rs::default_provider().install_default().is_err() {
        let ring_ok = rustls::crypto::ring::default_provider().install_default().is_ok();
        if !ring_ok {
            #[cfg(target_os = "android")]
            tracing::error!(
                "No TLS crypto provider available on Android (aws-lc-rs and ring both failed) — HTTPS will fail"
            );
            #[cfg(not(target_os = "android"))]
            tracing::warn!("No TLS crypto provider available, HTTPS connections may fail");
        } else {
            tracing::info!("TLS: aws-lc-rs unavailable, using ring fallback");
        }
    }

    #[cfg(target_os = "android")]
    crate::android_utils::mark_startup_phase("register_plugins_start");
    let builder = tauri::Builder::default();
    let builder = init::register_plugins(builder);
    #[cfg(target_os = "android")]
    crate::android_utils::mark_startup_phase("register_plugins_done");

    let build_result = builder
        .invoke_handler(register_all_commands!())
        .setup(|app| {
            android_utils::mark_startup_phase("setup_start");

            #[cfg(target_os = "macos")]
            {
                use objc2::msg_send;
                use objc2::rc::Retained;
                use objc2::runtime::{AnyClass, AnyObject};
                // SAFETY:
                // 1. objc2 msg_send! 调用的都是 macOS Foundation 框架中文档完备的 API
                //    (NSUserDefaults、NSString)，其行为和线程安全性有明确保证。
                // 2. AnyClass::get() 使用 .expect() 进行检查，若类不存在会 panic，
                //    这在 #[cfg(target_os = "macos")] 限定下是可接受的——这些类在 macOS 上必然存在。
                // 3. c"" 语法的字符串常量是合法的 C 字符串，以 null 结尾，生命周期为 'static，
                //    传递给 stringWithUTF8String: 是安全的。
                // 4. Retained<AnyObject> 确保返回的 Objective-C 对象遵循正确的引用计数管理，
                //    不会提前释放或泄漏。
                unsafe {
                    let defaults_cls = AnyClass::get(c"NSUserDefaults").expect("NSUserDefaults class exists on macOS");
                    let defaults: Retained<AnyObject> = msg_send![defaults_cls, standardUserDefaults];
                    let str_cls = AnyClass::get(c"NSString").expect("NSString class exists on macOS");
                    let key: Retained<AnyObject> = msg_send![str_cls, stringWithUTF8String: c"AppleShowScrollBars".as_ptr()];
                    let value: Retained<AnyObject> = msg_send![str_cls, stringWithUTF8String: c"WhenScrolling".as_ptr()];
                    let _: () = msg_send![&*defaults, setObject: &*value, forKey: &*key];
                }
            }

            // ── 在主线程解析并创建 axagent_home ──
            // Android 子线程中 dirs::data_dir() 因缺少 JNI 上下文返回 None，
            // 回退到 / 导致 Permission denied。必须在主线程完成目录创建。
            let app_dir = {
                let dir = crate::paths::axagent_home();
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    tracing::error!("Failed to create AxAgent home dir: {}", e);
                    android_utils::report_fatal_error(&format!(
                        "Failed to create AxAgent home dir: {}",
                        e
                    ));
                    panic!("Fatal: AxAgent home dir creation failed: {}", e);
                }
                tracing::info!("axagent_home ready: {}", dir.display());
                dir
            };

            android_utils::mark_startup_phase("db_init_start");

            // 直接使用 Tauri 主 runtime 初始化数据库（无需 spawn_block_on）。
            // sqlx 连接池的 background task 必须创建在长寿命 runtime 上，否则
            //   spawn_block_on 的临时 current_thread runtime 被 drop 后，连接池
            //   内部任务孤儿化 → 后续 acquire() 触发 acquire_timeout(15s) 超时，
            //   精确导致启动时 15 秒空白。
            let db_result = match tauri::async_runtime::block_on(init::init_database_with_dir(app_dir.clone())) {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("Database initialization failed: {}", e);
                    android_utils::report_fatal_error(&format!("Database init failed: {}", e));
                    #[cfg(target_os = "windows")]
                    {
                        windows_utils::show_error_dialog("AxInvest", &format!("数据库初始化失败: {}", e));
                    }
                    // 不要 panic 导致栈溢出；返回 Err 让 Tauri 优雅退出
                    let err_msg = format!("数据库初始化失败: {}", e);
                    return Err(err_msg.into());
                }
            };

            android_utils::mark_startup_phase("db_init_done");

            android_utils::mark_startup_phase("state_init_start");
            let state = match tauri::async_runtime::block_on(init::state::create_app_state(db_result)) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("App state init returned error: {}", e);
                    android_utils::report_fatal_error(&format!("App state init failed: {}", e));
                    return Ok(());
                }
            };

            android_utils::mark_startup_phase("state_init_done");

            app.manage(state);

            let state = app.state::<AppState>();
            let sea_db = state.harness.db().clone();

            // 直接在 Tauri 主 runtime 上 reset 会话（同上：避免连接池跨 runtime 孤儿化）
            if let Err(e) = tauri::async_runtime::block_on(async {
                axagent_dao::repo::agent_session::reset_running_sessions(&sea_db).await
            }) {
                tracing::error!("Session reset failed: {:?}", e);
            }

            // Seed OPC knowledge sources (Wiki + Memory) on first launch
            {
                tauri::async_runtime::block_on(async {
                    init::opc_knowledge::seed_opc_knowledge(&sea_db).await;
                });
            }

            // 同步 OPC 行业包/领域包资产到用户数据目录（CWD 无关，供 seed 与命令读取）
            crate::commands::opc_workflows::ensure_opc_config_synced(&app_dir);

            // Seed OPC professional workflow templates（行业数据资产包驱动）
            {
                tauri::async_runtime::block_on(async {
                    if let Err(e) = crate::commands::opc_workflows::ensure_opc_workflows_seeded(&sea_db, Some(&app_dir)).await {
                        tracing::error!("[opc-workflows] Seed failed: {e}");
                    }
                });
            }

            // Seed OPC company architecture (CEO/CTO/CFO + expert profiles)
            {
                tauri::async_runtime::block_on(async {
                    if let Err(e) = crate::commands::opc_setup::ensure_opc_company_seeded(&sea_db).await {
                        tracing::error!("[opc-company] Seed failed: {e}");
                    }
                });
            }

            // Initialize pricing configuration from pricing.toml
            commands::agent::init_pricing_config(app.handle());

            // 注入 Orchestrator 流式报告器（绑定 AppHandle 以便推送事件到前端）
            {
                let reporter = commands::orchestrator::create_stream_reporter(app.handle().clone());
                let stream_reporter = state.stream_reporter.clone();
                tauri::async_runtime::block_on(async {
                    *stream_reporter.write().await = Some(reporter);
                });
            }

            // m7: validate agent_roles.yaml schema at startup
            {
                let config_dir = app_dir.join("config");
                let roles_path = config_dir.join("agent_roles.yaml");
                if roles_path.exists() {
                    config_validator::validate_agent_roles(&roles_path.to_string_lossy());

                    // 将 YAML 中启用的角色 upsert 到 agent_roles 表
                    // （AxInvest 本地：OPC 角色 opc_financial_clerk 等由此入 DB）
                    if let Ok(content) = std::fs::read_to_string(&roles_path) {
                        let roles = config_validator::parse_enabled_roles(&content);
                        if !roles.is_empty() {
                            let db = state.harness.db().clone();
                            tauri::async_runtime::block_on(async {
                                for r in &roles {
                                    let name = r.name.as_deref().unwrap_or("");
                                    let prompt = r.system_prompt.as_deref().unwrap_or("");
                                    let tools: Vec<String> =
                                        r.allowed_tools.clone().unwrap_or_default();
                                    let max_conc = r.max_concurrent.unwrap_or(1) as i32;
                                    let timeout = r.timeout_seconds.unwrap_or(600) as i64;

                                    match axagent_dao::repo::agent_role::upsert_agent_role(
                                        &db, name, name, None, prompt, &tools, &[], max_conc,
                                        timeout, "file:agent_roles.yaml",
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            tracing::info!("[opc] Seeded agent role: {name}")
                                        }
                                        Err(e) => tracing::warn!(
                                            "[opc] Failed to seed role {name}: {e}"
                                        ),
                                    }
                                }
                            });
                        }
                    }
                }
            }

            if let Some(home) = dirs::home_dir() {
                let user_md_path = home.join(".axinvest").join("USER.md");
                if user_md_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&user_md_path) {
                        if let Some(profile) = axagent_trajectory::UserProfile::from_user_md(&content) {
                            let user_profile = state.user_profile.clone();
                            spawn_block_on("user_profile", async move {
                                let mut p = user_profile.write().await;
                                *p = profile;
                                tracing::info!("[user-profile] Loaded profile from USER.md ({} preferences, {} expertise domains)",
                                    p.preferences.len(), p.expertise.len());
                            })
                            .unwrap_or_else(|e| {
                                tracing::error!("User profile thread panicked: {:?}", e);
                            });
                        }
                    }
                }
            }

            if let Ok(persisted) = tauri::async_runtime::block_on(state.trajectory_storage.get_patterns()) {
                if !persisted.is_empty() {
                    let pattern_count = persisted.len();
                    let pattern_learner = state.pattern_learner.clone();
                    spawn_block_on("pattern_learner", async move {
                        let mut pl = pattern_learner.write().await;
                        for pattern in &persisted {
                            pl.learn_from_trajectory(&axagent_trajectory::Trajectory {
                                id: pattern.id.clone(),
                                session_id: String::new(),
                                user_id: String::new(),
                                topic: pattern.name.clone(),
                                summary: pattern.description.clone(),
                                outcome: if pattern.success_rate >= 0.5 {
                                    axagent_trajectory::TrajectoryOutcome::Success
                                } else {
                                    axagent_trajectory::TrajectoryOutcome::Failure
                                },
                                duration_ms: 0,
                                quality: axagent_trajectory::TrajectoryQuality {
                                    overall: pattern.average_quality,
                                    task_completion: pattern.average_quality,
                                    tool_efficiency: pattern.average_quality,
                                    reasoning_quality: pattern.average_quality,
                                    user_satisfaction: pattern.average_quality,
                                },
                                value_score: pattern.average_value_score,
                                patterns: vec![],
                                steps: vec![],
                                rewards: vec![],
                                created_at: pattern.created_at,
                                replay_count: 0,
                                last_replay_at: None,
                            });
                        }
                    })
                    .unwrap_or_else(|e| {
                        tracing::error!("Pattern learner thread panicked: {:?}", e);
                    });
                    tracing::info!("[P5] Loaded {} persisted patterns into PatternLearner", pattern_count);
                }
            }

            let app_dir = state.app_data_dir.clone();

            #[cfg(not(mobile))]
            if let Some(main_window) = app.get_webview_window("main") {
                #[cfg(target_os = "windows")]
                {
                    let _ = main_window.set_decorations(false);
                    let _ = main_window.set_minimizable(true);
                    let _ = main_window.set_maximizable(true);
                }

                if let Some(saved_state) = window_state::load_window_state(&app_dir) {
                    let restored_state = if let Ok(Some(monitor)) = main_window.current_monitor() {
                        let monitor_size = monitor.size().to_logical::<f64>(main_window.scale_factor().unwrap_or(1.0));
                        window_state::clamp_window_state_to_monitor(saved_state, monitor_size.width, monitor_size.height)
                    } else {
                        saved_state
                    };

                    let _ = main_window.set_size(tauri::LogicalSize::new(restored_state.width, restored_state.height));
                    if let (Some(x), Some(y)) = (restored_state.x, restored_state.y) {
                        let _ = main_window.set_position(tauri::LogicalPosition::new(x, y));
                    } else {
                        let _ = main_window.center();
                    }
                    if restored_state.fullscreen {
                        let _ = main_window.set_fullscreen(true);
                    } else if restored_state.maximized {
                        let _ = main_window.maximize();
                    }
                }
            }

            #[cfg(mobile)]
            if let Some(ref sync_engine) = state.sync_engine {
                tracing::info!("[mobile] Starting cloud sync engine...");
                let engine = sync_engine.clone();
                spawn_block_on("cloud_sync", async move {
                    match engine.backend.check_connection().await {
                        Ok(true) => tracing::info!("[mobile] Cloud sync backend connected"),
                        Ok(false) => tracing::warn!("[mobile] Cloud sync backend unreachable"),
                        Err(e) => tracing::warn!("[mobile] Cloud sync connection check failed: {}", e),
                    }
                }).unwrap_or_else(|e| {
                    tracing::error!("Mobile sync thread panicked: {:?}", e);
                });
            }

            let state = app.state::<AppState>();
            #[cfg(not(mobile))]
            let tray_language = {
                let db = state.harness.db().clone();
                tauri::async_runtime::block_on(
                    axagent_dao::repo::settings::get_settings(&db),
                )
                .map(|s| s.language)
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to get tray language: {}", e);
                    "en".to_string()
                })
            };
            #[cfg(mobile)]
            let tray_language = "en".to_string();

            // 异步启动：不阻塞 UI — 种子化股票分析专家/角色/Profile/工作流模板（UPSERT 幂等）
            let seed_db = state.harness.db().clone();
            // 仅提取 'static 安全的 Arc/PathBuf 字段进 async 闭包（State 借用本身不可跨闭包）
            let concept_index = state.concept_index.clone();
            let app_data_dir = state.app_data_dir.clone();
            tauri::async_runtime::spawn(async move {
                // 1. 种子化股票分析专家/角色/Profile/工作流模板
                //   （OPC 行业无运行时容器，命令直读行业包，无需启动初始化）
                if let Err(e) = crate::commands::stock_analysis_setup::ensure_stock_analysis_experts_seeded(&seed_db).await {
                    tracing::error!("[startup] 股票业务种子化失败: {e}");
                }
                // 2. 构建全局 ConceptIndex（49行业+163概念本体 + lemonhu 知识库）
                {
                    let idx = crate::commands::stock_analysis_setup::seed_concept_index::ensure_concept_index(&seed_db, &app_data_dir).await;
                    let mut w = concept_index.write().await;
                    *w = idx;
                    tracing::info!("[startup] ConceptIndex 构建完成");
                }
                // 3. 同步内置 SKILL.md 到用户目录（同步操作，快速完成）
                crate::commands::skills::seed_builtin_skills();
                // 4. Multi-Agent 固定角色（analyst/implementer/reviewer）种子化
                // 幂等 upsert，每次启动都调用，确保三个内置角色记录存在
                if let Err(e) = crate::commands::multi_agent_setup::seed_multi_agent_roles::seed_multi_agent_roles(&seed_db).await {
                    tracing::warn!("[multi_agent_setup] 种子化 Multi-Agent 角色失败: {}", e);
                }
            });
            // 注入 OPC 通知发送 channel + 后台 worker（AxInvest 本地薄补丁：
            // OPC 台账工具 OpcSendNotification 经 8 渠道消息网关发送）
            {
                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                axagent_tools::tools::opc::set_opc_notify_tx(tx);

                let pm = state.platform_manager.clone();
                let db = state.harness.db().clone();
                tauri::async_runtime::spawn(async move {
                    while let Some(notif) = rx.recv().await {
                        let config =
                            axagent_dao::repo::platform_config::get_platform_config(&db).await;
                        match pm.get_adapter(&notif.platform).await {
                            Some(adapter) => {
                                if let Err(e) = adapter
                                    .send_message(&config, &notif.chat_id, &notif.message, None)
                                    .await
                                {
                                    tracing::error!(
                                        "[opc-notify] {}/{}: {e}",
                                        notif.platform,
                                        notif.chat_id
                                    );
                                } else {
                                    tracing::info!(
                                        "[opc-notify] Sent via {} to {}",
                                        notif.platform,
                                        notif.chat_id
                                    );
                                }
                            }
                            None => tracing::warn!(
                                "[opc-notify] Platform {} not available",
                                notif.platform
                            ),
                        }
                    }
                });
                tracing::info!("[opc] Started OPC notify worker");
            }

            init::services::start_background_services(app.handle(), &state, app_dir.clone(), tray_language);

            android_utils::mark_startup_phase("setup_complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                use std::sync::atomic::Ordering;
                match event {
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                        #[cfg(not(mobile))]
                        {
                            let app = window.app_handle();
                            let state = app.state::<AppState>();
                            let maximized = window.is_maximized().unwrap_or(false);
                            let fullscreen = window.is_fullscreen().unwrap_or(false);
                            let scale_factor = window.scale_factor().unwrap_or(1.0);
                            let prev = window_state::load_window_state(&state.app_data_dir);
                            if maximized || fullscreen {
                                if let Some(mut prev) = prev {
                                    prev.maximized = maximized;
                                    prev.fullscreen = fullscreen;
                                    let _ = window_state::save_window_state(&state.app_data_dir, prev);
                                }
                            } else if let (Ok(size), Ok(pos)) = (window.inner_size(), window.outer_position()) {
                                let logical_w = size.width as f64 / scale_factor;
                                let logical_h = size.height as f64 / scale_factor;
                                let logical_x = pos.x as f64 / scale_factor;
                                let logical_y = pos.y as f64 / scale_factor;
                                let _ = window_state::save_window_state(&state.app_data_dir, window_state::PersistedWindowState {
                                    width: logical_w, height: logical_h, maximized: false, fullscreen: false,
                                    x: Some(logical_x), y: Some(logical_y),
                                });
                            }
                        }
                    }
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        let app = window.app_handle();
                        let state = app.state::<AppState>();
                        if state.close_to_tray.load(Ordering::Acquire) {
                            let _ = window.hide();
                            api.prevent_close();
                        } else {
                            api.prevent_close();
                            let _ = app.emit("app-close-requested", ());
                        }
                    }
                    _ => {}
                }
            }
            if window.label() == "quickbar" {
                if let tauri::WindowEvent::Focused(false) = event {
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!());

    #[cfg(target_os = "android")]
    crate::android_utils::mark_startup_phase("build_done");

    let app = match build_result {
        Ok(app) => app,
        Err(e) => {
            let error_msg = e.to_string();
            tracing::error!("Failed to build Tauri application: {}", error_msg);
            android_utils::report_fatal_error(&format!("Tauri build failed: {}", error_msg));
            #[cfg(target_os = "windows")]
            {
                let lower = error_msg.to_lowercase();
                if lower.contains("webview2") || lower.contains("webview") || lower.contains("edge")
                {
                    const WEBVIEW2_DOWNLOAD_URL: &str = "https://developer.microsoft.com/en-us/microsoft-edge/webview2/?form=MA13LH#download";
                    let user_ok = windows_utils::show_warning_ok_cancel(
                        "AxInvest",
                        "未检测到 Microsoft Edge WebView2 Runtime，AxInvest 无法启动。\n\n点击「确定」打开下载页面进行安装，安装完成后重新启动 AxInvest。",
                    );
                    if user_ok {
                        let _ = open::that(WEBVIEW2_DOWNLOAD_URL);
                    }
                } else {
                    windows_utils::show_error_dialog(
                        "AxInvest",
                        &format!("应用启动失败：{}", error_msg),
                    );
                }
            }
            // SECURITY (C11): 替换 process::exit 为 panic!，让构建框架的回调
            // 负责清理（WAL 刷写 / 资源释放），而非直接硬杀进程。
            panic!("Fatal: application build failed: {}", error_msg);
        },
    };

    #[cfg(target_os = "android")]
    crate::android_utils::mark_startup_phase("run_loop_start");

    app.run(|_app, _event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen {
            has_visible_windows,
            ..
        } = _event
        {
            if !has_visible_windows {
                if let Some(w) = _app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        }

        // 优雅关闭：先关闭所有 WebView 窗口，让 WebView2 有时间完成内部清理，
        // 避免退出时出现 "Failed to unregister class Chrome_WidgetWin_0. Error = 1412" 错误。
        if let tauri::RunEvent::Exit = _event {
            let state = _app.state::<AppState>();
            state.shutdown_token.cancel();
            if let Some(flag) = state.skill_watcher_shutdown.get() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            tracing::info!("[shutdown] 正在停止后台任务...");

            // 显式销毁所有 WebView 窗口。必须用 destroy() 而非 close()：
            // close() 会触发 CloseRequested 事件，而本应用在 on_window_event 中
            // 对 main 窗口无条件 prevent_close()，导致 close() 请求被自己拦截、
            // 窗口永远关不掉。进程退出时 Chromium 在 browser process 内注销
            // Chrome_WidgetWin_0 窗口类，发现该类仍有存活窗口句柄，
            // UnregisterClass 返回 ERROR_CLASS_HAS_WINDOWS (1412)。
            // destroy() 绕过 CloseRequested，同步销毁 HWND，保证退出时窗口类计数归零。
            let windows: Vec<_> = _app.webview_windows().keys().cloned().collect();
            for label in &windows {
                if let Some(w) = _app.get_webview_window(label) {
                    tracing::info!("[shutdown] 正在销毁窗口: {}", label);
                    let _ = w.destroy();
                }
            }

            // 在独立线程中创建 current_thread runtime 执行清理，
            // 避免在已有 tokio runtime 中调用 block_on 导致 panic。
            // 即使当前线程已在 runtime 中，新线程中创建独立 runtime 是安全的。
            let auto_backup_handle = state.auto_backup_handle.clone();
            let webdav_sync_handle = state.webdav_sync_handle.clone();
            let api_server_handle = state.api_server_handle.clone();
            let trajectory_cleanup_handle = state.trajectory_cleanup_handle.clone();
            let plugin_manager = state.plugin_manager.clone();
            let dashboard_registry = state.dashboard_registry.clone();
            let task_manager = state.task_manager.clone();

            let shutdown_result = spawn_block_on("shutdown", async move {
                let timeout = std::time::Duration::from_secs(5);

                macro_rules! await_handle {
                    ($handle:expr, $name:expr) => {
                            let mut guard = $handle.lock().await;
                            if let Some(mut h) = guard.take() {
                                match tokio::time::timeout(timeout, &mut h).await {
                                    Ok(Ok(())) => tracing::info!("[shutdown] {} 已优雅停止", $name),
                                    Ok(Err(e)) => tracing::warn!("[shutdown] {} join 错误: {}", $name, e),
                                    Err(_) => {
                                        tracing::warn!("[shutdown] {} 超时 ({:?})，强制中止", $name, timeout);
                                        h.abort();
                                    },
                                }
                            }
                        };
                    }

                    await_handle!(auto_backup_handle, "auto_backup");
                    await_handle!(webdav_sync_handle, "webdav_sync");
                    await_handle!(api_server_handle, "api_server");
                    await_handle!(trajectory_cleanup_handle, "trajectory_cleanup");

                    // 停止所有插件（MCP 服务、agents、skills）
                    tracing::info!("[shutdown] 正在停止插件...");
                    match tokio::task::spawn_blocking(move || {
                        let mut manager = plugin_manager.blocking_write();
                        manager.stop_all_plugins();
                    })
                    .await
                    {
                        Ok(()) => tracing::info!("[shutdown] 所有插件已停止"),
                        Err(e) => tracing::warn!("[shutdown] 插件停止任务异常: {e}"),
                    }

                    // 停止 Dashboard 插件
                    if let Some(registry) = dashboard_registry {
                        tracing::info!("[shutdown] 正在卸载 Dashboard 插件...");
                        let plugins = registry.list_plugins().await;
                        for plugin_info in plugins {
                            if let Err(e) = registry.unregister(&plugin_info.id).await {
                                tracing::warn!(
                                    "[shutdown] 卸载 Dashboard 插件 {} 失败: {e}",
                                    plugin_info.id
                                );
                            }
                        }
                        tracing::info!("[shutdown] Dashboard 插件已卸载");
                    }

                    // 集中式 TaskManager 兜底清理
                    task_manager.shutdown(std::time::Duration::from_secs(5)).await;
                    tracing::info!("[shutdown] 退出完成");
                });

            if let Err(e) = shutdown_result {
                tracing::error!("[shutdown] 清理线程 panic: {:?}", e);
            }
        }
    });
}
