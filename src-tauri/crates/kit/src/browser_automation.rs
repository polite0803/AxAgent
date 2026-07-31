// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(target_os = "android"))]
use anyhow::Result;
#[cfg(not(target_os = "android"))]
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "android"))]
use std::net::ToSocketAddrs;
#[cfg(not(target_os = "android"))]
use std::process::Stdio;
#[cfg(not(target_os = "android"))]
use std::sync::{Arc, OnceLock};
#[cfg(not(target_os = "android"))]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(not(target_os = "android"))]
use tokio::process::{Child, Command};
#[cfg(not(target_os = "android"))]
use tokio::sync::Mutex;

#[cfg(target_os = "android")]
use axagent_harness::constants::android_msg;

#[cfg(not(target_os = "android"))]
#[derive(Debug, Serialize, Deserialize)]
struct BrowserRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[cfg(not(target_os = "android"))]
#[derive(Debug, Serialize, Deserialize)]
struct BrowserResponse {
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

#[cfg(not(target_os = "android"))]
pub struct PlaywrightClient {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

/// SECURITY (S4): 校验浏览器可访问的 URL，防止 SSRF。
///
/// 仅允许 `http` / `https` 协议；禁止回环、私网、链路本地、未指定地址、
/// `0.0.0.0/8`、保留段以及 IPv4 映射的 IPv6 地址（`::ffff:127.0.0.1` 等）。
/// 对域名会做 DNS 解析并逐一校验解析到的 IP（缓解 DNS 重绑定；
/// 注意 TOCTOU：若需绝对安全应在解析后固定 IP 并携带 Host 头连接）。
#[cfg(not(target_os = "android"))]
pub fn validate_browser_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|_| "无效的 URL：无法解析".to_string())?;
    match parsed.scheme() {
        "http" | "https" => {},
        _ => return Err("仅允许 http/https 协议".to_string()),
    }
    let host = match parsed.host_str() {
        Some(h) => h.to_lowercase(),
        None => return Err("URL 缺少主机名".to_string()),
    };

    // 字面量 IP 直接校验
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        check_ip(&ip)?;
        return Ok(());
    }

    // 主机名黑名单：常见本地/内网标识（部分云元数据域名形如 metadata 但解析到私网，交由下方 DNS 校验兜底）
    if host == "localhost" || host.ends_with(".localhost") || host == "[::1]" {
        return Err("禁止访问 localhost".to_string());
    }

    // DNS 解析并校验每一个解析到的 IP
    let addrs = format!("{}:0", host)
        .to_socket_addrs()
        .map_err(|_| "DNS 解析失败".to_string())?
        .map(|s| s.ip())
        .collect::<Vec<_>>();
    if addrs.is_empty() {
        return Err("DNS 解析无结果".to_string());
    }
    for ip in &addrs {
        check_ip(ip)?;
    }
    Ok(())
}

/// 校验单个 IP 是否为禁止访问的保留/内网地址。
#[cfg(not(target_os = "android"))]
fn check_ip(ip: &std::net::IpAddr) -> Result<(), String> {
    match ip {
        std::net::IpAddr::V4(v4) => {
            // 0.0.0.0/8：首字节为 0（含 0.0.0.0 本身，亦被 is_unspecified 覆盖）
            let is_zero_net = v4.octets()[0] == 0;
            // 保留段 240.0.0.0/4（即首字节 >= 240），stable Rust 无 is_reserved
            let is_reserved = v4.octets()[0] >= 240;
            if v4.is_loopback()
                || v4.is_private()
                || v4.is_unspecified()
                || v4.is_link_local()
                || is_reserved
                || is_zero_net
            {
                return Err(format!("禁止访问保留/内网地址 {}", ip));
            }
        },
        std::net::IpAddr::V6(v6) => {
            // IPv4 映射地址（::ffff:x.x.x.x）按内嵌的 IPv4 校验
            if let Some(v4) = v6.to_ipv4_mapped() {
                let is_zero_net = v4.octets()[0] == 0;
                let is_reserved = v4.octets()[0] >= 240;
                if v4.is_loopback()
                    || v4.is_private()
                    || v4.is_unspecified()
                    || v4.is_link_local()
                    || is_reserved
                    || is_zero_net
                {
                    return Err(format!("禁止访问保留/内网地址 {}", ip));
                }
                return Ok(());
            }
            // fe80::/10（链路本地）：前 10 位为 1111111010，即前两个字节为 0xfe80-0xfebf
            let is_v6_link_local = (v6.segments()[0] & 0xffc0) == 0xfe80;
            if v6.is_loopback() || v6.is_unspecified() || is_v6_link_local {
                return Err(format!("禁止访问保留/内网地址 {}", ip));
            }
        },
    }
    Ok(())
}

#[cfg(not(target_os = "android"))]
impl PlaywrightClient {
    /// 定位 browser-automation.mjs 脚本。
    ///
    /// 优先级：
    /// 1. exe 同目录 `scripts/`（打包安装版，tauri.conf.json resources 已声明）
    /// 2. 源码目录 `src-tauri/scripts/`（dev 模式：current_exe 在 target/debug，其下无 scripts，
    ///    node 直接启动即崩溃退出，stdout EOF 导致 `serde_json::from_str("")` 报
    ///    "EOF while parsing a value at line 1 column 0"）
    fn resolve_script_path() -> Result<std::path::PathBuf> {
        let exe_scripts = std::env::current_exe()?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot find exe directory"))?
            .join("scripts")
            .join("browser-automation.mjs");
        if exe_scripts.exists() {
            return Ok(exe_scripts);
        }
        // CARGO_MANIFEST_DIR = <repo>/src-tauri/crates/kit，上溯两级到 src-tauri
        let dev_scripts = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("scripts").join("browser-automation.mjs"))
            .filter(|p| p.exists())
            .unwrap_or(exe_scripts);
        Ok(dev_scripts)
    }

    pub async fn launch() -> Result<Self> {
        let script_path = Self::resolve_script_path()?;

        let mut child_builder = Command::new("node");
        child_builder
            .arg(&script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        crate::utils::hide_window(child_builder.as_std_mut());
        let mut child = child_builder.spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("No stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("No stdout"))?;
        let stdout_reader = BufReader::new(stdout);

        let mut client = Self { child, stdin, stdout_reader, next_id: 1 };

        let mut ready_line = String::new();
        client.stdout_reader.read_line(&mut ready_line).await?;
        let ready_msg: serde_json::Value = serde_json::from_str(&ready_line)?;
        if !ready_msg["ready"].as_bool().unwrap_or(false) {
            anyhow::bail!("Playwright bridge failed to start");
        }

        Ok(client)
    }

    async fn call(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = BrowserRequest { id, method: method.to_string(), params };

        let request_json = serde_json::to_string(&request)? + "\n";
        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.flush().await?;

        let mut response_line = String::new();
        // 超时保护：避免 page.goto / fetch 卡死时整个浏览器池（单 Mutex）永久阻塞（修复 #6）
        match tokio::time::timeout(
            std::time::Duration::from_secs(60),
            self.stdout_reader.read_line(&mut response_line),
        )
        .await
        {
            Ok(Ok(_)) => {},
            Ok(Err(e)) => anyhow::bail!("读取浏览器响应失败: {}", e),
            Err(_) => {
                // 超时：子进程可能已卡死，杀掉以释放资源；调用方会丢弃池中死实例并重建。
                let _ = self.child.start_kill();
                anyhow::bail!("浏览器操作超时（60s），已终止卡死的浏览器子进程");
            },
        }
        let response: BrowserResponse = serde_json::from_str(response_line.trim())?;

        if let Some(error) = response.error {
            anyhow::bail!("Browser automation error: {}", error);
        }

        response.result.ok_or_else(|| anyhow::anyhow!("Empty response"))
    }

    pub async fn navigate(&mut self, url: &str) -> Result<NavigateResult> {
        let result = self.call("navigate", serde_json::json!({ "url": url })).await?;
        serde_json::from_value(result).map_err(Into::into)
    }

    pub async fn screenshot(&mut self, full_page: bool) -> Result<ScreenshotResult> {
        let result = self.call("screenshot", serde_json::json!({ "fullPage": full_page })).await?;
        serde_json::from_value(result).map_err(Into::into)
    }

    pub async fn click(&mut self, selector: &str) -> Result<()> {
        self.call("click", serde_json::json!({ "selector": selector })).await?;
        Ok(())
    }

    pub async fn fill(&mut self, selector: &str, value: &str) -> Result<()> {
        self.call("fill", serde_json::json!({ "selector": selector, "value": value })).await?;
        Ok(())
    }

    pub async fn type_text(&mut self, selector: &str, text: &str) -> Result<()> {
        self.call("type", serde_json::json!({ "selector": selector, "text": text })).await?;
        Ok(())
    }

    pub async fn extract_text(&mut self, selector: &str) -> Result<String> {
        let result = self.call("extract_text", serde_json::json!({ "selector": selector })).await?;
        result["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No text in response"))
    }

    pub async fn extract_all(&mut self, selector: &str) -> Result<Vec<ExtractedElement>> {
        let result = self.call("extract_all", serde_json::json!({ "selector": selector })).await?;
        let elements = result["elements"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No elements in response"))?;
        elements.iter().map(|v| serde_json::from_value(v.clone()).map_err(Into::into)).collect()
    }

    pub async fn get_content(&mut self) -> Result<String> {
        let result = self.call("get_content", serde_json::json!({})).await?;
        result["html"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No html in response"))
    }

    pub async fn wait_for(&mut self, selector: &str, timeout: Option<u32>) -> Result<()> {
        self.call(
            "wait_for",
            serde_json::json!({
                "selector": selector,
                "timeout": timeout
            }),
        )
        .await?;
        Ok(())
    }

    pub async fn select_option(&mut self, selector: &str, value: &str) -> Result<()> {
        self.call("select", serde_json::json!({ "selector": selector, "value": value })).await?;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        self.call("close", serde_json::json!({})).await?;
        Ok(())
    }

    /// 健康检查：子进程是否仍在运行。
    /// 用于浏览器池的自动重建（子进程崩溃后检测到已退出即重新启动，修复 #14）。
    pub fn is_alive(&mut self) -> bool {
        self.child.try_wait().map(|s| s.is_none()).unwrap_or(false)
    }

    /// 在浏览器上下文中执行任意 JS 代码并返回序列化结果。
    /// `arg` 作为 `page.evaluate(code, arg)` 的第二个参数传入（参数化，避免字符串拼接注入）。
    /// 可用于绕过 TLS 指纹限制（如 EastMoney WAF），因为 Chromium 的 TLS 指纹与真实浏览器一致。
    pub async fn evaluate(
        &mut self,
        code: &str,
        arg: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut params = serde_json::json!({ "code": code });
        if let Some(a) = arg {
            params["arg"] = a;
        }
        self.call("evaluate", params).await
    }

    /// 通过浏览器 fetch API 发送 HTTP GET 请求，绕过 TLS 指纹检测。
    /// 内部使用参数化的 page.evaluate(code, arg) 传递 url/headers，杜绝 JS 代码注入（修复 #11）。
    pub async fn fetch_json_via_browser(
        &mut self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let headers_obj: serde_json::Value = headers
            .iter()
            .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
            .collect::<serde_json::Map<_, _>>()
            .into();

        let code = r#"(async (arg) => {
            try {
                const resp = await fetch(arg.url, {
                    method: "GET",
                    headers: arg.headers,
                    credentials: "omit",
                });
                const text = await resp.text();
                return { ok: resp.ok, status: resp.status, body: text };
            } catch (e) {
                return { ok: false, status: 0, error: e.message };
            }
        })()"#;

        self.evaluate(code, Some(serde_json::json!({ "url": url, "headers": headers_obj }))).await
    }

    /// 通过页面导航发送 HTTP GET 请求（绕过 CORS 和 TLS 指纹限制）
    /// 内部使用 page.goto() 直接导航到目标 URL，提取 body 纯文本
    /// CORS 不适用于页面导航，因此可绕过 EastMoney 等不设 CORS 头的 API
    /// 返回 { body, navigatedUrl, pageTitle, contentType } 的 JSON 结构
    pub async fn http_get_via_browser(&mut self, url: &str) -> Result<serde_json::Value> {
        self.call("http_get", serde_json::json!({ "url": url })).await
    }

    /// 通过当前页面的 fetch() 获取 JSON 内容（不导航，保持 cookies 有效）
    pub async fn http_get_via_fetch(&mut self, url: &str) -> Result<serde_json::Value> {
        self.call("http_json", serde_json::json!({ "url": url })).await
    }
}

#[cfg(not(target_os = "android"))]
impl Drop for PlaywrightClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

#[cfg(not(target_os = "android"))]
static SHARED_BROWSER: OnceLock<Arc<Mutex<Option<PlaywrightClient>>>> = OnceLock::new();

#[cfg(not(target_os = "android"))]
pub fn shared_browser_pool() -> &'static Arc<Mutex<Option<PlaywrightClient>>> {
    SHARED_BROWSER.get_or_init(|| Arc::new(Mutex::new(None)))
}

#[cfg(not(target_os = "android"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct NavigateResult {
    pub url: String,
    pub title: String,
}

#[cfg(not(target_os = "android"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub image_base64: String,
}

#[cfg(not(target_os = "android"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedElement {
    pub tag: String,
    pub text: Option<String>,
    pub href: Option<String>,
    #[serde(rename = "type")]
    pub input_type: Option<String>,
    pub placeholder: Option<String>,
}

#[cfg(target_os = "android")]
pub struct PlaywrightClient;

#[cfg(target_os = "android")]
impl PlaywrightClient {
    pub async fn launch() -> anyhow::Result<Self> {
        anyhow::bail!(android_msg::BROWSER_NOT_AVAILABLE)
    }
}

#[cfg(target_os = "android")]
pub fn shared_browser_pool() -> Option<&'static PlaywrightClient> {
    None
}

/// 通过共享浏览器池发送 HTTP GET 请求并返回 JSON
/// 自动获取/启动共享浏览器实例，执行 fetch 后返回响应体 JSON
/// 用于绕过 TLS 指纹封锁（如 EastMoney WAF 的 JA3 检测）
#[cfg(not(target_os = "android"))]
pub async fn browser_http_get_json(
    url: &str,
    headers: &[(&str, &str)],
) -> anyhow::Result<serde_json::Value> {
    let pool = shared_browser_pool();
    let mut guard = pool.lock().await;
    if guard.is_none() {
        *guard = Some(PlaywrightClient::launch().await?);
    }
    let client = guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("browser pool not initialized after launch"))?;
    client.fetch_json_via_browser(url, headers).await
}

/// 通过共享浏览器池发送 HTTP GET 请求并返回纯文本（绕过 CORS）
/// 使用 page.goto() 导航而非 fetch()，避免 CORS 限制
/// 返回 { body, navigatedUrl, pageTitle, contentType }
#[cfg(not(target_os = "android"))]
pub async fn browser_http_get_text(url: &str) -> anyhow::Result<serde_json::Value> {
    let pool = shared_browser_pool();
    let mut guard = pool.lock().await;
    if guard.is_none() {
        *guard = Some(PlaywrightClient::launch().await?);
    }
    let client = guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("browser pool not initialized after launch"))?;
    client.http_get_via_browser(url).await
}

/// 获取浏览器池，通过当前页面的 fetch() 获取 JSON
#[cfg(not(target_os = "android"))]
pub async fn browser_http_get_json_page(url: &str) -> anyhow::Result<serde_json::Value> {
    let pool = shared_browser_pool();
    let mut guard = pool.lock().await;
    if guard.is_none() {
        *guard = Some(PlaywrightClient::launch().await?);
    }
    let client = guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("browser pool not initialized after launch"))?;
    client.http_get_via_fetch(url).await
}
