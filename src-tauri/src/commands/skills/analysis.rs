use super::install::collect_skill_content;
use super::install::skills_dir;
use crate::app_state::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::skill_err;
use axagent_crypto::decrypt_key;
use axagent_harness::types::provider_model::ProviderType;
use axagent_harness::types::settings_chat::ChatContent;
use axagent_harness::types::{ChatMessage, ChatRequest};
use tauri::State;

#[tauri::command]
pub async fn skill_analyze_frontend(
    state: State<'_, AppState>,
    name: String,
) -> Result<serde_json::Value, String> {
    // 读取技能内容
    // P2 #7: 使用 SkillState 中缓存的 PluginManager
    let plugin_manager = state.skill.plugin_manager.read().await;
    let report = plugin_manager
        .plugin_registry_report()
        .map_err(|e| e.to_string())?;
    let plugins = report.into_registry_allowing_failures();
    let plugin = plugins
        .summaries()
        .into_iter()
        .find(|p| p.metadata.name == name)
        .ok_or_else(|| format!("Skill '{}' not found", name))?;

    let skill_dir = plugin
        .metadata
        .root
        .ok_or_else(|| "Skill has no root dir".to_string())?;
    let raw_content = collect_skill_content(&skill_dir);

    let max_content_len = 8000;
    let skill_content = if raw_content.len() > max_content_len {
        format!(
            "{}...(内容已截断，总长度 {} 字符)",
            &raw_content[..max_content_len],
            raw_content.len()
        )
    } else {
        raw_content
    };

    if skill_content.trim().is_empty() {
        return Err(ErrorResponse::err_with_detail(
            skill_err::CONTENT_EMPTY,
            "Skill 内容为空，无法分析",
        ));
    }

    // 获取默认 Provider 配置
    let settings = axagent_dao::repo::settings::get_settings(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    let provider_id = settings.default_provider_id.as_ref().ok_or_else(|| {
        ErrorResponse::new(skill_err::MODEL_PROVIDER_NOT_CONFIGURED)
            .with_detail("未配置默认模型提供商".to_string())
    })?;
    let model_id = settings.default_model_id.as_ref().ok_or_else(|| {
        ErrorResponse::new(skill_err::MODEL_NOT_CONFIGURED)
            .with_detail("未配置默认模型".to_string())
    })?;

    let provider = axagent_dao::repo::provider::get_provider(state.harness.db(), provider_id)
        .await
        .map_err(|e| e.to_string())?;
    let key_row = axagent_dao::repo::provider::get_active_key(state.harness.db(), &provider.id)
        .await
        .map_err(|e| e.to_string())?;
    let decrypted_key = decrypt_key(&key_row.key_encrypted, state.harness.master_key())
        .map_err(|e| e.to_string())?;

    let registry_key = match provider.provider_type {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    };
    let adapter = state
        .harness
        .provider_registry()
        .get(registry_key)
        .ok_or_else(|| format!("未找到 Provider adapter: {}", registry_key))?;

    let prompt = format!(
        r#"你是一个 UI 扩展分析专家。分析以下 Skill 文档，生成 skill-manifest.json 的 capabilities 和 permissions 配置。

技能名称：{name}

技能内容：
---
{skill_content}
---

## 输出格式

只输出 JSON，不要有其他文字。格式：
{{
  "name": "{name}",
  "version": "0.1.0",
  "description": "技能描述",
  "capabilities": [
    {{
      "type": "page",
      "id": "页面ID",
      "title": "页面标题",
      "componentType": "Sandbox",
      "componentConfig": {{ "entry": "index.html" }}
    }},
    {{
      "type": "panel",
      "id": "面板ID",
      "title": "面板标题",
      "componentType": "Sandbox",
      "componentConfig": {{ "entry": "panel.html" }},
      "position": "Sidebar",
      "size": "Medium"
    }},
    {{
      "type": "toolbar",
      "id": "按钮ID",
      "title": "按钮标题",
      "icon": "lucide:Puzzle",
      "tooltip": "提示",
      "position": "left",
      "priority": 0,
      "onClick": []
    }},
    {{
      "type": "chatCommand",
      "id": "命令ID",
      "title": "命令标题",
      "commandName": "/command",
      "description": "命令描述",
      "mode": "agentic",
      "actions": []
    }},
    {{
      "type": "statusBar",
      "id": "状态栏ID",
      "title": "标题",
      "alignment": "left",
      "text": "文本"
    }},
    {{
      "type": "navigation",
      "id": "导航ID",
      "title": "标题",
      "icon": "lucide:Puzzle",
      "pageId": "页面ID",
      "position": 1
    }},
    {{
      "type": "settings",
      "id": "设置ID",
      "title": "设置标题",
      "settingsGroup": "extensions",
      "componentType": "Sandbox",
      "componentConfig": {{ "entry": "settings.html" }}
    }}
  ],
  "permissions": {{
    "commands": [],
    "events": [],
    "storeRead": [],
    "storeWrite": [],
    "navigate": []
  }}
}}

capability type 可选: page, panel, toolbar, chatCommand, statusBar, navigation, settings。
componentType 可选: "Sandbox" (沙箱页面) 或 "Markdown" (纯文档)。
不需要的 capability 类型不要包含。根据技能内容合理推断。"#,
        name = name,
        skill_content = skill_content,
    );

    let base_url =
        axagent_harness::resolve_base_url_for_type(&provider.api_host, &provider.provider_type);
    let ctx = axagent_harness::ProviderRequestContext {
        api_key: decrypted_key,
        key_id: key_row.id,
        provider_id: provider.id.clone(),
        base_url: Some(base_url),
        api_path: provider.api_path.clone(),
        proxy_config: provider.proxy_config.clone(),
        custom_headers: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    // 带重试的 LLM 调用（最多 3 次）
    let max_retries = 3;
    let mut last_error = String::new();

    for attempt in 1..=max_retries {
        let llm_request = ChatRequest {
            model: model_id.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Text(if attempt > 1 {
                    format!("之前的输出格式不正确，请严格只输出 JSON。\n\n{}", prompt)
                } else {
                    prompt.clone()
                }),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            }],
            temperature: Some(0.3),
            top_p: None,
            max_tokens: Some(4096),
            stream: false,
            tools: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        let response = match adapter.chat(&ctx, llm_request).await {
            Ok(r) => r,
            Err(e) => {
                last_error = format!("LLM 调用失败: {}", e);
                if attempt < max_retries {
                    continue;
                }
                return Err(last_error);
            },
        };

        let content = response.content.trim();
        let json_str = extract_json(content);

        match serde_json::from_str::<serde_json::Value>(json_str) {
            Ok(value) => {
                // 校验必需字段
                if value.get("capabilities").is_some() {
                    return Ok(value);
                }
                last_error = "响应缺少 capabilities 字段".to_string();
                if attempt < max_retries {
                    continue;
                }
            },
            Err(e) => {
                last_error = format!(
                    "解析 LLM 响应失败 (尝试 {}/{}): {}。原始响应: {}",
                    attempt,
                    max_retries,
                    e,
                    &content[..content.len().min(300)]
                );
                if attempt < max_retries {
                    continue;
                }
            },
        }
    }

    Err(last_error)
}

fn extract_json(content: &str) -> &str {
    let content = content.trim();
    // 尝试 markdown 代码块
    if let Some(start) = content.find("```json") {
        let after = &content[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
        return after.trim();
    }
    if let Some(start) = content.find("```") {
        let after = &content[start + 3..];
        if let Some(end) = after.find("```") {
            return after[..end].trim();
        }
        return after.trim();
    }
    // 尝试直接提取花括号
    if let Some(start) = content.find('{') {
        if let Some(end) = content.rfind('}') {
            return &content[start..=end];
        }
    }
    content
}

/// 读取技能目录下的资源文件内容（用于 HTML/JS/CSS 等静态资源）
#[tauri::command]
pub fn skill_read_asset(name: String, file_name: String) -> Result<String, ErrorResponse> {
    // P1 #3: 对 name 参数增加路径遍历校验（防止 ../../ 跳出 skills 目录）
    if name.contains("..") || name.contains('\\') || name.contains('/') || name.is_empty() {
        return Err("Invalid name: path traversal or empty".into());
    }
    if name.len() >= 2 {
        let b = name.as_bytes();
        if b[0].is_ascii_alphabetic() && b[1] == b':' {
            return Err("Invalid name: absolute path not allowed".into());
        }
    }
    if name.starts_with('/') {
        return Err("Invalid name: absolute path not allowed".into());
    }

    // P1 #3: 对 file_name 参数增加独立的路径遍历校验
    if file_name.contains("..")
        || file_name.contains('\\')
        || file_name.contains('/')
        || file_name.is_empty()
    {
        return Err("Invalid file_name: path traversal or empty".into());
    }
    // 拒绝绝对路径（Windows 盘符或 Unix 根路径）
    if file_name.len() >= 2 {
        let b = file_name.as_bytes();
        if b[0].is_ascii_alphabetic() && b[1] == b':' {
            return Err("Invalid file_name: absolute path not allowed".into());
        }
    }
    if file_name.starts_with('/') {
        return Err("Invalid file_name: absolute path not allowed".into());
    }

    let skill_dir = skills_dir().join(&name);
    if !skill_dir.exists() {
        return Err(format!("Skill '{}' not found", name).into());
    }

    // 安全检查：防止路径遍历攻击
    let requested = skill_dir.join(&file_name);
    let canonical_dir = skill_dir.canonicalize().map_err(|e| e.to_string())?;
    let canonical_requested = requested.canonicalize().map_err(|e| e.to_string())?;

    if !canonical_requested.starts_with(&canonical_dir) {
        return Err("Access denied: file is outside skill directory".into());
    }

    if !canonical_requested.is_file() {
        return Err(format!("File '{}' not found in skill '{}'", file_name, name).into());
    }

    // 允许文本类文件和常见二进制资源
    let ext = canonical_requested
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let allowed = [
        "html", "htm", "md", "txt", "css", "js", "json", "svg", "xml", "png", "jpg", "jpeg", "gif",
        "webp", "ico", "woff", "woff2", "ttf", "otf",
    ];
    if !allowed.contains(&ext.as_str()) {
        return Err(format!("File type '{}' is not allowed for direct reading", ext).into());
    }

    Ok(std::fs::read_to_string(&canonical_requested).map_err(|e| e.to_string())?)
}
