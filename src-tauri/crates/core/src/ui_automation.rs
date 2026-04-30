use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElement {
    pub role: String,
    pub name: String,
    pub value: Option<String>,
    pub bounds: CGRect,
    pub is_clickable: bool,
    pub is_editable: bool,
    pub children_count: Option<usize>,
    pub application: String,
    pub window_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CGRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElementQuery {
    pub role: Option<String>,
    pub name_contains: Option<String>,
    pub value_contains: Option<String>,
    pub application: Option<String>,
    pub window_title: Option<String>,
    pub max_depth: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyModifier {
    Alt,
    Control,
    Shift,
    Super,
}

pub struct UIAutomation;

impl UIAutomation {
    #[allow(unused_variables)]
    pub async fn get_accessible_elements(query: &UIElementQuery) -> Result<Vec<UIElement>> {
        #[cfg(target_os = "windows")]
        {
            Self::get_windows_elements(query).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            anyhow::bail!("UI automation not yet supported on this platform")
        }
    }

    #[allow(unused_variables)]
    pub async fn click(x: f64, y: f64, button: MouseButton) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            Self::windows_click(x, y, button).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            anyhow::bail!("Click not yet supported on this platform")
        }
    }

    #[allow(unused_variables)]
    pub async fn type_text(text: &str, x: Option<f64>, y: Option<f64>) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            Self::windows_type_text(text, x, y).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            anyhow::bail!("Type text not yet supported on this platform")
        }
    }

    pub async fn press_key(key: &str, modifiers: Vec<KeyModifier>) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            Self::windows_press_key(key, modifiers).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            anyhow::bail!("Key press not yet supported on this platform")
        }
    }

    #[allow(unused_variables)]
    pub async fn scroll(x: f64, y: f64, delta: i32) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            Self::windows_scroll(x, y, delta).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            anyhow::bail!("Scroll not yet supported on this platform")
        }
    }

    #[cfg(target_os = "windows")]
    async fn get_windows_elements(query: &UIElementQuery) -> Result<Vec<UIElement>> {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
$ui = [System.Windows.Automation.AutomationElement]::RootElement
$cond = [System.Windows.Automation.Condition]::TrueCondition
$elements = $ui.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)
$results = @()
foreach ($el in $elements) {
    try {
        $rect = $el.Current.BoundingRectangle
        if ($rect.Width -gt 0 -and $rect.Height -gt 0) {
            $results += @{
                role = $el.Current.ControlType.ProgrammaticName
                name = $el.Current.Name
                x = $rect.X
                y = $rect.Y
                width = $rect.Width
                height = $rect.Height
                isClickable = -not $el.Current.IsOffscreen
            }
        }
    } catch {}
}
$results | ConvertTo-Json -Compress
"#;

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", script])
            .output()
            .await?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let raw_elements: Vec<serde_json::Value> =
            serde_json::from_str(&json_str).unwrap_or_default();

        let mut elements = Vec::new();
        for raw in raw_elements {
            let name = raw["name"].as_str().unwrap_or("").to_string();
            if let Some(ref name_filter) = query.name_contains {
                if !name.contains(name_filter) {
                    continue;
                }
            }

            elements.push(UIElement {
                role: raw["role"].as_str().unwrap_or("unknown").to_string(),
                name,
                value: None,
                bounds: CGRect {
                    x: raw["x"].as_f64().unwrap_or(0.0),
                    y: raw["y"].as_f64().unwrap_or(0.0),
                    width: raw["width"].as_f64().unwrap_or(0.0),
                    height: raw["height"].as_f64().unwrap_or(0.0),
                },
                is_clickable: raw["isClickable"].as_bool().unwrap_or(false),
                is_editable: false,
                children_count: None,
                application: String::new(),
                window_title: String::new(),
            });
        }

        Ok(elements)
    }

    #[cfg(target_os = "windows")]
    async fn windows_click(x: f64, y: f64, button: MouseButton) -> Result<()> {
        let btn_str = match button {
            MouseButton::Left => "Left",
            MouseButton::Right => "Right",
            MouseButton::Middle => "Middle",
        };

        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({}, {})
Start-Sleep -Milliseconds 50
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Mouse {{
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);
}}
"@
$click_flag = switch ("{}") {{ "Left" {{ 0x02 }} "Right" {{ 0x08 }} "Middle" {{ 0x20 }} }}
$up_flag = switch ("{}") {{ "Left" {{ 0x04 }} "Right" {{ 0x10 }} "Middle" {{ 0x40 }} }}
[Mouse]::mouse_event($click_flag, 0, 0, 0, [IntPtr]::Zero)
Start-Sleep -Milliseconds 30
[Mouse]::mouse_event($up_flag, 0, 0, 0, [IntPtr]::Zero)
"#,
            x as i32, y as i32, btn_str, btn_str
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Click failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn windows_type_text(text: &str, x: Option<f64>, y: Option<f64>) -> Result<()> {
        if let (Some(cx), Some(cy)) = (x, y) {
            Self::windows_click(cx, cy, MouseButton::Left).await?;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        let escaped = text
            .replace("'", "''")
            .replace("+", "{+}")
            .replace("^", "{^}")
            .replace("%", "{%}")
            .replace("~", "{~}")
            .replace("(", "{(}")
            .replace(")", "{)}");

        let script = format!(
            r#"
$wshell = New-Object -ComObject WScript.Shell
$wshell.SendKeys('{}')
"#,
            escaped
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "Type text failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn windows_press_key(key: &str, modifiers: Vec<KeyModifier>) -> Result<()> {
        let mut key_str = String::new();
        for m in &modifiers {
            key_str.push_str(match m {
                KeyModifier::Alt => "%",
                KeyModifier::Control => "^",
                KeyModifier::Shift => "+",
                KeyModifier::Super => "^",
            });
        }
        key_str.push_str(key);

        let script = format!(
            r#"
$wshell = New-Object -ComObject WScript.Shell
$wshell.SendKeys('{}')
"#,
            key_str
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "Key press failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    #[cfg(target_os = "windows")]
    async fn windows_scroll(x: f64, y: f64, delta: i32) -> Result<()> {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({}, {})
Start-Sleep -Milliseconds 50
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Mouse {{
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);
}}
"@
[Mouse]::mouse_event(0x0800, 0, 0, {}, [IntPtr]::Zero)
"#,
            x as i32,
            y as i32,
            delta * 120
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("Scroll failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    #[allow(unused_variables)]
    pub async fn move_mouse(x: f64, y: f64) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            Self::windows_move_mouse(x, y).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            anyhow::bail!("Move mouse not yet supported on this platform")
        }
    }

    #[cfg(target_os = "windows")]
    async fn windows_move_mouse(x: f64, y: f64) -> Result<()> {
        let script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
[System.Windows.Forms.Cursor]::Position = New-Object System.Drawing.Point({}, {})
"#,
            x as i32, y as i32
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "Move mouse failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }
}
