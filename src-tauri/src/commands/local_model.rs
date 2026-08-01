// SPDX-License-Identifier: AGPL-3.0-only

//! 本地 llama.cpp 模型服务管理命令。
//!
//! 针对 `llama_cpp` 类型的模型供应商提供：
//! - `local_model_status`：探测运行状态（/health、/v1/models、/props、进程 PID/内存）
//! - `local_model_start`：托管启动 `llama-server` 子进程（CREATE_NO_WINDOW + 日志重定向）
//! - `local_model_stop`：停止服务（托管 PID 优先，端口探测兜底）
//! - `local_model_embed_test`：嵌入连通性测试（复用现有 provider adapter 链路）
//! - `local_model_logs`：读取服务日志尾部
//!
//! 状态探测不依赖"是否由 AxAgent 启动"——用户自己启动的 llama-server 也能被识别；
//! `managed` 字段标记该进程是否由本应用托管（settings 中记录 PID 匹配）。

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::local_model as lm_err;
use axagent_harness::core_error::Result as HarnessResult;
use axagent_harness::types::*;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tauri::State;

/// settings 中本地模型配置的键前缀（`{prefix}.{provider_id}.{field}`）
const SETTING_PREFIX: &str = "local_llama";
/// 默认启动探测超时（秒）
const STARTUP_TIMEOUT_SECS: u64 = 120;
/// 默认日志行数
const DEFAULT_LOG_LINES: u32 = 200;
/// 嵌入测试预览维度数
const PREVIEW_DIMS: usize = 8;

// ── DTO ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelStatus {
    pub running: bool,
    /// "ok" | "loading" | "unreachable"
    pub health: String,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
    pub memory_mb: Option<u64>,
    pub base_url: String,
    /// 是否由 AxAgent 托管启动（settings 记录 PID 匹配）
    pub managed: bool,
    pub model: Option<LocalModelInfo>,
    pub props: Option<LocalModelProps>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelInfo {
    pub id: String,
    pub n_embd: Option<u64>,
    pub n_ctx: Option<u64>,
    pub n_ctx_train: Option<u64>,
    pub n_params: Option<u64>,
    pub size_bytes: Option<u64>,
    pub ftype: Option<String>,
    pub n_vocab: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelProps {
    pub model_path: Option<String>,
    pub model_alias: Option<String>,
    pub model_ftype: Option<String>,
    pub n_ctx: Option<u64>,
    pub total_slots: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModelStartConfig {
    /// llama-server 可执行文件路径
    pub server_exe: String,
    /// GGUF 模型文件路径
    pub model_path: String,
    /// 监听地址，默认 127.0.0.1
    pub host: String,
    /// 监听端口，默认 8091
    pub port: u16,
    /// --alias 模型别名（影响 /v1/models 的 id 显示）
    pub alias: Option<String>,
    pub n_ctx: Option<u32>,
    pub n_gpu_layers: Option<i32>,
    /// 是否启用 embedding 模式（--embeddings），默认 true
    pub embedding_mode: Option<bool>,
    /// 附加原始参数
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedTestResult {
    pub dimensions: usize,
    pub prompt_tokens: Option<u32>,
    pub elapsed_ms: u64,
    /// 前 N 维预览
    pub preview: Vec<f32>,
}

#[derive(Debug)]
struct ProcessInfo {
    pid: u32,
    name: Option<String>,
    memory_mb: Option<u64>,
}

// ── 工具函数 ───────────────────────────────────────────────────────

/// 从 api_host 解析出 host 与端口（默认 8091）。
fn parse_host_port(base_or_host: &str) -> (String, u16) {
    let s = base_or_host.trim_end_matches('/');
    let s = s.strip_prefix("http://").or_else(|| s.strip_prefix("https://")).unwrap_or(s);
    let (host, port) = match s.split_once(':') {
        Some((h, rest)) => {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            let p = digits.parse::<u16>().unwrap_or(8091);
            (h.to_string(), p)
        },
        None => (s.to_string(), 8091),
    };
    (host, port)
}

/// 探测监听指定端口的进程（netstat + tasklist）。
#[cfg(windows)]
fn probe_process(port: u16) -> Option<ProcessInfo> {
    let out = std::process::Command::new("netstat").arg("-ano").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        if !line.to_ascii_lowercase().contains("listen") {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        // proto local foreign state pid
        if fields.len() >= 5 {
            let local = fields[1];
            if let Some(idx) = local.rfind(':') {
                let p = local[idx + 1..].parse::<u16>().ok();
                if p == Some(port) {
                    let pid = fields[4].parse::<u32>().ok()?;
                    let (name, memory_mb) = process_info_windows(pid);
                    return Some(ProcessInfo { pid, name, memory_mb });
                }
            }
        }
    }
    None
}

#[cfg(windows)]
fn process_info_windows(pid: u32) -> (Option<String>, Option<u64>) {
    let out = match std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return (None, None),
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let line = match text.lines().next() {
        Some(l) => l,
        None => return (None, None),
    };
    // CSV 格式: "llama-server.exe","8340","Console","2","1,196,944 K"
    let mut fields: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    for c in line.chars() {
        match c {
            '"' => in_quote = !in_quote,
            ',' if !in_quote => {
                fields.push(std::mem::take(&mut cur));
            },
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        fields.push(cur);
    }
    if fields.len() < 5 {
        return (None, None);
    }
    let name = fields.first().cloned().filter(|s| !s.is_empty());
    let mem = fields.get(4).cloned().unwrap_or_default();
    let mem_kb: u64 = mem
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .collect::<String>()
        .replace(',', "")
        .parse()
        .unwrap_or(0);
    (name, (mem_kb > 0).then_some(mem_kb / 1024))
}

/// 非 Windows：lsof + ps 探测（简化实现）。
#[cfg(not(windows))]
fn probe_process(port: u16) -> Option<ProcessInfo> {
    let out =
        std::process::Command::new("lsof").args(["-ti", &format!(":{port}")]).output().ok()?;
    let pid = String::from_utf8_lossy(&out.stdout).trim().lines().next()?.parse::<u32>().ok()?;
    let name = None;
    let memory_mb = None;
    Some(ProcessInfo { pid, name, memory_mb })
}

/// GET /health —— 服务健康状态。
async fn fetch_health(client: &reqwest::Client, base: &str) -> String {
    match client.get(format!("{base}/health")).send().await {
        Ok(r) if r.status().is_success() => r
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|v| v.get("status").and_then(|s| s.as_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "ok".to_string()),
        _ => "unreachable".to_string(),
    }
}

/// GET /v1/models —— 模型元信息（n_embd / n_ctx / 参数 / 量化）。
async fn fetch_models(client: &reqwest::Client, base: &str) -> Option<LocalModelInfo> {
    let resp = client.get(format!("{base}/models")).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    let first = v.get("data")?.as_array()?.first()?.clone();
    let id = first.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let meta = first.get("meta").cloned().unwrap_or_default();
    let num = |k: &str| meta.get(k).and_then(|x| x.as_u64());
    Some(LocalModelInfo {
        id,
        n_embd: num("n_embd"),
        n_ctx: num("n_ctx"),
        n_ctx_train: num("n_ctx_train"),
        n_params: num("n_params"),
        size_bytes: num("size"),
        ftype: meta.get("ftype").and_then(|x| x.as_str()).map(|s| s.to_string()),
        n_vocab: num("n_vocab"),
    })
}

/// GET /props —— llama.cpp 特有属性（model_path / alias / slots）。
async fn fetch_props(client: &reqwest::Client, base: &str) -> Option<LocalModelProps> {
    let resp = client.get(format!("{base}/props")).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    let str_opt = |k: &str| v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string());
    Some(LocalModelProps {
        model_path: str_opt("model_path"),
        model_alias: str_opt("model_alias"),
        model_ftype: str_opt("model_ftype"),
        n_ctx: v.get("n_ctx").and_then(|x| x.as_u64()),
        total_slots: v.get("total_slots").and_then(|x| x.as_u64()),
    })
}

/// 组合状态探测：health + models + props + 进程。
async fn probe(state: &AppState, base: &str, provider_id: &str) -> HarnessResult<LocalModelStatus> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| axagent_harness::core_error::AxAgentError::Provider(e.to_string()))?;

    // llama.cpp 的 /health、/props 在根路径；OpenAI 兼容端点（/v1/models 等）带 /v1 前缀
    let root = base.strip_suffix("/v1").unwrap_or(base);
    let health = fetch_health(&client, root).await;
    let running = health == "ok";
    let model = if running {
        fetch_models(&client, base).await
    } else {
        None
    };
    let props = if running {
        fetch_props(&client, root).await
    } else {
        None
    };

    let (_, port) = parse_host_port(base);
    let proc = probe_process(port);

    // managed：settings 记录的托管 PID 与当前进程 PID 一致
    let managed = match (
        axagent_dao::repo::settings::get_setting(
            state.harness.db(),
            &format!("{SETTING_PREFIX}.{provider_id}.pid"),
        )
        .await
        .ok()
        .flatten()
        .and_then(|p| p.parse::<u32>().ok()),
        proc.as_ref().map(|p| p.pid),
    ) {
        (Some(recorded), Some(current)) => recorded == current,
        _ => false,
    };

    Ok(LocalModelStatus {
        running,
        health,
        pid: proc.as_ref().map(|p| p.pid),
        process_name: proc.as_ref().and_then(|p| p.name.clone()),
        memory_mb: proc.as_ref().and_then(|p| p.memory_mb),
        base_url: base.to_string(),
        managed,
        model,
        props,
    })
}

/// 从 provider 记录解析 base_url（自动补 /v1）。
/// 统一先经 `resolve_provider_id` 解析（兼容 builtin_ 前缀等别名）。
async fn resolve_provider_base(
    db: &axagent_harness::DatabaseConnection,
    provider_id: &str,
) -> HarnessResult<(ProviderConfig, String)> {
    let real_id = axagent_dao::repo::provider::resolve_provider_id(db, provider_id).await?;
    let provider = axagent_dao::repo::provider::get_provider(db, &real_id).await?;
    let base = axagent_harness::url_utils::resolve_base_url_for_type(
        &provider.api_host,
        &provider.provider_type,
    );
    Ok((provider, base))
}

fn command_error(e: impl std::fmt::Display, code: &str) -> String {
    ErrorResponse::err_with_detail(code, e.to_string())
}

// ── 命令 ───────────────────────────────────────────────────────────

/// 查询本地模型服务运行状态。
#[tauri::command]
pub async fn local_model_status(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<LocalModelStatus, String> {
    let (_, base) = resolve_provider_base(state.harness.db(), &provider_id)
        .await
        .map_err(|e| command_error(e, lm_err::PROVIDER_NOT_FOUND))?;
    probe(&state, &base, &provider_id).await.map_err(|e| command_error(e, lm_err::STATUS_FAILED))
}

/// 托管启动 llama-server 子进程。
#[tauri::command]
pub async fn local_model_start(
    state: State<'_, AppState>,
    provider_id: String,
    config: LocalModelStartConfig,
) -> Result<LocalModelStatus, String> {
    let db = state.harness.db();
    let (_, base) = resolve_provider_base(db, &provider_id)
        .await
        .map_err(|e| command_error(e, lm_err::PROVIDER_NOT_FOUND))?;

    // 端口已被进程占用（含启动中 / 外部进程）：不重复启动，直接返回当前状态
    let existing = probe(&state, &base, &provider_id)
        .await
        .map_err(|e| command_error(e, lm_err::STATUS_FAILED))?;
    if existing.running || existing.pid.is_some() {
        return Ok(existing);
    }

    // 校验可执行文件与模型文件
    let exe_path = Path::new(&config.server_exe);
    if !exe_path.is_file() {
        return Err(ErrorResponse::err_with_detail(
            lm_err::INVALID_CONFIG,
            format!("llama-server 可执行文件不存在: {}", config.server_exe),
        ));
    }
    let model_path = Path::new(&config.model_path);
    if !model_path.is_file() {
        return Err(ErrorResponse::err_with_detail(
            lm_err::INVALID_CONFIG,
            format!("模型文件不存在: {}", config.model_path),
        ));
    }

    // 日志文件：{app_data_dir}/logs/llama-server-{port}.log
    let log_dir = state.app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|e| {
        ErrorResponse::err_with_detail(lm_err::START_FAILED, format!("创建日志目录失败: {e}"))
    })?;
    let log_file_path = log_dir.join(format!("llama-server-{}.log", config.port));
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)
        .map_err(|e| {
            ErrorResponse::err_with_detail(lm_err::START_FAILED, format!("打开日志文件失败: {e}"))
        })?;

    // 构造命令行
    let mut cmd = std::process::Command::new(&config.server_exe);
    cmd.arg("-m").arg(&config.model_path);
    cmd.arg("--host").arg(&config.host);
    cmd.arg("--port").arg(config.port.to_string());
    if let Some(alias) = config.alias.as_deref().filter(|a| !a.is_empty()) {
        cmd.arg("--alias").arg(alias);
    }
    if let Some(ctx) = config.n_ctx {
        cmd.arg("--ctx-size").arg(ctx.to_string());
    }
    if let Some(gl) = config.n_gpu_layers {
        cmd.arg("--n-gpu-layers").arg(gl.to_string());
    }
    if config.embedding_mode.unwrap_or(true) {
        cmd.arg("--embeddings");
    }
    for a in &config.extra_args {
        cmd.arg(a);
    }
    let log_writer = log_file.try_clone().map_err(|e| {
        ErrorResponse::err_with_detail(lm_err::START_FAILED, format!("日志文件克隆失败: {e}"))
    })?;
    cmd.stdout(Stdio::from(log_writer));
    cmd.stderr(Stdio::from(log_file));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let child = cmd.spawn().map_err(|e| {
        ErrorResponse::err_with_detail(lm_err::START_FAILED, format!("启动 llama-server 失败: {e}"))
    })?;
    let pid = child.id();

    // 记录托管信息
    let db = state.harness.db();
    let prefix = format!("{SETTING_PREFIX}.{provider_id}");
    let _ =
        axagent_dao::repo::settings::set_setting(db, &format!("{prefix}.pid"), &pid.to_string())
            .await;
    let _ =
        axagent_dao::repo::settings::set_setting(db, &format!("{prefix}.exe"), &config.server_exe)
            .await;
    let _ = axagent_dao::repo::settings::set_setting(
        db,
        &format!("{prefix}.model"),
        &config.model_path,
    )
    .await;
    let _ = axagent_dao::repo::settings::set_setting(
        db,
        &format!("{prefix}.port"),
        &config.port.to_string(),
    )
    .await;
    tracing::info!(
        pid,
        port = config.port,
        "[local_model] llama-server 已启动: {}",
        log_file_path.display()
    );

    // 轮询 /health 直至就绪
    let deadline = Instant::now() + Duration::from_secs(STARTUP_TIMEOUT_SECS);
    loop {
        let st = probe(&state, &base, &provider_id)
            .await
            .map_err(|e| command_error(e, lm_err::STATUS_FAILED))?;
        if st.health == "ok" {
            return Ok(st);
        }
        if Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }

    Err(ErrorResponse::err_with_detail(
        lm_err::START_FAILED,
        format!(
            "llama-server 启动超时（{STARTUP_TIMEOUT_SECS}s）。请查看日志文件: {}",
            log_file_path.display()
        ),
    ))
}

/// 停止本地模型服务。
#[tauri::command]
pub async fn local_model_stop(
    state: State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    let db = state.harness.db();
    let (_, base) = resolve_provider_base(db, &provider_id)
        .await
        .map_err(|e| command_error(e, lm_err::PROVIDER_NOT_FOUND))?;

    let (_, port) = parse_host_port(&base);
    // 托管 PID 优先，端口探测兜底
    let recorded = axagent_dao::repo::settings::get_setting(
        db,
        &format!("{SETTING_PREFIX}.{provider_id}.pid"),
    )
    .await
    .ok()
    .flatten()
    .and_then(|p| p.parse::<u32>().ok());
    let pid = match recorded {
        Some(p) => {
            // 确认该 PID 仍存活（否则回退端口探测）
            if process_alive(p) {
                Some(p)
            } else {
                probe_process(port).map(|i| i.pid)
            }
        },
        None => probe_process(port).map(|i| i.pid),
    };

    match pid {
        Some(p) => {
            #[cfg(windows)]
            {
                let out = std::process::Command::new("taskkill")
                    .args(["/PID", &p.to_string(), "/T", "/F"])
                    .output()
                    .map_err(|e| {
                        ErrorResponse::err_with_detail(
                            lm_err::STOP_FAILED,
                            format!("taskkill 执行失败: {e}"),
                        )
                    })?;
                if !out.status.success() {
                    let msg = String::from_utf8_lossy(&out.stderr);
                    return Err(ErrorResponse::err_with_detail(
                        lm_err::STOP_FAILED,
                        format!("停止进程 {p} 失败: {msg}"),
                    ));
                }
            }
            #[cfg(not(windows))]
            {
                let status = std::process::Command::new("kill")
                    .arg(p.to_string())
                    .status()
                    .map_err(|e| {
                        ErrorResponse::err_with_detail(
                            lm_err::STOP_FAILED,
                            format!("kill 执行失败: {e}"),
                        )
                    })?;
                if !status.success() {
                    return Err(ErrorResponse::err_with_detail(
                        lm_err::STOP_FAILED,
                        format!("kill 进程 {p} 失败"),
                    ));
                }
            }
            // 清理托管记录
            let prefix = format!("{SETTING_PREFIX}.{provider_id}");
            for key in ["pid", "exe", "model", "port"] {
                axagent_dao::repo::settings::set_setting(db, &format!("{prefix}.{key}"), "")
                    .await
                    .ok();
            }
            tracing::info!(pid = p, "[local_model] llama-server 已停止");
            Ok(())
        },
        None => Err(ErrorResponse::err(lm_err::NOT_RUNNING)),
    }
}

#[cfg(windows)]
fn process_alive(pid: u32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .output()
        .map(|o| {
            let t = String::from_utf8_lossy(&o.stdout);
            t.contains(&format!("{pid}"))
        })
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn process_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// 嵌入连通性测试。
#[tauri::command]
pub async fn local_model_embed_test(
    state: State<'_, AppState>,
    provider_id: String,
    text: String,
) -> Result<EmbedTestResult, String> {
    let db = state.harness.db();
    let real_id = axagent_dao::repo::provider::resolve_provider_id(db, &provider_id)
        .await
        .map_err(|e| command_error(e, lm_err::PROVIDER_NOT_FOUND))?;
    let (provider, _) = resolve_provider_base(db, &real_id)
        .await
        .map_err(|e| command_error(e, lm_err::PROVIDER_NOT_FOUND))?;
    let master_key = state.harness.master_key_owned();
    let registry = state.harness.provider_registry().clone();

    let (ctx, _cfg) = crate::indexing::build_embed_context(db, &master_key, &real_id)
        .await
        .map_err(|e| command_error(e, lm_err::EMBED_TEST_FAILED))?;

    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&provider.provider_type);
    let adapter = registry.get(registry_key).ok_or_else(|| {
        ErrorResponse::err_with_detail(
            lm_err::EMBED_TEST_FAILED,
            format!("适配器 {registry_key} 未注册"),
        )
    })?;

    // llama.cpp server 忽略 model 字段；优先取 provider 第一个 embedding 模型作为标识
    let model_id = provider
        .models
        .iter()
        .find(|m| m.model_type == ModelType::Embedding)
        .map(|m| m.model_id.clone())
        .unwrap_or_else(|| "bge-m3".to_string());

    let started = Instant::now();
    let resp = adapter
        .embed(&ctx, EmbedRequest { model: model_id, input: vec![text], dimensions: None })
        .await
        .map_err(|e| command_error(e, lm_err::EMBED_TEST_FAILED))?;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    let first = resp.embeddings.first().cloned().unwrap_or_default();
    let preview = first.iter().take(PREVIEW_DIMS).copied().collect();
    Ok(EmbedTestResult { dimensions: first.len(), prompt_tokens: None, elapsed_ms, preview })
}

/// 读取服务日志尾部。
#[tauri::command]
pub async fn local_model_logs(
    state: State<'_, AppState>,
    provider_id: String,
    max_lines: Option<u32>,
) -> Result<String, String> {
    let db = state.harness.db();
    let (_, base) = resolve_provider_base(db, &provider_id)
        .await
        .map_err(|e| command_error(e, lm_err::PROVIDER_NOT_FOUND))?;
    let (_, port) = parse_host_port(&base);
    let log_path = state.app_data_dir.join("logs").join(format!("llama-server-{port}.log"));
    if !log_path.exists() {
        return Err(ErrorResponse::err_with_detail(
            lm_err::LOG_NOT_FOUND,
            format!("日志文件不存在: {}", log_path.display()),
        ));
    }
    let max = max_lines.unwrap_or(DEFAULT_LOG_LINES).max(1) as usize;
    let mut f =
        std::fs::File::open(&log_path).map_err(|e| command_error(e, lm_err::LOG_READ_FAILED))?;
    let mut buf = String::new();
    f.read_to_string(&mut buf).map_err(|e| command_error(e, lm_err::LOG_READ_FAILED))?;
    let lines: Vec<&str> = buf.lines().collect();
    let tail: Vec<&str> = lines.iter().rev().take(max).rev().copied().collect();
    Ok(tail.join("\n"))
}

// ── 模型下载 ───────────────────────────────────────────────────────
//
// 下载目录可通过 UI 配置（settings `local_llama.download_dir`，默认
// `~/.axagent/models`）；模型列表即扫描该目录下的 *.gguf 文件，
// 与 llama.cpp 供应商的模型列表（refresh_models）打通。

use axagent_search::model_downloader::{ModelDownloader, PresetModel, PresetModelType};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::Mutex as StdMutex;

/// 下载任务状态表（filename → 任务）
static DOWNLOAD_TASKS: LazyLock<StdMutex<HashMap<String, DownloadTaskInfo>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskInfo {
    pub filename: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    /// "downloading" | "done" | "failed"
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRequest {
    pub filename: String,
    pub hf_repo: Option<String>,
    pub direct_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalFileModel {
    pub filename: String,
    pub size_bytes: u64,
    pub modified_at: Option<i64>,
    /// "chat" | "embedding" | "voice"（detect_model_type 判定）
    pub model_type: String,
    /// 存在 .download 临时文件（正在下载）
    pub is_downloading: bool,
    pub download_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetModelDto {
    pub filename: String,
    pub hf_repo: Option<String>,
    pub direct_url: Option<String>,
    pub display_name: String,
    pub size_bytes: u64,
    pub model_type: String,
    pub is_downloaded: bool,
}

/// 校验文件名安全（禁止路径分隔符与穿越）。
fn validate_filename(filename: &str) -> Result<(), String> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
        || filename.contains(':')
    {
        return Err(ErrorResponse::err_with_detail(
            lm_err::INVALID_CONFIG,
            format!("非法文件名: {filename}"),
        ));
    }
    Ok(())
}

/// 读取配置的下载目录（settings 持久化，默认 ~/.axagent/models）。
pub(crate) async fn download_dir(db: &axagent_harness::DatabaseConnection) -> PathBuf {
    match axagent_dao::repo::settings::get_setting(db, &format!("{SETTING_PREFIX}.download_dir"))
        .await
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
    {
        Some(dir) => PathBuf::from(dir),
        None => {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".axagent").join("models")
        },
    }
}

/// 读取配置的 HF 端点（默认 huggingface.co，可配 hf-mirror.com 等国内镜像）。
async fn hf_endpoint(db: &axagent_harness::DatabaseConnection) -> String {
    axagent_dao::repo::settings::get_setting(db, &format!("{SETTING_PREFIX}.hf_endpoint"))
        .await
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "https://huggingface.co".to_string())
}

/// 扫描下载目录中的 *.gguf 文件（排除 .download 临时文件）。
pub fn scan_gguf_files(dir: &Path) -> Vec<LocalFileModel> {
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.to_ascii_lowercase().ends_with(".gguf") {
            continue;
        }
        let meta = entry.metadata().ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = meta.and_then(|m| m.modified().ok()).map(|t| {
            t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
        });
        let tmp_name = format!("{name}.download");
        let tmp_size = dir.join(&tmp_name).metadata().map(|m| m.len()).unwrap_or(0);
        let model_type = axagent_harness::types::provider_model::detect_model_type(&name);
        out.push(LocalFileModel {
            filename: name,
            size_bytes: size,
            modified_at: modified,
            model_type: format!("{model_type}").to_lowercase(),
            is_downloading: tmp_size > 0,
            download_bytes: tmp_size,
        });
    }
    out.sort_by(|a, b| a.filename.cmp(&b.filename));
    out
}

/// 下载目录中的 GGUF 文件 → Provider 模型列表（供 refresh_models 使用）。
pub fn scan_gguf_models(provider_id: &str, dir: &Path) -> Vec<axagent_harness::types::Model> {
    scan_gguf_files(dir)
        .into_iter()
        .filter(|f| !f.is_downloading)
        .map(|f| {
            let model_type = axagent_harness::types::provider_model::detect_model_type(&f.filename);
            let capabilities = match model_type {
                axagent_harness::types::ModelType::Chat => {
                    vec![axagent_harness::types::ModelCapability::TextChat]
                },
                axagent_harness::types::ModelType::Embedding => vec![],
                axagent_harness::types::ModelType::Voice => {
                    vec![axagent_harness::types::ModelCapability::RealtimeVoice]
                },
            };
            axagent_harness::types::Model {
                provider_id: provider_id.to_string(),
                model_id: f.filename.clone(),
                name: f.filename.clone(),
                group_name: None,
                model_type,
                capabilities,
                max_tokens: axagent_kit::model_knowledge::get_model_context_window(&f.filename),
                max_output_tokens: None,
                enabled: true,
                param_overrides: None,
                input_price_per_mtok: None,
                output_price_per_mtok: None,
            }
        })
        .collect()
}

/// 获取当前下载目录。
#[tauri::command]
pub async fn local_model_get_download_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(download_dir(state.harness.db()).await.to_string_lossy().to_string())
}

/// 设置下载目录（自动创建）。
#[tauri::command]
pub async fn local_model_set_download_dir(
    state: State<'_, AppState>,
    dir: String,
) -> Result<String, String> {
    let path = PathBuf::from(dir.trim());
    if path.as_os_str().is_empty() {
        return Err(ErrorResponse::err(lm_err::INVALID_CONFIG));
    }
    std::fs::create_dir_all(&path).map_err(|e| {
        ErrorResponse::err_with_detail(lm_err::INVALID_CONFIG, format!("创建目录失败: {e}"))
    })?;
    axagent_dao::repo::settings::set_setting(
        state.harness.db(),
        &format!("{SETTING_PREFIX}.download_dir"),
        &path.to_string_lossy(),
    )
    .await
    .map_err(|e| command_error(e, lm_err::INVALID_CONFIG))?;
    Ok(path.to_string_lossy().to_string())
}

/// 获取 HF 镜像端点。
#[tauri::command]
pub async fn local_model_get_hf_endpoint(state: State<'_, AppState>) -> Result<String, String> {
    Ok(hf_endpoint(state.harness.db()).await)
}

/// 设置 HF 镜像端点（空值恢复默认 huggingface.co）。
#[tauri::command]
pub async fn local_model_set_hf_endpoint(
    state: State<'_, AppState>,
    endpoint: String,
) -> Result<String, String> {
    let ep = endpoint.trim().trim_end_matches('/').to_string();
    let store = if ep.is_empty() || ep == "https://huggingface.co" {
        String::new()
    } else {
        ep.clone()
    };
    axagent_dao::repo::settings::set_setting(
        state.harness.db(),
        &format!("{SETTING_PREFIX}.hf_endpoint"),
        &store,
    )
    .await
    .map_err(|e| command_error(e, lm_err::INVALID_CONFIG))?;
    Ok(if store.is_empty() {
        "https://huggingface.co".to_string()
    } else {
        ep
    })
}

/// 列出下载目录中的本地模型（含下载中状态）。
#[tauri::command]
pub async fn local_model_list_local_models(
    state: State<'_, AppState>,
) -> Result<Vec<LocalFileModel>, String> {
    let dir = download_dir(state.harness.db()).await;
    Ok(scan_gguf_files(&dir))
}

/// 推荐模型清单（含已下载标记）。
#[tauri::command]
pub async fn local_model_get_presets(
    state: State<'_, AppState>,
) -> Result<Vec<PresetModelDto>, String> {
    let dir = download_dir(state.harness.db()).await;
    Ok(ModelDownloader::preset_models()
        .into_iter()
        .map(|p: PresetModel| PresetModelDto {
            filename: p.filename.clone(),
            hf_repo: p.hf_repo.clone(),
            direct_url: p.direct_url.clone(),
            display_name: p.display_name.clone(),
            size_bytes: p.size_bytes,
            model_type: match p.model_type {
                PresetModelType::Reranker => "reranker".to_string(),
                PresetModelType::Judge => "judge".to_string(),
                PresetModelType::SparseEncoder => "sparse".to_string(),
                PresetModelType::Embedding => "embedding".to_string(),
            },
            is_downloaded: dir.join(&p.filename).exists(),
        })
        .collect())
}

/// 发起模型下载（后台任务 + 进度可查询）。
#[tauri::command]
pub async fn local_model_download(
    state: State<'_, AppState>,
    request: DownloadRequest,
) -> Result<DownloadTaskInfo, String> {
    let db = state.harness.db();
    validate_filename(&request.filename)?;
    if request.hf_repo.as_deref().unwrap_or("").is_empty()
        && request.direct_url.as_deref().unwrap_or("").is_empty()
    {
        return Err(ErrorResponse::err_with_detail(
            lm_err::INVALID_CONFIG,
            "请提供 HF 仓库或直接下载链接",
        ));
    }
    let dir = download_dir(db).await;
    std::fs::create_dir_all(&dir).map_err(|e| {
        ErrorResponse::err_with_detail(lm_err::START_FAILED, format!("创建下载目录失败: {e}"))
    })?;
    let endpoint = hf_endpoint(db).await;

    // 任务已存在（同文件正在下载）→ 直接返回
    {
        let tasks = DOWNLOAD_TASKS.lock().unwrap();
        if let Some(t) = tasks.get(&request.filename)
            && t.status == "downloading"
        {
            return Ok(t.clone());
        }
    }

    let info = DownloadTaskInfo {
        filename: request.filename.clone(),
        downloaded_bytes: 0,
        total_bytes: 0,
        status: "downloading".to_string(),
        error: None,
    };
    DOWNLOAD_TASKS.lock().unwrap().insert(request.filename.clone(), info.clone());

    let dl = ModelDownloader::with_cache_dir(dir.clone());
    let fname = request.filename.clone();
    let fname_cb = fname.clone();
    let repo = request.hf_repo.clone();
    let url = request.direct_url.clone();
    tokio::spawn(async move {
        let result = dl
            .download_with_progress(
                &fname,
                repo.as_deref(),
                url.as_deref(),
                &endpoint,
                "",
                Some(Box::new(move |downloaded, total| {
                    if let Ok(mut tasks) = DOWNLOAD_TASKS.lock() {
                        if let Some(t) = tasks.get_mut(&fname_cb) {
                            t.downloaded_bytes = downloaded;
                            t.total_bytes = total;
                        }
                    }
                })),
            )
            .await;
        let mut tasks = DOWNLOAD_TASKS.lock().unwrap();
        match result {
            Ok(path) => {
                let t = tasks.entry(fname.clone()).or_insert_with(|| DownloadTaskInfo {
                    filename: fname.clone(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    status: "done".to_string(),
                    error: None,
                });
                t.status = "done".to_string();
                t.downloaded_bytes = t.total_bytes.max(t.downloaded_bytes);
                tracing::info!("[local_model] 模型下载完成: {}", path.display());
            },
            Err(e) => {
                let t = tasks.entry(fname.clone()).or_insert_with(|| DownloadTaskInfo {
                    filename: fname.clone(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    status: "failed".to_string(),
                    error: Some(e.to_string()),
                });
                t.status = "failed".to_string();
                t.error = Some(e.to_string());
                tracing::error!("[local_model] 模型下载失败: {}", e);
            },
        }
    });

    Ok(info)
}

/// 查询所有下载任务进度（前端轮询）。
#[tauri::command]
pub async fn local_model_download_progress() -> Result<Vec<DownloadTaskInfo>, String> {
    let tasks = DOWNLOAD_TASKS.lock().unwrap();
    Ok(tasks.values().cloned().collect())
}

/// 删除本地模型文件（含 .download 残留）。
#[tauri::command]
pub async fn local_model_delete_local_model(
    state: State<'_, AppState>,
    filename: String,
) -> Result<(), String> {
    validate_filename(&filename)?;
    let dir = download_dir(state.harness.db()).await;
    let path = dir.join(&filename);
    let canonical_base = dir.canonicalize().unwrap_or(dir.clone());
    if path.exists() {
        let canonical = path.canonicalize().map_err(|e| command_error(e, lm_err::DELETE_FAILED))?;
        if !canonical.starts_with(&canonical_base) {
            return Err(ErrorResponse::err_with_detail(lm_err::INVALID_CONFIG, "路径穿越检测失败"));
        }
        std::fs::remove_file(&path).map_err(|e| command_error(e, lm_err::DELETE_FAILED))?;
    }
    // 清理下载残留
    let tmp = dir.join(format!("{filename}.download"));
    if tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
    }
    DOWNLOAD_TASKS.lock().unwrap().remove(&filename);
    Ok(())
}
