use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenAnalysis {
    pub elements: Vec<UIElementInfo>,
    pub suggested_actions: Vec<SuggestedAction>,
    pub reasoning: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElementInfo {
    pub element_type: String,
    pub name: String,
    pub description: String,
    pub bounds: ElementBounds,
    pub clickable: bool,
    pub editable: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub action_type: ActionType,
    pub target_element: String,
    pub description: String,
    pub reasoning: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Click,
    DoubleClick,
    RightClick,
    Type,
    Hover,
    Scroll,
    Select,
    None,
}

impl ActionType {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "click" => ActionType::Click,
            "double_click" | "doubleclick" => ActionType::DoubleClick,
            "right_click" | "rightclick" => ActionType::RightClick,
            "type" | "input" => ActionType::Type,
            "hover" | "mouse_over" => ActionType::Hover,
            "scroll" => ActionType::Scroll,
            "select" => ActionType::Select,
            _ => ActionType::None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionPrompt {
    pub task_description: String,
    pub image_base64: String,
}

pub struct ScreenVisionAnalyzer {
    provider: VisionProvider,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisionProvider {
    #[default]
    Anthropic,
    OpenAI,
    Gemini,
}

impl ScreenVisionAnalyzer {
    pub fn new(provider: VisionProvider) -> Self {
        Self { provider }
    }

    pub async fn analyze_screen(
        &self,
        image_base64: &str,
        task_description: &str,
    ) -> Result<ScreenAnalysis> {
        let prompt = self.build_analysis_prompt(task_description);
        let response = self.send_to_vision_model(image_base64, &prompt).await?;
        self.parse_analysis_response(&response)
    }

    pub async fn find_element(
        &self,
        image_base64: &str,
        element_description: &str,
    ) -> Result<Option<UIElementInfo>> {
        let prompt = format!(
            "Find the UI element that matches: '{}'. Return only the element details in JSON format. If no matching element found, return {{\"found\": false}}.",
            element_description
        );
        let response = self.send_to_vision_model(image_base64, &prompt).await?;
        self.parse_element_response(&response)
    }

    pub async fn suggest_next_action(
        &self,
        image_base64: &str,
        current_task: &str,
    ) -> Result<Vec<SuggestedAction>> {
        let prompt = format!(
            "Given the current screen and task '{}', what action should be taken next? Return a JSON array of suggested actions.",
            current_task
        );
        let response = self.send_to_vision_model(image_base64, &prompt).await?;
        self.parse_actions_response(&response)
    }

    fn build_analysis_prompt(&self, task: &str) -> String {
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

    async fn send_to_vision_model(&self, image_base64: &str, prompt: &str) -> Result<String> {
        match self.provider {
            VisionProvider::Anthropic => self.send_to_anthropic(image_base64, prompt).await,
            VisionProvider::OpenAI => self.send_to_openai(image_base64, prompt).await,
            VisionProvider::Gemini => self.send_to_gemini(image_base64, prompt).await,
        }
    }

    async fn send_to_anthropic(&self, image_base64: &str, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": "image/png",
                            "data": image_base64
                        }
                    },
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]
            }]
        });

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header(
                "x-api-key",
                std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            )
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("text"))
            .map(|t| t.as_str().unwrap_or(""))
            .unwrap_or("");

        Ok(content.to_string())
    }

    async fn send_to_openai(&self, image_base64: &str, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "model": "gpt-4o",
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", image_base64)
                        }
                    },
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]
            }],
            "max_tokens": 1024
        });

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header(
                "authorization",
                format!(
                    "Bearer {}",
                    std::env::var("OPENAI_API_KEY").unwrap_or_default()
                ),
            )
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["choices"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("message"))
            .and_then(|msg| msg.get("content"))
            .map(|c| c.as_str().unwrap_or(""))
            .unwrap_or("");

        Ok(content.to_string())
    }

    async fn send_to_gemini(&self, image_base64: &str, prompt: &str) -> Result<String> {
        let request_body = serde_json::json!({
            "contents": [{
                "parts": [
                    {
                        "inline_data": {
                            "mime_type": "image/png",
                            "data": image_base64
                        }
                    },
                    {
                        "text": prompt
                    }
                ]
            }]
        });

        let client = reqwest::Client::new();
        let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_default();
        let response = client
            .post(format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={}", api_key))
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["candidates"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|parts| parts.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("text"))
            .map(|t| t.as_str().unwrap_or(""))
            .unwrap_or("");

        Ok(content.to_string())
    }

    fn parse_analysis_response(&self, response: &str) -> Result<ScreenAnalysis> {
        let json_str = self.extract_json(response);

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
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {} - Response: {}", e, response))?;

        let elements: Vec<UIElementInfo> = parsed["elements"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|e| {
                        UIElementInfo {
                            element_type: e["element_type"]
                                .as_str()
                                .unwrap_or("unknown")
                                .to_string(),
                            name: e["name"].as_str().unwrap_or("").to_string(),
                            description: e["description"].as_str().unwrap_or("").to_string(),
                            bounds: ElementBounds {
                                x: e["bounds"]["x"].as_f64().unwrap_or(0.0),
                                y: e["bounds"]["y"].as_f64().unwrap_or(0.0),
                                width: e["bounds"]["width"].as_f64().unwrap_or(0.0),
                                height: e["bounds"]["height"].as_f64().unwrap_or(0.0),
                            },
                            clickable: e["clickable"].as_bool().unwrap_or(false),
                            editable: e["editable"].as_bool().unwrap_or(false),
                            confidence: e["confidence"].as_f64().unwrap_or(0.5) as f32,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let suggested_actions: Vec<SuggestedAction> = parsed["suggested_actions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|a| {
                        SuggestedAction {
                            action_type: ActionType::from_str(
                                a["action_type"].as_str().unwrap_or("none"),
                            ),
                            target_element: a["target_element"].as_str().unwrap_or("").to_string(),
                            description: a["description"].as_str().unwrap_or("").to_string(),
                            reasoning: a["reasoning"].as_str().unwrap_or("").to_string(),
                        }
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

    fn parse_element_response(&self, response: &str) -> Result<Option<UIElementInfo>> {
        let json_str = self.extract_json(response);

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
            .map_err(|e| anyhow::anyhow!("Failed to parse element: {}", e))?;

        Ok(Some(UIElementInfo {
            element_type: parsed["element_type"]
                .as_str()
                .unwrap_or("unknown")
                .to_string(),
            name: parsed["name"].as_str().unwrap_or("").to_string(),
            description: parsed["description"].as_str().unwrap_or("").to_string(),
            bounds: ElementBounds {
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

    fn parse_actions_response(&self, response: &str) -> Result<Vec<SuggestedAction>> {
        let json_str = self.extract_json(response);

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
            .map_err(|e| anyhow::anyhow!("Failed to parse actions: {}", e))?;

        let actions: Vec<SuggestedAction> = parsed
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|a| {
                        SuggestedAction {
                            action_type: ActionType::from_str(
                                a["action_type"].as_str().unwrap_or("none"),
                            ),
                            target_element: a["target_element"].as_str().unwrap_or("").to_string(),
                            description: a["description"].as_str().unwrap_or("").to_string(),
                            reasoning: a["reasoning"].as_str().unwrap_or("").to_string(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(actions)
    }

    fn extract_json(&self, text: &str) -> String {
        let trimmed = text.trim();

        if trimmed.starts_with('{') {
            if let Some(end) = trimmed.rfind('}') {
                return trimmed[..=end].to_string();
            }
        }

        if let Some(json_start) = trimmed.find("```json") {
            let after_json = &trimmed[json_start + 7..];
            if let Some(json_end) = after_json.find("```") {
                return after_json[..json_end].trim().to_string();
            }
        }

        text.to_string()
    }
}

impl Default for ScreenVisionAnalyzer {
    fn default() -> Self {
        Self::new(VisionProvider::Anthropic)
    }
}
