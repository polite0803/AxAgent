//! MCP 测试服务器 — 纯 Rust 实现
//!
//! 替代 Python 测试脚本，用于 MCP stdio 传输的集成测试。
//! 行为通过环境变量控制，支持所有测试场景。
//!
//! 环境变量：
//! - MCP_SERVER_LABEL              服务器名称（默认 "server"）
//! - MCP_LOG_PATH                  日志文件路径
//! - MCP_ECHO_MODE                 简单回显模式（非 JSON-RPC）
//! - MCP_EXIT_AFTER_TOOLS_LIST     在 tools/list 后退出
//! - MCP_FAIL_ONCE_MODE            一次性失败模式
//! - MCP_FAIL_ONCE_MARKER          一次性失败标记文件
//! - MCP_TOOL_CALL_DELAY_MS        工具调用延迟（毫秒）
//! - MCP_INVALID_TOOL_CALL_RESPONSE 返回非法响应
//! - MCP_LOWERCASE_CONTENT_LENGTH  使用小写 content-length
//! - MCP_MISMATCHED_RESPONSE_ID    返回不匹配的 id
//! - MCP_INITIALIZE_DISCONNECT     读取第一个消息后立即退出
//! - MCP_RESOURCES_ENABLED         启用 resources/list 和 resources/read

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

fn capabilities_json(resources_enabled: bool) -> serde_json::Value {
    if resources_enabled {
        serde_json::json!({ "tools": {}, "resources": {} })
    } else {
        serde_json::json!({ "tools": {} })
    }
}

fn main() {
    let label = std::env::var("MCP_SERVER_LABEL").unwrap_or_else(|_| "server".to_string());
    let log_path = std::env::var("MCP_LOG_PATH").ok();
    let echo_mode = std::env::var("MCP_ECHO_MODE").is_ok();
    let exit_after_tools_list = std::env::var("MCP_EXIT_AFTER_TOOLS_LIST").is_ok();
    let fail_once_mode = std::env::var("MCP_FAIL_ONCE_MODE").ok();
    let fail_once_marker = std::env::var("MCP_FAIL_ONCE_MARKER").ok();
    let tool_call_delay_ms: u64 = std::env::var("MCP_TOOL_CALL_DELAY_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let invalid_tool_call_response = std::env::var("MCP_INVALID_TOOL_CALL_RESPONSE").is_ok();
    let lowercase_content_length = std::env::var("MCP_LOWERCASE_CONTENT_LENGTH").is_ok();
    let mismatched_response_id = std::env::var("MCP_MISMATCHED_RESPONSE_ID").is_ok();
    let initialize_disconnect = std::env::var("MCP_INITIALIZE_DISCONNECT").is_ok();
    let resources_enabled = std::env::var("MCP_RESOURCES_ENABLED").is_ok();

    // 简单回显模式（非 JSON-RPC，用于 echo_script 测试）
    if echo_mode {
        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout();
        let mut line = String::new();
        if BufReader::new(stdin).read_line(&mut line).is_ok() {
            let _ = writeln!(stdout, "ECHO:{}", line.trim());
        }
        return;
    }

    // 立即断开模式
    if initialize_disconnect {
        let mut stdin = std::io::stdin();
        let mut buf = [0u8; 4096];
        let _ = stdin.read(&mut buf);
        return;
    }

    let mut stdin = BufReader::new(std::io::stdin());
    let mut stdout = std::io::stdout();
    let mut initialize_count: u32 = 0;

    fn log_call(log_path: &Option<String>, method: &str) {
        if let Some(path) = log_path {
            if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "{method}");
            }
        }
    }

    fn should_fail_once(mode: &Option<String>, marker: &Option<String>) -> bool {
        let (Some(mode), Some(marker)) = (mode, marker) else {
            return false;
        };
        if Path::new(marker).exists() {
            return false;
        }
        let _ = fs::write(marker, mode);
        true
    }

    fn read_frame(reader: &mut BufReader<std::io::Stdin>) -> Option<serde_json::Value> {
        let mut header = Vec::new();
        loop {
            let mut buf = [0u8; 1];
            if reader.read_exact(&mut buf).is_err() {
                return None;
            }
            header.push(buf[0]);
            if header.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let header_str = String::from_utf8_lossy(&header);
        let content_length = header_str
            .lines()
            .find(|l| l.to_lowercase().starts_with("content-length:"))
            .and_then(|l| l.split(':').nth(1))
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(0);

        let mut payload = vec![0u8; content_length];
        if reader.read_exact(&mut payload).is_err() {
            return None;
        }
        serde_json::from_slice(&payload).ok()
    }

    fn write_frame(
        stdout: &mut std::io::Stdout,
        response: &serde_json::Value,
        use_lowercase: bool,
    ) {
        let body = serde_json::to_vec(response).unwrap();
        let header_name = if use_lowercase {
            "content-length"
        } else {
            "Content-Length"
        };
        let header = format!("{header_name}: {}\r\n\r\n", body.len());
        let _ = stdout.write_all(header.as_bytes());
        let _ = stdout.write_all(&body);
        let _ = stdout.flush();
    }

    while let Some(request) = read_frame(&mut stdin) {
        let method = request["method"].as_str().unwrap_or("");
        log_call(&log_path, method);

        match method {
            "initialize" => {
                if fail_once_mode.as_deref() == Some("initialize_hang")
                    && should_fail_once(&fail_once_mode, &fail_once_marker)
                {
                    log_call(&log_path, "initialize-hang");
                    // 模拟挂起：无限循环（测试会用 timeout 处理）
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                    break;
                }
                initialize_count += 1;
                let response_id = if mismatched_response_id {
                    serde_json::Value::String("wrong-id".to_string())
                } else {
                    request["id"].clone()
                };
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": response_id,
                    "result": {
                        "protocolVersion": request["params"]["protocolVersion"],
                        "capabilities": capabilities_json(resources_enabled),
                        "serverInfo": { "name": label, "version": "1.0.0" }
                    }
                });
                write_frame(&mut stdout, &response, lowercase_content_length);
            },
            "tools/list" => {
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request["id"],
                    "result": {
                        "tools": [{
                            "name": "echo",
                            "description": format!("Echo tool for {label}"),
                            "inputSchema": {
                                "type": "object",
                                "properties": { "text": { "type": "string" } },
                                "required": ["text"]
                            }
                        }]
                    }
                });
                write_frame(&mut stdout, &response, lowercase_content_length);
                if exit_after_tools_list {
                    break;
                }
            },
            "tools/call" => {
                if invalid_tool_call_response {
                    let _ = stdout.write_all(b"Content-Length: 5\r\n\r\nnope!");
                    let _ = stdout.flush();
                    continue;
                }
                if fail_once_mode.as_deref() == Some("tool_call_disconnect")
                    && should_fail_once(&fail_once_mode, &fail_once_marker)
                {
                    log_call(&log_path, "tools/call-disconnect");
                    break;
                }
                if tool_call_delay_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(tool_call_delay_ms));
                }
                let args = request["params"]
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                let tool_name = request["params"]["name"].as_str().unwrap_or("");

                if tool_name == "fail" {
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": request["id"],
                        "error": { "code": -32001, "message": "tool failed" }
                    });
                    write_frame(&mut stdout, &response, lowercase_content_length);
                } else {
                    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let response = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": request["id"],
                        "result": {
                            "content": [{ "type": "text", "text": format!("{label}:{text}") }],
                            "structuredContent": {
                                "server": label,
                                "echoed": text,
                                "initializeCount": initialize_count
                            },
                            "isError": false
                        }
                    });
                    write_frame(&mut stdout, &response, lowercase_content_length);
                }
            },
            "resources/list" if resources_enabled => {
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request["id"],
                    "result": {
                        "resources": [{
                            "uri": "file://guide.txt",
                            "name": "guide",
                            "description": "Guide text",
                            "mimeType": "text/plain"
                        }]
                    }
                });
                write_frame(&mut stdout, &response, lowercase_content_length);
            },
            "resources/read" if resources_enabled => {
                let uri = request["params"]["uri"].as_str().unwrap_or("");
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request["id"],
                    "result": {
                        "contents": [{
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": format!("contents for {uri}")
                        }]
                    }
                });
                write_frame(&mut stdout, &response, lowercase_content_length);
            },
            _ => {
                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": request["id"],
                    "error": { "code": -32601, "message": format!("unknown method: {method}") }
                });
                write_frame(&mut stdout, &response, lowercase_content_length);
            },
        }
    }
}
