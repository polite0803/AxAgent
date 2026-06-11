// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[deprecated(
    since = "0.1.0",
    note = "Use ProviderType from axagent_harness::types instead. VisionProvider is no longer used by the provider-based screen_vision implementation."
)]
#[allow(deprecated)]
pub enum VisionProvider {
    #[default]
    Anthropic,
    OpenAI,
    Gemini,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(
    since = "0.1.0",
    note = "No longer used. The provider-based screen_vision functions accept image_base64 and task_description directly."
)]
pub struct VisionPrompt {
    pub task_description: String,
    pub image_base64: String,
}

#[deprecated(
    since = "0.1.0",
    note = "Use axagent_providers::screen_vision functions with a ProviderAdapter and ProviderRequestContext instead."
)]
#[allow(deprecated)]
pub struct ScreenVisionAnalyzer {
    pub provider: VisionProvider,
}

#[allow(deprecated)]
impl ScreenVisionAnalyzer {
    pub fn new(provider: VisionProvider) -> Self {
        Self { provider }
    }

    pub async fn analyze_screen(
        &self,
        _image_base64: &str,
        _task_description: &str,
    ) -> anyhow::Result<ScreenAnalysis> {
        Err(anyhow::anyhow!(
            "ScreenVisionAnalyzer is deprecated. Use axagent_providers::screen_vision::analyze_screen instead."
        ))
    }

    pub async fn find_element(
        &self,
        _image_base64: &str,
        _element_description: &str,
    ) -> anyhow::Result<Option<UIElementInfo>> {
        Err(anyhow::anyhow!(
            "ScreenVisionAnalyzer is deprecated. Use axagent_providers::screen_vision::find_element instead."
        ))
    }

    pub async fn suggest_next_action(
        &self,
        _image_base64: &str,
        _current_task: &str,
    ) -> anyhow::Result<Vec<SuggestedAction>> {
        Err(anyhow::anyhow!(
            "ScreenVisionAnalyzer is deprecated. Use axagent_providers::screen_vision::suggest_next_action instead."
        ))
    }
}

#[allow(deprecated)]
impl Default for ScreenVisionAnalyzer {
    fn default() -> Self {
        Self::new(VisionProvider::Anthropic)
    }
}

impl FromStr for ActionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "click" => Ok(ActionType::Click),
            "double_click" | "doubleclick" => Ok(ActionType::DoubleClick),
            "right_click" | "rightclick" => Ok(ActionType::RightClick),
            "type" | "input" => Ok(ActionType::Type),
            "hover" | "mouse_over" => Ok(ActionType::Hover),
            "scroll" => Ok(ActionType::Scroll),
            "select" => Ok(ActionType::Select),
            _ => Err(()),
        }
    }
}
