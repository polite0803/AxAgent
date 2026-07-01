// SPDX-License-Identifier: AGPL-3.0-only

//! Webhook HTTP 服务器。
//!
//! 基于 `tiny_http` 创建轻量级 HTTP 服务器，按注册路径路由到工作流。
//! 支持同步（等待完成）和异步（立即返回 202）两种响应模式。

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

use crate::work_engine::WorkEngine;

use super::WebhookRoute;

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

    handle
        .await
        .map_err(|e| format!("Webhook 服务器任务错误: {e}"))
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

    // 提取请求体 JSON
    let input = extract_json_body(&mut request);

    let wf_id = route.workflow_id.clone();
    let response_mode = route.response_mode.clone();

    match response_mode.as_str() {
        "sync" => {
            // 同步模式：等待工作流完成再返回结果
            let run_opts = crate::work_engine::RunOptions {
                input: Some(input),
                ..Default::default()
            };
            match engine.run_workflow(&wf_id, run_opts).await {
                Ok(workflow) => {
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
                            .unwrap(),
                    );
                    let _ = request.respond(response);
                },
                Err(e) => {
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
                            .unwrap(),
                    );
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
            .with_header(
                "Content-Type: application/json"
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            );
            let _ = request.respond(response);

            // 后台触发工作流
            let engine = engine.clone();
            let wf_id = wf_id.clone();
            tokio::spawn(async move {
                let run_opts = crate::work_engine::RunOptions {
                    input: Some(input),
                    ..Default::default()
                };
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
fn extract_json_body(request: &mut tiny_http::Request) -> serde_json::Value {
    let mut body_str = String::new();
    let reader = request.as_reader();
    let _ = reader.read_to_string(&mut body_str);
    if body_str.trim().is_empty() {
        return serde_json::json!({});
    }
    serde_json::from_str(&body_str).unwrap_or_else(|e| {
        tracing::warn!("Webhook 请求体 JSON 解析失败: {e}, raw={body_str}");
        serde_json::json!({"raw_body": body_str})
    })
}
