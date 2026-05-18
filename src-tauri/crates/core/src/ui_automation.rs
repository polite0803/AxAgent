use anyhow::Result;
#[cfg(not(target_os = "android"))]
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
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
    pub async fn get_accessible_elements(query: &UIElementQuery) -> Result<Vec<UIElement>> {
        #[cfg(target_os = "windows")]
        {
            Self::get_windows_elements(query).await
        }
        #[cfg(target_os = "macos")]
        {
            Self::get_macos_elements(query).await
        }
        #[cfg(target_os = "linux")]
        {
            Self::get_linux_elements(query).await
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            let _ = query;
            anyhow::bail!("UI 元素枚举不支持当前平台")
        }
    }

    #[cfg(not(target_os = "android"))]
    pub async fn click(x: f64, y: f64, button: MouseButton) -> Result<()> {
        let btn = match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        };
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Enigo 初始化失败: {e}"))?;
        enigo
            .move_mouse(x as i32, y as i32, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("鼠标移动失败: {e}"))?;
        enigo
            .button(btn, Direction::Click)
            .map_err(|e| anyhow::anyhow!("鼠标点击失败: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub async fn click(_x: f64, _y: f64, _button: MouseButton) -> Result<()> {
        anyhow::bail!("UI automation is not supported on Android")
    }

    #[cfg(not(target_os = "android"))]
    pub async fn type_text(text: &str, x: Option<f64>, y: Option<f64>) -> Result<()> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Enigo 初始化失败: {e}"))?;
        if let (Some(cx), Some(cy)) = (x, y) {
            enigo
                .move_mouse(cx as i32, cy as i32, Coordinate::Abs)
                .map_err(|e| anyhow::anyhow!("鼠标移动失败: {e}"))?;
            enigo
                .button(Button::Left, Direction::Click)
                .map_err(|e| anyhow::anyhow!("点击聚焦失败: {e}"))?;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        enigo
            .text(text)
            .map_err(|e| anyhow::anyhow!("文本输入失败: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub async fn type_text(_text: &str, _x: Option<f64>, _y: Option<f64>) -> Result<()> {
        anyhow::bail!("UI automation is not supported on Android")
    }

    #[cfg(not(target_os = "android"))]
    pub async fn press_key(key: &str, modifiers: Vec<KeyModifier>) -> Result<()> {
        let key_enum = map_key(key);
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Enigo 初始化失败: {e}"))?;

        for m in &modifiers {
            let mk = modifier_key(*m);
            enigo
                .key(mk, Direction::Press)
                .map_err(|e| anyhow::anyhow!("按下修饰键失败: {e}"))?;
        }

        enigo
            .key(key_enum, Direction::Click)
            .map_err(|e| anyhow::anyhow!("按键失败: {e}"))?;

        for m in modifiers.iter().rev() {
            let mk = modifier_key(*m);
            enigo
                .key(mk, Direction::Release)
                .map_err(|e| anyhow::anyhow!("释放修饰键失败: {e}"))?;
        }

        Ok(())
    }

    #[cfg(target_os = "android")]
    pub async fn press_key(_key: &str, _modifiers: Vec<KeyModifier>) -> Result<()> {
        anyhow::bail!("UI automation is not supported on Android")
    }

    #[cfg(not(target_os = "android"))]
    pub async fn scroll(x: f64, y: f64, delta: i32) -> Result<()> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Enigo 初始化失败: {e}"))?;
        enigo
            .move_mouse(x as i32, y as i32, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("鼠标移动失败: {e}"))?;
        enigo
            .scroll(-delta, Axis::Vertical)
            .map_err(|e| anyhow::anyhow!("滚动失败: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub async fn scroll(_x: f64, _y: f64, _delta: i32) -> Result<()> {
        anyhow::bail!("UI automation is not supported on Android")
    }

    #[cfg(not(target_os = "android"))]
    pub async fn move_mouse(x: f64, y: f64) -> Result<()> {
        let mut enigo = Enigo::new(&Settings::default())
            .map_err(|e| anyhow::anyhow!("Enigo 初始化失败: {e}"))?;
        enigo
            .move_mouse(x as i32, y as i32, Coordinate::Abs)
            .map_err(|e| anyhow::anyhow!("鼠标移动失败: {e}"))?;
        Ok(())
    }

    #[cfg(target_os = "android")]
    pub async fn move_mouse(_x: f64, _y: f64) -> Result<()> {
        anyhow::bail!("UI automation is not supported on Android")
    }

    // ── Windows 专属: UI 元素枚举 ──

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
            if let Some(name_filter) = query.name_contains
                && !name.contains(&name_filter)
            {
                continue;
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

    // ── macOS: UI 元素枚举 ──

    #[cfg(target_os = "macos")]
    async fn get_macos_elements(query: &UIElementQuery) -> Result<Vec<UIElement>> {
        let script = r#"
tell application "System Events"
    set allProcs to every process whose visible is true
    set output to ""
    repeat with proc in allProcs
        set procName to name of proc
        try
            set winList to every window of proc
            repeat with w in winList
                set {xpos, ypos} to position of w
                set {ww, wh} to size of w
                set winTitle to title of w
                set output to output & procName & "|||" & winTitle & "|||" & xpos & "|||" & ypos & "|||" & ww & "|||" & wh & "\n"
            end repeat
        end try
    end repeat
end tell
return output
"#;

        let output = tokio::process::Command::new("osascript")
            .args(["-e", script])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "osascript 执行失败: {}\n提示: 请在 系统偏好设置 > 隐私与安全性 > 辅助功能 中授权终端/Tauri 应用",
                stderr.trim()
            );
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut elements = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split("|||").collect();
            if parts.len() < 6 {
                continue;
            }
            let app = parts[0].trim();
            let title = parts[1].trim();
            let x: f64 = parts[2].trim().parse().unwrap_or(0.0);
            let y: f64 = parts[3].trim().parse().unwrap_or(0.0);
            let w: f64 = parts[4].trim().parse().unwrap_or(0.0);
            let h: f64 = parts[5].trim().parse().unwrap_or(0.0);

            if let Some(name_filter) = query.name_contains
                && !title.contains(&name_filter)
                && !app.contains(&name_filter)
            {
                continue;
            }

            elements.push(UIElement {
                role: "window".to_string(),
                name: format!("{} - {}", app, title),
                value: None,
                bounds: CGRect {
                    x,
                    y,
                    width: w,
                    height: h,
                },
                is_clickable: w > 0.0 && h > 0.0,
                is_editable: false,
                children_count: None,
                application: app.to_string(),
                window_title: title.to_string(),
            });
        }

        Ok(elements)
    }

    // ── Linux: UI 元素枚举 ──

    #[cfg(target_os = "linux")]
    async fn get_linux_elements(query: &UIElementQuery) -> Result<Vec<UIElement>> {
        // 尝试 wmctrl (X11 窗口管理器)
        if which::which("wmctrl").is_ok() {
            let output = tokio::process::Command::new("wmctrl")
                .args(["-l", "-G"])
                .output()
                .await?;

            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let mut elements = Vec::new();
                for line in text.lines() {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    // wmctrl -l -G 输出格式: 窗口ID 桌面 X Y W H 主机名 标题...
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 8 {
                        let x: f64 = parts[2].parse().unwrap_or(0.0);
                        let y: f64 = parts[3].parse().unwrap_or(0.0);
                        let w: f64 = parts[4].parse().unwrap_or(0.0);
                        let h: f64 = parts[5].parse().unwrap_or(0.0);
                        let title = parts[7..].join(" ");

                        if let Some(name_filter) = query.name_contains
                            && !title.contains(&name_filter)
                        {
                            continue;
                        }

                        elements.push(UIElement {
                            role: "window".to_string(),
                            name: title.clone(),
                            value: None,
                            bounds: CGRect {
                                x,
                                y,
                                width: w,
                                height: h,
                            },
                            is_clickable: w > 0.0 && h > 0.0,
                            is_editable: false,
                            children_count: None,
                            application: String::new(),
                            window_title: title,
                        });
                    }
                }
                return Ok(elements);
            }
        }

        // Wayland 回退: 尝试 AT-SPI2 通过 gdbus
        if which::which("gdbus").is_ok() {
            let output = tokio::process::Command::new("gdbus")
                .args([
                    "call",
                    "--session",
                    "--dest",
                    "org.a11y.atspi.Registry",
                    "--object-path",
                    "/org/a11y/atspi/accessible/root",
                    "--method",
                    "org.a11y.atspi.Accessible.GetChildCount",
                ])
                .output()
                .await?;

            if output.status.success() {
                // AT-SPI2 可用，返回有限的信息
                return Ok(vec![UIElement {
                    role: "desktop".to_string(),
                    name: "Accessible desktop (AT-SPI2)".to_string(),
                    value: None,
                    bounds: CGRect {
                        x: 0.0,
                        y: 0.0,
                        width: 0.0,
                        height: 0.0,
                    },
                    is_clickable: false,
                    is_editable: false,
                    children_count: None,
                    application: String::new(),
                    window_title: String::new(),
                }]);
            }
        }

        anyhow::bail!(
            "Linux 窗口枚举需要 wmctrl (X11) 或 AT-SPI2 (Wayland)。\n\
             安装 wmctrl: sudo apt install wmctrl / sudo pacman -S wmctrl"
        )
    }
}

#[cfg(not(target_os = "android"))]
fn map_key(key: &str) -> Key {
    match key {
        "Enter" | "enter" | "Return" => Key::Return,
        "Tab" | "tab" => Key::Tab,
        "Escape" | "escape" | "Esc" | "esc" => Key::Escape,
        "Backspace" | "backspace" => Key::Backspace,
        "Delete" | "delete" | "Del" => Key::Delete,
        "Space" | "space" | " " => Key::Space,
        "Up" | "up" | "ArrowUp" => Key::UpArrow,
        "Down" | "down" | "ArrowDown" => Key::DownArrow,
        "Left" | "left" | "ArrowLeft" => Key::LeftArrow,
        "Right" | "right" | "ArrowRight" => Key::RightArrow,
        "Home" | "home" => Key::Home,
        "End" | "end" => Key::End,
        "PageUp" | "pageup" => Key::PageUp,
        "PageDown" | "pagedown" => Key::PageDown,
        "Insert" | "insert" => {
            #[cfg(not(target_os = "macos"))]
            {
                Key::Insert
            }
            #[cfg(target_os = "macos")]
            {
                Key::Unicode('⎀')
            }
        },
        "CapsLock" | "capslock" => Key::CapsLock,
        s if s.starts_with('F') || s.starts_with('f') => {
            let n: u8 = s[1..].parse().unwrap_or(1);
            match n {
                1 => Key::F1,
                2 => Key::F2,
                3 => Key::F3,
                4 => Key::F4,
                5 => Key::F5,
                6 => Key::F6,
                7 => Key::F7,
                8 => Key::F8,
                9 => Key::F9,
                10 => Key::F10,
                11 => Key::F11,
                12 => Key::F12,
                13 => Key::F13,
                14 => Key::F14,
                15 => Key::F15,
                16 => Key::F16,
                17 => Key::F17,
                18 => Key::F18,
                19 => Key::F19,
                20 => Key::F20,
                _ => Key::F1,
            }
        },
        _ => {
            // 尝试作为单字符 Unicode
            if let Some(c) = key.chars().next() {
                Key::Unicode(c)
            } else {
                Key::Unicode('?')
            }
        },
    }
}

#[cfg(not(target_os = "android"))]
fn modifier_key(m: KeyModifier) -> Key {
    match m {
        KeyModifier::Alt => Key::Alt,
        KeyModifier::Control => Key::Control,
        KeyModifier::Shift => Key::Shift,
        KeyModifier::Super => Key::Meta,
    }
}
