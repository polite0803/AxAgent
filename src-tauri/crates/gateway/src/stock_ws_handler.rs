// SPDX-License-Identifier: AGPL-3.0-only

//! P3: WebSocket 行情推送处理器
//!
//! 通过 `MarketDataStreamer` trait 订阅指定股票的实时行情，以 JSON 格式
//! 逐笔推送到 WebSocket 客户端。
//!
//! 数据源由 wiring 层注入的 `MarketDataStreamer` 实现决定：
//! - `HttpPollingStreamer`：HTTP 轮询（当前默认，2s 间隔）
//! - `WebSocketStreamer`：真实 WS 数据源（未来，无需改 consumer 代码）

use std::sync::Arc;

use axum::{
    extract::ws::{Message, WebSocket},
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio::time::{Duration, Instant};

use axagent_harness::market_data::MarketDataStreamer;

use crate::server::GatewayAppState;

/// 行情流空闲超时：客户端不发送任何消息（含 Ping）超过此时间则断开
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

/// 心跳间隔：服务端每 30s 发一次 Ping
const STREAM_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// WebSocket 单条消息上限（4MB，与 realtime 一致）
const STREAM_MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Deserialize)]
pub struct StockQuoteStreamQuery {
    /// 逗号分隔的股票代码列表，例如 `000001,600519,00700.HK`
    pub codes: String,
}

/// GET /v1/stock/quote/stream — WebSocket 行情推送端点
///
/// 受 `auth_middleware` 保护，调用方需先通过 API key 认证。
///
/// # 查询参数
/// - `codes`: 逗号分隔的股票代码（必填，至少一个）
///
/// # WebSocket 协议
/// - 服务端推送：`QuoteUpdate` JSON 消息
/// - 客户端可发送 Ping 维持连接
/// - 空闲超时 120s 后服务端关闭连接
pub async fn stock_quote_stream_handler(
    State(state): State<GatewayAppState>,
    Query(params): Query<StockQuoteStreamQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let streamer = match &state.market_data_streamer {
        Some(s) => s.clone(),
        None => {
            tracing::warn!("[stock_quote_stream] market_data_streamer 未注入，返回 503");
            return (StatusCode::SERVICE_UNAVAILABLE, "Market data streamer not available")
                .into_response();
        },
    };

    let codes: Vec<String> =
        params.codes.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

    if codes.is_empty() {
        return (StatusCode::BAD_REQUEST, "codes 查询参数不能为空（示例: ?codes=000001,600519）")
            .into_response();
    }

    let ws = ws.max_message_size(STREAM_MAX_MESSAGE_BYTES);

    tracing::info!(
        "[stock_quote_stream] WS upgrade: codes={:?}, source={}",
        codes,
        streamer.source_type()
    );

    ws.on_upgrade(move |socket| handle_stock_quote_stream(socket, streamer, codes))
}

/// WebSocket 会话主循环：接收行情更新并转发到 WS
async fn handle_stock_quote_stream(
    socket: WebSocket,
    streamer: Arc<dyn MarketDataStreamer>,
    codes: Vec<String>,
) {
    let mut rx = match streamer.subscribe(codes.clone()).await {
        Ok(rx) => rx,
        Err(e) => {
            tracing::error!("[stock_quote_stream] subscribe 失败: {:?}, error: {}", codes, e);
            return;
        },
    };

    let mut socket = socket;
    let mut last_activity = Instant::now();
    let mut heartbeat = tokio::time::interval_at(
        tokio::time::Instant::now() + STREAM_HEARTBEAT_INTERVAL,
        STREAM_HEARTBEAT_INTERVAL,
    );
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            biased;

            // 空闲超时保护
            _ = tokio::time::sleep_until(last_activity + STREAM_IDLE_TIMEOUT) => {
                tracing::warn!(
                    "[stock_quote_stream] idle timeout ({}s), closing",
                    STREAM_IDLE_TIMEOUT.as_secs()
                );
                let _ = socket.send(Message::Close(None)).await;
                break;
            }

            // 心跳 Ping
            _ = heartbeat.tick() => {
                if socket.send(Message::Ping(axum::body::Bytes::new())).await.is_err() {
                    break;
                }
            }

            // 行情更新 → 转发到 WS
            update = rx.recv() => {
                let update = match update {
                    Some(u) => u,
                    None => {
                        tracing::debug!("[stock_quote_stream] streamer 通道关闭");
                        break;
                    },
                };

                last_activity = Instant::now();

                let json = match serde_json::to_string(&update) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::warn!("[stock_quote_stream] 序列化 QuoteUpdate 失败: {}", e);
                        continue;
                    },
                };

                if socket.send(Message::Text(json.into())).await.is_err() {
                    tracing::debug!("[stock_quote_stream] 客户端断开");
                    break;
                }
            }

            // 客户端消息处理
            msg = socket.recv() => {
                match msg {
                    Some(Ok(m)) => {
                        match m {
                            Message::Ping(data) => {
                                last_activity = Instant::now();
                                let _ = socket.send(Message::Pong(data)).await;
                            },
                            Message::Pong(_) => {
                                last_activity = Instant::now();
                            },
                            Message::Close(_) => {
                                tracing::debug!("[stock_quote_stream] 客户端主动关闭");
                                break;
                            },
                            // 忽略其他文本/二进制消息
                            _ => {
                                last_activity = Instant::now();
                            },
                        }
                    },
                    Some(Err(e)) => {
                        tracing::debug!("[stock_quote_stream] recv error: {}", e);
                        break;
                    },
                    None => break,
                }
            }
        }
    }

    tracing::debug!("[stock_quote_stream] 会话结束");
}
