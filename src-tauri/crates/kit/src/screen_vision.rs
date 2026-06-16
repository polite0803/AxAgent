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
