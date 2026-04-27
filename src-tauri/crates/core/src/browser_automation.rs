use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug, Serialize, Deserialize)]
struct BrowserRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct BrowserResponse {
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

pub struct PlaywrightClient {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

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
        let response: BrowserResponse = serde_json::from_str(&response_line.trim())?;

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
        self.call(
            "fill",
            serde_json::json!({ "selector": selector, "value": value }),
        )
        .await?;
        Ok(())
    }

    pub async fn type_text(&mut self, selector: &str, text: &str) -> Result<()> {
        self.call(
            "type",
            serde_json::json!({ "selector": selector, "text": text }),
        )
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
        self.call(
            "select",
            serde_json::json!({ "selector": selector, "value": value }),
        )
        .await?;
        Ok(())
    }

    pub async fn close(&mut self) -> Result<()> {
        self.call("close", serde_json::json!({})).await?;
        Ok(())
    }
}

impl Drop for PlaywrightClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NavigateResult {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub image_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedElement {
    pub tag: String,
    pub text: Option<String>,
    pub href: Option<String>,
    #[serde(rename = "type")]
    pub input_type: Option<String>,
    pub placeholder: Option<String>,
}
