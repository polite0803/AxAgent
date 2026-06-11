// SPDX-License-Identifier: AGPL-3.0-only

use crate::{ProviderAdapter, ProviderRequestContext};
use axagent_core::error::{AxAgentError, Result};
use axagent_core::screen_vision::{ScreenAnalysis, SuggestedAction, UIElementInfo};
use axagent_harness::types::*;

/// Analyze a screen using the given provider adapter, context, and model.
pub async fn analyze_screen(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: String,
    image_base64: &str,
    task_description: &str,
) -> Result<ScreenAnalysis> {
    let prompt = build_analysis_prompt(task_description);
    let response_text = send_to_vision_model(adapter, ctx, model, image_base64, &prompt).await?;
    parse_analysis_response(&response_text)
}

/// Find a UI element on the screen using the given provider adapter, context, and model.
pub async fn find_element(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: String,
    image_base64: &str,
    element_description: &str,
) -> Result<Option<UIElementInfo>> {
    let prompt = format!(
        "Find the UI element that matches: '{}'. Return only the element details in JSON format. If no matching element found, return {{\"found\": false}}.",
        element_description
    );
    let response_text = send_to_vision_model(adapter, ctx, model, image_base64, &prompt).await?;
    parse_element_response(&response_text)
}

/// Suggest the next action based on the current screen using the given provider adapter, context, and model.
pub async fn suggest_next_action(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: String,
    image_base64: &str,
    current_task: &str,
) -> Result<Vec<SuggestedAction>> {
    let prompt = format!(
        "Given the current screen and task '{}', what action should be taken next? Return a JSON array of suggested actions.",
        current_task
    );
    let response_text = send_to_vision_model(adapter, ctx, model, image_base64, &prompt).await?;
    parse_actions_response(&response_text)
}

async fn send_to_vision_model(
    adapter: &dyn ProviderAdapter,
    ctx: &ProviderRequestContext,
    model: String,
    image_base64: &str,
    prompt: &str,
) -> Result<String> {
    let chat_request = ChatRequest {
        model,
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Multipart(vec![
                ContentPart {
                    r#type: "image_url".to_string(),
                    text: None,
                    image_url: Some(ImageUrl {
                        url: format!("data:image/png;base64,{}", image_base64),
                    }),
                },
                ContentPart {
                    r#type: "text".to_string(),
                    text: Some(prompt.to_string()),
                    image_url: None,
                },
            ]),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        }],
        stream: false,
        temperature: None,
        top_p: None,
        max_tokens: Some(2048),
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

    let response = adapter.chat(ctx, chat_request).await?;
    Ok(response.content)
}

fn build_analysis_prompt(task: &str) -> String {
    format!(
        r#"Analyze this screen screenshot and provide:
1. A list of all interactive UI elements (buttons, text fields, menus, etc.) with their approximate screen coordinates
2. Suggested actions to accomplish the task: '{}'

Return the analysis in this JSON format:
{{
  "elements": [
    {{
      "element_type": "button|text_field|menu|checkbox|...",
      "name": "visible name or label",
      "description": "brief description",
      "bounds": {{"x": 100, "y": 200, "width": 150, "height": 40}},
      "clickable": true/false,
      "editable": true/false,
      "confidence": 0.0-1.0
    }}
  ],
  "suggested_actions": [
    {{
      "action_type": "click|type|scroll|...",
      "target_element": "name of element",
      "description": "what this action does",
      "reasoning": "why this action is needed"
    }}
  ],
  "reasoning": "overall analysis of the screen",
  "confidence": 0.0-1.0
}}"#,
        task
    )
}

fn parse_analysis_response(response: &str) -> Result<ScreenAnalysis> {
    let json_str = extract_json(response);

    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .or_else(|_| {
            serde_json::from_str(
                response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_end_matches("```")
                    .trim(),
            )
        })
        .map_err(|e| {
            AxAgentError::Provider(format!("Failed to parse JSON: {} - Response: {}", e, response))
        })?;

    let elements: Vec<UIElementInfo> = parsed["elements"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|e| UIElementInfo {
                    element_type: e["element_type"].as_str().unwrap_or("unknown").to_string(),
                    name: e["name"].as_str().unwrap_or("").to_string(),
                    description: e["description"].as_str().unwrap_or("").to_string(),
                    bounds: axagent_core::screen_vision::ElementBounds {
                        x: e["bounds"]["x"].as_f64().unwrap_or(0.0),
                        y: e["bounds"]["y"].as_f64().unwrap_or(0.0),
                        width: e["bounds"]["width"].as_f64().unwrap_or(0.0),
                        height: e["bounds"]["height"].as_f64().unwrap_or(0.0),
                    },
                    clickable: e["clickable"].as_bool().unwrap_or(false),
                    editable: e["editable"].as_bool().unwrap_or(false),
                    confidence: e["confidence"].as_f64().unwrap_or(0.5) as f32,
                })
                .collect()
        })
        .unwrap_or_default();

    let suggested_actions: Vec<SuggestedAction> = parsed["suggested_actions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|a| SuggestedAction {
                    action_type: a["action_type"]
                        .as_str()
                        .unwrap_or("none")
                        .parse()
                        .unwrap_or(axagent_core::screen_vision::ActionType::None),
                    target_element: a["target_element"].as_str().unwrap_or("").to_string(),
                    description: a["description"].as_str().unwrap_or("").to_string(),
                    reasoning: a["reasoning"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(ScreenAnalysis {
        elements,
        suggested_actions,
        reasoning: parsed["reasoning"].as_str().unwrap_or("").to_string(),
        confidence: parsed["confidence"].as_f64().unwrap_or(0.5) as f32,
    })
}

fn parse_element_response(response: &str) -> Result<Option<UIElementInfo>> {
    let json_str = extract_json(response);

    if json_str.contains("\"found\": false") || json_str.is_empty() {
        return Ok(None);
    }

    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .or_else(|_| {
            serde_json::from_str(
                response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_end_matches("```")
                    .trim(),
            )
        })
        .map_err(|e| AxAgentError::Provider(format!("Failed to parse element: {}", e)))?;

    Ok(Some(UIElementInfo {
        element_type: parsed["element_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        name: parsed["name"].as_str().unwrap_or("").to_string(),
        description: parsed["description"].as_str().unwrap_or("").to_string(),
        bounds: axagent_core::screen_vision::ElementBounds {
            x: parsed["bounds"]["x"].as_f64().unwrap_or(0.0),
            y: parsed["bounds"]["y"].as_f64().unwrap_or(0.0),
            width: parsed["bounds"]["width"].as_f64().unwrap_or(0.0),
            height: parsed["bounds"]["height"].as_f64().unwrap_or(0.0),
        },
        clickable: parsed["clickable"].as_bool().unwrap_or(false),
        editable: parsed["editable"].as_bool().unwrap_or(false),
        confidence: parsed["confidence"].as_f64().unwrap_or(0.5) as f32,
    }))
}

fn parse_actions_response(response: &str) -> Result<Vec<SuggestedAction>> {
    let json_str = extract_json(response);

    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .or_else(|_| {
            serde_json::from_str(
                response
                    .trim()
                    .trim_start_matches("```json")
                    .trim_end_matches("```")
                    .trim(),
            )
        })
        .map_err(|e| AxAgentError::Provider(format!("Failed to parse actions: {}", e)))?;

    let actions: Vec<SuggestedAction> = parsed
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|a| SuggestedAction {
                    action_type: a["action_type"]
                        .as_str()
                        .unwrap_or("none")
                        .parse()
                        .unwrap_or(axagent_core::screen_vision::ActionType::None),
                    target_element: a["target_element"].as_str().unwrap_or("").to_string(),
                    description: a["description"].as_str().unwrap_or("").to_string(),
                    reasoning: a["reasoning"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(actions)
}

fn extract_json(text: &str) -> String {
    let trimmed = text.trim();

    if trimmed.starts_with('{')
        && let Some(end) = trimmed.rfind('}')
    {
        return trimmed[..=end].to_string();
    }

    if let Some(json_start) = trimmed.find("```json") {
        let after_json = &trimmed[json_start + 7..];
        if let Some(json_end) = after_json.find("```") {
            return after_json[..json_end].trim().to_string();
        }
    }

    text.to_string()
}
