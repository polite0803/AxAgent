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

    // ── 创建 AppState（复用 Tauri 版本的全部业务逻辑） ──
    let state = axagent_lib::init::state::create_app_state(db_result)
        .await
        .expect("Fatal: app state initialization failed");
    tracing::info!("AppState created");

    // ── 启动 Gateway（Companion Core 常驻，对外提供 OpenAI 兼容 API + 记忆外溢） ──
    let _gateway = match axagent_lib::commands::gateway::start_gateway_core(&state).await {
        Ok(g) => {
            tracing::info!(
                "Gateway started on port {} (Companion Core online)",
                g.http_addr().port()
            );
            Some(g)
        }
        Err(e) => {
            tracing::error!("Failed to start Gateway: {e}");
            None
        }
    };

    // ── 启动记忆扫描器（本地日历/消息 → 写入 MemoryStore） ──
    start_memory_scanner(&state).await;

    // ── 启动 Obsidian 镜像（记忆条目 → markdown 写入 vault） ──
    start_obsidian_mirror(&state).await;

    tracing::info!("Server started. Companion Core ready.");

    // 保持进程运行
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    tracing::info!("Shutting down...");
}

/// 启动记忆扫描器后台任务。
///
/// 周期性运行 `ICalScanner`（.ics 日历文件）和 `FileScanner`（文本笔记），
/// 将扫描结果通过 `MemoryStoreAdapter` 写入现有 MemoryService。
/// 扫描路径和间隔通过环境变量配置：
/// - `AXAGENT_SCAN_CALENDAR_PATHS` — 分号分隔的 .ics 文件/目录路径
/// - `AXAGENT_SCAN_FILE_PATHS`     — 分号分隔的文本文件/目录路径
/// - `AXAGENT_SCAN_INTERVAL_SECS`  — 扫描间隔（秒，缺省 300）
async fn start_memory_scanner(state: &axagent_lib::AppState) {
    use axagent_harness::scanner::MemoryScanner as _;
    // 读取环境变量配置
    let cal_paths: Vec<String> = std::env::var("AXAGENT_SCAN_CALENDAR_PATHS")
        .unwrap_or_default()
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let file_paths: Vec<String> = std::env::var("AXAGENT_SCAN_FILE_PATHS")
        .unwrap_or_default()
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let interval_secs: u64 = std::env::var("AXAGENT_SCAN_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    if cal_paths.is_empty() && file_paths.is_empty() {
        tracing::warn!(
            "Memory scanner skipped: set AXAGENT_SCAN_CALENDAR_PATHS and/or \
             AXAGENT_SCAN_FILE_PATHS to enable"
        );
        return;
    }

    let cal_config = axagent_harness::scanner::ScannerConfig {
        enabled: !cal_paths.is_empty(),
        interval_secs,
        paths: cal_paths,
        max_items: 500,
    };
    let file_config = axagent_harness::scanner::ScannerConfig {
        enabled: !file_paths.is_empty(),
        interval_secs,
        paths: file_paths,
        max_items: 500,
    };

    let cal_scanner = axagent_scanner::ICalScanner;
    let file_scanner = axagent_scanner::FileScanner;

    let memory_store = build_memory_store(state);

    // 后台扫描循环
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            tick.tick().await;

            // 扫描日历
            if cal_config.enabled {
                let result = cal_scanner.scan(&cal_config).await;
                ingest_scan_result(&memory_store, &result).await;
            }

            // 扫描文件
            if file_config.enabled {
                let result = file_scanner.scan(&file_config).await;
                ingest_scan_result(&memory_store, &result).await;
            }
        }
    });
}

/// 构建 MemoryStoreAdapter（包装 MemoryService）。
fn build_memory_store(
    state: &axagent_lib::AppState,
) -> std::sync::Arc<dyn axagent_harness::memory::MemoryStore> {
    struct Wrapper {
        svc: std::sync::Arc<tokio::sync::RwLock<axagent_trajectory::MemoryService>>,
    }
    #[async_trait::async_trait]
    impl axagent_harness::memory::MemoryStore for Wrapper {
        async fn add_memory(
            &self,
            req: axagent_harness::memory::MemoryAddRequest,
        ) -> Result<axagent_harness::memory::MemoryActionResultDto, String> {
            let svc = self.svc.read().await;
            let traj_req = axagent_trajectory::AddMemoryRequest {
                target: req.target,
                content: req.content,
                tier: axagent_trajectory::MemoryTier::from_str(&req.tier),
                importance: req.importance,
                nature: if req.nature.is_empty() {
                    axagent_trajectory::MemoryNature::Semantic
                } else {
                    axagent_trajectory::MemoryNature::from_str(&req.nature)
                },
                tags: req.tags,
                expires_at: req.expires_at,
                namespace_id: req.namespace_id,
                provenance: Some(axagent_trajectory::MemoryProvenance {
                    conversation_id: None,
                    message_id: None,
                    extraction_method: "scanner".to_string(),
                }),
            };
            let result = svc.add_memory_advanced(traj_req).await;
            Ok(axagent_harness::memory::MemoryActionResultDto {
                success: result.success,
                message: result.message,
            })
        }

        async fn search(
            &self,
            _req: axagent_harness::memory::MemorySearchRequest,
        ) -> Result<Vec<axagent_harness::memory::MemorySearchItem>, String> {
            Ok(Vec::new())
        }
        async fn tree(&self) -> Result<Vec<axagent_harness::memory::MemoryTreeItem>, String> {
            Ok(Vec::new())
        }
        async fn working(&self) -> Result<Vec<axagent_harness::memory::MemoryTreeItem>, String> {
            Ok(Vec::new())
        }
        async fn grouped(&self) -> Result<axagent_harness::memory::MemoryGroupedDto, String> {
            Ok(Default::default())
        }
        async fn update_importance(
            &self,
            _req: axagent_harness::memory::MemoryFeedbackRequest,
        ) -> Result<axagent_harness::memory::MemoryActionResultDto, String> {
            Ok(axagent_harness::memory::MemoryActionResultDto {
                success: false,
                message: "not implemented".to_string(),
            })
        }
        async fn delete_memory(
            &self,
            id: &str,
        ) -> Result<axagent_harness::memory::MemoryActionResultDto, String> {
            let svc = self.svc.read().await;
            svc.storage()
                .delete_memory(id)
                .await
                .map_err(|e| e.to_string())?;
            svc.storage()
                .delete_memory_fts(id)
                .await
                .map_err(|e| e.to_string())?;
            Ok(axagent_harness::memory::MemoryActionResultDto {
                success: true,
                message: format!("deleted {id}"),
            })
        }
        async fn update_memory(
            &self,
            id: &str,
            req: axagent_harness::memory::MemoryUpdateRequest,
        ) -> Result<axagent_harness::memory::MemoryActionResultDto, String> {
            let svc = self.svc.read().await;
            let all = svc.storage().get_all_memories().await.map_err(|e| e.to_string())?;
            let found = all.into_iter().find(|e| e.id == id);
            match found {
                None => Ok(axagent_harness::memory::MemoryActionResultDto {
                    success: false,
                    message: format!("memory not found: {id}"),
                }),
                Some(mut entry) => {
                    if let Some(content) = req.content {
                        entry.content = content;
                    }
                    if let Some(tier) = req.tier {
                        entry.tier = axagent_trajectory::MemoryTier::from_str(&tier);
                    }
                    if let Some(tags) = req.tags {
                        entry.tags = tags;
                    }
                    svc.storage().save_memory(&entry).await.map_err(|e| e.to_string())?;
                    Ok(axagent_harness::memory::MemoryActionResultDto {
                        success: true,
                        message: format!("updated {id}"),
                    })
                }
            }
        }
    }

    std::sync::Arc::new(Wrapper {
        svc: state.memory_service.clone(),
    })
}

/// 把扫描结果写入 MemoryStore。
async fn ingest_scan_result(
    store: &std::sync::Arc<dyn axagent_harness::memory::MemoryStore>,
    result: &axagent_harness::scanner::ScanResult,
) {
    for item in &result.items {
        let req = axagent_harness::memory::MemoryAddRequest {
            target: item.memory_type.clone(),
            content: format!("[{}] {}\n{}", item.source, item.title, item.content),
            tier: "long_term".to_string(),
            importance: 0.5,
            nature: "semantic".to_string(),
            tags: item.tags.clone(),
            expires_at: None,
            namespace_id: Some(item.source.clone()),
        };
        if let Err(e) = store.add_memory(req).await {
            tracing::warn!("Memory scan ingest failed for {}: {e}", item.external_id);
        }
    }

    for err in &result.errors {
        tracing::warn!("Memory scan error: {err}");
    }
}

/// 启动 Obsidian 回忆镜像后台任务。
///
/// 周期性读取全部记忆条目，通过 `ObsidianMirror` 写入 vault 的 markdown 文件。
/// 通过 `AXAGENT_OBSIDIAN_VAULT_PATH` 环境变量配置 vault 目录。
/// 同步间隔通过 `AXAGENT_SCAN_INTERVAL_SECS` 配置（与扫描器共用，缺省 300s）。
async fn start_obsidian_mirror(state: &axagent_lib::AppState) {
    let vault_path = match std::env::var("AXAGENT_OBSIDIAN_VAULT_PATH") {
        Ok(p) if !p.is_empty() => p,
        _ => {
            tracing::warn!(
                "Obsidian mirror skipped: set AXAGENT_OBSIDIAN_VAULT_PATH to enable"
            );
            return;
        }
    };

    let interval_secs: u64 = std::env::var("AXAGENT_SCAN_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);

    let mirror = axagent_scanner::ObsidianMirror::new(&vault_path, None);
    let memory_store = build_memory_store(state);

    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            tick.tick().await;

            // 获取全部记忆条目
            let entries = match memory_store.tree().await {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Obsidian mirror: tree() failed: {e}");
                    continue;
                }
            };

            let mut synced = 0usize;
            for entry in &entries {
                match mirror.sync_entry(
                    &entry.id,
                    &entry.memory_type,
                    &entry.content,
                    &entry.tags,
                    entry.importance,
                    &entry.tier,
                    entry.created_at,
                    entry.updated_at,
                ) {
                    Ok(Some(path)) => {
                        tracing::debug!("Obsidian mirror wrote: {}", path.display());
                        synced += 1;
                    }
                    Ok(None) => {} // 已是最新，跳过
                    Err(e) => {
                        tracing::warn!("Obsidian mirror sync failed for {}: {e}", entry.id);
                    }
                }
            }

            if synced > 0 {
                tracing::info!("Obsidian mirror synced {synced} entries");
            }
        }
    });
}
