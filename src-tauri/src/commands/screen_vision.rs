use axagent_core::screen_capture::ScreenCapture;
use axagent_core::screen_vision::{ScreenVisionAnalyzer, UIElementInfo};
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenAnalysisResult {
    pub elements: Vec<UIElementInfo>,
    pub suggested_actions: Vec<SuggestedActionInfo>,
    pub reasoning: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestedActionInfo {
    pub action_type: String,
    pub target_element: String,
    pub description: String,
    pub reasoning: String,
    pub x: f64,
    pub y: f64,
}

#[command]
pub async fn analyze_screen(
    task_description: String,
    monitor_index: Option<u32>,
) -> Result<ScreenAnalysisResult, String> {
    let capture = ScreenCapture::new();
    let screenshot = capture
        .capture_full(monitor_index)
        .await
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    let analyzer = ScreenVisionAnalyzer::default();
    let analysis = analyzer
        .analyze_screen(&screenshot.image_base64, &task_description)
        .await
        .map_err(|e| format!("Screen analysis failed: {}", e))?;

    let suggested_actions: Vec<SuggestedActionInfo> = analysis
        .suggested_actions
        .iter()
        .map(|action| {
            let (x, y) = if let Some(element) = analysis
                .elements
                .iter()
                .find(|e| e.name == action.target_element)
            {
                (
                    element.bounds.x + element.bounds.width / 2.0,
                    element.bounds.y + element.bounds.height / 2.0,
                )
            } else {
                (0.0, 0.0)
            };

            SuggestedActionInfo {
                action_type: format!("{:?}", action.action_type).to_lowercase(),
                target_element: action.target_element.clone(),
                description: action.description.clone(),
                reasoning: action.reasoning.clone(),
                x,
                y,
            }
        })
        .collect();

    Ok(ScreenAnalysisResult {
        elements: analysis.elements,
        suggested_actions,
        reasoning: analysis.reasoning,
        confidence: analysis.confidence,
    })
}

#[command]
pub async fn find_element_on_screen(
    element_description: String,
    monitor_index: Option<u32>,
) -> Result<Option<UIElementInfo>, String> {
    let capture = ScreenCapture::new();
    let screenshot = capture
        .capture_full(monitor_index)
        .await
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    let analyzer = ScreenVisionAnalyzer::default();
    let element = analyzer
        .find_element(&screenshot.image_base64, &element_description)
        .await
        .map_err(|e| format!("Element search failed: {}", e))?;

    Ok(element)
}

#[command]
pub async fn suggest_screen_action(
    current_task: String,
    monitor_index: Option<u32>,
) -> Result<Vec<SuggestedActionInfo>, String> {
    let capture = ScreenCapture::new();
    let screenshot = capture
        .capture_full(monitor_index)
        .await
        .map_err(|e| format!("Screen capture failed: {}", e))?;

    let analyzer = ScreenVisionAnalyzer::default();
    let analysis = analyzer
        .analyze_screen(&screenshot.image_base64, &current_task)
        .await
        .map_err(|e| format!("Screen analysis failed: {}", e))?;

    let suggested_actions: Vec<SuggestedActionInfo> = analysis
        .suggested_actions
        .iter()
        .map(|action| {
            let (x, y) = if let Some(element) = analysis
                .elements
                .iter()
                .find(|e| e.name == action.target_element)
            {
                (
                    element.bounds.x + element.bounds.width / 2.0,
                    element.bounds.y + element.bounds.height / 2.0,
                )
            } else {
                (0.0, 0.0)
            };

            SuggestedActionInfo {
                action_type: format!("{:?}", action.action_type).to_lowercase(),
                target_element: action.target_element.clone(),
                description: action.description.clone(),
                reasoning: action.reasoning.clone(),
                x,
                y,
            }
        })
        .collect();

    Ok(suggested_actions)
}

#[command]
pub async fn click_element_at_position(
    x: f64,
    y: f64,
    button: Option<String>,
) -> Result<(), String> {
    use axagent_core::ui_automation::MouseButton;

    let btn = match button.as_deref().unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    };

    axagent_core::ui_automation::UIAutomation::click(x, y, btn)
        .await
        .map_err(|e| format!("Click failed: {}", e))?;

    Ok(())
}

#[command]
pub async fn execute_vision_action(
    action_type: String,
    x: f64,
    y: f64,
    text: Option<String>,
) -> Result<(), String> {
    use axagent_core::ui_automation::UIAutomation;

    match action_type.to_lowercase().as_str() {
        "click" => {
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Click failed: {}", e))?;
        }
        "double_click" | "doubleclick" => {
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Click failed: {}", e))?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Left)
                .await
                .map_err(|e| format!("Double click failed: {}", e))?;
        }
        "right_click" | "rightclick" => {
            UIAutomation::click(x, y, axagent_core::ui_automation::MouseButton::Right)
                .await
                .map_err(|e| format!("Right click failed: {}", e))?;
        }
        "type" | "input" => {
            if let Some(text) = text {
                UIAutomation::type_text(&text, Some(x), Some(y))
                    .await
                    .map_err(|e| format!("Type failed: {}", e))?;
            }
        }
        "hover" => {
            UIAutomation::move_mouse(x, y)
                .await
                .map_err(|e| format!("Hover failed: {}", e))?;
        }
        _ => {
            return Err(format!("Unknown action type: {}", action_type));
        }
    }

    Ok(())
}
