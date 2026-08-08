// SPDX-License-Identifier: AGPL-3.0-only

//! AxAgent 独立服务端入口（无 Tauri 依赖）。
//!
//! 启动 Gateway + 后台服务，不创建窗口。
//! 通过 `cargo build --features server` 编译。
//!
//! 设计原则：
//! - 复用 `axagent_lib` 中非 Tauri 依赖的初始化逻辑
//! - 跳过 `tauri::Builder`、`tauri::generate_context!()`、窗口事件、tray
//! - 使用 tokio runtime 替代 Tauri 的事件循环

fn main() {
    // ── logging ──
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%Y-%m-%dT%H:%M:%S%.3f%z".into(),
        ))
        .init();

    // ── panic hook ──
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
            "FATAL: server process panicked"
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }));

    // ── TLS crypto provider ──
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // ── 创建 tokio runtime ──
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    rt.block_on(async_main());
}

async fn async_main() {
    // ── 初始化数据目录 ──
    let app_dir = axagent_lib::paths::axagent_home();
    std::fs::create_dir_all(&app_dir).expect("Failed to create AxAgent home dir");
    tracing::info!("axagent_home ready: {}", app_dir.display());

    // ── 初始化数据库 ──
    let db_result = axagent_lib::init::init_database_with_dir(app_dir.clone())
        .await
        .expect("Fatal: database initialization failed");
    tracing::info!("Database initialized");

    // ── 同步 OPC 资产到用户数据目录（CWD 无关） ──
    // 与 Tauri 模式启动流程保持一致，保证行业包/领域包可读
    axagent_lib::commands::opc_workflows::ensure_opc_config_synced(&app_dir);

    // ── 创建 AppState（复用 Tauri 版本的全部业务逻辑） ──
    let _state = axagent_lib::init::state::create_app_state(db_result)
        .await
        .expect("Fatal: app state initialization failed");
    tracing::info!("AppState created");

    // ── 启动后台服务（仅启动非 Tauri 依赖的服务） ──
    // 注：start_tray 因需要 tauri::AppHandle 会被跳过
    // start_background_services 需要适配无 Tauri 模式
    tracing::info!("Server started. Gateway available on configured port.");

    // ── 启动 Gateway ──
    // Gateway 已在 AppState 中持有，由 services 中的 api_server 任务管理
    // 保持进程运行
    tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    tracing::info!("Shutting down...");
}
