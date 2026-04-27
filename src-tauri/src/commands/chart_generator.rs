use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, ChatResponse};
use axagent_providers::openai::OpenAIAdapter;
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChartGenResult {
    pub option: Value,
    pub chart_type: String,
    pub title: String,
}

#[command]
pub async fn generate_chart_config(
    description: String,
    data: Option<Value>,
    chart_type: Option<String>,
    title: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> Result<ChartGenResult, String> {
    let api_key = api_key.ok_or_else(|| "API key required for chart generation".to_string())?;
    let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
    let model = model.unwrap_or_else(|| "gpt-4o-mini".to_string());

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

    let user_message = if let Some(ref d) = data {
        format!(
            "Description: {}\n\nData:\n{}",
            description,
            serde_json::to_string_pretty(d).map_err(|e| e.to_string())?
        )
    } else {
        format!("Description: {}", description)
    };

    let chat_request = ChatRequest {
        model: model.clone(),
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
        stream: false,
        temperature: Some(0.1),
        max_tokens: None,
        top_p: None,
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

    let ctx = ProviderRequestContext {
        api_key,
        key_id: "chart-gen".to_string(),
        provider_id: "openai".to_string(),
        base_url: Some(base_url),
        api_path: Some("/chat/completions".to_string()),
        proxy_config: Default::default(),
        custom_headers: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let adapter: Arc<dyn ProviderAdapter> = Arc::new(OpenAIAdapter::new());
    let response: ChatResponse = adapter
        .chat(&ctx, chat_request)
        .await
        .map_err(|e| e.to_string())?;

    let text = response.content;

    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let mut option: Value =
        serde_json::from_str(cleaned).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let chart_type_result = option["_chartType"]
        .as_str()
        .unwrap_or(chart_type.as_deref().unwrap_or("bar"))
        .to_string();
    let title_result = option["_title"]
        .as_str()
        .unwrap_or(title.as_deref().unwrap_or(&description))
        .to_string();

    if let Some(obj) = option.as_object_mut() {
        obj.remove("_chartType");
        obj.remove("_title");
    }

    Ok(ChartGenResult {
        option,
        chart_type: chart_type_result,
        title: title_result,
    })
}
