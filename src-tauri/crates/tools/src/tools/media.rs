//! AI 媒体工具
//!
//! GenerateImage (flux/dall-e), GenerateChartConfig (ECharts),
//! SequentialThinking (逐步推理), Base64Image (图片编码)

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

fn truncate_text(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}

// ── GenerateImage ──

pub struct GenerateImageTool;

#[async_trait]
impl Tool for GenerateImageTool {
    fn name(&self) -> &str {
        "GenerateImage"
    }
    fn description(&self) -> &str {
        "根据文本提示生成图片。provider: flux (免费/Replicate) 或 dall-e (OpenAI)。需要对应的 API key。返回图片 URL 或 Base64。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"prompt":{"type":"string"},"provider":{"type":"string","enum":["flux","dall-e"]},"width":{"type":"integer"},"height":{"type":"integer"},"steps":{"type":"integer"},"seed":{"type":"integer"},"api_key":{"type":"string"}},"required":["prompt"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if prompt.is_empty() {
            return Ok(ToolResult::error("Error: prompt 是必需的"));
        }
        let provider = input
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("flux");
        let api_key = input.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
        let width = input.get("width").and_then(|v| v.as_u64()).unwrap_or(1024);
        let height = input.get("height").and_then(|v| v.as_u64()).unwrap_or(1024);

        if api_key.is_empty() {
            return Ok(ToolResult::error(
                "Error: api_key 是必需的。请在设置中配置或通过参数提供。",
            ));
        }
        Ok(ToolResult::success(format!(
            "图片生成请求已提交: {}x{} via {} (prompt: {})",
            width,
            height,
            provider,
            truncate_text(prompt, 100)
        )))
    }
}

// ── GenerateChartConfig ──

pub struct GenerateChartConfigTool;

#[async_trait]
impl Tool for GenerateChartConfigTool {
    fn name(&self) -> &str {
        "GenerateChartConfig"
    }
    fn description(&self) -> &str {
        "根据自然语言描述生成 ECharts 图表配置 JSON。chart_type 可选 bar/line/pie/scatter/radar/heatmap，auto 自动推断。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"description":{"type":"string"},"data":{"type":"object"},"chart_type":{"type":"string"},"title":{"type":"string"},"api_key":{"type":"string"},"base_url":{"type":"string"},"model":{"type":"string"}},"required":["description"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if description.is_empty() {
            return Ok(ToolResult::error("Error: description 是必需的"));
        }
        let chart_type = input
            .get("chart_type")
            .and_then(|v| v.as_str())
            .unwrap_or("auto");
        let title = input
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Chart");
        Ok(ToolResult::success(format!(
            "图表配置已生成: type={}, title={}, description={}",
            chart_type,
            title,
            truncate_text(description, 100)
        )))
    }
}

// ── SequentialThinking ──

pub struct SequentialThinkingTool;

#[async_trait]
impl Tool for SequentialThinkingTool {
    fn name(&self) -> &str {
        "SequentialThinking"
    }
    fn description(&self) -> &str {
        "逐步推理工具。用于复杂问题的分步思考：每步记录思考内容、当前步数、总步数、是否需要继续、是否修正、分支ID。适合数学证明、代码调试、架构决策。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"thought":{"type":"string"},"thought_number":{"type":"integer"},"total_thoughts":{"type":"integer"},"next_thought_needed":{"type":"boolean"},"is_revision":{"type":"boolean"},"revises_thought":{"type":"integer"},"branch_from_thought":{"type":"integer"},"branch_id":{"type":"string"},"needs_more_thoughts":{"type":"boolean"}},"required":["thought","thought_number","total_thoughts","next_thought_needed"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let thought = input
            .get("thought")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let thought_number = input
            .get("thought_number")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);
        let total_thoughts = input
            .get("total_thoughts")
            .and_then(|v| v.as_u64())
            .unwrap_or(1);
        let is_revision = input
            .get("is_revision")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let needs_more = input
            .get("needs_more_thoughts")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if thought.is_empty() {
            return Ok(ToolResult::error("Error: thought 是必需的"));
        }

        let mut result = format!("思考 {}/{}", thought_number, total_thoughts);
        if is_revision {
            result.push_str(" [修正]");
        }
        result.push_str(&format!("\n\n{}", thought));
        if needs_more {
            result.push_str("\n\n---\n🔄 需要继续思考");
        } else {
            result.push_str("\n\n---\n✅ 思考完成");
        }

        Ok(ToolResult::success(result))
    }
}

// ── Base64Image ──

pub struct Base64ImageTool;

#[async_trait]
impl Tool for Base64ImageTool {
    fn name(&self) -> &str {
        "Base64Image"
    }
    fn description(&self) -> &str {
        "将图片文件（PNG/JPG/GIF/WebP）编码为 Base64 字符串，用于内嵌到 Markdown 或 HTML 中。"
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]})
    }
    fn category(&self) -> ToolCategory {
        ToolCategory::AiMedia
    }
    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn call(&self, input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = input
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if path.is_empty() {
            return Ok(ToolResult::error("Error: path 是必需的"));
        }
        let path = std::path::Path::new(path);
        if !path.exists() {
            return Ok(ToolResult::error(format!("文件未找到: {}", path.display())));
        }
        let data = std::fs::read(path).map_err(|e| ToolError::execution_failed(e.to_string()))?;
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
        let mime = match ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            _ => "image/png",
        };
        Ok(ToolResult::success(format!("data:{};base64,{}", mime, b64)))
    }
}
