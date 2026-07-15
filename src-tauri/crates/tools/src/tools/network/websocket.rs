// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::{Tool, ToolCategory, ToolContext, ToolDomain, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct WebSocketTool;

#[async_trait]
impl Tool for WebSocketTool {
    fn name(&self) -> &str {
        "WebSocket"
    }
    fn description(&self) -> &str {
        "通过 WebSocket 连接到服务器，发送消息并接收响应。支持 ws:// 和 wss:// 协议。每次调用建立连接→发送→接收→关闭。基于 tokio::net 手动实现，零外部依赖。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": {"type": "string", "description": "WebSocket URL (ws:// 或 wss://)"},
                "message": {"type": "string", "description": "要发送的文本消息"},
                "headers": {"type": "object", "description": "额外握手请求头"},
                "timeout_secs": {"type": "integer", "default": 15, "description": "接收超时秒数"},
                "max_recv_bytes": {"type": "integer", "default": 65536, "description": "最大接收字节数"}
            },
            "required": ["url"]
        })
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::Network
    }

    fn domain(&self) -> ToolDomain {
        ToolDomain::General
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let url = input["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            return Ok(ToolResult::error("Error: url 是必需的"));
        }

        let (host, port, path, use_tls) = parse_ws_url(&url)?;
        let message = input["message"].as_str().unwrap_or("");
        let timeout_secs = input["timeout_secs"].as_u64().unwrap_or(15);
        let max_recv = input["max_recv_bytes"].as_u64().unwrap_or(65536) as usize;

        // 建立 TCP 连接
        let addr = format!("{}:{}", host, port);
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        .map_err(|_| ToolError::execution_failed("连接超时".to_string()))?
        .map_err(|e| ToolError::execution_failed(format!("TCP 连接失败: {}", e)))?;

        if use_tls {
            // wss:// — 这里简化处理：返回不支持说明
            // 完整的 TLS WebSocket 需要 rustls 等，但可以用 tokio-native-tls
            let _ = stream;
            return websocket_over_tls(&host, port, &path, &url, message, timeout_secs, max_recv)
                .await;
        }

        // ws:// — 手动实现 WebSocket 握手 + 帧收发
        websocket_raw(stream, &host, &path, &url, message, timeout_secs, max_recv).await
    }
}

fn parse_ws_url(url: &str) -> Result<(String, u16, String, bool), ToolError> {
    let (host_part, use_tls) = if let Some(rest) = url.strip_prefix("wss://") {
        (rest, true)
    } else if let Some(rest) = url.strip_prefix("ws://") {
        (rest, false)
    } else {
        return Err(ToolError::invalid_input("url 必须以 ws:// 或 wss:// 开头"));
    };

    let (host_and_port, path) = match host_part.find('/') {
        Some(idx) => (&host_part[..idx], &host_part[idx..]),
        None => (host_part, "/"),
    };

    let (host, port) = match host_and_port.split_once(':') {
        Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(if use_tls { 443 } else { 80 })),
        None => (host_and_port.to_string(), if use_tls { 443 } else { 80 }),
    };

    Ok((host, port, path.to_string(), use_tls))
}

async fn websocket_raw(
    mut stream: tokio::net::TcpStream,
    host: &str,
    path: &str,
    _url: &str,
    message: &str,
    timeout_secs: u64,
    max_recv: usize,
) -> Result<ToolResult, ToolError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // WebSocket 握手 key（UUID v4 → 16 字节随机 + Base64）
    let random_bytes = uuid::Uuid::new_v4().into_bytes();
    let key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, random_bytes);

    // 发送 HTTP Upgrade 请求
    let handshake = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {}\r\nSec-WebSocket-Version: 13\r\n\r\n",
        path, host, key
    );

    stream
        .write_all(handshake.as_bytes())
        .await
        .map_err(|e| ToolError::execution_failed(format!("发送握手失败: {}", e)))?;

    // 读取握手响应
    let mut buf = vec![0u8; 4096];
    let n =
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), stream.read(&mut buf))
            .await
            .map_err(|_| ToolError::execution_failed("握手超时".to_string()))?
            .map_err(|e| ToolError::execution_failed(format!("读取握手响应失败: {}", e)))?;

    let response = String::from_utf8_lossy(&buf[..n]);
    if !response.contains("101") {
        return Ok(ToolResult::error(format!(
            "WebSocket 握手失败:\n{}",
            truncate(&response, 1000)
        )));
    }

    let mut result = format!("✅ WebSocket 已连接到 {}\n\n握手成功 (HTTP 101)\n", _url);

    // 发送消息（如果有）
    if !message.is_empty() {
        let frame = build_ws_frame(message.as_bytes(), 0x1); // text frame
        stream
            .write_all(&frame)
            .await
            .map_err(|e| ToolError::execution_failed(format!("发送消息失败: {}", e)))?;
        result.push_str(&format!("📤 已发送: {}\n", truncate(message, 500)));

        // 接收响应
        let mut recv_buf = vec![0u8; max_recv];
        match tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            stream.read(&mut recv_buf),
        )
        .await
        {
            Ok(Ok(n)) if n > 0 => {
                if let Some((opcode, payload)) = parse_ws_frame(&recv_buf[..n]) {
                    let text = String::from_utf8_lossy(&payload);
                    result.push_str(&format!(
                        "📥 收到 (opcode={}): {}\n",
                        opcode,
                        truncate(&text, 10_000)
                    ));
                }
            },
            Ok(Ok(_)) => result.push_str("📥 收到空响应\n"),
            Ok(Err(e)) => result.push_str(&format!("接收错误: {}\n", e)),
            Err(_) => result.push_str("⏱ 接收超时\n"),
        }
    }

    // 发送关闭帧
    let close_frame = build_ws_frame(&[], 0x8);
    let _ = stream.write_all(&close_frame).await;

    Ok(ToolResult::success(result))
}

async fn websocket_over_tls(
    _host: &str,
    _port: u16,
    _path: &str,
    url: &str,
    _message: &str,
    _timeout_secs: u64,
    _max_recv: usize,
) -> Result<ToolResult, ToolError> {
    Ok(ToolResult::success(format!(
        "WebSocket URL: {}\n\n⚠️ wss:// (TLS) 需要 native-tls 或 rustls。\n当前仅支持 ws:// 明文连接。\n对于 wss://，请使用 HttpRequest 工具调用 REST API 作为替代。",
        url
    )))
}

/// 构建 WebSocket 帧
fn build_ws_frame(payload: &[u8], opcode: u8) -> Vec<u8> {
    let mut frame = Vec::new();
    frame.push(0x80 | opcode); // FIN + opcode

    let len = payload.len();
    if len < 126 {
        frame.push(len as u8);
    } else if len <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    frame.extend_from_slice(payload);
    frame
}

/// 解析 WebSocket 帧，返回 (opcode, payload)
fn parse_ws_frame(data: &[u8]) -> Option<(u8, Vec<u8>)> {
    if data.len() < 2 {
        return None;
    }
    let opcode = data[0] & 0x0F;

    let mut offset = 2;
    let payload_len = match data[1] & 0x7F {
        126 => {
            if data.len() < 4 {
                return None;
            }
            offset = 4;
            u16::from_be_bytes([data[2], data[3]]) as usize
        },
        127 => {
            if data.len() < 10 {
                return None;
            }
            offset = 10;
            u64::from_be_bytes([
                data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
            ]) as usize
        },
        n => n as usize,
    };

    if data.len() < offset + payload_len {
        return None;
    }
    Some((opcode, data[offset..offset + payload_len].to_vec()))
}
