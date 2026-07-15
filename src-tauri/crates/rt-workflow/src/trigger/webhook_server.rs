// SPDX-License-Identifier: AGPL-3.0-only

//! Webhook HTTP 服务器。
//!
//! 基于 `tiny_http` 创建轻量级 HTTP 服务器，按注册路径路由到工作流。
//! 支持同步（等待完成）和异步（立即返回 202）两种响应模式。

use std::collections::HashMap;
use std::sync::Arc;

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::work_engine::WorkEngine;

use super::WebhookRoute;

/// P0-6: 请求体上限 1MB。超过此大小直接 413 拒绝，避免内存耗尽 DoS。
const MAX_BODY_SIZE: usize = 1024 * 1024;

/// 启动 Webhook HTTP 服务器（阻塞调用，应在 tokio::spawn_blocking 中运行）。
pub async fn serve(
    bind_addr: String,
    engine: Arc<WorkEngine>,
    routes: HashMap<String, WebhookRoute>,
) -> Result<(), String> {
    let routes = Arc::new(routes);

    let server = tiny_http::Server::http(&bind_addr)
        .map_err(|e| format!("Webhook 服务器启动失败 ({}): {e}", bind_addr))?;

    tracing::info!(bind_addr = %bind_addr, "Webhook 服务器已就绪");

    // tiny_http 是阻塞式的，需要 spawn_blocking 避免阻塞 async runtime
    let handle = tokio::task::spawn_blocking(move || {
        for request in server.incoming_requests() {
            let path = request.url().to_string();
            let method = request.method().to_string();

            let routes = routes.clone();
            let engine = engine.clone();

            tokio::task::block_in_place(|| {
                let handle = tokio::runtime::Handle::current();
                handle.block_on(async {
                    handle_request(request, &path, &method, &routes, &engine).await;
                });
            });
        }
    });

    handle.await.map_err(|e| format!("Webhook 服务器任务错误: {e}"))
}

/// P0-6: HMAC-SHA256 签名校验，constant_time_eq 比对。
/// `route.secret` 为 None 时跳过校验（仅用于本地开发/调试；生产应配置 secret）。
/// 客户端需要发送 `X-Webhook-Signature: hex(hmac_sha256(secret, body))`。
fn verify_hmac(secret: &str, body: &[u8], sig_hex: &str) -> bool {
    if sig_hex.len() != 64 || !sig_hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    let Ok(sig_bytes) = hex::decode(sig_hex) else {
        return false;
    };
    let Ok(mut mac) = <Hmac<Sha256> as KeyInit>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    let computed = mac.finalize().into_bytes();
    bool::from(computed.ct_eq(&sig_bytes))
}

/// 处理单个 HTTP 请求。
async fn handle_request(
    mut request: tiny_http::Request,
    path: &str,
    method: &str,
    routes: &HashMap<String, WebhookRoute>,
    engine: &Arc<WorkEngine>,
) {
    // 路径匹配（支持尾部斜杠归一化）
    let normalized = path.trim_end_matches('/');
    let route = if let Some(r) = routes.get(normalized) {
        r
    } else if let Some(r) = routes.get(&format!("{normalized}/")) {
        r
    } else {
        // 未匹配到路由
        let response = tiny_http::Response::from_string("404 Not Found").with_status_code(404);
        let _ = request.respond(response);
        return;
    };

    // 方法校验
    if !route.method.eq_ignore_ascii_case(method) && route.method != "*" {
        let response = tiny_http::Response::from_string(format!(
            "405 Method Not Allowed (expected {})",
            route.method
        ))
        .with_status_code(405);
        let _ = request.respond(response);
        return;
    }

    // 提取请求体（受 MAX_BODY_SIZE 限制）
    let (body_bytes, body_opt) = match extract_json_body(&mut request) {
        Ok(b) => b,
        Err(reason) => {
            let response = tiny_http::Response::from_string(reason.clone()).with_status_code(413);
            let _ = request.respond(response);
            return;
        },
    };

    // P0-6: HMAC 签名校验（如果 route 配置了 secret，则必须校验通过）
    if let Some(secret) = route.secret.as_deref() {
        let sig = request
            .headers()
            .iter()
            .find(|h| h.field.equiv("X-Webhook-Signature"))
            .map(|h| h.value.as_str().to_string());
        let sig = match sig {
            Some(s) => s,
            None => {
                let response = tiny_http::Response::from_string(
                    "401 Unauthorized: missing X-Webhook-Signature",
                )
                .with_status_code(401);
                let _ = request.respond(response);
                return;
            },
        };
        if !verify_hmac(secret, &body_bytes, &sig) {
            let response = tiny_http::Response::from_string("401 Unauthorized: invalid signature")
                .with_status_code(401);
            let _ = request.respond(response);
            return;
        }
    }

    let input = body_opt;
    let wf_id = route.workflow_id.clone();
    let response_mode = route.response_mode.clone();

    match response_mode.as_str() {
        "sync" => {
            // 同步模式：等待工作流完成再返回结果
            let run_opts =
                crate::work_engine::RunOptions { input: Some(input), ..Default::default() };
            // P0-6: 即使在 sync 模式，也用 tokio::spawn 异步执行，避免阻塞 tokio runtime
            // 当前 sync 语义下我们仍要等待结果；改用 spawn + oneshot 接收结果，保持 HTTP
            // 请求处理线程可取消（不阻塞其他 inbound 请求的 tiny_http accept 循环）。
            let (tx, rx) = tokio::sync::oneshot::channel();
            let engine_for_task = engine.clone();
            let wf_id_for_task = wf_id.clone();
            tokio::spawn(async move {
                let result = engine_for_task.run_workflow(&wf_id_for_task, run_opts).await;
                let _ = tx.send(result);
            });
            match rx.await {
                Ok(Ok(workflow)) => {
                    let body = serde_json::json!({
                        "status": format!("{:?}", workflow.status),
                        "workflow_id": wf_id,
                        "results": workflow.results,
                    });
                    let response = tiny_http::Response::from_string(
                        serde_json::to_string_pretty(&body).unwrap_or_default(),
                    )
                    .with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .expect("Content-Type: application/json is a valid header"),
                    );
                    let _ = request.respond(response);
                },
                Ok(Err(e)) => {
                    let body = serde_json::json!({
                        "error": e.to_string(),
                        "workflow_id": wf_id,
                    });
                    let response = tiny_http::Response::from_string(
                        serde_json::to_string_pretty(&body).unwrap_or_default(),
                    )
                    .with_status_code(500)
                    .with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .expect("Content-Type: application/json is a valid header"),
                    );
                    let _ = request.respond(response);
                },
                Err(_) => {
                    let response = tiny_http::Response::from_string(
                        "500 Internal Server Error: workflow task dropped",
                    )
                    .with_status_code(500);
                    let _ = request.respond(response);
                },
            }
        },
        _ => {
            // 异步模式（默认）：立即返回 202，后台触发工作流
            let response = tiny_http::Response::from_string(
                serde_json::json!({
                    "status": "accepted",
                    "workflow_id": wf_id,
                })
                .to_string(),
            )
            .with_status_code(202)
            .with_header("Content-Type: application/json".parse::<tiny_http::Header>().unwrap());
            let _ = request.respond(response);

            // 后台触发工作流
            let engine = engine.clone();
            let wf_id = wf_id.clone();
            tokio::spawn(async move {
                let run_opts =
                    crate::work_engine::RunOptions { input: Some(input), ..Default::default() };
                if let Err(e) = engine.run_workflow(&wf_id, run_opts).await {
                    tracing::error!(
                        workflow_id = %wf_id,
                        error = %e,
                        "Webhook 异步触发工作流失败"
                    );
                }
            });
        },
    }
}

/// 从请求体中提取 JSON，失败时返回空对象。
///
/// P0-6: 限制 body 上限为 1MB；超过返回 413。
/// 返回 `(原始字节, 解析后的 JSON Value)`：
/// - 字节用于 HMAC 校验
/// - JSON 用于 `run_workflow` 的 input
fn extract_json_body(
    request: &mut tiny_http::Request,
) -> Result<(Vec<u8>, serde_json::Value), String> {
    use std::io::Read;
    let mut buf = Vec::new();
    let reader = request.as_reader();
    // take(MAX_BODY_SIZE+1) 用于探测是否超限
    let mut limited = reader.take((MAX_BODY_SIZE as u64) + 1);
    if let Err(e) = limited.read_to_end(&mut buf) {
        return Err(format!("Failed to read request body: {e}"));
    }
    if buf.len() > MAX_BODY_SIZE {
        return Err(format!("Request body too large (limit {MAX_BODY_SIZE} bytes)"));
    }
    let body_str = String::from_utf8_lossy(&buf);
    if body_str.trim().is_empty() {
        return Ok((buf, serde_json::json!({})));
    }
    let value = serde_json::from_str(&body_str).unwrap_or_else(|e| {
        tracing::warn!("Webhook 请求体 JSON 解析失败: {e}");
        serde_json::json!({"raw_body": body_str.to_string()})
    });
    Ok((buf, value))
}
