#![allow(clippy::result_large_err)]
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::needless_borrow)]

mod android_utils;
mod commands;
mod context_manager;
mod indexing;
mod indexing_triggers;
mod init;
mod knowledge_integration;
mod memory_extract;

#[macro_use]
mod register_commands;

mod paths;
mod semantic_cache;
mod smart_router;
pub mod state;
mod util;

#[cfg(not(mobile))]
mod tray;
#[cfg(not(mobile))]
mod window_state;

#[cfg(mobile)]
mod tray {
    #[tauri::command]
    pub fn set_tray_labels(_app: tauri::AppHandle, _show_label: String, _quit_label: String) {}
}

#[cfg(target_os = "windows")]
mod windows_utils;

#[allow(clippy::disallowed_types)]
mod app_state;

use tauri::{Emitter, Manager};

pub use app_state::AppState;

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
    if rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .is_err()
    {
        let ring_ok = rustls::crypto::ring::default_provider()
            .install_default()
            .is_ok();
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

            let db_result = match std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .or_else(|e| {
                        tracing::warn!("Failed to create multi-threaded runtime for DB init: {} — falling back to current-thread", e);
                        tokio::runtime::Builder::new_current_thread().enable_all().build()
                    })
                    .unwrap_or_else(|e| {
                        android_utils::report_fatal_error(&format!("Failed to create db init runtime: {}", e));
                        panic!("Fatal: db init runtime creation failed: {}", e);
                    });
                rt.block_on(init::init_database_with_dir(app_dir))
            }).join() {
                Ok(Ok(result)) => result,
                Ok(Err(e)) => {
                    tracing::error!("Database initialization failed: {}", e);
                    android_utils::report_fatal_error(&format!("Database init failed: {}", e));
                    #[cfg(target_os = "windows")]
                    {
                        windows_utils::show_error_dialog("AxAgent", &format!("数据库初始化失败: {}", e));
                    }
                    panic!("Fatal: database initialization failed: {}", e);
                }
                Err(e) => {
                    tracing::error!("DB init thread panicked: {:?}", e);
                    android_utils::report_fatal_error(&format!("DB init thread panicked: {:?}", e));
                    panic!("Fatal: DB init thread panicked: {:?}", e);
                }
            };

            android_utils::mark_startup_phase("db_init_done");

            // 在独立线程中运行初始化，避免在 Tauri 的 tokio runtime 内创建嵌套 Runtime
            android_utils::mark_startup_phase("state_init_start");
            let state = match std::thread::spawn(move || {
                init::state::create_app_state(db_result)
            }).join() {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => {
                    tracing::error!("App state init returned error: {}", e);
                    android_utils::report_fatal_error(&format!("App state init failed: {}", e));
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!("App state init thread panicked: {:?}", e);
                    android_utils::report_fatal_error(&format!("App state init thread panicked: {:?}", e));
                    panic!("Fatal: App state init thread panicked: {:?}", e);
                }
            };

            android_utils::mark_startup_phase("state_init_done");

            app.manage(state);

            let state = app.state::<AppState>();
            let sea_db = state.harness.db().clone();

            let sea_db2 = sea_db.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new()
                    .or_else(|e| {
                        tracing::warn!("Failed to create multi-threaded runtime for session reset: {} — falling back to current-thread", e);
                        tokio::runtime::Builder::new_current_thread().enable_all().build()
                    })
                    .unwrap_or_else(|e| {
                        android_utils::report_fatal_error(&format!("Failed to create session reset runtime: {}", e));
                        panic!("Fatal: session reset runtime creation failed: {}", e);
                    });
                rt.block_on(async {
                    let _ = axagent_core::repo::agent_session::reset_running_sessions(&sea_db2).await;
                });
            }).join().unwrap_or_else(|e| {
                tracing::error!("Session reset thread panicked: {:?}", e);
            });

            // Initialize pricing configuration from pricing.toml
            commands::agent::init_pricing_config(app.handle());

            if let Some(home) = dirs::home_dir() {
                let user_md_path = home.join(".axinvest").join("USER.md");
                if user_md_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&user_md_path) {
                        if let Some(profile) = axagent_trajectory::UserProfile::from_user_md(&content) {
                            let user_profile = state.user_profile.clone();
                            std::thread::spawn(move || {
                                let rt = tokio::runtime::Runtime::new()
                                    .or_else(|e| {
                                        tracing::warn!("Failed to create multi-threaded runtime for user profile: {} — falling back to current-thread", e);
                                        tokio::runtime::Builder::new_current_thread().enable_all().build()
                                    })
                                    .unwrap_or_else(|e| {
                                        android_utils::report_fatal_error(&format!("Failed to create tokio runtime: {}", e));
                                        panic!("Fatal: user profile runtime creation failed: {}", e);
                                    });
                                rt.block_on(async {
                                    let mut p = user_profile.write().await;
                                    *p = profile;
                                    tracing::info!("[user-profile] Loaded profile from USER.md ({} preferences, {} expertise domains)",
                                        p.preferences.len(), p.expertise.len());
                                });
                            }).join().unwrap_or_else(|e| {
                                tracing::error!("User profile thread panicked: {:?}", e);
                            });
                        }
                    }
                }
            }

            if let Ok(persisted) = state.trajectory_storage.get_patterns() as Result<Vec<axagent_trajectory::TrajectoryPattern>, _> {
                if !persisted.is_empty() {
                    let pattern_count = persisted.len();
                    let pattern_learner = state.pattern_learner.clone();
                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new()
                            .or_else(|e| {
                                tracing::warn!("Failed to create multi-threaded runtime for pattern learner: {} — falling back to current-thread", e);
                                tokio::runtime::Builder::new_current_thread().enable_all().build()
                            })
                            .unwrap_or_else(|e| {
                                android_utils::report_fatal_error(&format!("Failed to create tokio runtime: {}", e));
                                panic!("Fatal: pattern learner runtime creation failed: {}", e);
                            });
                        rt.block_on(async {
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
                        });
                    }).join().unwrap_or_else(|e| {
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
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new()
                        .or_else(|e| {
                            tracing::warn!("Failed to create multi-threaded runtime for cloud sync: {} — falling back to current-thread", e);
                            tokio::runtime::Builder::new_current_thread().enable_all().build()
                        })
                        .unwrap_or_else(|e| {
                            android_utils::report_fatal_error(&format!("Failed to create cloud sync runtime: {}", e));
                            panic!("Fatal: cloud sync runtime creation failed: {}", e);
                        });
                    rt.block_on(async {
                        match engine.backend.check_connection().await {
                            Ok(true) => tracing::info!("[mobile] Cloud sync backend connected"),
                            Ok(false) => tracing::warn!("[mobile] Cloud sync backend unreachable"),
                            Err(e) => tracing::warn!("[mobile] Cloud sync connection check failed: {}", e),
                        }
                    });
                }).join().unwrap_or_else(|e| {
                    tracing::error!("Mobile sync thread panicked: {:?}", e);
                });
            }

            let state = app.state::<AppState>();
            #[cfg(not(mobile))]
            let tray_language = {
                let db = state.harness.db().clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap_or_else(|e| {
                                android_utils::report_fatal_error(&format!("Failed to create tokio runtime: {}", e));
                                panic!("Fatal: tray language runtime creation failed: {}", e);
                            });
                    rt.block_on(axagent_core::repo::settings::get_settings(&db))
                        .map(|s| s.language)
                        .unwrap_or_else(|_| "en".to_string())
                }).join().unwrap_or_else(|e| {
                    tracing::error!("Tray language thread panicked: {:?}", e);
                    "en".to_string()
                })
            };
            #[cfg(mobile)]
            let tray_language = "en".to_string();

            // 异步启动：不阻塞 UI
            let _seed_db = state.harness.db().clone();
            tauri::async_runtime::spawn(async move {
                // 股票业务种子化（stock_analysis_setup）已在另一分支维护
            });
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
                        "AxAgent",
                        "未检测到 Microsoft Edge WebView2 Runtime，AxAgent 无法启动。\n\n点击「确定」打开下载页面进行安装，安装完成后重新启动 AxAgent。",
                    );
                    if user_ok {
                        let _ = open::that(WEBVIEW2_DOWNLOAD_URL);
                    }
                } else {
                    windows_utils::show_error_dialog(
                        "AxAgent",
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

        // 优雅关闭：通知后台任务停止并等待完成 (S-39)
        if let tauri::RunEvent::Exit = _event {
            let state = _app.state::<AppState>();
            state.shutdown_token.cancel();
            if let Some(flag) = state.skill_watcher_shutdown.get() {
                flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            tracing::info!("[shutdown] 正在停止后台任务...");

            let rt_handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| {
                tokio::runtime::Runtime::new()
                    .expect("Failed to create runtime for cleanup")
                    .handle()
                    .clone()
            });

            let timeout = std::time::Duration::from_secs(5);
            let await_handle = |handle: &std::sync::Arc<
                tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
            >,
                                name: &str| {
                let mut guard = rt_handle.block_on(handle.lock());
                if let Some(mut h) = guard.take() {
                    match rt_handle.block_on(async { tokio::time::timeout(timeout, &mut h).await })
                    {
                        Ok(Ok(())) => tracing::info!("[shutdown] {} 已优雅停止", name),
                        Ok(Err(e)) => tracing::warn!("[shutdown] {} join 错误: {}", name, e),
                        Err(_) => {
                            tracing::warn!("[shutdown] {} 超时 ({:?})，强制中止", name, timeout);
                            h.abort();
                        },
                    }
                }
            };

            await_handle(&state.auto_backup_handle, "auto_backup");
            await_handle(&state.webdav_sync_handle, "webdav_sync");
            await_handle(&state.api_server_handle, "api_server");
            await_handle(&state.trajectory_cleanup_handle, "trajectory_cleanup");
            // 集中式 TaskManager 兜底清理
            rt_handle.block_on(
                state
                    .task_manager
                    .shutdown(std::time::Duration::from_secs(5)),
            );
            tracing::info!("[shutdown] 退出完成");
        }
    });
}
