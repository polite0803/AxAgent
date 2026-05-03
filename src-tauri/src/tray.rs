use std::sync::LazyLock;
use std::sync::RwLock;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

const TRAY_ID: &str = "axagent-tray";

/// 托盘标签由前端 i18n 系统通过 `set_tray_labels` 命令传入
static TRAY_LABELS: LazyLock<RwLock<(String, String)>> =
    LazyLock::new(|| RwLock::new(("显示主窗口".to_string(), "退出 AxAgent".to_string())));

/// 前端调用：设置托盘菜单标签文本
#[tauri::command]
pub fn set_tray_labels(app: AppHandle, show_label: String, quit_label: String) {
    *TRAY_LABELS.write().unwrap() = (show_label.clone(), quit_label.clone());
    // 同步更新已存在的托盘菜单
    let _ = sync_tray_menu(&app);
}

fn build_menu(app: &AppHandle) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let (show_label, quit_label) = TRAY_LABELS.read().unwrap().clone();
    let show = MenuItem::with_id(app, "show", &show_label, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", &quit_label, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    Ok(menu)
}

fn sync_tray_menu(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app)?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))?;
        Ok(())
    } else {
        create_tray_inner(app)
    }
}

pub fn create_tray(app: &AppHandle, _language: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 忽略传入的 language 参数，实际标签由前端 set_tray_labels 设置
    create_tray_inner(app)
}

fn create_tray_inner(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app)?;
    let icon = Image::from_path("icons/icon.png").unwrap_or_else(|_| {
        Image::from_bytes(include_bytes!("../icons/32x32.png"))
            .expect("failed to load fallback tray icon")
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("AxAgent")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            },
            "quit" => {
                app.exit(0);
            },
            _ => {},
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

/// 前端语言变更时调用（保持兼容）
pub fn sync_tray_language(
    app: &AppHandle,
    _language: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 不依赖 language 参数，实际标签经 set_tray_labels 已更新
    sync_tray_menu(app)
}
