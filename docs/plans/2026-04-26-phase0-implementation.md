# Phase 0: 基础增强 - 详细实施计划

> 阶段: Phase 0
> 时间: 2026-04-26 至 2026-05-15
> 目标: 修复已知问题，增强现有功能稳定性

---

## 1. 安全修复（P0）

### 1.1 SQL 注入风险修复

**文件**: `src-tauri/crates/core/src/builtin_tools.rs`
**位置**: 第 1044-1065 行

**当前代码问题**:
```rust
// Fallback: direct interpolation with basic escaping
let safe_query = query.replace('"', "\"\"").replace('\'', "''");
let fallback_sql = format!(
    "SELECT ... WHERE messages_fts MATCH '\"{}\"' ...",
    safe_query, limit
);
```

**修复方案**: 使用参数化查询

```rust
// 方案 1: 使用参数化查询（推荐）
let fallback_sql = r#"
    SELECT id, conversation_id, role, content, name, created_at
    FROM messages
    WHERE messages_fts MATCH ?1
    ORDER BY created_at DESC
    LIMIT ?2
"#;

// 方案 2: 如果 FTS5 不支持参数化，使用更严格的输入验证
fn sanitize_fts_query(query: &str) -> Result<String> {
    // 只允许字母、数字、空格和基本标点
    let sanitized: String = query.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .collect();

    if sanitized.is_empty() {
        return Err(anyhow!("Invalid search query"));
    }

    // 转义特殊 FTS5 字符
    Ok(sanitized
        .replace('"', "\"\"")
        .replace('*', "")
        .replace(':', ""))
}
```

**验收标准**:
- [ ] 不存在任何字符串插值的 SQL 查询
- [ ] 所有用户输入都经过验证或参数化
- [ ] 添加 fuzzing 测试覆盖

### 1.2 路径遍历风险修复

**文件**: `src-tauri/crates/core/src/builtin_tools.rs`
**位置**: 第 496-515 行

**当前代码问题**:
```rust
async fn read_file(path: &str) -> Result<McpToolResult> {
    // 无路径验证直接读取
    match tokio::fs::read_to_string(path).await {
        ...
    }
}
```

**修复方案**:

```rust
use std::path::PathBuf;

const ALLOWED_DIRECTORIES: &[&str] = &[
    // 项目工作目录
    "workspace",
    // 临时目录
    "/tmp/axagent",
    // 下载目录
    "downloads",
];

fn validate_path(path: &str, allowed_base: &str) -> Result<PathBuf> {
    let requested_path = PathBuf::from(path);

    // 解析为绝对路径并规范化
    let absolute_path = if requested_path.is_absolute() {
        requested_path
    } else {
        PathBuf::from(allowed_base).join(&requested_path)
    }
    .canonicalize()
    .map_err(|_| anyhow!("Invalid path: {}", path))?;

    // 检查是否在允许的目录内
    for allowed_dir in ALLOWED_DIRECTORIES {
        let allowed = PathBuf::from(allowed_dir).canonicalize()?;
        if absolute_path.starts_with(&allowed) {
            return Ok(absolute_path);
        }
    }

    Err(anyhow!("Path outside allowed directories: {}", path))
}

async fn read_file(path: &str) -> Result<McpToolResult> {
    let validated_path = validate_path(path, "workspace")?;

    match tokio::fs::read_to_string(&validated_path).await {
        Ok(content) => Ok(McpToolResult::success(content)),
        Err(e) => Ok(McpToolResult::error(format!("Failed to read file: {}", e))),
    }
}
```

**验收标准**:
- [ ] `../../etc/passwd` 等路径遍历尝试被阻止
- [ ] 所有文件操作都经过路径验证
- [ ] 验证失败时返回清晰的错误信息

### 1.3 Base64 解码大小限制

**文件**: `src-tauri/crates/core/src/builtin_tools.rs`
**位置**: 第 1283 行

**当前代码问题**:
```rust
fn base64_decode(input: &str) -> Result<Vec<u8>> {
    use base64::Engine;
    Engine::decode(&base64::engine::general_purpose::STANDARD, input)
        .map_err(...)
}
```

**修复方案**:

```rust
const MAX_BASE64_DECODE_SIZE: usize = 100 * 1024 * 1024; // 100MB
const MAX_BASE64_INPUT_SIZE: usize = MAX_BASE64_DECODE_SIZE * 4 / 3 + 10; // 考虑 padding

fn base64_decode(input: &str) -> Result<Vec<u8>> {
    // 检查输入大小
    if input.len() > MAX_BASE64_INPUT_SIZE {
        return Err(anyhow!(
            "Input too large: {} bytes (max: {} bytes)",
            input.len(),
            MAX_BASE64_INPUT_SIZE
        ));
    }

    use base64::Engine;

    let decoded = Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        input
    ).map_err(|e| anyhow!("Base64 decode error: {}", e))?;

    // 检查解码后大小
    if decoded.len() > MAX_BASE64_DECODE_SIZE {
        return Err(anyhow!(
            "Decoded data too large: {} bytes (max: {} bytes)",
            decoded.len(),
            MAX_BASE64_DECODE_SIZE
        ));
    }

    Ok(decoded)
}
```

**验收标准**:
- [ ] 超大 Base64 输入被正确拒绝
- [ ] 错误信息清晰，不泄露系统信息
- [ ] 添加单元测试覆盖边界情况

---

## 2. Artifact 系统增强

### 2.1 扩展 ArtifactFormat

**文件**: `src/types/artifact.ts`

**当前定义**:
```typescript
export type ArtifactFormat = "markdown" | "text" | "json";
```

**扩展为**:
```typescript
export type ArtifactFormat =
  | "markdown"
  | "text"
  | "json"
  | "html"
  | "css"
  | "javascript"
  | "typescript"
  | "jsx"
  | "tsx"
  | "python"
  | "svg"
  | "mermaid"
  | "d2";

export type ArtifactLanguage = ArtifactFormat; // 别名，便于使用
```

**文件变更**:
```diff
export type Artifact = {
  id: string;
  conversationId: string;
  kind: ArtifactKind;
  title: string;
  content: string;
  format: ArtifactFormat;
+ language?: ArtifactLanguage;  // 编程语言（用于代码片段）
+ previewMode?: 'split' | 'preview' | 'code';  // 预览模式
+ metadata?: {
+   lineCount?: number;
+   lastExecuted?: string;
+   executionOutput?: string;
+ };
  pinned: boolean;
  updatedAt: string;
};
```

### 2.2 ArtifactPreview 组件设计

**文件**: `src/components/chat/ArtifactPreview/`

```
ArtifactPreview/
├── index.tsx              # 主入口
├── CodePreview.tsx        # 代码预览（Monaco 集成）
├── HtmlPreview.tsx        # HTML/CSS/JS 预览
├── ReactPreview.tsx       # React 组件预览
├── ChartPreview.tsx       # 图表预览
├── MarkdownPreview.tsx    # Markdown 渲染
└── SplitView.tsx          # 分栏视图
```

**核心功能**:

1. **CodePreview** - Monaco 编辑器集成
```typescript
interface CodePreviewProps {
  code: string;
  language: ArtifactLanguage;
  readOnly?: boolean;
  onChange?: (code: string) => void;
}
```

2. **HtmlPreview** - iframe 沙箱渲染
```typescript
interface HtmlPreviewProps {
  html: string;
  css?: string;
  js?: string;
  sandbox?: 'allow-scripts' | 'allow-same-origin';
}
```

3. **SplitView** - 分栏编辑/预览
```typescript
interface SplitViewProps {
  code: string;
  language: ArtifactLanguage;
  splitDirection?: 'horizontal' | 'vertical';
  showPreview?: boolean;
}
```

### 2.3 ArtifactStore 增强

**文件**: `src/stores/shared/artifactStore.ts`

**新增功能**:
```typescript
interface ArtifactState {
  // 现有...
  artifacts: Artifact[];
  loading: boolean;
  error: string | null;

  // 新增
  previewArtifact: Artifact | null;
  previewMode: 'split' | 'preview' | 'code';

  // 新增 Actions
  setPreviewArtifact: (artifact: Artifact | null) => void;
  setPreviewMode: (mode: 'split' | 'preview' | 'code') => void;
  executeCode: (artifactId: string) => Promise<ExecutionResult>;
  duplicateArtifact: (id: string) => Promise<Artifact | null>;
}
```

### 2.4 HTML 渲染引擎

**文件**: `src/lib/artifactRenderer.ts`

```typescript
import { createRoot } from 'react-dom/client';
import * as Babel from '@babel/standalone';
import * as React from 'react';
import * as ReactDOM from 'react-dom/client';

export class ArtifactRenderer {
  private iframe: HTMLIFrameElement | null = null;

  // 渲染 HTML/CSS/JS
  renderHtml(html: string, css?: string, js?: string): void {
    const content = `
      <!DOCTYPE html>
      <html>
      <head>
        <style>${css || ''}</style>
      </head>
      <body>
        ${html}
        <script>${js || ''}</script>
      </body>
      </html>
    `;

    this.iframe.srcdoc = content;
  }

  // 渲染 React 组件
  async renderReact(code: string, container: HTMLElement): Promise<void> {
    // 使用 Babel 转换 JSX
    const transformed = Babel.transform(code, {
      presets: ['react'],
      filename: 'component.tsx',
    });

    // 执行转换后的代码
    const fn = new Function('React', 'ReactDOM', 'container', transformed.code);
    fn(React, ReactDOM, container);
  }

  // 渲染 Chart.js 图表
  renderChart(config: ChartConfiguration, container: HTMLElement): void {
    new Chart(container, config);
  }
}
```

### 2.5 Monaco 编辑器集成

**文件**: `src/components/shared/MonacoEditor.tsx`

```typescript
import { Editor, Monaco } from '@monaco-editor/react';
import type { ArtifactLanguage } from '@/types/artifact';

interface MonacoEditorProps {
  value: string;
  language: ArtifactLanguage;
  onChange?: (value: string) => void;
  readOnly?: boolean;
  height?: string | number;
}

const LANGUAGE_MAP: Record<ArtifactLanguage, string> = {
  javascript: 'javascript',
  typescript: 'typescript',
  jsx: 'javascript',
  tsx: 'typescript',
  html: 'html',
  css: 'css',
  python: 'python',
  markdown: 'markdown',
  json: 'json',
  svg: 'xml',
  mermaid: 'markdown',
  d2: 'markdown',
};

export function MonacoEditor({
  value,
  language,
  onChange,
  readOnly = false,
  height = '100%',
}: MonacoEditorProps) {
  return (
    <Editor
      height={height}
      language={LANGUAGE_MAP[language] || 'plaintext'}
      value={value}
      onChange={(v) => onChange?.(v || '')}
      options={{
        readOnly,
        minimap: { enabled: false },
        fontSize: 13,
        lineNumbers: 'on',
        scrollBeyondLastLine: false,
        automaticLayout: true,
      }}
    />
  );
}
```

---

## 3. 代码解释器增强

### 3.1 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                    代码解释器架构                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐   │
│  │   前端 UI   │────▶│ Tauri IPC   │────▶│ 沙箱进程    │   │
│  │             │◀────│             │◀────│ Manager     │   │
│  └─────────────┘     └─────────────┘     └─────────────┘   │
│        │                                        │           │
│        ▼                                        ▼           │
│  ┌─────────────┐                         ┌─────────────┐   │
│  │ 执行结果    │                         │  Node.js    │   │
│  │ 展示        │                         │  沙箱进程   │   │
│  └─────────────┘                         └─────────────┘   │
│                                                 │           │
│                                                 ▼           │
│                                         ┌─────────────┐   │
│                                         │ Python      │   │
│                                         │ (Pyodide)   │   │
│                                         └─────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Rust 沙箱进程管理

**文件**: `src-tauri/crates/core/src/sandbox_runner.rs`

```rust
use std::process::Stdio;
use tokio::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SANDBOX_TIMEOUT_SECS: u64 = 30;
const MAX_OUTPUT_SIZE: usize = 1024 * 1024; // 1MB

pub struct SandboxRunner {
    node_path: String,
}

impl SandboxRunner {
    pub fn new() -> Self {
        Self {
            node_path: std::env::var("NODE_PATH")
                .unwrap_or_else(|_| "node".to_string()),
        }
    }

    pub async fn execute(&self, code: &str, language: &str) -> Result<ExecutionResult> {
        match language {
            "javascript" | "js" => self.execute_js(code).await,
            "python" => self.execute_python(code).await,
            _ => Err(anyhow!("Unsupported language: {}", language)),
        }
    }

    async fn execute_js(&self, code: &str) -> Result<ExecutionResult> {
        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join(format!("axagent_sandbox_{}.js", uuid::Uuid::new_v4()));

        tokio::fs::write(&script_path, code).await?;

        // 执行脚本（带超时和资源限制）
        let output = Command::new(&self.node_path)
            .arg(&script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(SANDBOX_TIMEOUT_SECS),
            output,
        )
        .await
        .map_err(|_| anyhow!("Execution timeout"))??;

        // 清理临时文件
        let _ = tokio::fs::remove_file(&script_path).await;

        Ok(ExecutionResult {
            stdout: String::from_utf8_lossy(&result.stdout).to_string(),
            stderr: String::from_utf8_lossy(&result.stderr).to_string(),
            exit_code: result.status.code().unwrap_or(-1),
        })
    }

    async fn execute_python(&self, code: &str) -> Result<ExecutionResult> {
        // Python 通过 Pyodide 在前端执行
        // 这里返回错误，由前端处理
        Err(anyhow!("Python execution handled by frontend Pyodide"))
    }
}

#[derive(Debug, Serialize)]
pub struct ExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
```

### 3.3 前端代码执行器

**文件**: `src/lib/codeExecutor.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface ExecutionResult {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
}

export interface CodeExecutorOptions {
  language: 'javascript' | 'python' | 'typescript';
  code: string;
  timeout?: number;
}

class CodeExecutor {
  private pyodide: any = null;

  // 初始化 Pyodide（懒加载）
  async initPyodide(): Promise<void> {
    if (this.pyodide) return;

    // @ts-ignore - Pyodide 从 CDN 加载
    import('https://cdn.jsdelivr.net/pyodide/v0.24.1/full/pyodide.js');

    // @ts-ignore
    this.pyodide = await window.loadPyodide({
      indexURL: 'https://cdn.jsdelivr.net/pyodide/v0.24.1/full/',
    });
  }

  // 执行 JavaScript/TypeScript
  async executeJS(code: string): Promise<ExecutionResult> {
    const start = performance.now();

    try {
      const result = await invoke<ExecutionResult>('execute_sandbox', {
        code,
        language: 'javascript',
      });

      return {
        ...result,
        duration_ms: performance.now() - start,
      };
    } catch (error) {
      return {
        stdout: '',
        stderr: String(error),
        exit_code: -1,
        duration_ms: performance.now() - start,
      };
    }
  }

  // 执行 Python
  async executePython(code: string): Promise<ExecutionResult> {
    const start = performance.now();

    try {
      await this.initPyodide();

      // 重定向 stdout
      await this.pyodide.runPythonAsync(`
import sys
from io import StringIO
sys.stdout = StringIO()
sys.stderr = StringIO()
      `);

      // 执行代码
      await this.pyodide.runPythonAsync(code);

      // 获取输出
      const stdout = await this.pyodide.runPythonAsync('sys.stdout.getvalue()');
      const stderr = await this.pyodide.runPythonAsync('sys.stderr.getvalue()');

      return {
        stdout,
        stderr,
        exit_code: 0,
        duration_ms: performance.now() - start,
      };
    } catch (error) {
      return {
        stdout: '',
        stderr: String(error),
        exit_code: -1,
        duration_ms: performance.now() - start,
      };
    }
  }

  // 执行代码
  async execute(options: CodeExecutorOptions): Promise<ExecutionResult> {
    switch (options.language) {
      case 'javascript':
      case 'typescript':
        return this.executeJS(options.code);
      case 'python':
        return this.executePython(options.code);
      default:
        return {
          stdout: '',
          stderr: `Unsupported language: ${options.language}`,
          exit_code: -1,
          duration_ms: 0,
        };
    }
  }
}

export const codeExecutor = new CodeExecutor();
```

### 3.4 代码执行面板

**文件**: `src/components/chat/CodeExecutorPanel.tsx`

```typescript
import { useState } from 'react';
import { Button, Input, Select, Space, Typography, Terminal } from 'antd';
import { codeExecutor, type ExecutionResult } from '@/lib/codeExecutor';

const { TextArea } = Input;
const { Title } = Typography;

interface CodeExecutorPanelProps {
  initialCode?: string;
  language?: 'javascript' | 'python' | 'typescript';
}

export function CodeExecutorPanel({
  initialCode = '',
  language = 'javascript',
}: CodeExecutorPanelProps) {
  const [code, setCode] = useState(initialCode);
  const [execLanguage, setExecLanguage] = useState(language);
  const [result, setResult] = useState<ExecutionResult | null>(null);
  const [loading, setLoading] = useState(false);

  const handleExecute = async () => {
    setLoading(true);
    try {
      const execResult = await codeExecutor.execute({
        code,
        language: execLanguage,
      });
      setResult(execResult);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 16 }}>
      <Space direction="vertical" style={{ width: '100%' }} size="large">
        <Space>
          <Select
            value={execLanguage}
            onChange={setExecLanguage}
            options={[
              { value: 'javascript', label: 'JavaScript' },
              { value: 'typescript', label: 'TypeScript' },
              { value: 'python', label: 'Python' },
            ]}
          />
          <Button
            type="primary"
            onClick={handleExecute}
            loading={loading}
          >
            执行
          </Button>
        </Space>

        <TextArea
          value={code}
          onChange={(e) => setCode(e.target.value)}
          rows={10}
          monospace
          placeholder="输入代码..."
        />

        {result && (
          <div>
            <Title level={5}>执行结果</Title>
            <Space direction="vertical" style={{ width: '100%' }}>
              <div style={{ color: result.exit_code === 0 ? 'green' : 'red' }}>
                退出码: {result.exit_code} | 耗时: {result.duration_ms.toFixed(2)}ms
              </div>

              {result.stdout && (
                <Terminal
                  content={result.stdout}
                  theme={{ background: '#1e1e1e', color: '#d4d4d4' }}
                />
              )}

              {result.stderr && (
                <Terminal
                  content={result.stderr}
                  theme={{ background: '#2d1f1f', color: '#f48771' }}
                />
              )}
            </Space>
          </div>
        )}
      </Space>
    </div>
  );
}
```

---

## 4. 自我验证机制（基础版）

### 4.1 验证节点类型

**文件**: `src/components/workflow/Nodes/ValidationNode.tsx`

```typescript
import { memo } from 'react';
import { Handle, Position } from 'reactflow';
import { Card, Tag } from 'antd';

export interface ValidationNodeData {
  assertions: Assertion[];
  onFail: 'stop' | 'retry' | 'continue';
  maxRetries: number;
}

export interface Assertion {
  type: 'equals' | 'contains' | 'matches' | 'exists' | 'custom';
  expected?: string;
  actual?: string;
  expression?: string; // 自定义 JS 表达式
}

export const ValidationNode = memo(({ data }: { data: ValidationNodeData }) => {
  return (
    <Card size="small" style={{ minWidth: 200 }}>
      <Handle type="target" position={Position.Top} />

      <div>
        <Tag color="blue">验证</Tag>
        <div style={{ marginTop: 8 }}>
          {data.assertions.length} 个断言
        </div>
        <div style={{ fontSize: 12, color: '#666' }}>
          失败策略: {data.onFail}
          {data.onFail === 'retry' && ` (最多${data.maxRetries}次)`}
        </div>
      </div>

      <Handle type="source" position={Position.Bottom} />
      <Handle
        type="source"
        position={Position.Bottom}
        id="fail"
        style={{ left: '30%' }}
      />
    </Card>
  );
});
```

### 4.2 验证执行器

**文件**: `src-tauri/crates/runtime/src/validation_executor.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    pub assertion_type: AssertionType,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub expression: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    Equals,
    Contains,
    Matches,
    Exists,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub failed_assertions: Vec<FailedAssertion>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedAssertion {
    pub assertion: Assertion,
    pub error: String,
}

pub struct ValidationExecutor;

impl ValidationExecutor {
    pub fn validate(
        assertions: &[Assertion],
        context: &serde_json::Value,
    ) -> ValidationResult {
        let start = std::time::Instant::now();
        let mut failed_assertions = Vec::new();

        for assertion in assertions {
            if let Err(e) = Self::check_assertion(assertion, context) {
                failed_assertions.push(FailedAssertion {
                    assertion: assertion.clone(),
                    error: e,
                });
            }
        }

        ValidationResult {
            passed: failed_assertions.is_empty(),
            failed_assertions,
            execution_time_ms: start.elapsed().as_millis() as u64,
        }
    }

    fn check_assertion(
        assertion: &Assertion,
        context: &serde_json::Value,
    ) -> Result<(), String> {
        match assertion.assertion_type {
            AssertionType::Equals => {
                let expected = assertion.expected.as_ref().ok_or("Missing expected value")?;
                let actual = context
                    .get(&assertion.actual.as_ref().ok_or("Missing actual path")?)
                    .and_then(|v| v.as_str())
                    .ok_or("Cannot extract actual value");

                if actual == expected {
                    Ok(())
                } else {
                    Err(format!("Expected '{}' but got '{}'", expected, actual))
                }
            }
            AssertionType::Contains => {
                let expected = assertion.expected.as_ref().ok_or("Missing expected value")?;
                let actual = context
                    .get(assertion.actual.as_ref().ok_or("Missing actual path")?)
                    .and_then(|v| v.as_str())
                    .ok_or("Cannot extract actual value");

                if actual.contains(expected) {
                    Ok(())
                } else {
                    Err(format!("'{}' does not contain '{}'", actual, expected))
                }
            }
            AssertionType::Matches => {
                let pattern = assertion.expected.as_ref().ok_or("Missing regex pattern")?;
                let actual = context
                    .get(assertion.actual.as_ref().ok_or("Missing actual path")?)
                    .and_then(|v| v.as_str())
                    .ok_or("Cannot extract actual value");

                if let Ok(re) = regex::Regex::new(pattern) {
                    if re.is_match(actual) {
                        Ok(())
                    } else {
                        Err(format!("'{}' does not match pattern '{}'", actual, pattern))
                    }
                } else {
                    Err(format!("Invalid regex pattern: {}", pattern))
                }
            }
            AssertionType::Exists => {
                if context.get(assertion.actual.as_ref().ok_or("Missing path")?).is_some() {
                    Ok(())
                } else {
                    Err(format!("Path '{}' does not exist", assertion.actual.as_ref().unwrap()))
                }
            }
            AssertionType::Custom => {
                // 使用 JS 表达式验证（需要沙箱执行）
                Err("Custom assertions require sandbox execution".to_string())
            }
        }
    }
}
```

---

## 5. 验收标准汇总

### 5.1 安全修复

| 项目 | 验收标准 | 测试方法 |
|------|---------|---------|
| SQL 注入修复 | 所有 FTS 查询使用参数化 | 代码审查 + fuzzing 测试 |
| 路径遍历修复 | 超出目录的路径访问被拒绝 | 单元测试 |
| Base64 限制 | 超过 100MB 的输入被拒绝 | 边界测试 |

### 5.2 Artifact 增强

| 项目 | 验收标准 | 测试方法 |
|------|---------|---------|
| 格式扩展 | 支持 HTML/CSS/JS/TS/React 预览 | 功能测试 |
| 实时预览 | 代码修改后预览即时更新 | 集成测试 |
| Monaco 集成 | 编辑器功能完整（高亮/补全） | 手动测试 |

### 5.3 代码解释器

| 项目 | 验收标准 | 测试方法 |
|------|---------|---------|
| JS 执行 | 正确执行并返回结果 | 单元测试 |
| Python 执行 | Pyodide 正确加载和执行 | 集成测试 |
| 超时控制 | 30 秒超时正确生效 | 边界测试 |

### 5.4 验证机制

| 项目 | 验收标准 | 测试方法 |
|------|---------|---------|
| 断言类型 | 支持 equals/contains/matches/exists | 单元测试 |
| 失败策略 | stop/retry/continue 正确执行 | 集成测试 |
| 错误信息 | 清晰的失败原因描述 | 手动测试 |

---

## 6. 任务分解

### Week 1: 安全修复

| 任务 | 负责人 | 预计时间 | 依赖 |
|------|--------|---------|------|
| SQL 注入修复 | AI | 0.5d | - |
| 路径遍历修复 | AI | 0.5d | - |
| Base64 限制添加 | AI | 0.5d | - |
| 安全测试编写 | AI | 0.5d | 修复完成 |

### Week 2: Artifact 增强

| 任务 | 负责人 | 预计时间 | 依赖 |
|------|--------|---------|------|
| ArtifactFormat 扩展 | AI | 0.5d | - |
| ArtifactPreview 组件 | AI | 2d | - |
| Monaco 集成 | AI | 1d | - |
| HTML 渲染引擎 | AI | 1d | - |

### Week 3: 代码解释器

| 任务 | 负责人 | 预计时间 | 依赖 |
|------|--------|---------|------|
| 沙箱进程管理 | AI | 1d | - |
| 前端执行器 | AI | 1d | - |
| Pyodide 集成 | AI | 1d | - |
| 执行面板 UI | AI | 1d | 前端执行器 |

### Week 4: 验证机制 + 收尾

| 任务 | 负责人 | 预计时间 | 依赖 |
|------|--------|---------|------|
| 验证节点组件 | AI | 0.5d | - |
| 验证执行器 | AI | 1d | - |
| 集成测试 | AI | 1d | 所有功能 |
| 文档更新 | AI | 0.5d | - |

---

## 7. 风险与注意事项

### 7.1 技术风险

| 风险 | 影响 | 缓解方案 |
|------|------|---------|
| Pyodide 加载慢 | 用户体验 | 懒加载 + loading 状态 |
| 沙箱执行不安全 | 安全 | 进程隔离 + 资源限制 |
| Monaco 性能问题 | 性能 | 按需加载 + 虚拟化 |

### 7.2 注意事项

1. **安全第一**: 所有用户输入都必须验证和清理
2. **向后兼容**: 扩展不影响现有功能
3. **性能**: 注意大文件处理和渲染性能
4. **用户体验**: 提供清晰的 loading 和错误状态