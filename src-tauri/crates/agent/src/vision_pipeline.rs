use axagent_harness::types::{ChatContent, ChatMessage, ChatRequest, ContentPart, ImageUrl};
use axagent_harness::{ProviderAdapter, ProviderRequestContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VisionTask {
    ImageDescription,
    Ocr,
    UiElementDetection,
    ChartAnalysis,
    CodeScreenshotReading,
}

impl VisionTask {
    fn system_prompt(&self) -> &'static str {
        match self {
            VisionTask::ImageDescription => {
                "You are an image analysis assistant. Describe the provided image in detail, \
                 covering all visible elements, colors, layout, text, and context.\n\n\
                 ## 禁区\n\
                 - Do not speculate about content that is not clearly visible\n\
                 - Do not make assumptions about people's identity, age, or emotions unless clearly visible\n\n\
                 ## 自验环节\n\
                 Before output: Have I covered all visible elements? Are any descriptions based on assumption?"
            },
            VisionTask::Ocr => {
                "You are an OCR assistant. Extract all text from the provided image. \
                 Output only the extracted text, preserving the original formatting and line breaks.\n\n\
                 ## 禁区\n\
                 - Do not correct or interpret the text — extract verbatim\n\
                 - Do not add descriptions or commentary\n\n\
                 ## 自验环节\n\
                 Before output: Have I extracted every visible text string? Is formatting preserved?"
            },
            VisionTask::UiElementDetection => {
                "You are a UI analysis assistant. Analyze the provided screenshot and list all \
                 interactive elements (buttons, inputs, links, menus, toggles, etc.) with their \
                 labels, types, and positions. Format as a structured list.\n\n\
                 ## 交付物\n\
                 - Each element must include: type, label (if any), position, whether actionable\n\
                 - Output as structured list or JSON array\n\n\
                 ## 禁区\n\
                 - Do not list non-interactive elements (static text, background images)\n\
                 - Do not guess function of an element if its label is unclear\n\n\
                 ## 自验环节\n\
                 Before output: Have I covered all interactive elements? Are labels and positions accurate?"
            },
            VisionTask::ChartAnalysis => {
                "You are a chart analysis assistant. Analyze the provided chart/graph image. \
                 Extract data points, labels, axes information, trends, and key insights. \
                 Provide both a summary and structured data when possible.\n\n\
                 ## 交付物\n\
                 - Summary: chart type, title, key trends\n\
                 - Structured data: data points table or JSON\n\
                 - Key insights: 2-3 actionable takeaways\n\n\
                 ## 禁区\n\
                 - Do not extrapolate data beyond what is visible in the chart\n\
                 - Do not invent data points not explicitly shown\n\n\
                 ## 自验环节\n\
                 Before output: Are extracted data points consistent with the visual chart?"
            },
            VisionTask::CodeScreenshotReading => {
                "You are a code reading assistant. Extract all code visible in the provided \
                 screenshot. Output only the code as plain text, preserving indentation and formatting.\n\n\
                 ## 禁区\n\
                 - Do not modify, fix, or interpret the code — extract verbatim\n\
                 - Do not add line numbers, annotations, or explanations\n\
                 - If code is truncated in the screenshot, note the truncation\n\n\
                 ## 自验环节\n\
                 Before output: Is the extracted code identical to what is shown in the screenshot?"
            },
        }
    }

    fn user_prompt(&self) -> &'static str {
        match self {
            VisionTask::ImageDescription => "Describe this image in detail.",
            VisionTask::Ocr => "Extract all text from this image.",
            VisionTask::UiElementDetection => {
                "Analyze this UI screenshot and list all interactive elements."
            },
            VisionTask::ChartAnalysis => "Analyze this chart and extract the data and insights.",
            VisionTask::CodeScreenshotReading => "Read the code in this screenshot.",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    pub element_type: String,
    pub label: Option<String>,
    pub bounding_box: Option<BoundingBox>,
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
    pub model: String,
}

pub struct VisionPipeline {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    model: String,
}

impl VisionPipeline {
    pub fn new(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        model: String,
    ) -> Self {
        Self {
            adapter,
            ctx,
            model,
        }
    }

    pub async fn analyze(
        &self,
        image_data: &[u8],
        task: VisionTask,
    ) -> Result<VisionResult, String> {
        use base64::Engine;
        let base64_image = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(image_data)
        );

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::Text(task.system_prompt().to_string()),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::Multipart(vec![
                    ContentPart {
                        r#type: "text".to_string(),
                        text: Some(task.user_prompt().to_string()),
                        image_url: None,
                    },
                    ContentPart {
                        r#type: "image_url".to_string(),
                        text: None,
                        image_url: Some(ImageUrl { url: base64_image }),
                    },
                ]),
                tool_calls: None,
                tool_call_id: None,
                thinking: None,
            },
        ];

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(0.1),
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

        let response = self
            .adapter
            .chat(&self.ctx, request)
            .await
            .map_err(|e| format!("Vision analysis failed: {}", e))?;

        let text_content = if matches!(task, VisionTask::Ocr | VisionTask::CodeScreenshotReading) {
            Some(response.content.clone())
        } else {
            None
        };

        Ok(VisionResult {
            task,
            description: response.content,
            elements: vec![],
            text_content,
            confidence: 0.0,
            model: response.model,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_task_system_prompt() {
        assert!(!VisionTask::ImageDescription.system_prompt().is_empty());
        assert!(!VisionTask::Ocr.system_prompt().is_empty());
        assert!(!VisionTask::UiElementDetection.system_prompt().is_empty());
        assert!(!VisionTask::ChartAnalysis.system_prompt().is_empty());
        assert!(!VisionTask::CodeScreenshotReading.system_prompt().is_empty());
    }

    #[test]
    fn test_vision_task_user_prompt() {
        assert!(!VisionTask::ImageDescription.user_prompt().is_empty());
        assert!(!VisionTask::Ocr.user_prompt().is_empty());
        assert!(!VisionTask::UiElementDetection.user_prompt().is_empty());
        assert!(!VisionTask::ChartAnalysis.user_prompt().is_empty());
        assert!(!VisionTask::CodeScreenshotReading.user_prompt().is_empty());
    }

    #[test]
    fn test_vision_task_variants() {
        let tasks = [
            VisionTask::ImageDescription,
            VisionTask::Ocr,
            VisionTask::UiElementDetection,
            VisionTask::ChartAnalysis,
            VisionTask::CodeScreenshotReading,
        ];
        assert_eq!(tasks.len(), 5);
    }

    #[test]
    fn test_ui_element_serialization() {
        let element = UiElement {
            element_type: "button".to_string(),
            label: Some("Submit".to_string()),
            bounding_box: Some(BoundingBox {
                x: 10.0,
                y: 20.0,
                width: 100.0,
                height: 40.0,
            }),
            actionable: true,
        };
        let json = serde_json::to_string(&element).unwrap();
        let deserialized: UiElement = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.element_type, "button");
        assert_eq!(deserialized.label, Some("Submit".to_string()));
        assert!(deserialized.actionable);
    }

    #[test]
    fn test_bounding_box_serialization() {
        let bbox = BoundingBox {
            x: 1.0,
            y: 2.0,
            width: 100.0,
            height: 50.0,
        };
        let json = serde_json::to_string(&bbox).unwrap();
        let deserialized: BoundingBox = serde_json::from_str(&json).unwrap();
        assert!((deserialized.x - 1.0).abs() < f32::EPSILON);
        assert!((deserialized.width - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_vision_result_serialization() {
        let result = VisionResult {
            task: VisionTask::Ocr,
            description: "Extracted text".to_string(),
            elements: vec![],
            text_content: Some("Extracted text".to_string()),
            confidence: 0.95,
            model: "gpt-4o".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: VisionResult = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.task, VisionTask::Ocr));
        assert_eq!(deserialized.model, "gpt-4o");
        assert!(deserialized.text_content.is_some());
    }

    #[test]
    fn test_vision_result_no_text_content_for_description() {
        let result = VisionResult {
            task: VisionTask::ImageDescription,
            description: "A cat".to_string(),
            elements: vec![],
            text_content: None,
            confidence: 0.8,
            model: "gpt-4o".to_string(),
        };
        assert!(result.text_content.is_none());
    }

    #[test]
    fn test_ui_element_no_bounding_box() {
        let element = UiElement {
            element_type: "link".to_string(),
            label: None,
            bounding_box: None,
            actionable: true,
        };
        assert!(element.bounding_box.is_none());
        assert!(element.label.is_none());
    }

    #[test]
    fn test_vision_task_serialization() {
        let task = VisionTask::ChartAnalysis;
        let json = serde_json::to_string(&task).unwrap();
        let deserialized: VisionTask = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, VisionTask::ChartAnalysis));
    }
}
