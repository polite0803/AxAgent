//! 浏览器自动化工具
//!
//! 将 builtin_handlers 中的 10 个 browser_* handler 迁移为 Tool trait。

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

macro_rules! browser_tool {
    ($name:ident, $display:literal, $desc:literal, $schema:expr, |$c:ident, $input:ident| $body:expr) => {
        pub struct $name;
        #[async_trait]
        impl Tool for $name {
            fn name(&self) -> &str { $display }
            fn description(&self) -> &str { $desc }
            fn input_schema(&self) -> Value { $schema }
            fn category(&self) -> ToolCategory { ToolCategory::Network }

            async fn call(&self, $input: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
                let mut client = axagent_core::browser_automation::PlaywrightClient::launch()
                    .await
                    .map_err(|e| ToolError::execution_failed(e.to_string()))?;
                let $c = &mut client;
                $body
            }
        }
    };
}

browser_tool!(BrowserNavigateTool, "BrowserNavigate",
    "在浏览器中导航到指定 URL。",
    serde_json::json!({"type":"object","properties":{"url":{"type":"string"}},"required":["url"]}),
    |c, input| {
        let url = input.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if url.is_empty() { return Ok(ToolResult::error("Error: url 是必需的")); }
        let r = c.navigate(&url).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(format!("已导航到 {} - 标题: {}", r.url, r.title)))
    }
);

browser_tool!(BrowserScreenshotTool, "BrowserScreenshot",
    "截取浏览器页面的屏幕截图。",
    serde_json::json!({"type":"object","properties":{"full_page":{"type":"boolean"}}}),
    |c, input| {
        let full_page = input.get("full_page").and_then(|v| v.as_bool()).unwrap_or(false);
        let r = c.screenshot(full_page).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(format!("截图已捕获 ({} bytes)", r.image_base64.len())))
    }
);

browser_tool!(BrowserClickTool, "BrowserClick",
    "点击浏览器页面中的元素。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}),
    |c, input| {
        let sel = input.get("selector").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if sel.is_empty() { return Ok(ToolResult::error("Error: selector 是必需的")); }
        c.click(&sel).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("点击成功"))
    }
);

browser_tool!(BrowserFillTool, "BrowserFill",
    "在浏览器表单元素中填入值。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"},"value":{"type":"string"}},"required":["selector"]}),
    |c, input| {
        let sel = input.get("selector").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let val = input.get("value").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if sel.is_empty() { return Ok(ToolResult::error("Error: selector 是必需的")); }
        c.fill(&sel, &val).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("填入成功"))
    }
);

browser_tool!(BrowserTypeTool, "BrowserType",
    "在浏览器元素中输入文本（模拟键盘输入）。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"},"text":{"type":"string"}},"required":["selector"]}),
    |c, input| {
        let sel = input.get("selector").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let txt = input.get("text").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if sel.is_empty() { return Ok(ToolResult::error("Error: selector 是必需的")); }
        c.type_text(&sel, &txt).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("输入成功"))
    }
);

browser_tool!(BrowserExtractTextTool, "BrowserExtractText",
    "从浏览器页面提取指定元素的文本。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}),
    |c, input| {
        let sel = input.get("selector").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if sel.is_empty() { return Ok(ToolResult::error("Error: selector 是必需的")); }
        let text = c.extract_text(&sel).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(text))
    }
);

browser_tool!(BrowserExtractAllTool, "BrowserExtractAll",
    "提取浏览器页面中所有匹配元素的详细信息。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"}},"required":["selector"]}),
    |c, input| {
        let sel = input.get("selector").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if sel.is_empty() { return Ok(ToolResult::error("Error: selector 是必需的")); }
        let elements = c.extract_all(&sel).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(serde_json::to_string_pretty(&elements).unwrap_or_default()))
    }
);

browser_tool!(BrowserGetContentTool, "BrowserGetContent",
    "获取浏览器页面的完整 HTML 内容。",
    serde_json::json!({"type":"object","properties":{}}),
    |c, _input| {
        let html = c.get_content().await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success(html))
    }
);

browser_tool!(BrowserSelectTool, "BrowserSelect",
    "在浏览器下拉选择框中选择选项。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"},"value":{"type":"string"}},"required":["selector"]}),
    |c, input| {
        let sel = input.get("selector").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let val = input.get("value").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if sel.is_empty() { return Ok(ToolResult::error("Error: selector 是必需的")); }
        c.select_option(&sel, &val).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("选择成功"))
    }
);

browser_tool!(BrowserWaitForTool, "BrowserWaitFor",
    "等待浏览器页面中的元素出现。",
    serde_json::json!({"type":"object","properties":{"selector":{"type":"string"},"timeout":{"type":"integer"}},"required":["selector"]}),
    |c, input| {
        let sel = input.get("selector").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let to = input.get("timeout").and_then(|v| v.as_u64()).map(|v| v as u32);
        if sel.is_empty() { return Ok(ToolResult::error("Error: selector 是必需的")); }
        c.wait_for(&sel, to).await.map_err(|e| ToolError::execution_failed(e.to_string()))?;
        Ok(ToolResult::success("等待成功"))
    }
);
