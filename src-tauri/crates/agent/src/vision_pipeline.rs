use axagent_core::types::{ChatContent, ChatMessage, ChatRequest, ContentPart, ImageUrl};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
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
                 covering all visible elements, colors, layout, text, and context."
            }
            VisionTask::Ocr => {
                "You are an OCR assistant. Extract all text from the provided image. \
                 Output only the extracted text, preserving the original formatting and line breaks."
            }
            VisionTask::UiElementDetection => {
                "You are a UI analysis assistant. Analyze the provided screenshot and list all \
                 interactive elements (buttons, inputs, links, menus, toggles, etc.) with their \
                 labels, types, and positions. Format as a structured list."
            }
            VisionTask::ChartAnalysis => {
                "You are a chart analysis assistant. Analyze the provided chart/graph image. \
                 Extract data points, labels, axes information, trends, and key insights. \
                 Provide both a summary and structured data when possible."
            }
            VisionTask::CodeScreenshotReading => {
                "You are a code reading assistant. Extract all code visible in the provided \
                 screenshot. Output only the code as plain text, preserving indentation and formatting."
            }
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
