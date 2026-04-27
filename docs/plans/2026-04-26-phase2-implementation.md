# Phase 2: 主动 Agent 能力 - 详细实施计划

> 阶段: Phase 2
> 时间: 2026-07-01 至 2026-10-01（12 周）
> 前置: Phase 1 已完成（图像生成、Artifact 预览增强、图表生成）
> 目标: 赋予 AxAgent 计算机控制能力，实现真正的自主操作
> 基线审计: 2026-04-26 基于实际代码分析更新

---

## 现有代码基线（基于实际代码审计）

### 已完成模块

| 模块 | 状态 | 实际位置 | 说明 |
|------|------|---------|------|
| Provider 适配器 | ✅ | `crates/providers/` | 12 个 provider 模块（含 image_gen.rs），支持 image_urls 多模态输入 |
| 图像生成 | ✅ | `crates/providers/src/image_gen.rs` (249行) | FluxProvider(Replicate) + DallEProvider(OpenAI)，通过 `ImageGenProvider` trait |
| 图像生成命令 | ✅ | `commands/image_gen.rs` (43行) + `commands/image_gen_settings.rs` (57行) | Tauri 命令，**非 builtin tool**，需从前端传 apiKey |
| 图像生成 UI | ✅ | `ImageGenPanel.tsx` (157行) + `ImageGenSettings.tsx` (100行) + `imageGenStore.ts` (96行) | Flux/DALL-E 切换、尺寸预设、历史记录 |
| 图表生成 | ✅ | `commands/chart_generator.rs` (132行) | Tauri 命令，LLM→ECharts option，**非 builtin tool** |
| 图表生成前端 | ✅ | `chartGenerator.ts` (65行) + `ChartPreview.tsx` | 意图检测 + ECharts iframe 渲染（CDN） |
| Artifact 预览 | ✅ | `ArtifactPreview/` (7个文件) | ChartPreview/CodePreview/HtmlPreview/MarkdownPreview/ReactPreview/SplitView |
| 沙箱执行 | ✅ | `commands/sandbox.rs` + `codeExecutor.ts` (144行) | JS: Tauri sandbox 命令；Python: Pyodide (WASM CDN) |
| HTML 解析 | ✅ | `htmlParser.ts` (59行) | DOMParser 分离 CSS/JS/HTML + isChartOption() 检测 |
| Artifact 渲染 | ✅ | `artifactRenderer.ts` (104行) | sandbox/SVG/Mermaid(mermaid.ink)/D2(d2lang.com) 渲染 |

### 内置工具系统（实际架构）

**工具定义**位于 `builtin_tools_registry.rs`，**处理器**位于 `builtin_tools.rs`，两者分离：

| Server ID | Server Name | 工具列表 |
|-----------|-------------|---------|
| builtin-fetch | @axagent/fetch | fetch_url, fetch_markdown |
| builtin-search-file | @axagent/search-file | read_file, list_directory, search_files, grep_content |
| builtin-filesystem | @axagent/filesystem | write_file, edit_file, delete_file, create_directory, file_exists, get_file_info, move_file |
| builtin-shell | @axagent/shell | run_command, get_system_info, list_processes |
| builtin-web | @axagent/web | web_search |
| builtin-knowledge | @axagent/knowledge | knowledge_search |
| builtin-storage | @axagent/storage | get_storage_info, list_storage_files, upload_storage_file, download_storage_file, delete_storage_file |

**动态工具**: skill_manage, session_search, memory_flush（通过 `get_dynamic_builtin_tools()`）

⚠️ **重要**: `generate_image` 和 `generate_chart_config` 是 **Tauri 命令**（注册在 `lib.rs`），**不是 builtin tool**。它们没有注册到工具系统中，LLM 无法通过工具调用链路使用。

### 文件安全机制

- `validate_and_resolve_path()`: 路径遍历防护
- `ALLOWED_FILE_DIRECTORIES`: `["workspace", "documents", "downloads", "skills"]`
- `sanitize_fts5_query()`: FTS5 SQL 注入防护
- `MAX_BASE64_DECODE_SIZE = 100MB`

### Phase 1 未完成项（需在 Phase 2 中补齐或接受现状）

| 项目 | 计划 | 实际 | 影响 |
|------|------|------|------|
| ArtifactPanel 升级 | 完整工作区 UI + 工具栏 + 预览模式切换 | 仍为简单 Card + "comingSoon" | Phase 2 计算机控制面板需新建独立组件 |
| MarkdownPreview 升级 | markstream-react | 仍为基础 regex 替换 | 不影响 Phase 2 核心，可后续补齐 |
| generate_image/generate_chart 工具化 | 注册为 builtin tool | 仅 Tauri 命令 | Phase 2 需补齐工具注册 |
| npm 新依赖 | echarts, playwright 等 | 未添加（ECharts 用 CDN） | Phase 2 需添加 playwright npm 包 |

### 现有缺口（Phase 2 需填补）

- ❌ 无屏幕截图能力
- ❌ 无 UI 元素定位与交互
- ❌ 无浏览器自动化
- ❌ 无键盘/鼠标模拟
- ❌ 文件操作仍受限，缺少临时授权机制
- ❌ generate_image/generate_chart 未注册为 builtin tool（LLM 无法自主调用）
- ❌ 无操作审计与风险确认机制
- ❌ ArtifactPanel 仍为简陋原型

---

## 模块 1: 屏幕感知与计算机控制（Week 1-5）

### 1.1 架构设计

```
用户指令 → LLM 规划 → 计算机控制工具调用
                         ↓
              ┌──────────┼──────────┐
              ↓          ↓          ↓
         screen_     click/      type_text
         capture     hover       (keyboard)
              ↓          ↓          ↓
         视觉模型分析   Tauri 命令层   Tauri 命令层
              ↓
         返回 UI 元素信息 → LLM 决策下一步
```

核心设计原则：
- **像素级**不是唯一路径——优先使用 accessibility API 定位元素
- **截图+视觉模型**作为 fallback，用于无 accessibility 信息的场景
- **用户确认**：高风险操作（删除文件、发送消息）必须确认
- **操作录制**：所有计算机控制操作可回放、可审计

### 1.2 Rust 后端：屏幕截图

**新增文件**: `src-tauri/crates/core/src/screen_capture.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenCaptureResult {
    pub image_base64: String,
    pub width: u32,
    pub height: u32,
    pub monitor_index: u32,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 屏幕截图引擎
pub struct ScreenCapture {
    /// 截图保存目录
    temp_dir: PathBuf,
}

impl ScreenCapture {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("axagent_captures");
        let _ = std::fs::create_dir_all(&temp_dir);
        Self { temp_dir }
    }

    /// 截取全屏
    pub async fn capture_full(&self, monitor: Option<u32>) -> Result<ScreenCaptureResult> {
        #[cfg(target_os = "windows")]
        {
            self.capture_windows_full(monitor.unwrap_or(0)).await
        }
        #[cfg(target_os = "macos")]
        {
            self.capture_macos_full(monitor.unwrap_or(0)).await
        }
        #[cfg(target_os = "linux")]
        {
            self.capture_linux_full(monitor.unwrap_or(0)).await
        }
    }

    /// 截取指定区域
    pub async fn capture_region(&self, region: CaptureRegion) -> Result<ScreenCaptureResult> {
        #[cfg(target_os = "windows")]
        {
            self.capture_windows_region(region).await
        }
        // macOS/Linux 类似实现
        #[cfg(not(target_os = "windows"))]
        {
            // fallback: 截全屏后裁剪
            let full = self.capture_full(None).await?;
            self.crop_capture(&full, region)
        }
    }

    /// 截取指定窗口
    pub async fn capture_window(&self, window_title: &str) -> Result<ScreenCaptureResult> {
        #[cfg(target_os = "windows")]
        {
            self.capture_windows_by_title(window_title).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            // macOS: 使用 screencapture -l <windowid>
            // Linux: 使用 xdotool + import
            anyhow::bail!("Window capture not yet supported on this platform")
        }
    }
}

// --- Windows 实现 ---

#[cfg(target_os = "windows")]
impl ScreenCapture {
    async fn capture_windows_full(&self, monitor_index: u32) -> Result<ScreenCaptureResult> {
        use xcap::Monitor;

        let monitors = Monitor::all()?;
        let monitor = monitors.get(monitor_index as usize)
            .ok_or_else(|| anyhow::anyhow!("Monitor {} not found", monitor_index))?;

        let image = monitor.capture_image()?;
        let width = image.width();
        let height = image.height();

        let base64 = self.image_to_base64(&image)?;

        Ok(ScreenCaptureResult {
            image_base64: base64,
            width,
            height,
            monitor_index,
            captured_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn capture_windows_region(&self, region: CaptureRegion) -> Result<ScreenCaptureResult> {
        let full = self.capture_windows_full(0).await?;
        let full_image = self.base64_to_image(&full.image_base64)?;
        let cropped = crop_image(&full_image, region.x, region.y, region.width, region.height)?;
        let base64 = self.image_to_base64(&cropped)?;

        Ok(ScreenCaptureResult {
            image_base64: base64,
            width: region.width,
            height: region.height,
            monitor_index: 0,
            captured_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn capture_windows_by_title(&self, window_title: &str) -> Result<ScreenCaptureResult> {
        use xcap::Window;

        let windows = Window::all()?;
        let window = windows.iter().find(|w| {
            w.title().map(|t| t.contains(window_title)).unwrap_or(false)
        }).ok_or_else(|| anyhow::anyhow!("Window '{}' not found", window_title))?;

        let image = window.capture_image()?;
        let width = image.width();
        let height = image.height();
        let base64 = self.image_to_base64(&image)?;

        Ok(ScreenCaptureResult {
            image_base64: base64,
            width,
            height,
            monitor_index: 0,
            captured_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn image_to_base64(&self, image: &image::RgbaImage) -> Result<String> {
        let mut png_data = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
        encoder.write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )?;
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &png_data,
        ))
    }

    fn base64_to_image(&self, base64_str: &str) -> Result<image::RgbaImage> {
        let bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            base64_str,
        )?;
        let img = image::load_from_memory(&bytes)?;
        Ok(img.to_rgba8())
    }
}

fn crop_image(
    img: &image::RgbaImage,
    x: i32, y: i32,
    w: u32, h: u32,
) -> Result<image::RgbaImage> {
    let (img_w, img_h) = (img.width() as i32, img.height() as i32);
    let x0 = x.max(0) as u32;
    let y0 = y.max(0) as u32;
    let x1 = (x + w as i32).min(img_w) as u32;
    let y1 = (y + h as i32).min(img_h) as u32;
    Ok(image::imageops::crop(img, x0, y0, x1 - x0, y1 - y0).to_image())
}
```

**Cargo.toml 新增依赖**:

```toml
# src-tauri/crates/core/Cargo.toml
[target.'cfg(windows)'.dependencies]
xcap = "0.0.13"

[dependencies]
image = { version = "0.25", features = ["png"] }
base64 = "0.22"
chrono = "0.4"
```

### 1.3 Rust 后端：UI 元素定位

**新增文件**: `src-tauri/crates/core/src/ui_automation.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIElement {
    pub role: String,           // button, text, link, input, etc.
    pub name: String,           // accessible name
    pub value: Option<String>,  // current value
    pub bounds: CGRect,         // 屏幕坐标
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

/// UI 自动化引擎
pub struct UIAutomation;

impl UIAutomation {
    /// 获取当前焦点窗口的所有可访问元素
    pub async fn get_accessible_elements(
        query: &UIElementQuery,
    ) -> Result<Vec<UIElement>> {
        #[cfg(target_os = "windows")]
        {
            Self::get_windows_elements(query).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            anyhow::bail!("UI automation not yet supported on this platform")
        }
    }

    /// 点击指定坐标
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

    /// 在指定位置输入文本
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

    /// 按下键盘快捷键
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

    /// 滚动
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyModifier {
    Alt,
    Control,
    Shift,
    Super,
}

// --- Windows 实现（使用 Windows Accessibility API / UIAutomation）---

#[cfg(target_os = "windows")]
impl UIAutomation {
    async fn get_windows_elements(query: &UIElementQuery) -> Result<Vec<UIElement>> {
        // 使用 windows crate 调用 UIAutomation COM API
        // 简化版：通过 PowerShell 脚本获取
        let mut script = String::from(
            "Add-Type -AssemblyName System.Windows.Forms\n"
        );

        if let Some(ref app) = query.application {
            script.push_str(&format!(
                "$proc = Get-Process -Name '{}' -ErrorAction SilentlyContinue\n",
                app
            ));
        }

        // Fallback: 使用 UIAutomation via PowerShell
        script.push_str(
            r#"
$ui = [System.Windows.Automation.AutomationElement]::RootElement
$cond = [System.Windows.Automation.Condition]::TrueCondition
$elements = $ui.FindAll([System.Windows.Automation.TreeScope]::Children, $cond)
$results = @()
foreach ($el in $elements) {
    $rect = $el.Current.BoundingRectangle
    $results += @{
        role = $el.Current.ControlType.ProgrammaticName
        name = $el.Current.Name
        x = $rect.X
        y = $rect.Y
        width = $rect.Width
        height = $rect.Height
        isClickable = $el.Current.IsOffscreen -eq $false
    }
}
$results | ConvertTo-Json -Compress
"#,
        );

        let output = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
            .await?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let raw_elements: Vec<serde_json::Value> = serde_json::from_str(&json_str)
            .unwrap_or_default();

        let mut elements = Vec::new();
        for raw in raw_elements {
            let name = raw["name"].as_str().unwrap_or("").to_string();
            if let Some(ref name_filter) = query.name_contains {
                if !name.contains(name_filter) { continue; }
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
$wshell = New-Object -ComObject WScript.Shell
# 使用 mouse_event
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

    async fn windows_type_text(text: &str, x: Option<f64>, y: Option<f64>) -> Result<()> {
        // 如果有坐标，先点击
        if let (Some(cx), Some(cy)) = (x, y) {
            Self::windows_click(cx, cy, MouseButton::Left).await?;
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        // 使用 SendKeys
        let escaped = text.replace("'", "''").replace("+", "{+}").replace("^", "{^}").replace("%", "{%}");
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
            anyhow::bail!("Type text failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

    async fn windows_press_key(key: &str, modifiers: Vec<KeyModifier>) -> Result<()> {
        let mut key_str = String::new();
        for m in &modifiers {
            key_str.push_str(match m {
                KeyModifier::Alt => "%",
                KeyModifier::Control => "^",
                KeyModifier::Shift => "+",
                KeyModifier::Super => "^", // Windows键映射为 Ctrl
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
            anyhow::bail!("Key press failed: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(())
    }

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
# MOUSEEVENTF_WHEEL = 0x0800
[Mouse]::mouse_event(0x0800, 0, 0, {}, [IntPtr]::Zero)
"#,
            x as i32, y as i32, delta * 120  // WHEEL_DELTA = 120
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
}
```

### 1.4 Tauri 命令

**新增文件**: `src-tauri/src/commands/computer_control.rs`

```rust
use axagent_core::screen_capture::{ScreenCapture, CaptureRegion};
use axagent_core::ui_automation::{
    UIAutomation, UIElementQuery, MouseButton, KeyModifier,
};
use tauri::State;

#[tauri::command]
pub async fn screen_capture(
    monitor: Option<u32>,
    region: Option<CaptureRegion>,
    window_title: Option<String>,
) -> Result<serde_json::Value, String> {
    let capture = ScreenCapture::new();
    let result = match (region, window_title) {
        (Some(r), _) => capture.capture_region(r).await,
        (_, Some(title)) => capture.capture_window(&title).await,
        _ => capture.capture_full(monitor).await,
    };
    result.map(|r| serde_json::to_value(r).unwrap()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn find_ui_elements(
    query: UIElementQuery,
) -> Result<Vec<serde_json::Value>, String> {
    UIAutomation::get_accessible_elements(&query)
        .await
        .map(|elems| elems.iter().map(|e| serde_json::to_value(e).unwrap()).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_click(
    x: f64,
    y: f64,
    button: Option<String>,  // "left" | "right" | "middle"
) -> Result<(), String> {
    let btn = match button.as_deref().unwrap_or("left") {
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => MouseButton::Left,
    };
    UIAutomation::click(x, y, btn).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn type_text(
    text: String,
    x: Option<f64>,
    y: Option<f64>,
) -> Result<(), String> {
    UIAutomation::type_text(&text, x, y).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn press_key(
    key: String,
    modifiers: Vec<String>,  // ["alt", "control", "shift"]
) -> Result<(), String> {
    let mods: Vec<KeyModifier> = modifiers.iter().map(|m| match m.as_str() {
        "alt" => KeyModifier::Alt,
        "control" | "ctrl" => KeyModifier::Control,
        "shift" => KeyModifier::Shift,
        "super" | "meta" | "win" => KeyModifier::Super,
        _ => KeyModifier::Control,
    }).collect();
    UIAutomation::press_key(&key, mods).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mouse_scroll(
    x: f64,
    y: f64,
    delta: i32,  // 正数=向上, 负数=向下
) -> Result<(), String> {
    UIAutomation::scroll(x, y, delta).await.map_err(|e| e.to_string())
}
```

### 1.5 内置工具注册

**⚠️ 架构适配**: AxAgent 的工具系统采用"定义+处理器"分离架构：
- **工具定义**（schema）→ `builtin_tools_registry.rs` 的 `BuiltinServerDefinition` / `BuiltinToolDefinition`
- **处理器**（handler）→ `builtin_tools.rs` 的 `init_builtin_handlers()` 中 `register_builtin_handler()`
- **Tauri 命令**→ `commands/*.rs` 中 `#[tauri::command]`，在 `lib.rs` 的 `generate_handler![]` 中注册

Phase 2 的计算机控制工具需同时完成三处注册。

#### 1.5.1 工具定义（builtin_tools_registry.rs）

**修改文件**: `src-tauri/crates/core/src/builtin_tools_registry.rs`

在 `get_all_builtin_server_definitions()` 中新增：

```rust
BuiltinServerDefinition {
    server_id: "builtin-computer-control".to_string(),
    server_name: "@axagent/computer-control".to_string(),
    tools: vec![
        BuiltinToolDefinition {
            tool_name: "screen_capture".to_string(),
            description: "Capture a screenshot of the screen, a specific region, or a window. Returns a base64-encoded PNG image.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "monitor": { "type": "integer", "description": "Monitor index (default: 0)" },
                    "region": {
                        "type": "object",
                        "description": "Capture region (optional)",
                        "properties": {
                            "x": { "type": "integer" },
                            "y": { "type": "integer" },
                            "width": { "type": "integer" },
                            "height": { "type": "integer" }
                        }
                    },
                    "window_title": { "type": "string", "description": "Capture specific window by title (optional)" }
                }
            }),
        },
        BuiltinToolDefinition {
            tool_name: "find_ui_elements".to_string(),
            description: "Find accessible UI elements on screen using accessibility APIs. Returns element role, name, bounds, and interactivity.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "role": { "type": "string", "description": "Element role filter (button, text, link, input, etc.)" },
                    "name_contains": { "type": "string", "description": "Filter by element name" },
                    "application": { "type": "string", "description": "Filter by application name" },
                    "window_title": { "type": "string", "description": "Filter by window title" }
                }
            }),
        },
        BuiltinToolDefinition {
            tool_name: "mouse_click".to_string(),
            description: "Click at specified screen coordinates.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "x": { "type": "number", "description": "X coordinate" },
                    "y": { "type": "number", "description": "Y coordinate" },
                    "button": { "type": "string", "enum": ["left", "right", "middle"], "description": "Mouse button (default: left)" }
                },
                "required": ["x", "y"]
            }),
        },
        BuiltinToolDefinition {
            tool_name: "type_text".to_string(),
            description: "Type text at the current cursor position or at specified coordinates.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to type" },
                    "x": { "type": "number", "description": "Click at X before typing (optional)" },
                    "y": { "type": "number", "description": "Click at Y before typing (optional)" }
                },
                "required": ["text"]
            }),
        },
        BuiltinToolDefinition {
            tool_name: "press_key".to_string(),
            description: "Press a keyboard key with optional modifiers (Ctrl, Alt, Shift).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Key to press (e.g., 'Enter', 'Tab', 'a', 'F1')" },
                    "modifiers": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["alt", "control", "shift", "super"] },
                        "description": "Key modifiers"
                    }
                },
                "required": ["key"]
            }),
        },
        BuiltinToolDefinition {
            tool_name: "mouse_scroll".to_string(),
            description: "Scroll at specified screen coordinates.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "x": { "type": "number", "description": "X coordinate" },
                    "y": { "type": "number", "description": "Y coordinate" },
                    "delta": { "type": "integer", "description": "Scroll amount (positive=up, negative=down)" }
                },
                "required": ["x", "y", "delta"]
            }),
        },
    ],
},
```

#### 1.5.2 处理器注册（builtin_tools.rs）

**修改文件**: `src-tauri/crates/core/src/builtin_tools.rs`

在 `init_builtin_handlers()` 中新增 6 个 handler，调用 `commands::computer_control` 中的 Tauri 命令逻辑：

```rust
// screen_capture handler
register_builtin_handler(
    "@axagent/computer-control",
    "screen_capture",
    make_handler(|args: Value| {
        Box::pin(async move {
            let monitor = args.get("monitor").and_then(|v| v.as_u64()).map(|v| v as u32);
            let window_title = args.get("window_title").and_then(|v| v.as_str()).map(String::from);
            // 调用 screen_capture 模块逻辑
            let capture = ScreenCapture::new();
            let result = if let Some(title) = window_title {
                capture.capture_window(&title).await
            } else {
                capture.capture_full(monitor).await
            };
            result.map(|r| McpToolResult {
                content: vec![McpContent::Text { text: serde_json::to_string(&r).unwrap() }],
                is_error: false,
            }).map_err(|e| AxAgentError::Gateway(e.to_string()))
        })
    }),
);
// find_ui_elements, mouse_click, type_text, press_key, mouse_scroll 同理
```

#### 1.5.3 Phase 1 遗留：图像/图表工具化补齐

Phase 1 的 `generate_image` 和 `generate_chart` 仅实现了 Tauri 命令，LLM 无法通过工具调用链路使用。需在 Phase 2 中补齐：

```rust
// 在 builtin_tools_registry.rs 的 get_dynamic_builtin_tools() 中新增：
tools.insert(
    "generate_image".to_string(),
    BuiltinDynamicTool {
        server_id: "builtin-image-gen".to_string(),
        server_name: "@axagent/image-gen".to_string(),
        tool_name: "generate_image".to_string(),
        description: "Generate an image from a text prompt using AI. Supports Flux and DALL-E providers.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "Text description of the image to generate" },
                "provider": { "type": "string", "enum": ["flux", "dalle"], "description": "Image generation provider (default: flux)" },
                "size": { "type": "string", "description": "Image size, e.g. '1024x1024'" },
                "negative_prompt": { "type": "string", "description": "What to avoid in the image" }
            },
            "required": ["prompt"]
        }),
    },
);
// generate_chart 同理
```

### 1.6 前端：计算机控制面板

**新增文件**: `src/components/chat/ComputerControlPanel.tsx`

```typescript
import { useState, useRef, useEffect } from "react";
import { Button, Card, Image, Space, Switch, Typography, Tooltip, message } from "antd";
import {
  Monitor, MousePointer, Type, Keyboard, Search,
  PlayCircle, PauseCircle, Screenshot,
} from "lucide-react";
import { invoke } from "@/lib/invoke";

interface CaptureResult {
  image_base64: string;
  width: number;
  height: number;
}

interface UIElement {
  role: string;
  name: string;
  bounds: { x: number; y: number; width: number; height: number };
  is_clickable: boolean;
}

export function ComputerControlPanel() {
  const [screenshot, setScreenshot] = useState<string | null>(null);
  const [autoMode, setAutoMode] = useState(false);
  const [elements, setElements] = useState<UIElement[]>([]);
  const [loading, setLoading] = useState(false);
  const [clickCoords, setClickCoords] = useState<{ x: number; y: number } | null>(null);
  const imgRef = useRef<HTMLImageElement>(null);

  // 截取屏幕
  const handleCapture = async () => {
    setLoading(true);
    try {
      const result = await invoke<CaptureResult>("screen_capture", { monitor: 0 });
      setScreenshot(`data:image/png;base64,${result.image_base64}`);
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  // 查找 UI 元素
  const handleFindElements = async (nameContains?: string) => {
    try {
      const result = await invoke<UIElement[]>("find_ui_elements", {
        query: { name_contains: nameContains },
      });
      setElements(result);
    } catch (e) {
      message.error(String(e));
    }
  };

  // 点击坐标（从截图上选择）
  const handleImageClick = (e: React.MouseEvent<HTMLImageElement>) => {
    if (!imgRef.current) return;
    const rect = imgRef.current.getBoundingClientRect();
    const scaleX = 1920 / rect.width;  // 假设原始分辨率为 1920
    const scaleY = 1080 / rect.height;
    const x = Math.round((e.clientX - rect.left) * scaleX);
    const y = Math.round((e.clientY - rect.top) * scaleY);
    setClickCoords({ x, y });
  };

  // 执行点击
  const executeClick = async (x: number, y: number) => {
    try {
      await invoke("mouse_click", { x, y, button: "left" });
      message.success(`点击 (${x}, ${y})`);
      // 点击后自动刷新截图
      setTimeout(handleCapture, 500);
    } catch (e) {
      message.error(String(e));
    }
  };

  // 输入文本
  const handleTypeText = async (text: string, x?: number, y?: number) => {
    try {
      await invoke("type_text", { text, x, y });
      message.success("输入完成");
    } catch (e) {
      message.error(String(e));
    }
  };

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}>
      {/* 工具栏 */}
      <Space>
        <Button
          icon={<Screenshot size={14} />}
          onClick={handleCapture}
          loading={loading}
        >
          截屏
        </Button>
        <Button
          icon={<Search size={14} />}
          onClick={() => handleFindElements()}
        >
          查找元素
        </Button>
        <Tooltip title="自动模式下，AI 将自主控制计算机">
          <Switch
            checked={autoMode}
            onChange={setAutoMode}
            checkedChildren="自动"
            unCheckedChildren="手动"
          />
        </Tooltip>
      </Space>

      {/* 截图展示 */}
      {screenshot && (
        <Card size="small" bodyStyle={{ padding: 0 }}>
          <div style={{ position: "relative", cursor: "crosshair" }}>
            <img
              ref={imgRef}
              src={screenshot}
              onClick={handleImageClick}
              style={{ width: "100%", display: "block" }}
            />
            {/* 点击坐标标记 */}
            {clickCoords && (
              <div
                style={{
                  position: "absolute",
                  left: `${(clickCoords.x / 1920) * 100}%`,
                  top: `${(clickCoords.y / 1080) * 100}%`,
                  width: 8,
                  height: 8,
                  borderRadius: "50%",
                  background: "red",
                  transform: "translate(-50%, -50%)",
                  pointerEvents: "none",
                }}
              />
            )}
            {/* UI 元素高亮 */}
            {elements.map((el, i) => (
              <div
                key={i}
                style={{
                  position: "absolute",
                  left: `${(el.bounds.x / 1920) * 100}%`,
                  top: `${(el.bounds.y / 1080) * 100}%`,
                  width: `${(el.bounds.width / 1920) * 100}%`,
                  height: `${(el.bounds.height / 1080) * 100}%`,
                  border: "2px solid #1890ff",
                  borderRadius: 4,
                  cursor: el.is_clickable ? "pointer" : "default",
                  pointerEvents: el.is_clickable ? "auto" : "none",
                }}
                onClick={(e) => {
                  e.stopPropagation();
                  executeClick(
                    el.bounds.x + el.bounds.width / 2,
                    el.bounds.y + el.bounds.height / 2
                  );
                }}
                title={`${el.role}: ${el.name}`}
              />
            ))}
          </div>
        </Card>
      )}

      {/* 坐标信息 */}
      {clickCoords && (
        <Space>
          <Typography.Text>
            坐标: ({clickCoords.x}, {clickCoords.y})
          </Typography.Text>
          <Button size="small" onClick={() => executeClick(clickCoords.x, clickCoords.y)}>
            执行点击
          </Button>
        </Space>
      )}

      {/* 元素列表 */}
      {elements.length > 0 && (
        <Card size="small" title={`发现 ${elements.length} 个元素`}>
          <div style={{ maxHeight: 200, overflow: "auto" }}>
            {elements.slice(0, 20).map((el, i) => (
              <div
                key={i}
                style={{
                  padding: "4px 8px",
                  cursor: "pointer",
                  borderRadius: 4,
                }}
                onClick={() =>
                  executeClick(
                    el.bounds.x + el.bounds.width / 2,
                    el.bounds.y + el.bounds.height / 2
                  )
                }
              >
                <Typography.Text type="secondary" style={{ fontSize: 11 }}>
                  {el.role}
                </Typography.Text>{" "}
                <Typography.Text>{el.name || "(unnamed)"}</Typography.Text>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
```

### 1.7 操作审计与确认机制

**新增文件**: `src-tauri/crates/core/src/operation_audit.rs`

```rust
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// 高风险操作类型 — 需要用户确认
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,      // 截屏、查找元素
    Medium,   // 点击、输入文本
    High,     // 发送消息、删除文件、执行脚本
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub operation: String,
    pub parameters: serde_json::Value,
    pub risk_level: RiskLevel,
    pub confirmed: bool,
    pub result: Option<String>,
}

/// 操作审计器
pub struct OperationAuditor {
    entries: Mutex<Vec<AuditEntry>>,
    /// 需要确认的风险级别阈值
    confirm_threshold: RiskLevel,
}

impl OperationAuditor {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            confirm_threshold: RiskLevel::Medium,
        }
    }

    /// 记录操作
    pub fn record(&self, entry: AuditEntry) {
        let mut entries = self.entries.lock().unwrap();
        entries.push(entry);
        // 限制历史长度
        if entries.len() > 1000 {
            entries.drain(0..100);
        }
    }

    /// 判断操作是否需要确认
    pub fn needs_confirmation(&self, risk: &RiskLevel) -> bool {
        matches!(
            (risk, &self.confirm_threshold),
            (RiskLevel::High, _) |
            (RiskLevel::Medium, RiskLevel::Medium) |
            (RiskLevel::Medium, RiskLevel::Low)
        )
    }

    /// 获取最近 N 条操作记录
    pub fn recent(&self, n: usize) -> Vec<AuditEntry> {
        let entries = self.entries.lock().unwrap();
        entries.iter().rev().take(n).cloned().collect()
    }
}
```

---

## 模块 2: 浏览器自动化（Week 5-9）

### 2.1 架构设计

```
用户指令 → LLM → browser_* 工具调用
                    ↓
           Rust PlaywrightClient
                    ↓
           Playwright Node.js 进程 (CDP)
                    ↓
           ┌────────┼────────┐
           ↓        ↓        ↓
       navigate   click    fill_form
       screenshot extract  wait_for
```

**选型决策**：不直接在 Rust 中实现浏览器协议，而是通过 Node.js Playwright 进程中转。
原因：Playwright 的 Node.js API 最成熟，Rust 生态的 chromiumoxide/philipsis 功能不完整。

### 2.2 Playwright 桥接服务

**新增文件**: `src-tauri/scripts/browser-automation.mjs`

```javascript
// Playwright 桥接服务 — 由 Tauri 后端启动，通过 stdin/stdout JSON 通信
import { chromium } from "playwright";

let browser = null;
let page = null;

async function init() {
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1280, height: 720 },
    locale: "zh-CN",
  });
  page = await context.newPage();
}

// JSON-RPC 式通信
process.stdin.on("data", async (data) => {
  const msg = JSON.parse(data.toString().trim());
  let result;

  try {
    switch (msg.method) {
      case "navigate": {
        await page.goto(msg.params.url, { waitUntil: "domcontentloaded", timeout: 30000 });
        result = { url: page.url(), title: await page.title() };
        break;
      }
      case "screenshot": {
        const buffer = await page.screenshot({ type: "png", fullPage: msg.params.fullPage || false });
        result = { image_base64: buffer.toString("base64") };
        break;
      }
      case "click": {
        await page.click(msg.params.selector, { timeout: 10000 });
        result = { success: true };
        break;
      }
      case "fill": {
        await page.fill(msg.params.selector, msg.params.value);
        result = { success: true };
        break;
      }
      case "type": {
        await page.locator(msg.params.selector).pressSequentially(msg.params.text, { delay: 50 });
        result = { success: true };
        break;
      }
      case "select": {
        await page.selectOption(msg.params.selector, msg.params.value);
        result = { success: true };
        break;
      }
      case "extract_text": {
        const text = await page.locator(msg.params.selector).textContent();
        result = { text };
        break;
      }
      case "extract_all": {
        const elements = await page.$$eval(msg.params.selector, (els) =>
          els.map((el) => ({
            tag: el.tagName.toLowerCase(),
            text: el.textContent?.trim().slice(0, 200),
            href: el.getAttribute("href"),
            type: el.getAttribute("type"),
            placeholder: el.getAttribute("placeholder"),
          }))
        );
        result = { elements, count: elements.length };
        break;
      }
      case "wait_for": {
        await page.waitForSelector(msg.params.selector, { timeout: msg.params.timeout || 10000 });
        result = { success: true };
        break;
      }
      case "get_content": {
        const html = await page.content();
        result = { html: html.slice(0, 100000) }; // 限制大小
        break;
      }
      case "close": {
        await browser.close();
        result = { success: true };
        break;
      }
      default:
        throw new Error(`Unknown method: ${msg.method}`);
    }

    process.stdout.write(JSON.stringify({ id: msg.id, result }) + "\n");
  } catch (error) {
    process.stdout.write(JSON.stringify({ id: msg.id, error: error.message }) + "\n");
  }
});

// 启动
init().then(() => {
  process.stdout.write(JSON.stringify({ ready: true }) + "\n");
});
```

### 2.3 Rust Playwright 客户端

**新增文件**: `src-tauri/crates/core/src/browser_automation.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

#[derive(Debug, Serialize, Deserialize)]
struct BrowserRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct BrowserResponse {
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

/// Playwright 浏览器自动化客户端
pub struct PlaywrightClient {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout_reader: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

impl PlaywrightClient {
    /// 启动 Playwright 桥接进程
    pub async fn launch() -> Result<Self> {
        let script_path = std::env::current_exe()?
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Cannot find exe directory"))?
            .join("scripts")
            .join("browser-automation.mjs");

        let mut child = Command::new("node")
            .arg(&script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow::anyhow!("No stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow::anyhow!("No stdout"))?;
        let stdout_reader = BufReader::new(stdout);

        let mut client = Self {
            child,
            stdin,
            stdout_reader,
            next_id: 1,
        };

        // 等待 ready 信号
        let mut ready_line = String::new();
        client.stdout_reader.read_line(&mut ready_line).await?;
        let ready_msg: serde_json::Value = serde_json::from_str(&ready_line)?;
        if !ready_msg["ready"].as_bool().unwrap_or(false) {
            anyhow::bail!("Playwright bridge failed to start");
        }

        Ok(client)
    }

    /// 发送命令并等待响应
    async fn call(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = BrowserRequest {
            id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)? + "\n";
        self.stdin.write_all(request_json.as_bytes()).await?;
        self.stdin.flush().await?;

        // 读取响应
        let mut response_line = String::new();
        self.stdout_reader.read_line(&mut response_line).await?;
        let response: BrowserResponse = serde_json::from_str(&response_line.trim())?;

        if let Some(error) = response.error {
            anyhow::bail!("Browser automation error: {}", error);
        }

        response.result.ok_or_else(|| anyhow::anyhow!("Empty response"))
    }

    /// 导航到 URL
    pub async fn navigate(&mut self, url: &str) -> Result<NavigateResult> {
        let result = self.call("navigate", serde_json::json!({ "url": url })).await?;
        serde_json::from_value(result).map_err(Into::into)
    }

    /// 截图
    pub async fn screenshot(&mut self, full_page: bool) -> Result<ScreenshotResult> {
        let result = self.call("screenshot", serde_json::json!({ "fullPage": full_page })).await?;
        serde_json::from_value(result).map_err(Into::into)
    }

    /// 点击元素
    pub async fn click(&mut self, selector: &str) -> Result<()> {
        self.call("click", serde_json::json!({ "selector": selector })).await?;
        Ok(())
    }

    /// 填写表单
    pub async fn fill(&mut self, selector: &str, value: &str) -> Result<()> {
        self.call("fill", serde_json::json!({ "selector": selector, "value": value })).await?;
        Ok(())
    }

    /// 输入文本（逐字符）
    pub async fn type_text(&mut self, selector: &str, text: &str) -> Result<()> {
        self.call("type", serde_json::json!({ "selector": selector, "text": text })).await?;
        Ok(())
    }

    /// 提取文本
    pub async fn extract_text(&mut self, selector: &str) -> Result<String> {
        let result = self.call("extract_text", serde_json::json!({ "selector": selector })).await?;
        result["text"].as_str().map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No text in response"))
    }

    /// 提取所有匹配元素
    pub async fn extract_all(&mut self, selector: &str) -> Result<Vec<ExtractedElement>> {
        let result = self.call("extract_all", serde_json::json!({ "selector": selector })).await?;
        let elements = result["elements"].as_array()
            .ok_or_else(|| anyhow::anyhow!("No elements in response"))?;
        elements.iter()
            .map(|v| serde_json::from_value(v.clone()).map_err(Into::into))
            .collect()
    }

    /// 获取页面内容
    pub async fn get_content(&mut self) -> Result<String> {
        let result = self.call("get_content", serde_json::json!({})).await?;
        result["html"].as_str().map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No html in response"))
    }

    /// 等待元素
    pub async fn wait_for(&mut self, selector: &str, timeout: Option<u32>) -> Result<()> {
        self.call("wait_for", serde_json::json!({
            "selector": selector,
            "timeout": timeout
        })).await?;
        Ok(())
    }

    /// 关闭浏览器
    pub async fn close(&mut self) -> Result<()> {
        self.call("close", serde_json::json!({})).await?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NavigateResult {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenshotResult {
    pub image_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedElement {
    pub tag: String,
    pub text: Option<String>,
    pub href: Option<String>,
    #[serde(rename = "type")]
    pub input_type: Option<String>,
    pub placeholder: Option<String>,
}

impl Drop for PlaywrightClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
```

### 2.4 浏览器自动化 Tauri 命令

**新增文件**: `src-tauri/src/commands/browser.rs`

```rust
use axagent_core::browser_automation::PlaywrightClient;
use std::sync::Mutex;
use tauri::State;

/// 全局 Playwright 客户端（懒初始化）
pub struct BrowserState {
    client: Mutex<Option<PlaywrightClient>>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self { client: Mutex::new(None) }
    }

    async fn get_or_create(&self) -> Result<PlaywrightClient, String> {
        // 由于 Mutex 不支持 async，这里简化处理
        // 实际生产中应使用 tokio::sync::Mutex
        let mut guard = self.client.lock().map_err(|e| e.to_string())?;
        if guard.is_none() {
            *guard = Some(PlaywrightClient::launch().await.map_err(|e| e.to_string())?);
        }
        // 这里有个问题：无法从 MutexGuard 中取出 PlaywrightClient
        // 实际实现需要重新设计 — 使用 Arc<tokio::sync::Mutex>
        Err("TODO: use async mutex".to_string())
    }
}

#[tauri::command]
pub async fn browser_navigate(url: String) -> Result<serde_json::Value, String> {
    // 实际实现中使用 AppState 中的 BrowserState
    Err("Not yet connected".to_string())
}

#[tauri::command]
pub async fn browser_screenshot(full_page: Option<bool>) -> Result<serde_json::Value, String> {
    Err("Not yet connected".to_string())
}

#[tauri::command]
pub async fn browser_click(selector: String) -> Result<(), String> {
    Err("Not yet connected".to_string())
}

#[tauri::command]
pub async fn browser_fill(selector: String, value: String) -> Result<(), String> {
    Err("Not yet connected".to_string())
}

#[tauri::command]
pub async fn browser_extract(selector: String) -> Result<serde_json::Value, String> {
    Err("Not yet connected".to_string())
}

#[tauri::command]
pub async fn browser_get_content() -> Result<String, String> {
    Err("Not yet connected".to_string())
}

#[tauri::command]
pub async fn browser_close() -> Result<(), String> {
    Err("Not yet connected".to_string())
}
```

### 2.5 浏览器工具注册

**修改文件**: `src-tauri/crates/core/src/builtin_tools_registry.rs`

在 `get_all_builtin_server_definitions()` 中新增 `builtin-browser` server：

```rust
BuiltinServerDefinition {
    server_id: "builtin-browser".to_string(),
    server_name: "@axagent/browser".to_string(),
    tools: vec![
        BuiltinToolDefinition {
            tool_name: "browser_navigate".to_string(),
            description: "Open a URL in the automated browser and wait for it to load.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL to navigate to" }
                },
                "required": ["url"]
            }),
        },
        BuiltinToolDefinition {
            tool_name: "browser_screenshot".to_string(),
            description: "Take a screenshot of the current browser page.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "full_page": { "type": "boolean", "description": "Capture full scrollable page (default: false)" }
                }
            }),
        },
        BuiltinToolDefinition {
            tool_name: "browser_click".to_string(),
            description: "Click an element on the page using a CSS selector.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector of the element to click" }
                },
                "required": ["selector"]
            }),
        },
        BuiltinToolDefinition {
            tool_name: "browser_fill".to_string(),
            description: "Fill a form field with a value (clears existing content first).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector of the input field" },
                    "value": { "type": "string", "description": "Value to fill in" }
                },
                "required": ["selector", "value"]
            }),
        },
        BuiltinToolDefinition {
            tool_name: "browser_type".to_string(),
            description: "Type text character by character into a field (simulates human typing).".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector" },
                    "text": { "type": "string", "description": "Text to type" }
                },
                "required": ["selector", "text"]
            }),
        },
        BuiltinToolDefinition {
            tool_name: "browser_extract".to_string(),
            description: "Extract text content or element information from the page using a CSS selector.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "selector": { "type": "string", "description": "CSS selector (use 'body' for full page text)" }
                },
                "required": ["selector"]
            }),
        },
        BuiltinToolDefinition {
            tool_name: "browser_get_content".to_string(),
            description: "Get the full HTML content of the current page.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
    ],
},
```

同时在 `builtin_tools.rs` 的 `init_builtin_handlers()` 中注册对应处理器。

### 2.6 前端：浏览器自动化面板

**新增文件**: `src/components/chat/BrowserAutomationPanel.tsx`

```typescript
import { useState } from "react";
import { Button, Card, Image, Input, Space, Typography, message } from "antd";
import { Globe, Camera, MousePointerClick, FileText, X } from "lucide-react";
import { invoke } from "@/lib/invoke";

export function BrowserAutomationPanel() {
  const [url, setUrl] = useState("https://");
  const [screenshot, setScreenshot] = useState<string | null>(null);
  const [pageContent, setPageContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleNavigate = async () => {
    setLoading(true);
    try {
      await invoke("browser_navigate", { url });
      message.success("页面加载完成");
      // 自动截图
      const result = await invoke<{ image_base64: string }>("browser_screenshot", {});
      setScreenshot(`data:image/png;base64,${result.image_base64}`);
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleScreenshot = async () => {
    try {
      const result = await invoke<{ image_base64: string }>("browser_screenshot", {});
      setScreenshot(`data:image/png;base64,${result.image_base64}`);
    } catch (e) {
      message.error(String(e));
    }
  };

  const handleExtract = async (selector: string) => {
    try {
      const result = await invoke<{ text?: string; elements?: unknown[] }>("browser_extract", { selector });
      if (result.text) {
        setPageContent(result.text);
      }
    } catch (e) {
      message.error(String(e));
    }
  };

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}>
      {/* URL 输入 */}
      <Space>
        <Globe size={16} />
        <Input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="输入 URL..."
          style={{ width: 400 }}
          onPressEnter={handleNavigate}
        />
        <Button type="primary" onClick={handleNavigate} loading={loading}>
          打开
        </Button>
      </Space>

      {/* 操作按钮 */}
      <Space>
        <Button icon={<Camera size={14} />} onClick={handleScreenshot}>截图</Button>
        <Button icon={<FileText size={14} />} onClick={() => handleExtract("body")}>提取文本</Button>
        <Button icon={<X size={14} />} onClick={() => invoke("browser_close")}>关闭浏览器</Button>
      </Space>

      {/* 截图展示 */}
      {screenshot && (
        <Card size="small" title="浏览器截图">
          <Image src={screenshot} style={{ width: "100%" }} />
        </Card>
      )}

      {/* 页面内容 */}
      {pageContent && (
        <Card size="small" title="页面内容" style={{ maxHeight: 300, overflow: "auto" }}>
          <Typography.Paragraph ellipsis={{ rows: 10, expandable: true }}>
            {pageContent}
          </Typography.Paragraph>
        </Card>
      )}
    </div>
  );
}
```

---

## 模块 3: 文件系统增强与临时授权（Week 9-12）

### 3.1 临时授权机制

**新增文件**: `src-tauri/crates/core/src/file_authorizer.rs`

```rust
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 文件操作授权
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAuthorization {
    pub path: String,
    pub permissions: Vec<FilePermission>,
    pub granted_at: String,
    pub expires_at: Option<String>,
    pub granted_by: String,  // "user" | "auto"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilePermission {
    Read,
    Write,
    Delete,
    Execute,
}

/// 授权管理器
pub struct FileAuthorizer {
    /// 已授权路径 → 授权信息
    authorizations: Mutex<HashMap<String, FileAuthorization>>,
    /// 用户工作目录
    workspace_dir: PathBuf,
    /// 授权过期时间（默认 1 小时）
    auth_ttl: Duration,
}

impl FileAuthorizer {
    pub fn new(workspace_dir: impl Into<PathBuf>) -> Self {
        Self {
            authorizations: Mutex::new(HashMap::new()),
            workspace_dir: workspace_dir.into(),
            auth_ttl: Duration::from_secs(3600),
        }
    }

    /// 检查路径是否有指定权限
    pub fn check_permission(&self, path: &str, perm: &FilePermission) -> Result<bool> {
        let path = PathBuf::from(path);

        // workspace 目录始终允许
        if path.starts_with(&self.workspace_dir) {
            return Ok(true);
        }

        // 检查临时授权
        let auths = self.authorizations.lock().unwrap();
        for (_, auth) in auths.iter() {
            let auth_path = PathBuf::from(&auth.path);
            if path.starts_with(&auth_path) {
                // 检查过期
                if let Some(ref expires) = auth.expires_at {
                    let expires_time = chrono::DateTime::parse_from_rfc3339(expires)?;
                    if chrono::Utc::now() > expires_time {
                        continue;  // 已过期
                    }
                }
                if auth.permissions.contains(perm) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// 授予权限
    pub fn grant(&self, auth: FileAuthorization) {
        let mut auths = self.authorizations.lock().unwrap();
        auths.insert(auth.path.clone(), auth);
    }

    /// 撤销权限
    pub fn revoke(&self, path: &str) {
        let mut auths = self.authorizations.lock().unwrap();
        auths.remove(path);
    }

    /// 列出所有授权
    pub fn list(&self) -> Vec<FileAuthorization> {
        let auths = self.authorizations.lock().unwrap();
        auths.values().cloned().collect()
    }

    /// 清理过期授权
    pub fn cleanup_expired(&self) {
        let mut auths = self.authorizations.lock().unwrap();
        let now = chrono::Utc::now();
        auths.retain(|_, auth| {
            if let Some(ref expires) = auth.expires_at {
                if let Ok(expires_time) = chrono::DateTime::parse_from_rfc3339(expires) {
                    return now < expires_time;
                }
            }
            true
        });
    }
}
```

### 3.2 前端：权限确认对话框

**新增文件**: `src/components/shared/FilePermissionDialog.tsx`

```typescript
import { Modal, Typography, Space, Tag } from "antd";
import { ShieldAlert, FolderOpen, FileEdit, Trash2 } from "lucide-react";

interface FilePermissionRequest {
  path: string;
  permissions: ("read" | "write" | "delete" | "execute")[];
  reason: string;
}

interface FilePermissionDialogProps {
  request: FilePermissionRequest | null;
  onApprove: (permanent: boolean) => void;
  onDeny: () => void;
}

const PERM_ICONS: Record<string, React.ReactNode> = {
  read: <FolderOpen size={14} />,
  write: <FileEdit size={14} />,
  delete: <Trash2 size={14} />,
  execute: <ShieldAlert size={14} />,
};

const PERM_COLORS: Record<string, string> = {
  read: "blue",
  write: "orange",
  delete: "red",
  execute: "purple",
};

export function FilePermissionDialog({
  request,
  onApprove,
  onDeny,
}: FilePermissionDialogProps) {
  if (!request) return null;

  return (
    <Modal
      open={!!request}
      title={
        <Space>
          <ShieldAlert size={18} style={{ color: "#faad14" }} />
          <span>文件访问授权请求</span>
        </Space>
      }
      onCancel={onDeny}
      footer={
        <Space>
          <button onClick={onDeny} className="ant-btn">拒绝</button>
          <button
            onClick={() => onApprove(false)}
            className="ant-btn ant-btn-default"
          >
            允许 1 小时
          </button>
          <button
            onClick={() => onApprove(true)}
            className="ant-btn ant-btn-primary"
          >
            永久允许
          </button>
        </Space>
      }
      width={480}
    >
      <div style={{ marginBottom: 16 }}>
        <Typography.Text>AI 代理请求访问以下路径：</Typography.Text>
        <Typography.Paragraph
          code
          style={{ marginTop: 8, padding: 8, background: "#f5f5f5", borderRadius: 4 }}
        >
          {request.path}
        </Typography.Paragraph>
      </div>

      <div style={{ marginBottom: 16 }}>
        <Typography.Text type="secondary">请求权限：</Typography.Text>
        <div style={{ marginTop: 4 }}>
          {request.permissions.map((perm) => (
            <Tag
              key={perm}
              color={PERM_COLORS[perm]}
              icon={PERM_ICONS[perm]}
              style={{ marginRight: 4 }}
            >
              {perm}
            </Tag>
          ))}
        </div>
      </div>

      {request.reason && (
        <div>
          <Typography.Text type="secondary">原因：</Typography.Text>
          <Typography.Paragraph style={{ marginTop: 4 }}>
            {request.reason}
          </Typography.Paragraph>
        </div>
      )}
    </Modal>
  );
}
```

---

## 完整文件变更清单

### 新增文件（11 个）

| 文件 | 类型 | 描述 |
|------|------|------|
| `src-tauri/crates/core/src/screen_capture.rs` | Rust | 屏幕截图（xcap + image） |
| `src-tauri/crates/core/src/ui_automation.rs` | Rust | UI 元素定位 + 鼠标键盘控制 |
| `src-tauri/crates/core/src/operation_audit.rs` | Rust | 操作审计与风险分级 |
| `src-tauri/crates/core/src/browser_automation.rs` | Rust | Playwright 桥接客户端 |
| `src-tauri/crates/core/src/file_authorizer.rs` | Rust | 文件临时授权管理 |
| `src-tauri/src/commands/computer_control.rs` | Rust | 计算机控制 Tauri 命令 |
| `src-tauri/src/commands/browser.rs` | Rust | 浏览器自动化 Tauri 命令 |
| `src-tauri/scripts/browser-automation.mjs` | JS | Playwright 桥接脚本 |
| `src/components/chat/ComputerControlPanel.tsx` | TSX | 计算机控制面板 |
| `src/components/chat/BrowserAutomationPanel.tsx` | TSX | 浏览器自动化面板 |
| `src/components/shared/FilePermissionDialog.tsx` | TSX | 文件权限确认对话框 |

### 修改文件（7 个）

| 文件 | 变更描述 |
|------|---------|
| `src-tauri/crates/core/src/builtin_tools_registry.rs` | 新增 `builtin-computer-control` server (6 工具) + `builtin-browser` server (7 工具) + generate_image/generate_chart 动态工具定义 |
| `src-tauri/crates/core/src/builtin_tools.rs` | 新增 13+2 个 handler 注册（6 计算机控制 + 7 浏览器 + generate_image + generate_chart） |
| `src-tauri/crates/core/src/lib.rs` | 新增 `pub mod screen_capture;` `pub mod ui_automation;` `pub mod operation_audit;` `pub mod browser_automation;` `pub mod file_authorizer;` |
| `src-tauri/src/commands/mod.rs` | 新增 `pub mod computer_control;` `pub mod browser;` |
| `src-tauri/src/lib.rs` | 注册 computer_control + browser 命令到 `generate_handler![]` |
| `src-tauri/crates/core/Cargo.toml` | 添加 xcap/image/chrono 依赖（Windows 平台条件编译） |
| `package.json` | 添加 playwright 依赖 |

---

## 验收标准

### 模块 1: 屏幕感知与计算机控制

| 验收项 | 标准 |
|--------|------|
| 全屏截图 | 返回 base64 PNG，分辨率正确 |
| 区域截图 | 指定坐标范围正确裁剪 |
| 窗口截图 | 按标题查找窗口并截图 |
| UI 元素查找 | 返回可交互元素列表（角色/名称/坐标） |
| 鼠标点击 | 左/右/中键点击指定坐标 |
| 键盘输入 | 文本输入和快捷键 |
| 滚动 | 上下滚动指定量 |
| 操作审计 | 所有操作记录可查 |
| 风险确认 | Medium+ 操作需用户确认 |

### 模块 2: 浏览器自动化

| 验收项 | 标准 |
|--------|------|
| 打开页面 | URL 导航，返回标题和 URL |
| 截图 | 全页/可视区域截图 |
| 点击元素 | CSS 选择器定位并点击 |
| 填写表单 | 输入框填充文本 |
| 逐字符输入 | 模拟人类打字 |
| 提取文本 | 选择器提取文本内容 |
| 获取页面 | 返回 HTML 内容 |
| 进程管理 | 浏览器进程启动/关闭正常 |
| 超时处理 | 30s 超时不会挂起 |

### 模块 3: 文件系统增强

| 验收项 | 标准 |
|--------|------|
| 临时授权 | 授权 1 小时后自动过期 |
| 权限分级 | read/write/delete/execute 独立控制 |
| 权限对话框 | 风险操作弹出确认对话框 |
| 授权撤销 | 可手动撤销已授权路径 |
| workspace 免授权 | 工作目录内操作无需确认 |

### Phase 1 遗留补齐

| 验收项 | 标准 |
|--------|------|
| generate_image 工具化 | LLM 可通过工具调用链路自主调用图像生成 |
| generate_chart 工具化 | LLM 可通过工具调用链路自主调用图表生成 |
| 工具定义完整性 | builtin_tools_registry.rs 中有完整 schema |
| 处理器注册 | builtin_tools.rs 中有对应 handler |

---

## 周计划

| 周 | 任务 | 交付物 |
|----|------|--------|
| W1 | screen_capture.rs（全屏+区域+窗口）+ core/lib.rs 模块注册 | 截图功能可用 |
| W2 | ui_automation.rs（UI 元素查找 via PowerShell + Accessibility API） | 元素定位可用 |
| W3 | ui_automation.rs（鼠标+键盘模拟 via PowerShell + Win32 API） | 交互操作可用 |
| W4 | computer_control Tauri 命令 + builtin_tools_registry.rs 工具定义 + builtin_tools.rs handler | 后端工具链路完整 |
| W5 | ComputerControlPanel + operation_audit | 计算机控制面板完整 |
| W6 | browser-automation.mjs + PlaywrightClient (Rust) + npm playwright 依赖 | 浏览器桥接可用 |
| W7 | browser Tauri 命令 + builtin_tools_registry.rs 浏览器工具定义 + handler | 浏览器后端完整 |
| W8 | BrowserAutomationPanel | 浏览器面板完整 |
| W9 | file_authorizer + FilePermissionDialog + 文件操作工具授权集成 | 授权机制完整 |
| W10 | **Phase 1 遗留补齐**: generate_image/generate_chart 工具化 + ArtifactPanel 升级 | 工具调用链路完整 |
| W11 | 集成测试 + 安全审计 + 平台兼容性（macOS/Linux 适配） | 测试通过 |
| W12 | Bug 修复 + 文档更新 + 性能优化 | Phase 2 完整交付 |

---

## 安全注意事项

1. **操作审计**：所有计算机控制操作必须记录
2. **风险分级**：Medium+ 操作需要用户确认
3. **沙箱限制**：浏览器自动化使用 headless 模式
4. **权限最小化**：文件访问默认仅限 workspace（现有 `ALLOWED_FILE_DIRECTORIES`）
5. **超时保护**：所有操作 30s 超时
6. **进程隔离**：Playwright 进程独立，崩溃不影响主应用
7. **敏感操作拦截**：禁止自动化操作银行/支付类网站
8. **工具注册隔离**：新工具按 server 分组（`@axagent/computer-control`、`@axagent/browser`），避免与现有 `@axagent/filesystem` 等冲突
9. **API Key 安全**：generate_image 的 API key 不应通过工具参数传递，应从安全存储（`image_gen_config.json`）读取

## 架构适配说明

Phase 2 开发需严格遵循现有项目架构模式：

### Rust 代码组织

```
src-tauri/
├── crates/core/src/          ← 核心库（业务逻辑）
│   ├── builtin_tools.rs      ← 工具处理器（handler 实现）
│   ├── builtin_tools_registry.rs ← 工具定义（schema 声明）
│   ├── screen_capture.rs     ← [新增] 截图引擎
│   ├── ui_automation.rs      ← [新增] UI 自动化引擎
│   ├── browser_automation.rs ← [新增] Playwright 客户端
│   ├── operation_audit.rs    ← [新增] 操作审计
│   ├── file_authorizer.rs    ← [新增] 文件授权
│   └── lib.rs                ← [修改] 注册新模块
├── src/commands/             ← Tauri 命令层（薄封装）
│   ├── mod.rs                ← [修改] 注册新命令模块
│   ├── computer_control.rs   ← [新增] 计算机控制命令
│   └── browser.rs            ← [新增] 浏览器自动化命令
└── src/lib.rs                ← [修改] 注册 Tauri 命令到 generate_handler![]
```

### 工具注册流程

1. 在 `builtin_tools_registry.rs` 的 `get_all_builtin_server_definitions()` 中添加 `BuiltinServerDefinition`
2. 在 `builtin_tools.rs` 的 `init_builtin_handlers()` 中通过 `register_builtin_handler()` 注册处理器
3. 处理器内部调用 `core::screen_capture` / `core::ui_automation` 等模块

### 前端代码组织

```
src/components/chat/
├── ComputerControlPanel.tsx   ← [新增] 计算机控制面板
├── BrowserAutomationPanel.tsx ← [新增] 浏览器自动化面板
├── ArtifactPreview/           ← [已存在] Phase 1 预览组件
└── ArtifactPanel.tsx          ← [已存在] Phase 1 未升级，Phase 2 W10 补齐
src/components/shared/
└── FilePermissionDialog.tsx   ← [新增] 文件权限确认
```

### 关键依赖

| 依赖 | 用途 | 当前状态 |
|------|------|---------|
| `xcap` | 屏幕截图（Windows/macOS/Linux） | 需新增，`cfg(windows)` 条件编译 |
| `image` | 图片处理（PNG 编解码） | 需新增到 core crate |
| `base64` | Base64 编解码 | 已存在于项目 |
| `chrono` | 时间戳 | 需新增到 core crate |
| `playwright` (npm) | 浏览器自动化 | 需新增，devDependencies 中有 `@playwright/test` 但非自动化用途 |
