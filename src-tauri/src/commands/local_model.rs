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
use crate::commands::error::{ErrorCategory, ErrorResponse};
use crate::commands::error_code::local_model as lm_err;
use axagent_agent_macro::agent_command;
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
/// 端口等待释放超时（秒）
const PORT_RELEASE_TIMEOUT_SECS: u64 = 10;
/// 默认日志行数
const DEFAULT_LOG_LINES: u32 = 200;
/// 最大日志文件大小（字节），超过则轮转
const MAX_LOG_SIZE_BYTES: u64 = 50 * 1024 * 1024; // 50 MB
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

/// 安全过滤 extra_args：拒绝可能导致进程异常或安全风险的参数。
fn validate_extra_args(args: &[String]) -> Vec<String> {
    let dangerous_prefixes = [
        "--help",
        "--version",
        "--no-mmap",
        "--no-mlock",
        "--mlock",
        "--nobench",
        "--ignore-eos",
        "--special",
        "--interactive",
        "--interactive-first",
        "--log-disable",
        "--log-enable",
        "--log-file",
        "--training",
        "--grammar",
        "--grammar-file",
        "--speculative",
        "--speculative-num",
        "--cont-batching",
        "--no-cnv",
        "--parallel",
        "--cont-batch-size",
        "--flash-attn",
        "--no-flash-attn",
    ];
    let mut safe = Vec::new();
    for arg in args {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            continue;
        }
        // 检查是否以危险前缀开头
        let is_dangerous = dangerous_prefixes
            .iter()
            .any(|p| trimmed == *p || trimmed.starts_with(&format!("{p}=")));
        if !is_dangerous {
            safe.push(arg.clone());
        } else {
            tracing::warn!("[local_model] 过滤危险参数: {arg}");
        }
    }
    safe
}

/// 等待指定端口被释放（用于 stop 后确认进程已退出）。
async fn wait_for_port_release(port: u16) -> bool {
    let deadline = Instant::now() + Duration::from_secs(PORT_RELEASE_TIMEOUT_SECS);
    while Instant::now() < deadline {
        if probe_process(port).is_none() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

/// 检查模型文件是否可访问（读取权限 + 非空）。
fn check_model_accessible(model_path: &str) -> Result<(), String> {
    let path = Path::new(model_path);
    if !path.exists() {
        return Err(format!("模型文件不存在: {model_path}"));
    }
    if !path.is_file() {
        return Err(format!("模型路径不是文件: {model_path}"));
    }
    match std::fs::metadata(path) {
        Ok(m) if m.len() > 0 => Ok(()),
        Ok(_) => Err(format!("模型文件为空: {model_path}")),
        Err(e) => Err(format!("无法读取模型文件: {e}")),
    }
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
    let root = if base.ends_with("/v1") {
        base.trim_end_matches("/v1").to_string()
    } else {
        base.trim_end_matches('/').to_string()
    };
    let api_base = if base.ends_with("/v1") {
        base.to_string()
    } else {
        format!("{root}/v1")
    };
    let health = fetch_health(&client, &root).await;
    let running = health == "ok";
    let model = if running {
        fetch_models(&client, &api_base).await
    } else {
        None
    };
    let props = if running {
        fetch_props(&client, &root).await
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
#[agent_command(domain = model, safety = Safe, call_mode = StateInput, description = "查询本地模型服务状态")]
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
#[agent_command(domain = model, safety = Caution, call_mode = StateInput, description = "启动本地模型服务")]
#[tauri::command]
pub async fn local_model_start(
    state: State<'_, AppState>,
    provider_id: String,
    config: LocalModelStartConfig,
) -> Result<LocalModelStatus, String> {
    let db = state.harness.db();
    let (provider, base) = resolve_provider_base(db, &provider_id)
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

    // 模型文件可访问性检查
    if let Err(e) = check_model_accessible(&config.model_path) {
        return Err(ErrorResponse::err_with_detail(lm_err::INVALID_CONFIG, e));
    }

    // 端口冲突二次确认（防止 TOCTOU 竞态）
    let (_, port) = parse_host_port(&base);
    if let Some(proc) = probe_process(port) {
        return Err(ErrorResponse::err_with_detail(
            lm_err::PORT_IN_USE,
            format!(
                "端口 {} 已被进程 {} (PID {}) 占用，请先停止该进程或更换端口",
                port,
                proc.name.as_deref().unwrap_or("未知进程"),
                proc.pid
            ),
        ));
    }

    // 日志文件：{app_data_dir}/logs/llama-server-{port}.log
    let log_dir = state.app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|e| {
        ErrorResponse::err_with_detail(lm_err::START_FAILED, format!("创建日志目录失败: {e}"))
    })?;
    let log_file_path = log_dir.join(format!("llama-server-{}.log", config.port));

    // 日志轮转：如果文件超过最大限制，将旧日志重命名为 .old
    if log_file_path.exists() {
        if let Ok(meta) = std::fs::metadata(&log_file_path) {
            if meta.len() > MAX_LOG_SIZE_BYTES {
                let old_path = log_file_path.with_extension("log.old");
                let _ = std::fs::rename(&log_file_path, &old_path);
            }
        }
    }

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
    // 安全过滤 extra_args
    let safe_args = validate_extra_args(&config.extra_args);
    for a in &safe_args {
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
        filtered = config.extra_args.len() - safe_args.len(),
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
            // 启动成功后同步 api_host 到 provider，确保健康检查和后续 API 调用使用正确地址
            let new_api_host = format!("http://{}:{}", config.host, config.port);
            if provider.api_host != new_api_host {
                let _ = axagent_dao::repo::provider::update_provider(
                    db,
                    &provider.id,
                    UpdateProviderInput { api_host: Some(new_api_host), ..Default::default() },
                )
                .await;
            }
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
#[agent_command(domain = model, safety = Caution, call_mode = StateInput, description = "停止本地模型服务")]
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

            // 等待端口被释放，确保 stop 完成后 start 不会冲突
            let released = wait_for_port_release(port).await;
            if !released {
                tracing::warn!(
                    port,
                    "[local_model] 端口 {} 未在超时内释放，可能需要手动清理",
                    port
                );
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
#[agent_command(domain = model, safety = Safe, call_mode = StateInput, description = "本地模型嵌入测试")]
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

    // llama.cpp server 忽略 model 字段；优先取 provider 第一个 embedding 模型作为标识；
    // 若没有配置 embedding 模型，则使用 settings 中记录的模型文件名作为 fallback
    let model_id = match provider.models.iter().find(|m| m.model_type == ModelType::Embedding) {
        Some(m) => Some(m.model_id.clone()),
        None => {
            // 从 settings 中读取当前加载的模型文件名
            let stored = axagent_dao::repo::settings::get_setting(
                db,
                &format!("{SETTING_PREFIX}.{provider_id}.model"),
            )
            .await
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
            stored.and_then(|path| {
                Path::new(&path).file_stem().and_then(|s| s.to_str()).map(|s| s.to_string())
            })
        },
    }
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
#[agent_command(domain = model, safety = Safe, call_mode = StateInput, description = "读取本地模型日志")]
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
#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "获取模型下载目录")]
#[tauri::command]
pub async fn local_model_get_download_dir(state: State<'_, AppState>) -> Result<String, String> {
    Ok(download_dir(state.harness.db()).await.to_string_lossy().to_string())
}

/// 设置下载目录（自动创建）。
#[agent_command(domain = model, safety = Caution, call_mode = StateInput, description = "设置模型下载目录")]
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
#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "获取 HF 镜像端点")]
#[tauri::command]
pub async fn local_model_get_hf_endpoint(state: State<'_, AppState>) -> Result<String, String> {
    Ok(hf_endpoint(state.harness.db()).await)
}

/// 设置 HF 镜像端点（空值恢复默认 huggingface.co）。
#[agent_command(domain = model, safety = Caution, call_mode = StateInput, description = "设置 HF 镜像端点")]
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
#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "列出本地模型文件")]
#[tauri::command]
pub async fn local_model_list_local_models(
    state: State<'_, AppState>,
) -> Result<Vec<LocalFileModel>, String> {
    let dir = download_dir(state.harness.db()).await;
    Ok(scan_gguf_files(&dir))
}

/// 推荐模型清单（含已下载标记）。
#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "获取推荐模型列表")]
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
#[agent_command(domain = model, safety = Caution, call_mode = StateInput, description = "下载模型文件")]
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
        let tasks = DOWNLOAD_TASKS.lock().unwrap_or_else(|e| e.into_inner());
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
    DOWNLOAD_TASKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(request.filename.clone(), info.clone());

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
        let mut tasks = DOWNLOAD_TASKS.lock().unwrap_or_else(|e| e.into_inner());
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
#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "查询下载任务进度")]
#[tauri::command]
pub async fn local_model_download_progress() -> Result<Vec<DownloadTaskInfo>, String> {
    let tasks = DOWNLOAD_TASKS.lock().unwrap_or_else(|e| e.into_inner());
    Ok(tasks.values().cloned().collect())
}

/// 删除本地模型文件（含 .download 残留）。
#[agent_command(domain = model, safety = Dangerous, call_mode = StateInput, description = "删除本地模型文件")]
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
    DOWNLOAD_TASKS.lock().unwrap_or_else(|e| e.into_inner()).remove(&filename);
    Ok(())
}

// ── llama.cpp 安装管理 ────────────────────────────────────────────
//
// 检测系统中是否存在 llama-server，并提供从 GitHub Releases 自动下载安装的能力。
// 安装路径: {app_data_dir}/llama-cpp/
// 版本检测: 通过 GitHub API 获取最新 release 标签

use std::sync::atomic::{AtomicU64, Ordering};

/// GitHub API: llama.cpp 最新 release
const LLAMA_CPP_LATEST_API: &str =
    "https://api.github.com/repos/ggml-org/llama.cpp/releases/latest";
/// 下载 URL 模板: {tag} 替换为版本号
#[allow(dead_code)]
const LLAMA_CPP_DOWNLOAD_TEMPLATE: &str = "https://github.com/ggml-org/llama.cpp/releases/download/{tag}/llama-{tag}-bin-{platform}.{ext}";

/// 平台标识符（用于构建下载文件名）
fn platform_suffix() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "win-x64"
    }
    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "aarch64")]
        {
            "macos-arm64"
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            "macos-x64"
        }
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "ubuntu-arm64"
    }
    #[cfg(all(target_os = "linux", not(target_arch = "aarch64")))]
    {
        "ubuntu-x64"
    }
    #[cfg(target_os = "android")]
    {
        "android"
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "android"
    )))]
    {
        "unknown"
    }
}

/// 安装包文件扩展名
fn package_ext() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "zip"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "tar.gz"
    }
}

/// 查找可执行文件：先检查绝对路径，再在 PATH 中搜索。
fn find_executable(name_or_path: &str) -> Option<PathBuf> {
    let path = Path::new(name_or_path);
    if path.is_absolute() && path.is_file() {
        return Some(path.to_path_buf());
    }
    // 如果包含路径分隔符，说明是相对路径
    if name_or_path.contains('/') || name_or_path.contains('\\') {
        let canonical = std::env::current_dir().ok()?.join(name_or_path);
        if canonical.is_file() {
            return Some(canonical);
        }
        return None;
    }
    // 在 PATH 中搜索
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name_or_path);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{name_or_path}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

/// 安装目录: {app_data_dir}/llama-cpp/
fn install_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("llama-cpp")
}

/// 版本信息 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppVersionInfo {
    pub tag: String,
    pub name: String,
    pub published_at: String,
    pub download_url: String,
    pub file_name: String,
    pub file_size: Option<u64>,
}

/// 安装状态 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlamaCppInstallStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub install_path: Option<String>,
    pub executable_path: Option<String>,
    pub is_downloading: bool,
    pub download_progress: Option<f64>,
    pub download_error: Option<String>,
}

/// 正在进行的安装任务
struct InstallTask {
    tag: String,
    downloaded_bytes: AtomicU64,
    total_bytes: AtomicU64,
}

static INSTALL_TASKS: LazyLock<StdMutex<HashMap<String, InstallTask>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

/// 检测 llama-server 是否存在。
/// 支持绝对路径、相对路径或纯名称（在 PATH 中搜索）。
#[agent_command(domain = model, safety = Safe, call_mode = StateInput, description = "检测 llama-server 是否存在")]
#[tauri::command]
pub async fn local_model_check_server(server_exe: String) -> Result<Option<String>, String> {
    match find_executable(&server_exe) {
        Some(path) => Ok(Some(path.to_string_lossy().to_string())),
        None => Ok(None),
    }
}

/// 获取 llama.cpp 最新版本信息（用于前端展示下载选项）。
#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "获取 llama.cpp 最新版本")]
#[tauri::command]
pub async fn local_model_get_latest_version() -> Result<LlamaCppVersionInfo, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| command_error(e, lm_err::DOWNLOAD_FAILED))?;

    let resp = client
        .get(LLAMA_CPP_LATEST_API)
        .header("User-Agent", "AxAgent-llama-installer")
        .send()
        .await
        .map_err(|e| command_error(e, lm_err::DOWNLOAD_FAILED))?;

    if !resp.status().is_success() {
        return Err(ErrorResponse::err_with_detail(
            lm_err::DOWNLOAD_FAILED,
            format!("GitHub API 请求失败: HTTP {}", resp.status()),
        ));
    }

    let data: serde_json::Value =
        resp.json().await.map_err(|e| command_error(e, lm_err::DOWNLOAD_FAILED))?;

    let tag = data.get("tag_name").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let name = data.get("name").and_then(|v| v.as_str()).unwrap_or(&tag).to_string();
    let published_at = data.get("published_at").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let platform = platform_suffix();
    let ext = package_ext();
    let file_name = format!("llama-{tag}-bin-{platform}.{ext}");
    let download_url =
        format!("https://github.com/ggml-org/llama.cpp/releases/download/{tag}/{file_name}");

    // 估算文件大小: 从 assets 中查找匹配的文件
    let file_size = data.get("assets").and_then(|a| a.as_array()).and_then(|assets| {
        assets
            .iter()
            .find(|asset| {
                asset.get("name").and_then(|n| n.as_str()).map(|n| n == file_name).unwrap_or(false)
            })
            .and_then(|asset| asset.get("size"))
            .and_then(|s| s.as_u64())
    });

    Ok(LlamaCppVersionInfo { tag, name, published_at, download_url, file_name, file_size })
}

/// 下载并安装 llama.cpp（后台异步任务）。
/// 安装完成后返回可执行文件路径。
#[agent_command(domain = model, safety = Caution, call_mode = StateInput, description = "安装 llama.cpp")]
#[tauri::command]
pub async fn local_model_install_server(
    state: State<'_, AppState>,
    tag: String,
) -> Result<LlamaCppInstallStatus, String> {
    // 检查是否正在安装
    {
        let tasks = INSTALL_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(task) = tasks.get("current") {
            if !task.tag.is_empty() {
                return Err(ErrorResponse::err(lm_err::INSTALL_IN_PROGRESS));
            }
        }
    }

    let app_data = state.app_data_dir.clone();
    let install_dir = install_dir(&app_data);
    std::fs::create_dir_all(&install_dir).map_err(|e| {
        ErrorResponse::err_with_detail(lm_err::INSTALL_FAILED, format!("创建安装目录失败: {e}"))
    })?;

    let platform = platform_suffix();
    let ext = package_ext();
    let file_name = format!("llama-{tag}-bin-{platform}.{ext}");
    let download_url =
        format!("https://github.com/ggml-org/llama.cpp/releases/download/{tag}/{file_name}");

    let task = InstallTask {
        tag: tag.clone(),
        downloaded_bytes: AtomicU64::new(0),
        total_bytes: AtomicU64::new(0),
    };
    INSTALL_TASKS.lock().unwrap_or_else(|e| e.into_inner()).insert("current".to_string(), task);

    let install_dir_clone = install_dir.clone();
    let tag_clone = tag.clone();
    let filename_clone = file_name.clone();
    let url_clone = download_url.clone();

    tokio::spawn(async move {
        let result = async {
            let client = reqwest::Client::new();
            let resp = client
                .get(&url_clone)
                .header("User-Agent", "AxAgent-llama-installer")
                .send()
                .await
                .map_err(|e| format!("下载请求失败: {e}"))?;

            if !resp.status().is_success() {
                return Err(format!("下载失败: HTTP {}", resp.status()));
            }

            let total_size = resp.content_length().unwrap_or(0);
            {
                let tasks = INSTALL_TASKS.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(task) = tasks.get("current") {
                    task.total_bytes.store(total_size, Ordering::Relaxed);
                }
            }

            let download_path = install_dir_clone.join(&filename_clone);
            let mut file = std::fs::File::create(&download_path)
                .map_err(|e| format!("创建下载文件失败: {e}"))?;

            let mut downloaded: u64 = 0;
            let mut stream = resp.bytes_stream();
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| format!("下载流读取失败: {e}"))?;
                file.write_all(&chunk).map_err(|e| format!("写入下载文件失败: {e}"))?;
                downloaded += chunk.len() as u64;
                {
                    let tasks = INSTALL_TASKS.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(task) = tasks.get("current") {
                        task.downloaded_bytes.store(downloaded, Ordering::Relaxed);
                    }
                }
            }
            drop(file);

            // 解压
            let extract_dir = install_dir_clone.join(format!("llama-{tag_clone}"));
            std::fs::create_dir_all(&extract_dir).map_err(|e| format!("创建解压目录失败: {e}"))?;

            extract_archive(&download_path, &extract_dir).map_err(|e| format!("解压失败: {e}"))?;

            // 查找 llama-server 可执行文件
            let server_exe = find_llama_server(&extract_dir)
                .ok_or_else(|| "解压完成但未找到 llama-server 可执行文件".to_string())?;

            // 记录安装信息
            #[cfg(not(windows))]
            let installed_exe = install_dir_clone.join("llama-server");
            #[cfg(windows)]
            let installed_exe = install_dir_clone.join("llama-server.exe");

            // 复制或创建符号链接
            if server_exe != installed_exe {
                if installed_exe.exists() {
                    let _ = std::fs::remove_file(&installed_exe);
                }
                std::fs::copy(&server_exe, &installed_exe)
                    .map_err(|e| format!("复制可执行文件失败: {e}"))?;
            }

            // 保存版本信息
            let version_file = install_dir_clone.join("version.txt");
            std::fs::write(&version_file, &tag_clone)
                .map_err(|e| format!("写入版本信息失败: {e}"))?;

            // 清理下载包
            let _ = std::fs::remove_file(&download_path);

            tracing::info!(
                "[local_model] llama.cpp {} 安装完成: {}",
                tag_clone,
                installed_exe.display()
            );

            Ok(installed_exe.to_string_lossy().to_string())
        }
        .await;

        // 清理安装任务
        {
            let mut tasks = INSTALL_TASKS.lock().unwrap_or_else(|e| e.into_inner());
            tasks.remove("current");
        }

        if let Err(ref e) = result {
            tracing::error!("[local_model] llama.cpp 安装失败: {}", e);
        }
    });

    Ok(LlamaCppInstallStatus {
        installed: false,
        version: None,
        install_path: Some(install_dir.to_string_lossy().to_string()),
        executable_path: None,
        is_downloading: true,
        download_progress: Some(0.0),
        download_error: None,
    })
}

/// 查询当前安装状态（是否已安装、版本、下载进度）。
#[agent_command(domain = model, safety = Safe, call_mode = StateOnly, description = "查询 llama.cpp 安装状态")]
#[tauri::command]
pub async fn local_model_get_install_status(
    state: State<'_, AppState>,
) -> Result<LlamaCppInstallStatus, String> {
    let app_data = state.app_data_dir.clone();
    let dir = install_dir(&app_data);

    // 检查是否已安装
    #[cfg(windows)]
    let exe_path = dir.join("llama-server.exe");
    #[cfg(not(windows))]
    let exe_path = dir.join("llama-server");

    let installed = exe_path.is_file();

    // 读取版本
    let version = if installed {
        let version_file = dir.join("version.txt");
        std::fs::read_to_string(version_file).ok().map(|v| v.trim().to_string())
    } else {
        None
    };

    // 检查下载进度
    let (is_downloading, progress, error) = {
        let tasks = INSTALL_TASKS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(task) = tasks.get("current") {
            let downloaded = task.downloaded_bytes.load(Ordering::Relaxed);
            let total = task.total_bytes.load(Ordering::Relaxed);
            let p = if total > 0 {
                downloaded as f64 / total as f64 * 100.0
            } else {
                0.0
            };
            (true, Some(p), None)
        } else {
            (false, None, None)
        }
    };

    Ok(LlamaCppInstallStatus {
        installed,
        version,
        install_path: Some(dir.to_string_lossy().to_string()),
        executable_path: installed.then(|| exe_path.to_string_lossy().to_string()),
        is_downloading,
        download_progress: progress,
        download_error: error,
    })
}

/// 从已安装的路径获取 llama-server 可执行文件路径。
#[allow(dead_code)]
pub fn installed_server_path(app_data_dir: &Path) -> Option<PathBuf> {
    let dir = install_dir(app_data_dir);
    #[cfg(windows)]
    let exe = dir.join("llama-server.exe");
    #[cfg(not(windows))]
    let exe = dir.join("llama-server");
    if exe.is_file() { Some(exe) } else { None }
}

// ── 辅助函数 ──────────────────────────────────────────────────────

use std::io::Write;

/// 从解压目录中递归查找 llama-server 可执行文件
fn find_llama_server(dir: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    let target = "llama-server.exe";
    #[cfg(not(windows))]
    let target = "llama-server";

    for entry in walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_name().to_string_lossy() == target {
            return Some(entry.into_path());
        }
    }
    None
}

/// 解压归档文件（zip / tar.gz）
fn extract_archive(archive_path: &Path, dest: &Path) -> Result<(), String> {
    let ext = archive_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext {
        "zip" => extract_zip(archive_path, dest),
        "gz" => extract_tar_gz(archive_path, dest),
        _ => Err(format!("不支持的归档格式: {ext}")),
    }
}

fn extract_zip(archive_path: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
        })?;
        let out_path = dest.join(file.mangled_name());

        if file.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| {
                String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
            })?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
                })?;
            }
            let mut outfile = std::fs::File::create(&out_path).map_err(|e| {
                String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
            })?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| {
                String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
            })?;
        }
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, dest: &Path) -> Result<(), String> {
    use flate2::read::GzDecoder;

    let file = std::fs::File::open(archive_path)
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(dest)
        .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))?;
    Ok(())
}
