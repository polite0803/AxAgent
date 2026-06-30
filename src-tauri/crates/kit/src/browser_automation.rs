// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(not(target_os = "android"))]
use anyhow::Result;
#[cfg(not(target_os = "android"))]
use serde::{Deserialize, Serialize};
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

#[cfg(not(target_os = "android"))]
impl PlaywrightClient {
    pub async fn launch() -> Result<Self> {
        let script_path = std::env::current_exe()?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot find exe directory"))?
            .join("scripts")
            .join("browser-automation.mjs");

        let mut child = Command::new("node")
            .arg(&script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("No stdout"))?;
        let stdout_reader = BufReader::new(stdout);

        let mut client = Self {
            child,
            stdin,
            stdout_reader,
            next_id: 1,
        };

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

        let request = BrowserRequest {
            id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)? + "\n";
        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.flush().await?;

        let mut response_line = String::new();
        self.stdout_reader.read_line(&mut response_line).await?;
        let response: BrowserResponse = serde_json::from_str(response_line.trim())?;

        if let Some(error) = response.error {
            anyhow::bail!("Browser automation error: {}", error);
        }

        response
            .result
            .ok_or_else(|| anyhow::anyhow!("Empty response"))
    }

    pub async fn navigate(&mut self, url: &str) -> Result<NavigateResult> {
        let result = self
            .call("navigate", serde_json::json!({ "url": url }))
            .await?;
        serde_json::from_value(result).map_err(Into::into)
    }

    pub async fn screenshot(&mut self, full_page: bool) -> Result<ScreenshotResult> {
        let result = self
            .call("screenshot", serde_json::json!({ "fullPage": full_page }))
            .await?;
        serde_json::from_value(result).map_err(Into::into)
    }

    pub async fn click(&mut self, selector: &str) -> Result<()> {
        self.call("click", serde_json::json!({ "selector": selector }))
            .await?;
        Ok(())
    }

    pub async fn fill(&mut self, selector: &str, value: &str) -> Result<()> {
        self.call("fill", serde_json::json!({ "selector": selector, "value": value }))
            .await?;
        Ok(())
    }

    pub async fn type_text(&mut self, selector: &str, text: &str) -> Result<()> {
        self.call("type", serde_json::json!({ "selector": selector, "text": text }))
            .await?;
        Ok(())
    }

    pub async fn extract_text(&mut self, selector: &str) -> Result<String> {
        let result = self
            .call("extract_text", serde_json::json!({ "selector": selector }))
            .await?;
        result["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No text in response"))
    }

    pub async fn extract_all(&mut self, selector: &str) -> Result<Vec<ExtractedElement>> {
        let result = self
            .call("extract_all", serde_json::json!({ "selector": selector }))
            .await?;
        let elements = result["elements"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No elements in response"))?;
        elements
            .iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
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
        self.call("select", serde_json::json!({ "selector": selector, "value": value }))
            .await?;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        self.call("close", serde_json::json!({})).await?;
        Ok(())
    }

    /// 在浏览器上下文中执行任意 JS 代码并返回序列化结果
    /// 可用于绕过 TLS 指纹限制（如 EastMoney WAF），因为 Chromium 的 TLS 指纹与真实浏览器一致
    pub async fn evaluate(&mut self, code: &str) -> Result<serde_json::Value> {
        self.call("evaluate", serde_json::json!({ "code": code }))
            .await
    }

    /// 通过浏览器 fetch API 发送 HTTP GET 请求，绕过 TLS 指纹检测
    /// 内部使用 page.evaluate() 在 Chromium 上下文中执行 fetch，返回 JSON 响应
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

        let code = format!(
            r#"(async () => {{
                try {{
                    const resp = await fetch("{}", {{
                        method: "GET",
                        headers: {},
                        credentials: "omit",
                    }});
                    const text = await resp.text();
                    return {{ ok: resp.ok, status: resp.status, body: text }};
                }} catch (e) {{
                    return {{ ok: false, status: 0, error: e.message }};
                }}
            }})()"#,
            url, headers_obj
        );

        let raw = self.evaluate(&code).await?;
        Ok(raw)
    }

    /// 通过页面导航发送 HTTP GET 请求（绕过 CORS 和 TLS 指纹限制）
    /// 内部使用 page.goto() 直接导航到目标 URL，提取 body 纯文本
    /// CORS 不适用于页面导航，因此可绕过 EastMoney 等不设 CORS 头的 API
    /// 返回 { body, navigatedUrl, pageTitle, contentType } 的 JSON 结构
    pub async fn http_get_via_browser(&mut self, url: &str) -> Result<serde_json::Value> {
        self.call("http_get", serde_json::json!({ "url": url }))
            .await
    }

    /// 通过当前页面的 fetch() 获取 JSON 内容（不导航，保持 cookies 有效）
    pub async fn http_get_via_fetch(&mut self, url: &str) -> Result<serde_json::Value> {
        self.call("http_json", serde_json::json!({ "url": url }))
            .await
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
    let client = guard.as_mut().unwrap();
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
    let client = guard.as_mut().unwrap();
    client.http_get_via_browser(url).await
}

/// 获取浏览器池，通过当前页面的 fetch() 获取 JSON
pub async fn browser_http_get_json_page(url: &str) -> anyhow::Result<serde_json::Value> {
    let pool = shared_browser_pool();
    let mut guard = pool.lock().await;
    if guard.is_none() {
        *guard = Some(PlaywrightClient::launch().await?);
    }
    let client = guard.as_mut().unwrap();
    client.http_get_via_fetch(url).await
}
