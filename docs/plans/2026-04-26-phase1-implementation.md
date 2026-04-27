# Phase 1: 多模态输出能力 - 详细实施计划

> 阶段: Phase 1
> 时间: 2026-05-15 至 2026-07-15（8 周）
> 前置: Phase 0 已完成（安全修复、Artifact 基础增强、代码解释器）
> 目标: 实现与 Claude Artifacts 类似的多模态输出体验

---

## 现有代码基线

Phase 0 已交付的基础设施：

| 模块 | 文件 | 状态 |
|------|------|------|
| Artifact 类型 | `src/types/artifact.ts` | ✅ 已扩展格式（html/css/jsx/tsx/python/svg/mermaid/d2） |
| Artifact Store | `src/stores/shared/artifactStore.ts` | ✅ 已增加 previewArtifact/previewMode/executeCode |
| ArtifactPreview | `src/components/chat/ArtifactPreview/` | ✅ 4 个子组件（index/CodePreview/HtmlPreview/SplitView/MarkdownPreview） |
| MonacoEditor | `src/components/shared/MonacoEditor.tsx` | ✅ 已集成，支持 12 种语言 |
| 代码解释器 | `src/lib/codeExecutor.ts` + Rust sandbox | ✅ JS/Python 执行 |
| Provider 适配器 | `src-tauri/crates/providers/` | ✅ 8 个 provider，已支持 image_urls 输入 |

**现有缺口**（Phase 1 需填补）：
- ❌ 无图像生成能力（仅有图像输入）
- ❌ HtmlPreview 只有基础 iframe 渲染，无 React 组件预览
- ❌ 无图表生成系统
- ❌ Artifact 面板与聊天流未深度集成（未自动识别生成内容类型）
- ❌ MarkdownPreview 使用简陋正则替换，需升级

---

## 模块 1: 图像生成集成（Week 1-3）

### 1.1 架构设计

```
用户提示 → LLM 识别图像生成意图 → 调用 image_gen 工具
                                          ↓
                                   Rust ImageGenProvider
                                          ↓
                              ┌───────────┼───────────┐
                              ↓           ↓           ↓
                          Flux API    DALL-E API   Stable Diffusion
                          (Replicate)  (OpenAI)    (ComfyUI Local)
                              ↓           ↓           ↓
                           返回 base64/URL → 前端渲染 → 保存为 Artifact
```

### 1.2 Rust 后端：图像生成 Provider

**新增文件**: `src-tauri/crates/providers/src/image_gen.rs`

```rust
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// 图像生成请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: Option<u32>,      // 默认 1024
    pub height: Option<u32>,     // 默认 1024
    pub steps: Option<u32>,      // 推理步数
    pub seed: Option<u64>,
    pub model: Option<String>,   // 覆盖默认模型
    pub n: Option<u32>,          // 生成数量，默认 1
}

/// 图像生成响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenResponse {
    pub images: Vec<GeneratedImage>,
    pub model_used: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub url: Option<String>,        // 远程 URL
    pub base64: Option<String>,     // base64 编码
    pub width: u32,
    pub height: u32,
    pub seed: Option<u64>,
}

/// Provider trait
#[async_trait]
pub trait ImageGenProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn generate(&self, request: ImageGenRequest) -> Result<ImageGenResponse>;
}

// --- Flux Provider (Replicate API) ---

pub struct FluxProvider {
    api_token: String,
    client: reqwest::Client,
}

impl FluxProvider {
    pub fn new(api_token: String) -> Self {
        Self {
            api_token,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct ReplicatePrediction {
    version: String,
    input: ReplicateInput,
}

#[derive(Serialize)]
struct ReplicateInput {
    prompt: String,
    negative_prompt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    num_inference_steps: Option<u32>,
    seed: Option<u64>,
}

#[derive(Deserialize)]
struct ReplicateResponse {
    id: String,
    status: String,
    output: Option<Vec<String>>,
}

#[async_trait]
impl ImageGenProvider for FluxProvider {
    fn name(&self) -> &str { "flux" }

    async fn generate(&self, request: ImageGenRequest) -> Result<ImageGenResponse> {
        let start = std::time::Instant::now();

        let prediction = ReplicatePrediction {
            version: "black-forest-labs/flux-schnell".to_string(),
            input: ReplicateInput {
                prompt: request.prompt,
                negative_prompt: request.negative_prompt,
                width: request.width.or(Some(1024)),
                height: request.height.or(Some(1024)),
                num_inference_steps: request.steps.or(Some(4)),
                seed: request.seed,
            },
        };

        // 1. 创建 prediction
        let resp = self.client
            .post("https://api.replicate.com/v1/predictions")
            .header("Authorization", format!("Token {}", self.api_token))
            .json(&prediction)
            .send()
            .await?;

        let mut replicate_resp: ReplicateResponse = resp.json().await?;

        // 2. 轮询直到完成
        let poll_url = format!("https://api.replicate.com/v1/predictions/{}", replicate_resp.id);
        for _ in 0..60 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let poll_resp = self.client
                .get(&poll_url)
                .header("Authorization", format!("Token {}", self.api_token))
                .send()
                .await?;
            replicate_resp = poll_resp.json().await?;
            if replicate_resp.status == "succeeded" || replicate_resp.status == "failed" {
                break;
            }
        }

        let images = replicate_resp.output
            .unwrap_or_default()
            .into_iter()
            .map(|url| GeneratedImage {
                url: Some(url),
                base64: None,
                width: request.width.unwrap_or(1024),
                height: request.height.unwrap_or(1024),
                seed: request.seed,
            })
            .collect();

        Ok(ImageGenResponse {
            images,
            model_used: "flux-schnell".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }
}

// --- DALL-E Provider (OpenAI API) ---

pub struct DallEProvider {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl DallEProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ImageGenProvider for DallEProvider {
    fn name(&self) -> &str { "dall-e" }

    async fn generate(&self, request: ImageGenRequest) -> Result<ImageGenResponse> {
        let start = std::time::Instant::now();

        let body = serde_json::json!({
            "model": request.model.as_deref().unwrap_or("dall-e-3"),
            "prompt": request.prompt,
            "n": request.n.unwrap_or(1),
            "size": format!("{}x{}", request.width.unwrap_or(1024), request.height.unwrap_or(1024)),
            "quality": "standard",
            "response_format": "b64_json"
        });

        let resp = self.client
            .post(format!("{}/images/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct DallEResponse {
            data: Vec<DallEImage>,
        }
        #[derive(Deserialize)]
        struct DallEImage {
            b64_json: Option<String>,
            url: Option<String>,
            revised_prompt: Option<String>,
        }

        let dalle_resp: DallEResponse = resp.json().await?;

        let images = dalle_resp.data.into_iter().map(|img| GeneratedImage {
            url: img.url,
            base64: img.b64_json,
            width: request.width.unwrap_or(1024),
            height: request.height.unwrap_or(1024),
            seed: None,
        }).collect();

        Ok(ImageGenResponse {
            images,
            model_used: "dall-e-3".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }
}
```

**修改文件**: `src-tauri/crates/providers/src/lib.rs`

```rust
// 添加模块声明
pub mod image_gen;

// 重新导出
pub use image_gen::{
    ImageGenProvider, ImageGenRequest, ImageGenResponse, GeneratedImage,
    FluxProvider, DallEProvider,
};
```

### 1.3 Tauri 命令

**新增文件**: `src-tauri/src/commands/image_gen.rs`

```rust
use axagent_providers::image_gen::*;
use axagent_core::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn generate_image(
    state: State<'_, AppState>,
    prompt: String,
    negative_prompt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    steps: Option<u32>,
    seed: Option<u64>,
    model: Option<String>,
    provider: Option<String>,  // "flux" | "dall-e"
) -> Result<ImageGenResponse, String> {
    let provider = state.image_gen_provider(provider.as_deref())
        .map_err(|e| e.to_string())?;

    let request = ImageGenRequest {
        prompt,
        negative_prompt,
        width,
        height,
        steps,
        seed,
        model,
        n: Some(1),
    };

    provider.generate(request)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_image_gen_models(
    state: State<'_, AppState>,
) -> Result<Vec<ImageGenModelInfo>, String> {
    state.list_image_gen_models()
        .await
        .map_err(|e| e.to_string())
}
```

### 1.4 前端：图像生成 UI

**新增文件**: `src/components/chat/ImageGenPanel.tsx`

```typescript
import { useState } from "react";
import { Button, Input, Select, Slider, Space, Typography, Image, message } from "antd";
import { invoke } from "@/lib/invoke";

interface GeneratedImage {
  url?: string;
  base64?: string;
  width: number;
  height: number;
  seed?: number;
}

interface ImageGenResult {
  images: GeneratedImage[];
  model_used: string;
  elapsed_ms: number;
}

const SIZE_PRESETS = [
  { label: "1:1 (1024×1024)", width: 1024, height: 1024 },
  { label: "16:9 (1344×768)", width: 1344, height: 768 },
  { label: "9:16 (768×1344)", width: 768, height: 1344 },
  { label: "4:3 (1152×896)", width: 1152, height: 896 },
];

const PROVIDERS = [
  { value: "flux", label: "Flux (Replicate)" },
  { value: "dall-e", label: "DALL-E 3 (OpenAI)" },
];

export function ImageGenPanel() {
  const [prompt, setPrompt] = useState("");
  const [negativePrompt, setNegativePrompt] = useState("");
  const [provider, setProvider] = useState("flux");
  const [sizePreset, setSizePreset] = useState(0);
  const [steps, setSteps] = useState(4);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ImageGenResult | null>(null);

  const handleGenerate = async () => {
    if (!prompt.trim()) return;
    setLoading(true);
    try {
      const res = await invoke<ImageGenResult>("generate_image", {
        prompt,
        negative_prompt: negativePrompt || undefined,
        width: SIZE_PRESETS[sizePreset].width,
        height: SIZE_PRESETS[sizePreset].height,
        steps,
        provider,
      });
      setResult(res);
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}>
      <Space>
        <Select value={provider} onChange={setProvider} options={PROVIDERS} style={{ width: 200 }} />
        <Select
          value={sizePreset}
          onChange={setSizePreset}
          options={SIZE_PRESETS.map((s, i) => ({ value: i, label: s.label }))}
          style={{ width: 180 }}
        />
      </Space>

      <Input.TextArea
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        placeholder="描述你想生成的图片..."
        rows={3}
      />

      <Input
        value={negativePrompt}
        onChange={(e) => setNegativePrompt(e.target.value)}
        placeholder="负面提示词（可选）"
      />

      {provider === "flux" && (
        <div>
          <Typography.Text type="secondary">推理步数: {steps}</Typography.Text>
          <Slider min={1} max={50} value={steps} onChange={setSteps} />
        </div>
      )}

      <Button type="primary" onClick={handleGenerate} loading={loading} block>
        生成图片
      </Button>

      {result && (
        <div>
          <Typography.Text type="secondary">
            模型: {result.model_used} | 耗时: {(result.elapsed_ms / 1000).toFixed(1)}s
          </Typography.Text>
          <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 8 }}>
            {result.images.map((img, i) => (
              <Image
                key={i}
                src={img.base64 ? `data:image/png;base64,${img.base64}` : img.url}
                width={256}
                style={{ borderRadius: 8 }}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
```

**新增文件**: `src/stores/feature/imageGenStore.ts`

```typescript
import { invoke } from "@/lib/invoke";
import { create } from "zustand";

interface GeneratedImage {
  url?: string;
  base64?: string;
  width: number;
  height: number;
  seed?: number;
}

interface ImageGenState {
  generating: boolean;
  results: GeneratedImage[];
  history: Array<{
    prompt: string;
    images: GeneratedImage[];
    model: string;
    timestamp: number;
  }>;

  generate: (params: {
    prompt: string;
    negative_prompt?: string;
    width?: number;
    height?: number;
    provider?: string;
  }) => Promise<GeneratedImage[]>;
}

export const useImageGenStore = create<ImageGenState>((set, get) => ({
  generating: false,
  results: [],
  history: [],

  generate: async (params) => {
    set({ generating: true });
    try {
      const res = await invoke<{
        images: GeneratedImage[];
        model_used: string;
        elapsed_ms: number;
      }>("generate_image", {
        prompt: params.prompt,
        negative_prompt: params.negative_prompt,
        width: params.width,
        height: params.height,
        provider: params.provider,
      });

      set((s) => ({
        results: res.images,
        history: [
          ...s.history,
          {
            prompt: params.prompt,
            images: res.images,
            model: res.model_used,
            timestamp: Date.now(),
          },
        ],
      }));

      return res.images;
    } finally {
      set({ generating: false });
    }
  },
}));
```

### 1.5 图像生成内置工具注册

**修改文件**: `src-tauri/crates/core/src/builtin_tools.rs`

在 `BUILTIN_TOOL_DEFS` 中添加：

```rust
ToolDef {
    name: "generate_image",
    description: "Generate an image from a text prompt using AI image generation models (Flux, DALL-E, etc.)",
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "prompt": {
                "type": "string",
                "description": "Text description of the image to generate"
            },
            "negative_prompt": {
                "type": "string",
                "description": "What to avoid in the generated image"
            },
            "width": {
                "type": "integer",
                "description": "Image width in pixels (default: 1024)",
                "enum": [512, 768, 896, 1024, 1344]
            },
            "height": {
                "type": "integer",
                "description": "Image height in pixels (default: 1024)",
                "enum": [512, 768, 896, 1024, 1344]
            },
            "style": {
                "type": "string",
                "description": "Visual style preset",
                "enum": ["photorealistic", "digital-art", "anime", "oil-painting", "watercolor", "pixel-art"]
            }
        },
        "required": ["prompt"]
    }),
},
```

---

## 模块 2: Artifact 实时预览增强（Week 3-5）

### 2.1 增强 HtmlPreview：智能 HTML/CSS/JS 分离

**修改文件**: `src/components/chat/ArtifactPreview/HtmlPreview.tsx`

当前问题：HtmlPreview 只是简单拼接 html/css/js，需要增加：
- 智能从完整 HTML 文档中提取 `<style>` 和 `<script>` 标签
- 支持热更新（代码变更后自动刷新预览）
- 支持错误边界（JS 执行错误不崩溃 iframe）

```typescript
// 新增: src/lib/htmlParser.ts
/**
 * 从 HTML 内容中分离出 head 中的 style、body 中的 script
 * 以及 body 内容，用于 split view 独立编辑
 */
export interface ParsedHtml {
  html: string;   // body 内容
  css: string;    // 所有 <style> 合并
  js: string;     // 所有 <script> 合并
  full: string;   // 原始完整 HTML
}

export function parseHtmlContent(content: string): ParsedHtml {
  const parser = new DOMParser();
  const doc = parser.parseFromString(content, "text/html");

  const css = Array.from(doc.querySelectorAll("style"))
    .map((el) => el.textContent || "")
    .join("\n");

  const js = Array.from(doc.querySelectorAll("script:not([src])"))
    .map((el) => el.textContent || "")
    .join("\n");

  // 移除 script 和 style 标签后获取 body
  doc.querySelectorAll("style, script:not([src])").forEach((el) => el.remove());
  const html = doc.body?.innerHTML || content;

  return { html, css, js, full: content };
}

/**
 * 将分离的 html/css/js 重新组合为完整文档
 */
export function composeHtml(parts: Partial<ParsedHtml>): string {
  const html = parts.html || "";
  const css = parts.css || "";
  const js = parts.js || "";

  return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 16px; }
${css}
</style>
</head>
<body>
${html}
<script>
try { ${js} } catch(e) { document.body.innerHTML += '<pre style="color:red">Error: ' + e.message + '</pre>'; }
</script>
</body>
</html>`;
}
```

### 2.2 新增 ReactPreview 组件

**新增文件**: `src/components/chat/ArtifactPreview/ReactPreview.tsx`

核心功能：将 TSX 代码通过 Babel 转换后渲染到 iframe 中。

```typescript
import { memo, useEffect, useRef, useCallback } from "react";

interface ReactPreviewProps {
  code: string;      // TSX/JSX 源码
  css?: string;
  onError?: (error: string) => void;
}

/**
 * 在 iframe 沙箱中渲染 React 组件。
 * 使用 Babel standalone 在 iframe 内转换 JSX。
 */
export const ReactPreview = memo(function ReactPreview({
  code,
  css,
  onError,
}: ReactPreviewProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);

  const buildSrcDoc = useCallback(() => {
    return `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; padding: 16px; }
${css || ""}
</style>
<script src="https://unpkg.com/react@18/umd/react.development.js"><\/script>
<script src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"><\/script>
<script src="https://unpkg.com/@babel/standalone/babel.min.js"><\/script>
</head>
<body>
<div id="root"></div>
<script>
window.onerror = function(msg, src, line, col, err) {
  window.parent.postMessage({ type: 'react-preview-error', message: String(msg) }, '*');
};
try {
  var transformed = Babel.transform(${JSON.stringify(code)}, {
    presets: ['react'],
    filename: 'component.tsx'
  });
  var fn = new Function('React', 'ReactDOM', transformed.code);
  fn(React, ReactDOM);
} catch(e) {
  document.getElementById('root').innerHTML = '<pre style="color:red;padding:16px">' + e.message + '</pre>';
  window.parent.postMessage({ type: 'react-preview-error', message: e.message }, '*');
}
<\/script>
</body>
</html>`;
  }, [code, css]);

  useEffect(() => {
    if (iframeRef.current) {
      iframeRef.current.srcdoc = buildSrcDoc();
    }
  }, [buildSrcDoc]);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      if (event.data?.type === "react-preview-error") {
        onError?.(event.data.message);
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [onError]);

  return (
    <iframe
      ref={iframeRef}
      sandbox="allow-scripts allow-same-origin"
      style={{
        width: "100%",
        height: "100%",
        border: "none",
        background: "#fff",
        borderRadius: 8,
      }}
    />
  );
});
```

### 2.3 新增 ChartPreview 组件

**新增文件**: `src/components/chat/ArtifactPreview/ChartPreview.tsx`

```typescript
import { memo, useEffect, useRef } from "react";

interface ChartPreviewProps {
  /** ECharts option JSON 对象 */
  option: Record<string, unknown>;
  /** 图表宽度 */
  width?: number;
  /** 图表高度 */
  height?: number;
  /** 主题: light | dark */
  theme?: "light" | "dark";
}

/**
 * 在 iframe 中渲染 ECharts 图表。
 * 避免在主窗口加载 echarts 库（~1MB）。
 */
export const ChartPreview = memo(function ChartPreview({
  option,
  width,
  height,
  theme = "light",
}: ChartPreviewProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);

  useEffect(() => {
    if (iframeRef.current) {
      const bgColor = theme === "dark" ? "#1e1e1e" : "#ffffff";
      const textColor = theme === "dark" ? "#ccc" : "#333";

      iframeRef.current.srcdoc = `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<script src="https://cdn.jsdelivr.net/npm/echarts@5/dist/echarts.min.js"><\/script>
<style>
  body { margin: 0; background: ${bgColor}; }
  #chart { width: 100%; height: 100%; }
</style>
</head>
<body>
<div id="chart"></div>
<script>
var chart = echarts.init(document.getElementById('chart'), null, { renderer: 'canvas' });
var option = ${JSON.stringify(option)};
option.color = option.color || ['#5470c6','#91cc75','#fac858','#ee6666','#73c0de','#3ba272'];
if (!option.textStyle) option.textStyle = { color: '${textColor}' };
chart.setOption(option);
window.addEventListener('resize', function() { chart.resize(); });
<\/script>
</body>
</html>`;
    }
  }, [option, theme]);

  return (
    <iframe
      ref={iframeRef}
      sandbox="allow-scripts"
      style={{
        width: width || "100%",
        height: height || 400,
        border: "none",
        borderRadius: 8,
      }}
    />
  );
});
```

### 2.4 升级 ArtifactPreview 入口

**修改文件**: `src/components/chat/ArtifactPreview/index.tsx`

增加 React 和 Chart 格式的路由：

```typescript
import type { Artifact, ArtifactLanguage } from "@/types/artifact";
import { memo } from "react";
import { ChartPreview } from "./ChartPreview";
import { CodePreview } from "./CodePreview";
import { HtmlPreview } from "./HtmlPreview";
import { MarkdownPreview } from "./MarkdownPreview";
import { ReactPreview } from "./ReactPreview";
import { SplitView } from "./SplitView";

interface ArtifactPreviewProps {
  artifact: Artifact;
  previewMode?: "split" | "preview" | "code";
  onContentChange?: (content: string) => void;
}

export const ArtifactPreview = memo(function ArtifactPreview({
  artifact,
  previewMode = "code",
  onContentChange,
}: ArtifactPreviewProps) {
  const language = artifact.language || artifact.format;

  // 图表类型：检测内容中是否有 ECharts option 或 chart 配置
  if (artifact.format === "json" && isChartOption(artifact.content) && previewMode !== "code") {
    try {
      const option = JSON.parse(artifact.content);
      return <ChartPreview option={option} />;
    } catch { /* fall through */ }
  }

  // SVG 预览
  if (artifact.format === "svg" && previewMode !== "code") {
    return (
      <div style={{ padding: 16, background: "#fff" }} dangerouslySetInnerHTML={{ __html: artifact.content }} />
    );
  }

  // React/TSX 组件预览
  if ((artifact.format === "jsx" || artifact.format === "tsx") && previewMode !== "code") {
    return <ReactPreview code={artifact.content} />;
  }

  // HTML 预览
  if (artifact.format === "html" && previewMode !== "code") {
    return (
      <HtmlPreview
        html={artifact.content}
        language={language as ArtifactLanguage}
        previewMode={previewMode}
      />
    );
  }

  // Markdown 预览
  if ((artifact.format === "markdown" || artifact.format === "text") && previewMode === "preview") {
    return <MarkdownPreview content={artifact.content} />;
  }

  // 分栏模式
  if (previewMode === "split") {
    return (
      <SplitView
        code={artifact.content}
        language={language as ArtifactLanguage}
        onChange={onContentChange}
      />
    );
  }

  // 默认代码模式
  return (
    <CodePreview
      code={artifact.content}
      language={language as ArtifactLanguage}
      readOnly={!onContentChange}
      onChange={onContentChange}
    />
  );
});

/** 检测 JSON 内容是否为 ECharts option */
function isChartOption(content: string): boolean {
  try {
    const obj = JSON.parse(content);
    return !!(obj.series || obj.xAxis || obj.yAxis || obj.polar || obj.radiusAxis);
  } catch {
    return false;
  }
}
```

### 2.5 升级 MarkdownPreview

**修改文件**: `src/components/chat/ArtifactPreview/MarkdownPreview.tsx`

当前问题：使用简陋正则替换 HTML，需要升级为真正的 Markdown 渲染。项目已有 `markstream-react` 依赖。

```typescript
import { Markdown } from "markstream-react";
import { memo } from "react";

interface MarkdownPreviewProps {
  content: string;
}

export const MarkdownPreview = memo(function MarkdownPreview({ content }: MarkdownPreviewProps) {
  return (
    <div style={{ padding: 16, overflow: "auto", maxWidth: "100%" }}>
      <Markdown content={content} />
    </div>
  );
});
```

### 2.6 Artifact 面板深度集成

**修改文件**: `src/components/chat/ArtifactPanel.tsx`

将当前简陋的 Card 改为完整的 Artifact 工作区：

```typescript
import type { Artifact } from "@/types/artifact";
import { useArtifactStore } from "@/stores/shared/artifactStore";
import { ArtifactPreview } from "./ArtifactPreview";
import { Button, Segmented, Space, Typography } from "antd";
import { Copy, Download, Expand, Pin, Code, Eye, Split } from "lucide-react";

interface ArtifactPanelProps {
  artifact?: Artifact;
}

export function ArtifactPanel({ artifact }: ArtifactPanelProps) {
  const { previewMode, setPreviewMode, updateArtifact, deleteArtifact } = useArtifactStore();

  if (!artifact) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#999" }}>
        <Typography.Text type="secondary">选择一个 Artifact 预览</Typography.Text>
      </div>
    );
  }

  const handleContentChange = (content: string) => {
    updateArtifact(artifact.id, { content });
  };

  const handleCopy = () => {
    navigator.clipboard.writeText(artifact.content);
  };

  const handleDownload = () => {
    const ext = getExtension(artifact.format);
    const blob = new Blob([artifact.content], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${artifact.title}.${ext}`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      {/* 工具栏 */}
      <div style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "8px 12px",
        borderBottom: "1px solid var(--ant-color-border)",
      }}>
        <Space>
          <Typography.Text strong ellipsis style={{ maxWidth: 200 }}>
            {artifact.title}
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {artifact.format}
          </Typography.Text>
        </Space>

        <Space size={4}>
          <Segmented
            size="small"
            value={previewMode}
            onChange={(v) => setPreviewMode(v as "split" | "preview" | "code")}
            options={[
              { value: "code", icon: <Code size={14} /> },
              { value: "preview", icon: <Eye size={14} /> },
              { value: "split", icon: <Split size={14} /> },
            ]}
          />
          <Button type="text" size="small" icon={<Copy size={14} />} onClick={handleCopy} />
          <Button type="text" size="small" icon={<Download size={14} />} onClick={handleDownload} />
        </Space>
      </div>

      {/* 内容区 */}
      <div style={{ flex: 1, overflow: "hidden" }}>
        <ArtifactPreview
          artifact={artifact}
          previewMode={previewMode}
          onContentChange={handleContentChange}
        />
      </div>
    </div>
  );
}

function getExtension(format: string): string {
  const map: Record<string, string> = {
    markdown: "md", javascript: "js", typescript: "ts", jsx: "jsx", tsx: "tsx",
    html: "html", css: "css", python: "py", json: "json", svg: "svg",
    mermaid: "mmd", d2: "d2", text: "txt",
  };
  return map[format] || format;
}
```

---

## 模块 3: 图表生成系统（Week 5-8）

### 3.1 自然语言 → 图表配置

**新增文件**: `src/lib/chartGenerator.ts`

```typescript
import { invoke } from "@/lib/invoke";

/**
 * 图表生成请求
 */
export interface ChartGenRequest {
  description: string;    // 自然语言描述
  data?: Record<string, unknown>[];  // 数据（可选，可由描述中提取）
  chartType?: ChartType;  // 图表类型（可选，自动推断）
  title?: string;
}

export type ChartType =
  | "line" | "bar" | "pie" | "scatter"
  | "heatmap" | "radar" | "treemap"
  | "sankey" | "funnel" | "gauge";

export interface ChartGenResult {
  option: Record<string, unknown>;  // ECharts option
  chartType: ChartType;
  title: string;
}

/**
 * 使用 LLM 将自然语言描述转换为 ECharts option。
 * 流程：描述 + 数据 → LLM → 结构化 option JSON
 */
export async function generateChart(request: ChartGenRequest): Promise<ChartGenResult> {
  const systemPrompt = `You are a chart configuration generator. Given a natural language description and optional data, generate a valid ECharts option object.

Rules:
1. Output ONLY valid JSON (no markdown, no code fences)
2. The JSON must be a valid ECharts option
3. Use Chinese labels when the description is in Chinese
4. Include proper axis labels, legends, and tooltips
5. Use color palette: ['#5470c6','#91cc75','#fac858','#ee6666','#73c0de','#3ba272']
6. Set animation: false for performance
7. Include a "chartType" field with the inferred type
8. Include a "title" field with the chart title

Chart type mapping:
- "趋势" / "变化" / "增长" → line chart
- "对比" / "比较" / "排名" → bar chart
- "占比" / "比例" / "分布" → pie chart
- "关系" / "关联" / "相关性" → scatter chart
- "热力" / "密度" → heatmap`;

  const userMessage = request.data
    ? `Description: ${request.description}\n\nData:\n${JSON.stringify(request.data, null, 2)}`
    : `Description: ${request.description}`;

  // 调用已有的 LLM 基础设施生成图表配置
  const result = await invoke<ChartGenResult>("generate_chart_config", {
    system_prompt: systemPrompt,
    user_message: userMessage,
    chart_type: request.chartType,
  });

  return result;
}

/**
 * 从用户消息中检测图表生成意图
 */
export function detectChartIntent(message: string): ChartGenRequest | null {
  const patterns = [
    /(?:画|生成|绘制|创建|做一个|show)\s*(?:一个|一张)?\s*(.+?)\s*(?:图表|图|chart|graph)/i,
    /(?:可视化|visualize)\s+(.+)/i,
    /(.+?)\s*(?:的趋势|对比|分布|占比)(?:图)?/i,
  ];

  for (const pattern of patterns) {
    const match = message.match(pattern);
    if (match) {
      return {
        description: match[1] || message,
        chartType: inferChartType(message),
      };
    }
  }

  return null;
}

function inferChartType(message: string): ChartType | undefined {
  if (/趋势|变化|增长|时间|折线|line/i.test(message)) return "line";
  if (/对比|比较|排名|柱状|bar/i.test(message)) return "bar";
  if (/占比|比例|分布|饼图|pie/i.test(message)) return "pie";
  if (/关系|关联|散点|scatter/i.test(message)) return "scatter";
  if (/热力|密度|heatmap/i.test(message)) return "heatmap";
  return undefined;
}
```

### 3.2 Rust 后端：图表配置生成

**新增文件**: `src-tauri/crates/core/src/chart_generator.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChartGenRequest {
    pub description: String,
    pub data: Option<serde_json::Value>,
    pub chart_type: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChartGenResult {
    pub option: serde_json::Value,
    pub chart_type: String,
    pub title: String,
}

/// 使用 LLM 生成图表配置
pub async fn generate_chart_config(
    provider: &dyn axagent_providers::ProviderAdapter,
    ctx: &axagent_providers::ProviderRequestContext,
    request: ChartGenRequest,
) -> Result<ChartGenResult> {
    let system_prompt = r#"You are a chart configuration generator. Given a natural language description and optional data, generate a valid ECharts option object.

Rules:
1. Output ONLY valid JSON (no markdown, no code fences)
2. The JSON must be a valid ECharts option
3. Use Chinese labels when the description is in Chinese
4. Include proper axis labels, legends, and tooltips
5. Use color palette: ['#5470c6','#91cc75','#fac858','#ee6666','#73c0de','#3ba272']
6. Set animation: false
7. Include "_chartType" field with the inferred type (line/bar/pie/scatter/heatmap/radar/treemap/sankey/funnel/gauge)
8. Include "_title" field with the chart title"#;

    let user_message = match &request.data {
        Some(data) => format!(
            "Description: {}\n\nData:\n{}",
            request.description,
            serde_json::to_string_pretty(data)?
        ),
        None => format!("Description: {}", request.description),
    };

    use axagent_core::types::{ChatContent, ChatMessage, ChatRequest};

    let chat_request = ChatRequest {
        model: String::new(), // 使用默认模型
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(system_prompt.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(user_message),
                tool_calls: None,
                tool_call_id: None,
            },
        ],
        temperature: Some(0.1), // 低温度确保输出稳定
        stream: false,
        ..Default::default()
    };

    // 同步调用 LLM
    let response = provider.chat(ctx, chat_request).await?;

    // 提取响应文本
    let text = response.content
        .ok_or_else(|| anyhow::anyhow!("Empty response from LLM"))?;

    // 清理可能的 markdown code fence
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 解析 JSON
    let mut option: serde_json::Value = serde_json::from_str(cleaned)?;

    // 提取元数据
    let chart_type = option["_chartType"].as_str().unwrap_or("bar").to_string();
    let title = option["_title"].as_str().unwrap_or(&request.description).to_string();

    // 移除内部字段
    if let Some(obj) = option.as_object_mut() {
        obj.remove("_chartType");
        obj.remove("_title");
    }

    Ok(ChartGenResult {
        option,
        chart_type,
        title,
    })
}
```

### 3.3 图表生成内置工具

**修改文件**: `src-tauri/crates/core/src/builtin_tools.rs`

添加图表生成工具定义：

```rust
ToolDef {
    name: "generate_chart",
    description: "Generate an interactive chart (ECharts) from natural language description or data. Supports line, bar, pie, scatter, heatmap, radar, treemap, sankey, funnel, and gauge charts.",
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "description": {
                "type": "string",
                "description": "Natural language description of the chart to generate"
            },
            "data": {
                "type": "array",
                "description": "Data array for the chart (optional, can be inferred from description)",
                "items": { "type": "object" }
            },
            "chart_type": {
                "type": "string",
                "description": "Chart type",
                "enum": ["line", "bar", "pie", "scatter", "heatmap", "radar", "treemap", "sankey", "funnel", "gauge"]
            },
            "title": {
                "type": "string",
                "description": "Chart title"
            }
        },
        "required": ["description"]
    }),
},
```

### 3.4 图表渲染组件

**新增文件**: `src/components/shared/ChartRenderer.tsx`

统一入口，支持从 ECharts option 或自然语言描述生成图表：

```typescript
import { useState, useEffect } from "react";
import { ChartPreview } from "@/components/chat/ArtifactPreview/ChartPreview";
import { generateChart, type ChartGenRequest } from "@/lib/chartGenerator";
import { Spin, Button } from "antd";

interface ChartRendererProps {
  /** 直接提供 ECharts option（跳过 LLM） */
  option?: Record<string, unknown>;
  /** 或提供自然语言描述（通过 LLM 生成 option） */
  request?: ChartGenRequest;
  width?: number;
  height?: number;
  theme?: "light" | "dark";
}

export function ChartRenderer({
  option: directOption,
  request,
  width,
  height,
  theme,
}: ChartRendererProps) {
  const [option, setOption] = useState(directOption);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (directOption) {
      setOption(directOption);
      return;
    }

    if (request) {
      setLoading(true);
      generateChart(request)
        .then((result) => {
          setOption(result.option);
        })
        .catch((err) => setError(String(err)))
        .finally(() => setLoading(false));
    }
  }, [directOption, request]);

  if (loading) return <Spin />;
  if (error) return <div style={{ color: "red", padding: 16 }}>图表生成失败: {error}</div>;
  if (!option) return null;

  return <ChartPreview option={option} width={width} height={height} theme={theme} />;
}
```

---

## 模块 4: 设置页面集成（Week 7-8）

### 4.1 图像生成设置

**新增文件**: `src/components/settings/ImageGenSettings.tsx`

```typescript
import { Button, Card, Form, Input, Select, Space, Switch, Typography } from "antd";
import { invoke } from "@/lib/invoke";

interface ImageGenConfig {
  default_provider: string;     // "flux" | "dall-e"
  flux_api_token: string;
  openai_api_key: string;
  openai_base_url: string;
  default_width: number;
  default_height: number;
  default_steps: number;
  save_to_artifact: boolean;
}

export function ImageGenSettings() {
  const [form] = Form.useForm<ImageGenConfig>();

  const handleSave = async () => {
    const values = await form.validateFields();
    await invoke("save_image_gen_config", { config: values });
  };

  return (
    <Card title="图像生成" style={{ marginBottom: 16 }}>
      <Form form={form} layout="vertical" onFinish={handleSave}>
        <Form.Item name="default_provider" label="默认 Provider">
          <Select options={[
            { value: "flux", label: "Flux (Replicate)" },
            { value: "dall-e", label: "DALL-E 3 (OpenAI)" },
          ]} />
        </Form.Item>

        <Form.Item name="flux_api_token" label="Replicate API Token">
          <Input.Password placeholder="r8_..." />
        </Form.Item>

        <Form.Item name="openai_api_key" label="OpenAI API Key (DALL-E)">
          <Input.Password placeholder="sk-..." />
        </Form.Item>

        <Form.Item name="openai_base_url" label="OpenAI Base URL">
          <Input placeholder="https://api.openai.com/v1" />
        </Form.Item>

        <Form.Item name="save_to_artifact" label="自动保存为 Artifact" valuePropName="checked">
          <Switch />
        </Form.Item>

        <Form.Item>
          <Button type="primary" htmlType="submit">保存</Button>
        </Form.Item>
      </Form>
    </Card>
  );
}
```

---

## 完整文件变更清单

### 新增文件（12 个）

| 文件 | 类型 | 描述 |
|------|------|------|
| `src-tauri/crates/providers/src/image_gen.rs` | Rust | 图像生成 provider（Flux + DALL-E） |
| `src-tauri/src/commands/image_gen.rs` | Rust | 图像生成 Tauri 命令 |
| `src-tauri/crates/core/src/chart_generator.rs` | Rust | 图表配置生成器 |
| `src/lib/htmlParser.ts` | TS | HTML 智能解析器 |
| `src/lib/chartGenerator.ts` | TS | 自然语言 → 图表配置 |
| `src/components/chat/ImageGenPanel.tsx` | TSX | 图像生成面板 |
| `src/components/chat/ArtifactPreview/ReactPreview.tsx` | TSX | React 组件预览 |
| `src/components/chat/ArtifactPreview/ChartPreview.tsx` | TSX | ECharts 图表预览 |
| `src/components/shared/ChartRenderer.tsx` | TSX | 图表渲染统一入口 |
| `src/stores/feature/imageGenStore.ts` | TS | 图像生成状态管理 |
| `src/components/settings/ImageGenSettings.tsx` | TSX | 图像生成设置页 |

### 修改文件（7 个）

| 文件 | 变更描述 |
|------|---------|
| `src-tauri/crates/providers/src/lib.rs` | 添加 image_gen 模块 |
| `src-tauri/crates/core/src/builtin_tools.rs` | 添加 generate_image/generate_chart 工具定义 |
| `src/components/chat/ArtifactPreview/index.tsx` | 增加 React/Chart/SVG 路由 |
| `src/components/chat/ArtifactPreview/HtmlPreview.tsx` | 增强智能解析 |
| `src/components/chat/ArtifactPreview/MarkdownPreview.tsx` | 升级为 markstream-react |
| `src/components/chat/ArtifactPanel.tsx` | 完整工作区 UI |
| `src-tauri/src/main.rs` | 注册 image_gen 命令 |

---

## 验收标准

### 模块 1: 图像生成

| 验收项 | 标准 |
|--------|------|
| Flux API 集成 | 输入提示词 → 返回图片 URL |
| DALL-E 集成 | 输入提示词 → 返回 base64 图片 |
| 工具调用 | LLM 可通过 generate_image 工具自动生成图片 |
| UI 面板 | 完整的参数面板（provider/尺寸/步数） |
| 设置持久化 | API Key 保存到本地加密存储 |

### 模块 2: Artifact 预览增强

| 验收项 | 标准 |
|--------|------|
| HTML 预览 | iframe 实时渲染，支持 CSS/JS |
| React 预览 | TSX 代码通过 Babel 转换后渲染 |
| Chart 预览 | ECharts option JSON 渲染为交互图表 |
| SVG 预览 | 直接内联渲染 |
| Markdown 预览 | 使用 markstream-react 渲染 |
| 分栏模式 | 代码+预览同时显示 |
| 错误边界 | JS 错误不崩溃，显示红色错误信息 |

### 模块 3: 图表生成

| 验收项 | 标准 |
|--------|------|
| 自然语言 → 图表 | "画一个销售趋势图" → 生成折线图 |
| 数据驱动 | 传入 JSON 数据 → 生成对应图表 |
| 图表类型推断 | 自动识别折线/柱状/饼图等 |
| 工具调用 | LLM 可通过 generate_chart 工具自动生成图表 |
| 交互性 | 图表支持 hover tooltip、缩放、图例筛选 |

---

## 周计划

| 周 | 任务 | 交付物 |
|----|------|--------|
| W1 | Rust image_gen provider（Flux） | `image_gen.rs` |
| W2 | Rust image_gen provider（DALL-E）+ Tauri 命令 | 命令注册 + API 可用 |
| W3 | 前端 ImageGenPanel + Store + 内置工具注册 | 图像生成 UI 完整可用 |
| W4 | ReactPreview + htmlParser + MarkdownPreview 升级 | 预览组件增强 |
| W5 | ChartPreview + ArtifactPanel 升级 + ArtifactPreview 路由 | Artifact 工作区完整 |
| W6 | Rust chart_generator + 内置工具定义 | 图表生成后端 |
| W7 | 前端 chartGenerator + ChartRenderer + 设置页面 | 图表生成全链路 |
| W8 | 集成测试 + Bug 修复 + 文档更新 | Phase 1 完整交付 |
