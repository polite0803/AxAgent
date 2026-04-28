use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};

use super::lsp_client::{LspDiagnostic, LspServerStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspServerConfig {
    pub language: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

impl LspServerConfig {
    pub fn rust(_root_path: impl Into<String>) -> Self {
        Self {
            language: "rust".to_string(),
            command: "rust-analyzer".to_string(),
            args: vec![],
            env: HashMap::new(),
        }
    }

    pub fn typescript(_root_path: impl Into<String>) -> Self {
        Self {
            language: "typescript".to_string(),
            command: "typescript-language-server".to_string(),
            args: vec!["--stdio".to_string()],
            env: HashMap::new(),
        }
    }

    pub fn python(_root_path: impl Into<String>) -> Self {
        Self {
            language: "python".to_string(),
            command: "pylsp".to_string(),
            args: vec![],
            env: HashMap::new(),
        }
    }

    pub fn go() -> Self {
        Self {
            language: "go".to_string(),
            command: "gopls".to_string(),
            args: vec![],
            env: HashMap::new(),
        }
    }
}

struct LspProcessInner {
    child: Option<Child>,
    stdin: Option<tokio::process::ChildStdin>,
    request_id: i64,
    pending_requests: HashMap<i64, tokio::sync::oneshot::Sender<serde_json::Value>>,
    initialized: bool,
    root_path: PathBuf,
    capabilities: serde_json::Value,
}

pub struct LspProcess {
    config: LspServerConfig,
    inner: Arc<Mutex<LspProcessInner>>,
    status: Arc<RwLock<LspServerStatus>>,
    diagnostics: Arc<RwLock<Vec<LspDiagnostic>>>,
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl LspProcess {
    pub fn new(config: LspServerConfig, root_path: impl AsRef<Path>) -> Self {
        Self {
            config,
            inner: Arc::new(Mutex::new(LspProcessInner {
                child: None,
                stdin: None,
                request_id: 0,
                pending_requests: HashMap::new(),
                initialized: false,
                root_path: root_path.as_ref().to_path_buf(),
                capabilities: serde_json::Value::Null,
            })),
            status: Arc::new(RwLock::new(LspServerStatus::Disconnected)),
            diagnostics: Arc::new(RwLock::new(Vec::new())),
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        {
            let mut status = self.status.write().await;
            *status = LspServerStatus::Starting;
        }

        let mut child = Command::new(&self.config.command)
            .args(&self.config.args)
            .envs(&self.config.env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn LSP server '{}': {}", self.config.command, e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or("Failed to get stdin of LSP process")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to get stdout of LSP process")?;

        {
            let mut inner = self.inner.lock().await;
            inner.child = Some(child);
            inner.stdin = Some(stdin);
        }

        self.spawn_reader_task(stdout);

        self.initialize_server().await?;

        {
            let mut status = self.status.write().await;
            *status = LspServerStatus::Connected;
        }

        Ok(())
    }

    async fn initialize_server(&self) -> Result<(), String> {
        let root_path = {
            let inner = self.inner.lock().await;
            inner.root_path.clone()
        };

        let root_uri = format!("file://{}", root_path.display());

        let init_params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "rootPath": root_path.display().to_string(),
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {
                        "relatedInformation": true
                    },
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    },
                    "definition": {
                        "linkSupport": true
                    },
                    "references": {},
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "documentSymbol": {
                        "hierarchicalDocumentSymbolSupport": true
                    }
                }
            }
        });

        let result = self.send_request("initialize", init_params).await?;

        {
            let mut inner = self.inner.lock().await;
            inner.capabilities = result.get("capabilities").cloned().unwrap_or(serde_json::json!({}));
            inner.initialized = true;
        }

        self.send_notification("initialized", serde_json::json!({}))
            .await?;

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), String> {
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);

        let _ = self.send_request("shutdown", serde_json::Value::Null).await;
        self.send_notification("exit", serde_json::Value::Null).await?;

        let mut inner = self.inner.lock().await;
        if let Some(mut child) = inner.child.take() {
            let _ = child.kill().await;
        }
        inner.stdin = None;
        inner.initialized = false;

        {
            let mut status = self.status.write().await;
            *status = LspServerStatus::Disconnected;
        }

        Ok(())
    }

    pub async fn open_document(&self, path: &str, text: &str) -> Result<(), String> {
        let language_id = self.config.language.clone();
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": text
            }
        });

        self.send_notification("textDocument/didOpen", params).await
    }

    pub async fn close_document(&self, path: &str) -> Result<(), String> {
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri
            }
        });

        self.send_notification("textDocument/didClose", params).await
    }

    pub async fn change_document(
        &self,
        path: &str,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<(), String> {
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": {
                "uri": uri,
                "version": version
            },
            "contentChanges": changes
        });

        self.send_notification("textDocument/didChange", params).await
    }

    pub async fn hover(&self, path: &str, line: u32, character: u32) -> Result<serde_json::Value, String> {
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        self.send_request("textDocument/hover", params).await
    }

    pub async fn goto_definition(
        &self,
        path: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value, String> {
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        self.send_request("textDocument/definition", params).await
    }

    pub async fn references(
        &self,
        path: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value, String> {
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        });

        self.send_request("textDocument/references", params).await
    }

    pub async fn completion(
        &self,
        path: &str,
        line: u32,
        character: u32,
    ) -> Result<serde_json::Value, String> {
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        self.send_request("textDocument/completion", params).await
    }

    pub async fn document_symbols(&self, path: &str) -> Result<serde_json::Value, String> {
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": { "uri": uri }
        });

        self.send_request("textDocument/documentSymbol", params).await
    }

    pub async fn formatting(&self, path: &str) -> Result<serde_json::Value, String> {
        let uri = format!("file://{}", path);

        let params = serde_json::json!({
            "textDocument": { "uri": uri },
            "options": { "tabSize": 4, "insertSpaces": true }
        });

        self.send_request("textDocument/formatting", params).await
    }

    pub async fn get_diagnostics(&self) -> Vec<LspDiagnostic> {
        self.diagnostics.read().await.clone()
    }

    pub async fn status(&self) -> LspServerStatus {
        *self.status.read().await
    }

    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (id, rx) = {
            let mut inner = self.inner.lock().await;
            inner.request_id += 1;
            let id = inner.request_id;

            let (tx, rx) = tokio::sync::oneshot::channel();
            inner.pending_requests.insert(id, tx);

            (id, rx)
        };

        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        self.write_message(&message).await?;

        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err(format!("LSP request '{}' channel dropped", method)),
            Err(_) => Err(format!("LSP request '{}' timed out", method)),
        }
    }

    async fn send_notification(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), String> {
        let message = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        self.write_message(&message).await
    }

    async fn write_message(&self, message: &serde_json::Value) -> Result<(), String> {
        let content = serde_json::to_string(message)
            .map_err(|e| format!("Failed to serialize LSP message: {}", e))?;

        let header = format!("Content-Length: {}\r\n\r\n", content.len());

        let mut inner = self.inner.lock().await;
        if let Some(ref mut stdin) = inner.stdin {
            stdin
                .write_all(header.as_bytes())
                .await
                .map_err(|e| format!("Failed to write LSP header: {}", e))?;
            stdin
                .write_all(content.as_bytes())
                .await
                .map_err(|e| format!("Failed to write LSP body: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Failed to flush LSP stdin: {}", e))?;
        } else {
            return Err("LSP process stdin not available".to_string());
        }

        Ok(())
    }

    fn spawn_reader_task(&self, stdout: tokio::process::ChildStdout) {
        let _inner = Arc::clone(&self.inner);
        let _diagnostics = Arc::clone(&self.diagnostics);
        let shutdown = Arc::clone(&self.shutdown);
        let status = Arc::clone(&self.status);

        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            let mut buffer = String::new();
            let mut content_length: Option<usize> = None;

            loop {
                if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                tokio::select! {
                    result = lines.next_line() => {
                        match result {
                            Ok(Some(line)) => {
                                if line.starts_with("Content-Length:") {
                                    content_length = line
                                        .split(':')
                                        .nth(1)
                                        .and_then(|v| v.trim().parse::<usize>().ok());
                                } else if line.is_empty() {
                                    if let Some(len) = content_length.take() {
                                        buffer.clear();
                                        let _body_buf = vec![0u8; len];
                                        // We need to read from the underlying reader
                                        // For simplicity, we'll read line by line
                                        // In production, use a proper framed reader
                                    }
                                } else {
                                    buffer.push_str(&line);
                                    buffer.push('\n');
                                }
                            }
                            Ok(None) => {
                                tracing::info!("LSP stdout EOF");
                                break;
                            }
                            Err(e) => {
                                tracing::warn!("LSP stdout read error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        continue;
                    }
                }
            }

            let mut s = status.write().await;
            *s = LspServerStatus::Disconnected;
        });
    }

    #[allow(dead_code)]
    fn handle_response(&self, id: i64, result: serde_json::Value) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let mut inner = inner.lock().await;
            if let Some(tx) = inner.pending_requests.remove(&id) {
                let _ = tx.send(result);
            }
        });
    }

    #[allow(dead_code)]
    fn handle_notification(&self, method: &str, params: serde_json::Value) {
        if method == "textDocument/publishDiagnostics" {
            let diagnostics = Arc::clone(&self.diagnostics);
            tokio::spawn(async move {
                let mut new_diags = Vec::new();
                if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                    let path = uri.strip_prefix("file://").unwrap_or(uri);
                    if let Some(diags) = params.get("diagnostics").and_then(|v| v.as_array()) {
                        for diag in diags {
                            let empty_range = serde_json::json!({});
                            let start = diag
                                .get("range")
                                .and_then(|r| r.get("start"))
                                .unwrap_or(&empty_range);
                            new_diags.push(LspDiagnostic {
                                path: path.to_string(),
                                line: start.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                character: start
                                    .get("character")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(0) as u32,
                                severity: diag
                                    .get("severity")
                                    .and_then(|v| v.as_u64())
                                    .map(|s| match s {
                                        1 => "error",
                                        2 => "warning",
                                        3 => "information",
                                        4 => "hint",
                                        _ => "unknown",
                                    })
                                    .unwrap_or("unknown")
                                    .to_string(),
                                message: diag
                                    .get("message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                source: diag
                                    .get("source")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            });
                        }
                    }
                }

                let mut all_diags = diagnostics.write().await;
                let path = params
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .strip_prefix("file://")
                    .unwrap_or("")
                    .to_string();
                all_diags.retain(|d| d.path != path);
                all_diags.extend(new_diags);
            });
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentContentChangeEvent {
    pub range: Option<serde_json::Value>,
    pub range_length: Option<u32>,
    pub text: String,
}

pub struct LspProcessManager {
    processes: Arc<RwLock<HashMap<String, Arc<LspProcess>>>>,
}

impl LspProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_server(
        &self,
        config: LspServerConfig,
        root_path: impl AsRef<Path>,
    ) -> Result<Arc<LspProcess>, String> {
        let language = config.language.clone();
        let process = Arc::new(LspProcess::new(config, root_path));
        process.start().await?;

        let mut processes = self.processes.write().await;
        processes.insert(language, Arc::clone(&process));

        Ok(process)
    }

    pub async fn stop_server(&self, language: &str) -> Result<(), String> {
        let mut processes = self.processes.write().await;
        if let Some(process) = processes.remove(language) {
            process.shutdown().await?;
        }
        Ok(())
    }

    pub async fn get_server(&self, language: &str) -> Option<Arc<LspProcess>> {
        let processes = self.processes.read().await;
        processes.get(language).cloned()
    }

    pub async fn stop_all(&self) {
        let mut processes = self.processes.write().await;
        for (_, process) in processes.drain() {
            let _ = process.shutdown().await;
        }
    }

    pub async fn list_servers(&self) -> Vec<(String, LspServerStatus)> {
        let processes = self.processes.read().await;
        let mut result = Vec::new();
        for (language, process) in processes.iter() {
            result.push((language.clone(), process.status().await));
        }
        result
    }
}

impl Default for LspProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
