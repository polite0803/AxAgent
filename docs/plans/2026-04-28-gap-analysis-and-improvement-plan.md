# AxAgent 与最先进智能体软件差距分析及改进方案

> 分析日期：2026-04-28
> 对比基准：Claude Code (Opus 4.6)、Cursor、GitHub Copilot、Devin、OpenAI Codex CLI、Windsurf、Bolt.new
> 数据来源：项目代码库实际审查 + 竞品公开资料（2026-04）
> 用途：指导后续编码实现的优先级排序和方案设计

---

## 一、核心 AI 能力差距

### 差距 1：缺乏原生代码编辑与 LSP 深度集成 [P0]

#### 现状

`src-tauri/crates/runtime/src/lsp_client.rs` 仅实现了 LspRegistry（注册表）和类型定义（LspAction、LspDiagnostic、LspHoverResult 等），**实际并未启动任何 LSP 服务器进程**。`dispatch_action` 方法返回的是占位符：

```rust
// lsp_client.rs 第 273 行附近
Ok(serde_json::json!({
    "action": action,
    "path": path,
    "language": server.language,
    "status": "dispatched",
    "message": format!("LSP {} dispatched to {} server", action, server.language)
}))
```

这意味着 LSP 功能目前**完全不可用**，仅具备数据结构和注册接口。

#### 竞品做法

| 竞品 | 能力 | 基准 |
|------|------|------|
| Claude Code | 精确代码搜索/替换、结构化编辑、git-aware 操作 | SWE-bench 80.8% |
| Cursor | 基于 LSP 的实时诊断、符号跳转、内联编辑、Diff 视图、Tab 补全 | 深度 LSP 集成 |
| GitHub Copilot | 内联补全、多行生成、测试生成 | VSCode 原生集成 |

#### 改进方案

**Step 1：实现 LSP 进程管理器**

新建 `src-tauri/crates/runtime/src/lsp_process.rs`：

```rust
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

pub struct LspProcessManager {
    processes: Arc<RwLock<HashMap<String, LspProcess>>>,
}

struct LspProcess {
    child: Child,
    language: String,
    root_path: String,
    status: LspServerStatus,
}

impl LspProcessManager {
    pub async fn start_server(
        &self,
        language: &str,
        root_path: &str,
    ) -> Result<(), String> {
        let (cmd, args) = match language {
            "rust" => ("rust-analyzer", vec![]),
            "typescript" | "javascript" => (
                "typescript-language-server",
                vec!["--stdio"],
            ),
            "python" => ("pyright-langserver", vec!["--stdio"]),
            "go" => ("gopls", vec![]),
            _ => return Err(format!("Unsupported language: {language}")),
        };

        let child = Command::new(cmd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(root_path)
            .spawn()
            .map_err(|e| format!("Failed to start {cmd}: {e}"))?;

        let mut processes = self.processes.write().await;
        processes.insert(
            language.to_owned(),
            LspProcess {
                child,
                language: language.to_owned(),
                root_path: root_path.to_owned(),
                status: LspServerStatus::Starting,
            },
        );

        Ok(())
    }

    pub async fn stop_server(&self, language: &str) -> Result<(), String> {
        let mut processes = self.processes.write().await;
        if let Some(mut proc) = processes.remove(language) {
            proc.child.kill().await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn stop_all(&self) {
        let mut processes = self.processes.write().await;
        for (_, mut proc) in processes.drain() {
            let _ = proc.child.kill().await;
        }
    }
}
```

**Step 2：实现 LSP JSON-RPC 通信**

新建 `src-tauri/crates/runtime/src/lsp_protocol.rs`：

```rust
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;

pub struct LspClient {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    request_id: u64,
}

impl LspClient {
    pub async fn initialize(&mut self, root_path: &str) -> Result<Value, String> {
        self.send_request("initialize", json!({
            "processId": std::process::id(),
            "rootUri": format!("file://{}", root_path),
            "capabilities": {
                "textDocument": {
                    "completion": { "completionItem": { "snippetSupport": true } },
                    "hover": { "contentFormat": ["markdown", "plaintext"] },
                    "definition": { "linkSupport": true },
                    "publishDiagnostics": { "relatedInformation": true }
                }
            }
        })).await
    }

    pub async fn did_open(
        &mut self,
        uri: &str,
        language_id: &str,
        version: i32,
        text: &str,
    ) -> Result<(), String> {
        self.send_notification("textDocument/didOpen", json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": version,
                "text": text,
            }
        })).await
    }

    pub async fn completion(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        self.send_request("textDocument/completion", json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        })).await
    }

    pub async fn hover(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        self.send_request("textDocument/hover", json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        })).await
    }

    pub async fn goto_definition(
        &mut self,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Result<Value, String> {
        self.send_request("textDocument/definition", json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        })).await
    }

    async fn send_request(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        self.request_id += 1;
        let id = self.request_id;
        let message = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.write_message(&message).await?;
        let response = self.read_response(id).await?;
        Ok(response)
    }

    async fn send_notification(
        &mut self,
        method: &str,
        params: Value,
    ) -> Result<(), String> {
        let message = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.write_message(&message).await
    }

    async fn write_message(&mut self, message: &Value) -> Result<(), String> {
        let content = serde_json::to_string(message).map_err(|e| e.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        self.stdin
            .write_all(header.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        self.stdin
            .write_all(content.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        self.stdin.flush().await.map_err(|e| e.to_string())
    }

    async fn read_response(&mut self, expected_id: u64) -> Result<Value, String> {
        // Read Content-Length header
        let mut header_line = String::new();
        self.stdout
            .read_line(&mut header_line)
            .await
            .map_err(|e| e.to_string())?;

        let content_length: usize = header_line
            .strip_prefix("Content-Length: ")
            .and_then(|s| s.trim().parse().ok())
            .ok_or("Invalid LSP header")?;

        // Read empty line
        let mut empty = String::new();
        self.stdout
            .read_line(&mut empty)
            .await
            .map_err(|e| e.to_string())?;

        // Read body
        let mut body = vec![0u8; content_length];
        self.stdout
            .read_exact(&mut body)
            .await
            .map_err(|e| e.to_string())?;

        let response: Value =
            serde_json::from_slice(&body).map_err(|e| e.to_string())?;

        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }
}
```

**Step 3：前端 Monaco Editor LSP 集成**

修改 `src/components/chat/ArtifactPanel.tsx`，添加 LSP 功能：

```typescript
// 新增 LSP 相关接口
interface LspDiagnostics {
  uri: string;
  diagnostics: Array<{
    range: { start: Position; end: Position };
    severity: number;
    message: string;
    source?: string;
  }>;
}

// 在 Monaco Editor 中注册诊断信息
function applyLspDiagnostics(
  editor: monaco.editor.IStandaloneCodeEditor,
  diagnostics: LspDiagnostics
) {
  const model = editor.getModel();
  if (!model) return;

  const markers: monaco.editor.IMarkerData[] = diagnostics.diagnostics.map(
    (d) => ({
      severity: mapSeverity(d.severity),
      message: d.message,
      startLineNumber: d.range.start.line + 1,
      startColumn: d.range.start.character + 1,
      endLineNumber: d.range.end.line + 1,
      endColumn: d.range.end.character + 1,
      source: d.source,
    })
  );

  monaco.editor.setModelMarkers(model, "lsp", markers);
}
```

**Step 4：实现精确代码搜索/替换**

在 Agent 工具中添加 `search_replace` 工具（类似 Claude Code）：

```rust
// 新增工具定义到 builtin_tools_registry.rs
ToolDefinition {
    name: "search_replace".into(),
    description: "Search for exact text in a file and replace it. Requires exact match of old_str.".into(),
    input_schema: json!({
        "type": "object",
        "properties": {
            "path": { "type": "string", "description": "File path" },
            "old_str": { "type": "string", "description": "Exact text to search for" },
            "new_str": { "type": "string", "description": "Replacement text" },
        },
        "required": ["path", "old_str", "new_str"]
    }),
}
```

#### 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 新增 | `src-tauri/crates/runtime/src/lsp_process.rs` |
| 新增 | `src-tauri/crates/runtime/src/lsp_protocol.rs` |
| 修改 | `src-tauri/crates/runtime/src/lsp_client.rs` |
| 修改 | `src-tauri/crates/runtime/src/lib.rs` |
| 修改 | `src/components/chat/ArtifactPanel.tsx` |
| 修改 | `src-tauri/crates/core/src/builtin_tools_registry.rs` |
| 修改 | `src-tauri/crates/core/src/builtin_tools.rs` |

#### 验收标准

- [ ] LSP 服务器可自动启动/停止（至少支持 Rust、TypeScript、Python）
- [ ] 代码补全、hover、跳转定义功能可用
- [ ] 诊断信息实时推送到前端 Monaco Editor
- [ ] `search_replace` 工具可精确替换文件内容
- [ ] `cargo check` 和 `npm run typecheck` 通过

---

### 差距 2：多模态深度理解不足 [P2]

#### 现状

`src-tauri/crates/core/src/computer_control.rs` 可截屏，但缺乏视觉理解管道。图片生成（Flux + DallE）已实现，但**图像理解**能力薄弱。

#### 竞品做法

| 竞品 | 能力 |
|------|------|
| Devin | UI 截图理解 + 浏览器视觉导航 |
| Claude Code | Claude Vision 集成，图像/截图理解 |
| Gemini | 原生多模态（图像、视频、音频） |

#### 改进方案

**Step 1：实现视觉理解管道**

新建 `src-tauri/crates/agent/src/vision_pipeline.rs`：

```rust
use crate::provider_adapter::ProviderAdapter;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisionTask {
    ImageDescription,
    Ocr,
    UiElementDetection,
    ChartAnalysis,
    CodeScreenshotReading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    pub element_type: String,
    pub label: Option<String>,
    pub bounding_box: BoundingBox,
    pub actionable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionResult {
    pub task: VisionTask,
    pub description: String,
    pub elements: Vec<UiElement>,
    pub text_content: Option<String>,
    pub confidence: f32,
}

pub struct VisionPipeline {
    adapter: Arc<dyn ProviderAdapter>,
}

impl VisionPipeline {
    pub async fn analyze(
        &self,
        image_data: &[u8],
        task: VisionTask,
    ) -> Result<VisionResult, String> {
        let base64_image = base64::encode(image_data);
        let prompt = match task {
            VisionTask::ImageDescription => "Describe this image in detail.",
            VisionTask::Ocr => "Extract all text from this image. Output only the text.",
            VisionTask::UiElementDetection => {
                "Analyze this UI screenshot. List all interactive elements \
                 (buttons, inputs, links, menus) with their labels and positions."
            }
            VisionTask::ChartAnalysis => {
                "Analyze this chart/graph. Extract the data points, labels, \
                 and key insights."
            }
            VisionTask::CodeScreenshotReading => {
                "Read the code in this screenshot. Output the code as plain text."
            }
        };

        let response = self
            .adapter
            .vision_request(base64_image, prompt)
            .await
            .map_err(|e| e.to_string())?;

        Ok(VisionResult {
            task,
            description: response,
            elements: vec![],
            text_content: None,
            confidence: 0.0,
        })
    }
}
```

**Step 2：前端组件**

| 组件 | 功能 |
|------|------|
| `ImageAnalysisPanel.tsx` | 图像分析结果展示 |
| `ChartInterpreter.tsx` | 图表解读 |
| `UISnapshotViewer.tsx` | UI 截图标注 |

#### 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 新增 | `src-tauri/crates/agent/src/vision_pipeline.rs` |
| 修改 | `src-tauri/crates/agent/src/lib.rs` |
| 新增 | `src/components/chat/ImageAnalysisPanel.tsx` |
| 新增 | `src/components/chat/ChartInterpreter.tsx` |
| 新增 | `src/components/chat/UISnapshotViewer.tsx` |

---

### 差距 3：持续学习与个性化适应薄弱 [P2]

#### 现状

轨迹学习（`src-tauri/crates/trajectory/`）和 RL 优化器（`src-tauri/crates/agent/src/rl_optimizer/`）已实现，但缺乏从用户行为持续学习的闭环。

#### 竞品做法

| 竞品 | 能力 |
|------|------|
| Claude Code | 从用户编辑行为学习编码风格，CLAUDE.md 项目记忆 |
| Cursor | 学习用户的代码补全接受模式 |
| Copilot | 基于用户代码库的上下文感知 |

#### 改进方案

**Step 1：实现用户行为学习管道**

新建 `src-tauri/crates/trajectory/src/behavior_learner.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBehavior {
    pub action_type: String,
    pub context: String,
    pub accepted: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleProfile {
    pub user_id: String,
    pub coding_style: CodingStyle,
    pub preferred_patterns: Vec<String>,
    pub rejected_patterns: Vec<String>,
    pub common_apis: Vec<String>,
    pub naming_convention: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingStyle {
    pub indent_style: String,
    pub quote_style: String,
    pub semicolons: bool,
    pub max_line_length: usize,
    pub function_style: String,
}

pub struct BehaviorLearner {
    behaviors: Vec<UserBehavior>,
    style_profile: Option<StyleProfile>,
}

impl BehaviorLearner {
    pub fn new() -> Self {
        Self {
            behaviors: Vec::new(),
            style_profile: None,
        }
    }

    pub fn record_behavior(&mut self, behavior: UserBehavior) {
        self.behaviors.push(behavior);
        if self.behaviors.len() % 100 == 0 {
            self.recompute_profile();
        }
    }

    fn recompute_profile(&mut self) {
        // Analyze accepted vs rejected patterns
        let accepted: Vec<_> = self
            .behaviors
            .iter()
            .filter(|b| b.accepted)
            .collect();
        let rejected: Vec<_> = self
            .behaviors
            .iter()
            .filter(|b| !b.accepted)
            .collect();

        // Extract patterns from accepted behaviors
        let preferred_patterns = extract_patterns(&accepted);
        let rejected_patterns = extract_patterns(&rejected);

        self.style_profile = Some(StyleProfile {
            user_id: "default".into(),
            coding_style: CodingStyle::default(),
            preferred_patterns,
            rejected_patterns,
            common_apis: vec![],
            naming_convention: "camelCase".into(),
            updated_at: chrono::Utc::now().timestamp(),
        });
    }

    pub fn get_style_hints(&self) -> Vec<String> {
        self.style_profile
            .as_ref()
            .map(|p| {
                let mut hints = vec![];
                if !p.preferred_patterns.is_empty() {
                    hints.push(format!(
                        "User prefers these patterns: {}",
                        p.preferred_patterns.join(", ")
                    ));
                }
                if !p.rejected_patterns.is_empty() {
                    hints.push(format!(
                        "User tends to reject these patterns: {}",
                        p.rejected_patterns.join(", ")
                    ));
                }
                hints
            })
            .unwrap_or_default()
    }
}
```

**Step 2：项目级记忆（类似 CLAUDE.md）**

新建 `src-tauri/crates/agent/src/project_memory.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemory {
    pub project_path: String,
    pub conventions: Vec<String>,
    pub architecture_notes: Vec<String>,
    pub common_commands: Vec<String>,
    pub tech_stack: Vec<String>,
    pub user_preferences: Vec<String>,
}

impl ProjectMemory {
    const MEMORY_FILE: &'static str = ".axagent/memory.md";

    pub async fn load(project_path: &str) -> Result<Option<Self>, String> {
        let memory_path = PathBuf::from(project_path)
            .join(Self::MEMORY_FILE);
        if !memory_path.exists() {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&memory_path)
            .await
            .map_err(|e| e.to_string())?;
        // Parse markdown sections into structured memory
        Ok(Some(Self::parse_from_markdown(&content, project_path)))
    }

    pub async fn save(&self) -> Result<(), String> {
        let memory_path = PathBuf::from(&self.project_path)
            .join(Self::MEMORY_FILE);
        if let Some(parent) = memory_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        let content = self.to_markdown();
        tokio::fs::write(&memory_path, content)
            .await
            .map_err(|e| e.to_string())
    }

    fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Project Memory\n\n");

        if !self.tech_stack.is_empty() {
            md.push_str("## Tech Stack\n");
            for item in &self.tech_stack {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }

        if !self.conventions.is_empty() {
            md.push_str("## Conventions\n");
            for item in &self.conventions {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }

        if !self.common_commands.is_empty() {
            md.push_str("## Common Commands\n");
            for item in &self.common_commands {
                md.push_str(&format!("- {}\n", item));
            }
            md.push('\n');
        }

        md
    }

    fn parse_from_markdown(content: &str, project_path: &str) -> Self {
        // Simple section-based parsing
        let mut memory = Self {
            project_path: project_path.into(),
            conventions: vec![],
            architecture_notes: vec![],
            common_commands: vec![],
            tech_stack: vec![],
            user_preferences: vec![],
        };

        let mut current_section = "";
        for line in content.lines() {
            if line.starts_with("## ") {
                current_section = line.trim_start_matches("## ").trim();
            } else if line.starts_with("- ") {
                let item = line.trim_start_matches("- ").trim().to_string();
                match current_section {
                    "Tech Stack" => memory.tech_stack.push(item),
                    "Conventions" => memory.conventions.push(item),
                    "Common Commands" => memory.common_commands.push(item),
                    "Architecture" => memory.architecture_notes.push(item),
                    "User Preferences" => memory.user_preferences.push(item),
                    _ => {}
                }
            }
        }

        memory
    }
}
```

#### 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 新增 | `src-tauri/crates/trajectory/src/behavior_learner.rs` |
| 新增 | `src-tauri/crates/agent/src/project_memory.rs` |
| 修改 | `src-tauri/crates/agent/src/lib.rs` |
| 修改 | `src-tauri/crates/trajectory/src/lib.rs` |

---

## 二、开发体验差距

### 差距 4：终端集成不完整 [P0]

#### 现状

`src-tauri/crates/runtime/src/bash.rs` 有 bash 执行能力，但缺乏完整的终端模拟器体验。前端无 xterm.js 集成。

#### 竞品做法

| 竞品 | 能力 |
|------|------|
| Claude Code | 终端原生，命令建议、输出分析，Terminal-Bench 77.3% |
| Cursor | 终端命令生成、错误自动修复 |
| Windsurf | 终端感知的代码生成 |

#### 改进方案

**Step 1：前端集成 xterm.js**

新建 `src/components/terminal/IntegratedTerminal.tsx`：

```typescript
import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@/lib/invoke";
import "@xterm/xterm/css/xterm.css";

interface TerminalInstance {
  id: string;
  cwd: string;
  shell: string;
}

export function IntegratedTerminal({ instance }: { instance: TerminalInstance }) {
  const termRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);

  useEffect(() => {
    if (!termRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "'JetBrains Mono', 'Fira Code', monospace",
      theme: {
        background: "#1e1e2e",
        foreground: "#cdd6f4",
        cursor: "#f5e0dc",
      },
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();

    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.open(termRef.current);
    fitAddon.fit();

    term.onData(async (data) => {
      await invoke("terminal_write", {
        instanceId: instance.id,
        data,
      });
    });

    const unlisten = listen<string>(
      `terminal-output-${instance.id}`,
      (event) => {
        term.write(event.payload);
      }
    );

    xtermRef.current = term;

    return () => {
      unlisten.then((fn) => fn());
      term.dispose();
    };
  }, [instance.id]);

  return <div ref={termRef} style={{ height: "100%", width: "100%" }} />;
}
```

**Step 2：后端 PTY 管理**

新建 `src-tauri/crates/runtime/src/pty.rs`：

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

pub struct PtyManager {
    sessions: Arc<RwLock<HashMap<String, PtySession>>>,
}

struct PtySession {
    master: portable_pty::MasterPty,
    cwd: String,
    shell: String,
}

impl PtyManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        id: &str,
        cwd: &str,
    ) -> Result<String, String> {
        let pty_system = portable_pty::native_pty_system();
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());

        let pair = pty_system
            .openpty(portable_pty::PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let _child = pair
            .slave
            .spawn_command(
                portable_pty::CommandBuilder::new(&shell).cwd(cwd),
            )
            .map_err(|e| e.to_string())?;

        let mut sessions = self.sessions.write().await;
        sessions.insert(
            id.to_owned(),
            PtySession {
                master: pair.master,
                cwd: cwd.to_owned(),
                shell: shell.clone(),
            },
        );

        Ok(shell)
    }

    pub async fn write(&self, id: &str, data: &str) -> Result<(), String> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| format!("Session not found: {id}"))?;
        session
            .master
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())
    }

    pub async fn resize(
        &self,
        id: &str,
        rows: u16,
        cols: u16,
    ) -> Result<(), String> {
        let sessions = self.sessions.read().await;
        let session = sessions
            .get(id)
            .ok_or_else(|| format!("Session not found: {id}"))?;
        session
            .master
            .resize(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())
    }

    pub async fn kill_session(&self, id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        sessions
            .remove(id)
            .ok_or_else(|| format!("Session not found: {id}"))?;
        Ok(())
    }
}
```

**Step 3：终端输出分析**

新建 `src-tauri/crates/runtime/src/terminal_analyzer.rs`：

```rust
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalIssue {
    pub severity: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub suggestion: Option<String>,
}

pub struct TerminalOutputAnalyzer;

impl TerminalOutputAnalyzer {
    pub fn analyze(output: &str) -> Vec<TerminalIssue> {
        let mut issues = Vec::new();

        // Rust compiler errors
        let rust_error = Regex::new(r"error\[(E\d+)\]: (.*)").unwrap();
        let rust_location = Regex::new(r"  --> (.*):(\d+):(\d+)").unwrap();

        // TypeScript errors
        let ts_error = Regex::new(r"(.*\.tsx?):(\d+):(\d+) - error TS(\d+): (.*)")
            .unwrap();

        // Python tracebacks
        let py_error =
            Regex::new(r"  File \"(.*)\", line (\d+)").unwrap();

        // Generic patterns
        let warning = Regex::new(r"warning: (.*)").unwrap();
        let npm_error = Regex::new(r"npm ERR! (.*)").unwrap();

        for line in output.lines() {
            if let Some(caps) = rust_error.captures(line) {
                issues.push(TerminalIssue {
                    severity: "error".into(),
                    message: caps[2].into(),
                    file: None,
                    line: None,
                    suggestion: None,
                });
            } else if let Some(caps) = rust_location.captures(line) {
                if let Some(last) = issues.last_mut() {
                    last.file = Some(caps[1].into());
                    last.line = Some(caps[2].parse().unwrap_or(0));
                }
            } else if let Some(caps) = ts_error.captures(line) {
                issues.push(TerminalIssue {
                    severity: "error".into(),
                    message: format!("TS{}: {}", &caps[4], &caps[5]),
                    file: Some(caps[1].into()),
                    line: Some(caps[2].parse().unwrap_or(0)),
                    suggestion: None,
                });
            } else if let Some(caps) = py_error.captures(line) {
                issues.push(TerminalIssue {
                    severity: "error".into(),
                    message: "Python error".into(),
                    file: Some(caps[1].into()),
                    line: Some(caps[2].parse().unwrap_or(0)),
                    suggestion: None,
                });
            } else if let Some(caps) = warning.captures(line) {
                issues.push(TerminalIssue {
                    severity: "warning".into(),
                    message: caps[1].into(),
                    file: None,
                    line: None,
                    suggestion: None,
                });
            } else if let Some(caps) = npm_error.captures(line) {
                issues.push(TerminalIssue {
                    severity: "error".into(),
                    message: caps[1].into(),
                    file: None,
                    line: None,
                    suggestion: None,
                });
            }
        }

        issues
    }
}
```

#### 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 新增 | `src/components/terminal/IntegratedTerminal.tsx` |
| 新增 | `src/components/terminal/TerminalTab.tsx` |
| 新增 | `src/components/terminal/OutputAnalyzer.tsx` |
| 新增 | `src-tauri/crates/runtime/src/pty.rs` |
| 新增 | `src-tauri/crates/runtime/src/terminal_analyzer.rs` |
| 修改 | `src-tauri/crates/runtime/src/lib.rs` |
| 修改 | `src-tauri/src/lib.rs`（注册 Tauri 命令） |
| 修改 | `package.json`（添加 @xterm/xterm 依赖） |

#### 验收标准

- [ ] 终端可在应用内打开，支持 bash/zsh/PowerShell
- [ ] 终端输出实时推送到前端
- [ ] 终端窗口可调整大小
- [ ] 编译错误可自动检测并定位到源代码
- [ ] `cargo check` 和 `npm run typecheck` 通过

---

### 差距 5：Git 深度集成缺失 [P1]

#### 现状

`src-tauri/crates/runtime/src/git_context.rs` 仅提供基本 git 上下文信息（分支、diff 等）。

#### 竞品做法

| 竞品 | 能力 |
|------|------|
| Claude Code | 自动 commit、PR 描述生成、代码审查 |
| Cursor | 内联 blame、分支管理、冲突解决 |
| Codex | 本地与 cloud 权限/治理分层 |

#### 改进方案

**Step 1：增强 Git 工具集**

新建 `src-tauri/crates/runtime/src/git_tools.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffSummary {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub file_diffs: Vec<FileDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub status: String,
    pub insertions: usize,
    pub deletions: usize,
    pub hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub content: String,
}

pub struct GitTools;

impl GitTools {
    pub async fn generate_commit_message(
        repo_path: &str,
    ) -> Result<String, String> {
        let diff = Self::get_staged_diff(repo_path)?;
        // Will be called by LLM to generate commit message
        // Here we return the diff summary for context
        Ok(format!(
            "Based on the following changes, generate a concise commit message:\n\n{}",
            diff
        ))
    }

    pub async fn generate_pr_description(
        repo_path: &str,
        base_branch: &str,
    ) -> Result<String, String> {
        let diff = Self::get_branch_diff(repo_path, base_branch)?;
        let commits = Self::get_branch_commits(repo_path, base_branch)?;
        Ok(format!(
            "Based on the following changes and commits, generate a PR description:\n\nCommits:\n{}\n\nDiff summary:\n{}",
            commits.join("\n"),
            diff
        ))
    }

    pub async fn review_changes(
        repo_path: &str,
    ) -> Result<Vec<ReviewComment>, String> {
        let diff = Self::get_staged_diff(repo_path)?;
        // Will be called by LLM to review changes
        Ok(vec![])
    }

    fn get_staged_diff(repo_path: &str) -> Result<String, String> {
        let output = std::process::Command::new("git")
            .args(["diff", "--staged", "--stat"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| e.to_string())?;
        String::from_utf8(output.stdout).map_err(|e| e.to_string())
    }

    fn get_branch_diff(
        repo_path: &str,
        base_branch: &str,
    ) -> Result<String, String> {
        let output = std::process::Command::new("git")
            .args(["diff", base_branch, "--stat"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| e.to_string())?;
        String::from_utf8(output.stdout).map_err(|e| e.to_string())
    }

    fn get_branch_commits(
        repo_path: &str,
        base_branch: &str,
    ) -> Result<Vec<String>, String> {
        let output = std::process::Command::new("git")
            .args(["log", &format!("{}..HEAD", base_branch), "--oneline"])
            .current_dir(repo_path)
            .output()
            .map_err(|e| e.to_string())?;
        let stdout = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        Ok(stdout.lines().map(|l| l.to_string()).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub file: String,
    pub line: u32,
    pub severity: String,
    pub message: String,
    pub suggestion: Option<String>,
}
```

**Step 2：注册 Git 工具到 builtin_tools_registry**

```rust
// 添加到 builtin_tools_registry.rs
ToolDefinition {
    name: "git_commit".into(),
    description: "Generate a commit message based on staged changes and commit them.".into(),
    input_schema: json!({
        "type": "object",
        "properties": {
            "message": { "type": "string", "description": "Commit message" },
            "repo_path": { "type": "string", "description": "Repository path" },
        },
        "required": ["message", "repo_path"]
    }),
},
ToolDefinition {
    name: "git_review".into(),
    description: "Review staged changes and provide feedback.".into(),
    input_schema: json!({
        "type": "object",
        "properties": {
            "repo_path": { "type": "string", "description": "Repository path" },
        },
        "required": ["repo_path"]
    }),
},
```

#### 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 新增 | `src-tauri/crates/runtime/src/git_tools.rs` |
| 修改 | `src-tauri/crates/runtime/src/git_context.rs` |
| 修改 | `src-tauri/crates/runtime/src/lib.rs` |
| 修改 | `src-tauri/crates/core/src/builtin_tools_registry.rs` |
| 修改 | `src-tauri/crates/core/src/builtin_tools.rs` |

---

### 差距 6：缺乏实时协作能力 [P3]

#### 现状

单用户桌面应用，无协作能力。

#### 改进方案

基于 CRDT 的实时同步，WebSocket 协作服务器，会话共享和权限管理。此为长期目标，Phase 6 阶段实施。

---

## 三、智能体能力差距

### 差距 7：自主规划与执行能力不足 [P1]

#### 现状

`src-tauri/crates/agent/src/coordinator.rs` 使用 `Arc<std::sync::Mutex<dyn AgentImpl>>` 动态分发，架构上限制了长期自主规划。ReAct 引擎和任务分解已实现，但缺乏分层规划和自适应重规划。

#### 竞品做法

| 竞品 | 能力 |
|------|------|
| Claude Code | 规划→执行→验证完整闭环，支持子 Agent 并发执行 |
| Devin | 长期规划、子任务分解、进度追踪、自适应调整 |
| Codex | 多入口（CLI/IDE/web/app/SDK/Slack），本地与云端分层 |

#### 改进方案

**Step 1：将 `dyn AgentImpl` 重构为泛型 trait bounds**

修改 `src-tauri/crates/agent/src/coordinator.rs`：

```rust
// 当前（有问题）：
pub struct UnifiedAgentCoordinator {
    implementation: Arc<std::sync::Mutex<dyn AgentImpl>>,
    // ...
}

// 目标：
pub struct UnifiedAgentCoordinator<T: Agent> {
    implementation: Arc<T>,
    // ...
}

impl<T: Agent> UnifiedAgentCoordinator<T> {
    pub fn new(implementation: Arc<T>, event_bus: Option<Arc<AgentEventBus>>) -> Self {
        // ...
    }

    pub async fn execute(&self, input: AgentInput) -> Result<CoordinatorOutput, AgentError> {
        // 编译时类型检查，无需运行时动态分发
        self.implementation.execute(input).await
    }
}
```

**Step 2：实现分层规划器**

新建 `src-tauri/crates/agent/src/hierarchical_planner.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub goal: String,
    pub phases: Vec<Phase>,
    pub status: PlanStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub id: String,
    pub name: String,
    pub tasks: Vec<PlannedTask>,
    pub dependencies: Vec<String>,
    pub status: PhaseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedTask {
    pub id: String,
    pub description: String,
    pub action_type: String,
    pub parameters: serde_json::Value,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
    pub result: Option<serde_json::Value>,
    pub retry_count: u32,
    pub max_retries: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus { Draft, Executing, Paused, Completed, Failed }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseStatus { Pending, InProgress, Completed, Failed, Skipped }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus { Pending, InProgress, Completed, Failed, Skipped }

pub struct HierarchicalPlanner {
    current_plan: Option<Plan>,
    max_retries: u32,
}

impl HierarchicalPlanner {
    pub fn new() -> Self {
        Self {
            current_plan: None,
            max_retries: 3,
        }
    }

    pub fn create_plan(&mut self, goal: &str, phases: Vec<Phase>) -> &Plan {
        let plan = Plan {
            id: uuid::Uuid::new_v4().to_string(),
            goal: goal.into(),
            phases,
            status: PlanStatus::Draft,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
        };
        self.current_plan = Some(plan);
        self.current_plan.as_ref().unwrap()
    }

    pub fn get_next_executable_tasks(&self) -> Vec<&PlannedTask> {
        let plan = match &self.current_plan {
            Some(p) => p,
            None => return vec![],
        };

        let mut executable = vec![];
        for phase in &plan.phases {
            if phase.status != PhaseStatus::InProgress {
                continue;
            }
            for task in &phase.tasks {
                if task.status != TaskStatus::Pending {
                    continue;
                }
                let deps_met = task.dependencies.iter().all(|dep_id| {
                    phase.tasks.iter().any(|t| {
                        t.id == *dep_id && t.status == TaskStatus::Completed
                    })
                });
                if deps_met {
                    executable.push(task);
                }
            }
        }
        executable
    }

    pub fn mark_task_completed(
        &mut self,
        task_id: &str,
        result: serde_json::Value,
    ) {
        if let Some(plan) = &mut self.current_plan {
            for phase in &mut plan.phases {
                for task in &mut phase.tasks {
                    if task.id == task_id {
                        task.status = TaskStatus::Completed;
                        task.result = Some(result);
                        break;
                    }
                }
            }
            plan.updated_at = chrono::Utc::now().timestamp();
        }
    }

    pub fn mark_task_failed(&mut self, task_id: &str) {
        if let Some(plan) = &mut self.current_plan {
            for phase in &mut plan.phases {
                for task in &mut phase.tasks {
                    if task.id == task_id {
                        task.retry_count += 1;
                        if task.retry_count >= task.max_retries {
                            task.status = TaskStatus::Failed;
                        } else {
                            task.status = TaskStatus::Pending;
                        }
                        break;
                    }
                }
            }
            plan.updated_at = chrono::Utc::now().timestamp();
        }
    }
}
```

**Step 3：实现 Checkpoint 机制**

新建 `src-tauri/crates/agent/src/checkpoint.rs`：

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub plan_id: String,
    pub phase_index: usize,
    pub completed_task_ids: Vec<String>,
    pub state: serde_json::Value,
    pub timestamp: i64,
}

pub struct CheckpointManager {
    checkpoint_dir: PathBuf,
}

impl CheckpointManager {
    pub fn new(work_dir: &str) -> Self {
        let checkpoint_dir = PathBuf::from(work_dir).join(".axagent/checkpoints");
        Self { checkpoint_dir }
    }

    pub async fn save(&self, checkpoint: &Checkpoint) -> Result<(), String> {
        tokio::fs::create_dir_all(&self.checkpoint_dir)
            .await
            .map_err(|e| e.to_string())?;
        let path = self.checkpoint_dir.join(format!("{}.json", checkpoint.id));
        let content = serde_json::to_string_pretty(checkpoint)
            .map_err(|e| e.to_string())?;
        tokio::fs::write(path, content)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn load(&self, id: &str) -> Result<Checkpoint, String> {
        let path = self.checkpoint_dir.join(format!("{id}.json"));
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }

    pub async fn list(&self) -> Result<Vec<Checkpoint>, String> {
        let mut entries = tokio::fs::read_dir(&self.checkpoint_dir)
            .await
            .map_err(|e| e.to_string())?;
        let mut checkpoints = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
            if entry.path().extension().map_or(false, |e| e == "json") {
                let content = tokio::fs::read_to_string(entry.path())
                    .await
                    .map_err(|e| e.to_string())?;
                if let Ok(cp) = serde_json::from_str(&content) {
                    checkpoints.push(cp);
                }
            }
        }
        checkpoints.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(checkpoints)
    }
}
```

#### 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 修改 | `src-tauri/crates/agent/src/coordinator.rs`（重构为泛型） |
| 新增 | `src-tauri/crates/agent/src/hierarchical_planner.rs` |
| 新增 | `src-tauri/crates/agent/src/checkpoint.rs` |
| 修改 | `src-tauri/crates/agent/src/lib.rs` |
| 新增 | `src/components/agent/AutonomousPlanView.tsx` |
| 新增 | `src/components/agent/ProgressDashboard.tsx` |
| 新增 | `src/components/agent/TaskDependencyGraph.tsx` |

---

### 差距 8：多 Agent 协作系统不成熟 [P2]

#### 现状

`src-tauri/crates/trajectory/src/sub_agent.rs` 有 SubAgent 和 MessageBus，`src-tauri/crates/runtime/src/agent_roles.rs` 定义了 6 种角色（Coordinator、Researcher、Developer、Reviewer、Browser、Executor），但缺乏成熟的编排框架。

#### 竞品做法

| 竞品 | 能力 |
|------|------|
| Claude Code | 多 Agent 协调——并行子 Agent + 主 Agent 合并结果 |
| AutoGen | 多 Agent 对话、角色分工、辩论 |
| CrewAI | 角色分配、任务委派、结果聚合 |

#### 改进方案

**Step 1：实现 Agent 编排引擎**

新建 `src-tauri/crates/runtime/src/agent_orchestrator.rs`：

```rust
use crate::agent_roles::AgentRole;
use crate::message_gateway::{AgentMessage, MessagePayload};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationPlan {
    pub id: String,
    pub goal: String,
    pub agents: Vec<AgentAssignment>,
    pub communication_plan: Vec<CommunicationRule>,
    pub consensus_strategy: ConsensusStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssignment {
    pub agent_id: String,
    pub role: AgentRole,
    pub task: String,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationRule {
    pub from_role: AgentRole,
    pub to_role: AgentRole,
    pub message_type: String,
    pub trigger: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusStrategy {
    MajorityVote,
    Unanimous,
    LeaderDecides { leader_role: AgentRole },
    WeightedVote { weights: HashMap<String, f32> },
}

pub struct AgentOrchestrator {
    active_agents: Arc<RwLock<HashMap<String, ActiveAgent>>>,
    message_bus: Arc<crate::message_gateway::MessageGateway>,
}

struct ActiveAgent {
    id: String,
    role: AgentRole,
    status: AgentRunStatus,
    result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AgentRunStatus {
    Starting,
    Running,
    WaitingForInput,
    Completed,
    Failed,
}

impl AgentOrchestrator {
    pub fn new(
        message_bus: Arc<crate::message_gateway::MessageGateway>,
    ) -> Self {
        Self {
            active_agents: Arc::new(RwLock::new(HashMap::new())),
            message_bus,
        }
    }

    pub async fn execute_plan(
        &self,
        plan: OrchestrationPlan,
    ) -> Result<OrchestrationResult, String> {
        // 1. Create agents based on assignments
        for assignment in &plan.agents {
            self.create_agent(assignment).await?;
        }

        // 2. Execute agents respecting dependencies
        let mut completed = HashMap::new();
        let mut failed = Vec::new();

        while completed.len() + failed.len() < plan.agents.len() {
            let ready_agents = self.get_ready_agents(&plan, &completed).await;
            for agent_id in ready_agents {
                match self.run_agent(&agent_id).await {
                    Ok(result) => {
                        completed.insert(agent_id.clone(), result);
                    }
                    Err(e) => {
                        failed.push((agent_id.clone(), e));
                    }
                }
            }
            if ready_agents.is_empty() {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        // 3. Aggregate results based on consensus strategy
        let final_result = self
            .aggregate_results(&plan.consensus_strategy, &completed)
            .await;

        Ok(OrchestrationResult {
            plan_id: plan.id,
            goal: plan.goal,
            agent_results: completed,
            failures: failed,
            final_result,
        })
    }

    async fn get_ready_agents(
        &self,
        plan: &OrchestrationPlan,
        completed: &HashMap<String, serde_json::Value>,
    ) -> Vec<String> {
        let agents = self.active_agents.read().await;
        plan.agents
            .iter()
            .filter(|a| {
                agents.get(&a.agent_id).map_or(false, |ag| {
                    ag.status == AgentRunStatus::Starting
                }) && a.dependencies
                    .iter()
                    .all(|dep| completed.contains_key(dep))
            })
            .map(|a| a.agent_id.clone())
            .collect()
    }

    async fn aggregate_results(
        &self,
        strategy: &ConsensusStrategy,
        results: &HashMap<String, serde_json::Value>,
    ) -> serde_json::Value {
        match strategy {
            ConsensusStrategy::LeaderDecides { leader_role } => {
                results.values().next().cloned().unwrap_or(serde_json::json!(null))
            }
            ConsensusStrategy::MajorityVote => {
                // Simple majority: return the most common result
                results.values().next().cloned().unwrap_or(serde_json::json!(null))
            }
            _ => results.values().next().cloned().unwrap_or(serde_json::json!(null)),
        }
    }

    async fn create_agent(
        &self,
        assignment: &AgentAssignment,
    ) -> Result<(), String> {
        let mut agents = self.active_agents.write().await;
        agents.insert(
            assignment.agent_id.clone(),
            ActiveAgent {
                id: assignment.agent_id.clone(),
                role: assignment.role.clone(),
                status: AgentRunStatus::Starting,
                result: None,
            },
        );
        Ok(())
    }

    async fn run_agent(
        &self,
        agent_id: &str,
    ) -> Result<serde_json::Value, String> {
        let mut agents = self.active_agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.status = AgentRunStatus::Running;
        }
        // Actual execution delegated to the agent system
        Ok(serde_json::json!({ "agent_id": agent_id, "status": "completed" }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    pub plan_id: String,
    pub goal: String,
    pub agent_results: HashMap<String, serde_json::Value>,
    pub failures: Vec<(String, String)>,
    pub final_result: serde_json::Value,
}
```

#### 文件变更清单

| 操作 | 文件路径 |
|------|---------|
| 新增 | `src-tauri/crates/runtime/src/agent_orchestrator.rs` |
| 修改 | `src-tauri/crates/runtime/src/lib.rs` |
| 修改 | `src-tauri/crates/trajectory/src/sub_agent.rs` |
| 新增 | `src/components/agent/MultiAgentDashboard.tsx` |
| 新增 | `src/components/agent/AgentCommunicationGraph.tsx` |
| 新增 | `src/components/agent/ConsensusView.tsx` |

---

### 差距 9：Web 浏览与信息检索深度不足 [P2]

#### 现状

`src-tauri/crates/agent/src/web_search.rs` 和 `src-tauri/crates/agent/src/research_agent.rs` 已有基础搜索和研究能力，但缺乏深度 Web 集成。

#### 改进方案

- 完整网页渲染（Headless Chrome/Firefox）
- JavaScript 执行和动态内容提取
- 多源信息交叉验证
- 学术搜索增强（arXiv, Google Scholar, PubMed）

此部分已有详细实施计划在 `docs/plans/2026-04-26-phase4-implementation.md`，此处不再重复。

---

## 四、架构与工程质量差距

### 差距 10：架构设计问题 [P1]

#### 问题清单

| 问题 | 位置 | 影响 | 修复方案 |
|------|------|------|---------|
| `dyn AgentImpl` 动态分发 | `coordinator.rs:110` | 编译时无法检查接口兼容性，锁竞争 | 重构为泛型 trait bounds |
| 两套事件系统 | `event_bus.rs` + `event_emitter.rs` | 模块间通信不一致 | 统一为 `UnifiedEventBus` |
| 前端 Store 过大 | `conversationStore.ts` 50+ 字段 | 违反单一职责 | 拆分为 4 个子 Store |
| WorkEngine ↔ WorkflowEngine 未桥接 | 运行时模块 | 技能分解执行链路不通 | 实现桥接层 |
| NodeExecutor 未完成实际执行 | 运行时模块 | 工作流节点无法真正执行 | 实现 trait-based 执行器 |
| 大量 `#![allow(clippy::...)]` | 多个 crate | 掩盖潜在问题 | 逐步修复并移除 allow |
| 工具依赖硬编码为 Satisfied | 技能分解模块 | 工具可用性检查失效 | 实现真实依赖检查 |

#### 详细修复方案

**10.1 统一事件系统**（详见 `docs/plans/2026-04-28-implementation-guide.md` 第 2.2 节）

**10.2 前端 Store 拆分**（详见 `docs/plans/2026-04-28-implementation-guide.md` 第 2.3 节）

**10.3 WorkEngine ↔ WorkflowEngine 桥接**

新建 `src-tauri/crates/runtime/src/work_engine/bridge.rs`：

```rust
use crate::work_engine::engine::WorkEngine;
use crate::workflow_engine::WorkflowRunner;

pub struct WorkflowBridge {
    work_engine: Arc<WorkEngine>,
    workflow_runner: Arc<WorkflowRunner>,
}

impl WorkflowBridge {
    pub fn new(
        work_engine: Arc<WorkEngine>,
        workflow_runner: Arc<WorkflowRunner>,
    ) -> Self {
        Self { work_engine, workflow_runner }
    }

    pub async fn execute_skill_as_workflow(
        &self,
        skill_id: &str,
        skill_content: &str,
        input: &str,
    ) -> Result<serde_json::Value, String> {
        // 1. Parse skill content into workflow steps
        let steps = self.parse_skill_steps(skill_content)?;

        // 2. Convert to workflow definition
        let workflow = self.steps_to_workflow(&steps)?;

        // 3. Execute via WorkflowRunner
        let result = self.workflow_runner.run(workflow).await?;

        // 4. Record execution in WorkEngine
        self.work_engine
            .record_execution(skill_id, &result)
            .await?;

        Ok(result)
    }

    fn parse_skill_steps(
        &self,
        content: &str,
    ) -> Result<Vec<SkillStep>, String> {
        // Parse skill markdown content into structured steps
        Ok(vec![])
    }

    fn steps_to_workflow(
        &self,
        steps: &[SkillStep],
    ) -> Result<WorkflowDefinition, String> {
        // Convert skill steps to workflow definition
        Ok(WorkflowDefinition::default())
    }
}
```

---

### 差距 11：测试覆盖不足 [P1]

#### 现状

141 个单元测试通过，E2E 测试 11 个失败（环境问题），关键模块测试覆盖率不足。

#### 改进方案

| 模块 | 目标覆盖率 | 优先级 |
|------|-----------|--------|
| `agent/` (Rust) | >80% | P1 |
| `core/` (Rust) | >80% | P1 |
| `runtime/` (Rust) | >70% | P2 |
| 前端组件 (Vitest) | >60% | P2 |
| E2E (Playwright) | 核心流程全覆盖 | P2 |

引入标准评估基准：

- SWE-bench：代码修复能力评估
- Terminal-Bench：终端操作能力评估
- HumanEval：代码生成能力评估

---

### 差距 12：性能优化 [P1]

#### 改进方案

| 优化项 | 当前 | 目标 | 优先级 |
|--------|------|------|--------|
| 数据库连接池 | max=5, min=1 | max=20, min=5 | P1 |
| 消息列表渲染 | 全量渲染 | 虚拟列表 | P1 |
| 前端代码分割 | 无 | 按路由懒加载 | P2 |
| 向量搜索缓存 | 无 | LRU 缓存层 | P2 |
| 工具执行缓存 | 无 | 结果缓存 | P2 |

---

## 五、差异化优势（需保持和强化）

| 方向 | 说明 | 竞品现状 | 强化建议 |
|------|------|---------|---------|
| 开源多模型 Agent | 唯一开源的支持多模型的桌面 Agent | 竞品多为闭源单模型 | 持续增加模型支持 |
| 知识库 + Agent 深度结合 | RAG 与 Agent 的深度结合 | 竞品几乎无此能力 | 实现 RAG-aware Agent 工具选择 |
| 技能演化系统 | 独特的技能学习和进化机制 | 竞品无此功能 | 完善技能分解→执行闭环 |
| 本地优先 | 支持完全离线的本地模型运行 | 竞品几乎全依赖云端 | 优化 Ollama 集成体验 |
| MCP 生态 | 与 Claude Code 同级的 MCP 支持 | 仅 Claude Code 有 | 构建 MCP 工具市场 |
| 国际化 | 12 种语言支持 | 竞品几乎无 | 持续完善翻译质量 |
| API 网关 | 完整的本地 API 网关 | 竞品无 | 添加更多 API 兼容模式 |

---

## 六、优先级路线图

### Phase A：基础增强（1-2 个月）— 投入产出比最高

```
┌──────────────────────────────────────────────────────────────┐
│ [P0] LSP 深度集成 + 代码编辑体验                              │
│ [P0] 终端集成（xterm.js + PTY + 输出分析）                    │
│ [P0] 修复执行链路（WorkEngine ↔ WorkflowEngine 桥接）          │
│ [P1] 架构重构（trait bounds、统一事件系统、Store 拆分）         │
│ [P1] 测试覆盖（核心模块 >60%）                                │
│ [P1] 性能优化（虚拟列表 + 连接池调优）                         │
└──────────────────────────────────────────────────────────────┘
```

### Phase B：智能体增强（2-3 个月）— 核心差异化能力

```
┌──────────────────────────────────────────────────────────────┐
│ [P1] Git 深度集成（commit/PR/审查）                           │
│ [P1] 自主规划与执行能力（分层规划器 + Checkpoint）             │
│ [P2] 多 Agent 协作系统（编排引擎 + 共识机制）                  │
│ [P2] Web 浏览与信息检索深度集成                               │
└──────────────────────────────────────────────────────────────┘
```

### Phase C：高级能力（3-4 个月）— 前沿能力，构建护城河

```
┌──────────────────────────────────────────────────────────────┐
│ [P2] 多模态深度理解（视觉管道 + UI 元素检测）                  │
│ [P2] 持续学习与个性化适应（行为学习 + 项目记忆）               │
│ [P2] 多 Agent 协作完善（辩论/评审 + 性能评估）                 │
└──────────────────────────────────────────────────────────────┘
```

### Phase D：生态建设（4-6 个月）— 规模化与生态

```
┌──────────────────────────────────────────────────────────────┐
│ [P3] 实时协作能力（CRDT + WebSocket）                         │
│ [P3] 插件市场与社区建设                                       │
│ [P3] 标准评估基准（SWE-bench、Terminal-Bench）                │
└──────────────────────────────────────────────────────────────┘
```

---

## 七、工作量估算

| Phase | 内容 | 预估人天 |
|-------|------|---------|
| Phase A | 基础增强 | 40-50 |
| Phase B | 智能体增强 | 35-45 |
| Phase C | 高级能力 | 30-40 |
| Phase D | 生态建设 | 25-35 |
| **总计** | | **130-170** |

---

## 八、竞品功能矩阵对比（2026-04 更新）

| 功能 | AxAgent | Claude Code | Cursor | Copilot | Devin | Codex |
|------|---------|-------------|--------|---------|-------|-------|
| 多模型支持 | ✅ | ❌ | ✅ | ❌ | ✅ | ❌ |
| 本地模型 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 知识库/RAG | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| MCP 协议 | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| 工作流引擎 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 技能系统 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 轨迹学习/RL | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| API 网关 | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| 代码编辑 | ⚠️ 基础 | ✅ 深度 | ✅ 深度 | ✅ 深度 | ✅ 深度 | ✅ 深度 |
| LSP 集成 | ⚠️ 占位 | ✅ 完整 | ✅ 完整 | ✅ 完整 | ✅ 完整 | ✅ 完整 |
| 终端集成 | ⚠️ 基础 | ✅ 完整 | ✅ 完整 | ❌ | ✅ 完整 | ✅ 完整 |
| Git 集成 | ⚠️ 基础 | ✅ 完整 | ✅ 完整 | ✅ 基础 | ✅ 完整 | ✅ 完整 |
| 多模态理解 | ⚠️ 基础 | ✅ 视觉 | ❌ | ❌ | ✅ 视觉 | ❌ |
| 自主规划 | ⚠️ 基础 | ✅ 完整 | ⚠️ 基础 | ❌ | ✅ 完整 | ✅ 完整 |
| 多 Agent 协作 | ⚠️ 基础 | ✅ 并行子Agent | ❌ | ❌ | ✅ 完整 | ❌ |
| 实时协作 | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ |
| Web 浏览 | ⚠️ 基础 | ✅ 完整 | ❌ | ❌ | ✅ 完整 | ❌ |
| 国际化 | ✅ 12语言 | ❌ | ❌ | ❌ | ❌ | ❌ |
| 跨平台 | ✅ 全平台 | ✅ Mac/Linux | ✅ 全平台 | ✅ VSCode | ❌ Web | ✅ 全平台 |
| 开源 | ✅ AGPL | ❌ | ❌ | ❌ | ❌ | ❌ |

> ✅ = 完整支持  ⚠️ = 部分支持  ❌ = 不支持

---

## 九、总结

### 核心优势（需保持）

1. **开源 + 跨平台**：AGPL 许可，Windows/Mac/Linux 全平台支持
2. **多模型 + 本地模型**：支持 OpenAI、Anthropic、Gemini、Ollama 等
3. **知识库/RAG**：完整的文档解析、向量搜索、重排序管道
4. **MCP 协议**：与 Claude Code 同级的 MCP 支持
5. **技能系统 + 轨迹学习**：独特的技能演化和 RL 强化学习
6. **国际化**：12 种语言支持，远超竞品

### 最大差距（需优先补齐）

1. **代码编辑体验**：缺乏 LSP 深度集成和内联编辑
2. **终端集成**：缺乏完整的终端模拟器
3. **执行链路**：WorkEngine ↔ WorkflowEngine 未桥接
4. **架构债务**：动态分发、双事件系统、Store 过大
5. **测试覆盖**：测试覆盖率不足

### 差异化方向（可构建护城河）

1. **开源多模型 Agent**：唯一开源的支持多模型的桌面 Agent
2. **知识库 + Agent**：RAG 与 Agent 的深度结合
3. **技能演化系统**：独特的技能学习和进化机制
4. **本地优先**：支持完全离线的本地模型运行
5. **插件生态**：开放的插件系统，可扩展性强
