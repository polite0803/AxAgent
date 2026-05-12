//! 浏览器自动化工具（共享连接池）
//!
//! 10 个浏览器操作工具共享同一个 PlaywrightClient 连接。
//! 首次使用时自动启动浏览器，后续工具调用复用同一会话。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use axagent_core::browser_automation::{shared_browser_pool, PlaywrightClient};
use serde_json::Value;

macro_rules! browser_tool {
    ($name:ident, $display:literal, $desc:literal, $schema:expr, |$input:ident, $cl:ident| $body:expr) => {
        pub struct $name;
        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str {
                $display
            }
            fn description(&self) -> &str {
                $desc
            }
            fn input_schema(&self) -> Value {
                $schema
            }
            fn category(&self) -> ToolCategory {
                ToolCategory::Browser
            }
            fn is_concurrency_safe(&self) -> bool {
                false
            }

            async fn call(
                &self,
                $input: Value,
                _ctx: &ToolContext,
            ) -> Result<ToolResult, ToolError> {
                // 确保浏览器客户端已启动
                {
                    let mut guard = shared_browser_pool().lock().await;
                    if guard.is_none() {
                        *guard = Some(PlaywrightClient::launch().await.map_err(|e| {
                            ToolError::execution_failed(format!("浏览器启动失败: {}", e))
                        })?);
                    }
                }
                // 获取共享客户端引用并执行操作
                let mut guard = shared_browser_pool().lock().await;
                let $cl = guard
                    .as_mut()
                    .ok_or_else(|| ToolError::execution_failed("浏览器未启动".to_string()))?;
                $body
            }
        }
    };
}

browser_tool!(
    BrowserNavigateTool,
    "BrowserNavigate",
    "在浏览器中导航到指定 URL。导航后浏览器会话保持，后续点击/填充等操作在同一页面进行。",
    serde_json::json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}),
    |input, c| {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if url.is_empty() {
            return Ok(ToolResult::error("Error: url 是必需的"));
        }
        let r = c
            .navigate(&url)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(format!("已导航到 {} - 标题: {}", r.url, r.title)))
    }
);

browser_tool!(
    BrowserScreenshotTool,
    "BrowserScreenshot",
    "截取浏览器当前页面的屏幕截图。返回 Base64 编码图片。",
    serde_json::json!({"type":"object","properties":{"full_page":{"type":"boolean"}}}),
    |input, c| {
        let full_page = input
            .get("full_page")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let r = c
            .screenshot(full_page)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(format!("截图已捕获 ({} bytes)", r.image_base64.len())))
    }
);

browser_tool!(
    BrowserClickTool,
    "BrowserClick",
    "点击浏览器当前页面中的元素（CSS 选择器）。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}),
    |input, c| {
        let sel = input
            .get("selector")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if sel.is_empty() {
            return Ok(ToolResult::error("Error: selector 是必需的"));
        }
        c.click(&sel)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("点击成功"))
    }
);

browser_tool!(
    BrowserFillTool,
    "BrowserFill",
    "在浏览器表单元素中填入值（CSS 选择器 + 值）。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"},"value":{"type":"string"}},"required":["selector"]}),
    |input, c| {
        let sel = input
            .get("selector")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let val = input
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if sel.is_empty() {
            return Ok(ToolResult::error("Error: selector 是必需的"));
        }
        c.fill(&sel, &val)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("填入成功"))
    }
);

browser_tool!(
    BrowserTypeTool,
    "BrowserType",
    "在浏览器元素中逐字符输入文本（模拟键盘输入）。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"},"text":{"type":"string"}},"required":["selector"]}),
    |input, c| {
        let sel = input
            .get("selector")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let txt = input
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if sel.is_empty() {
            return Ok(ToolResult::error("Error: selector 是必需的"));
        }
        c.type_text(&sel, &txt)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("输入成功"))
    }
);

browser_tool!(
    BrowserExtractTextTool,
    "BrowserExtractText",
    "从浏览器当前页面提取指定元素的文本内容。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}),
    |input, c| {
        let sel = input
            .get("selector")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if sel.is_empty() {
            return Ok(ToolResult::error("Error: selector 是必需的"));
        }
        let text = c
            .extract_text(&sel)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(text))
    }
);

browser_tool!(
    BrowserExtractAllTool,
    "BrowserExtractAll",
    "提取浏览器页面中所有匹配元素的详细信息（JSON 数组）。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}),
    |input, c| {
        let sel = input
            .get("selector")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if sel.is_empty() {
            return Ok(ToolResult::error("Error: selector 是必需的"));
        }
        let elements = c
            .extract_all(&sel)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string_pretty(&elements).unwrap_or_default()))
    }
);

browser_tool!(
    BrowserGetContentTool,
    "BrowserGetContent",
    "获取浏览器当前页面的完整 HTML 内容。",
    serde_json::json!({"type":"object","properties":{}}),
    |_input, c| {
        let html = c
            .get_content()
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(html))
    }
);

browser_tool!(
    BrowserSelectTool,
    "BrowserSelect",
    "在浏览器下拉选择框中选择指定选项。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"},"value":{"type":"string"}},"required":["selector"]}),
    |input, c| {
        let sel = input
            .get("selector")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let val = input
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if sel.is_empty() {
            return Ok(ToolResult::error("Error: selector 是必需的"));
        }
        c.select_option(&sel, &val)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("选择成功"))
    }
);

browser_tool!(
    BrowserWaitForTool,
    "BrowserWaitFor",
    "等待浏览器页面中指定元素出现（可选超时毫秒）。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"},"timeout_ms":{"type":"integer","default":5000}},"required":["selector"]}),
    |input, c| {
        let sel = input
            .get("selector")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let to = input
            .get("timeout_ms")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        if sel.is_empty() {
            return Ok(ToolResult::error("Error: selector 是必需的"));
        }
        c.wait_for(&sel, to)
            .await
            .map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("等待成功"))
    }
);
