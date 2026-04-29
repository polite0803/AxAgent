use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

const QUICKBAR_LABEL: &str = "quickbar";
const QUICKBAR_WIDTH: f64 = 650.0;
const QUICKBAR_HEIGHT: f64 = 58.0;

fn quickbar_url(app: &AppHandle) -> WebviewUrl {
    match app.config().build.dev_url.as_ref() {
        Some(dev_url) => {
            let base = dev_url.trim_end_matches('/');
            WebviewUrl::External(
                format!("{}/index.html?__route=quickbar", base)
                    .parse()
                    .expect("valid quickbar dev URL"),
            )
        }
        None => WebviewUrl::App("index.html".into()),
    }
}

#[tauri::command]
pub async fn show_quickbar(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(QUICKBAR_LABEL) {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        let _ = window.center();
        return Ok(());
    }

    let url = quickbar_url(&app);

    let window = WebviewWindowBuilder::new(&app, QUICKBAR_LABEL, url)
        .title("AxAgent QuickBar")
        .inner_size(QUICKBAR_WIDTH, QUICKBAR_HEIGHT)
        .min_inner_size(400.0, 52.0)
        .decorations(false)
        .always_on_top(true)
        .resizable(true)
        .visible(true)
        .center()
        .build()
        .map_err(|e| format!("Failed to create quickbar window: {}", e))?;

    window.set_focus().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn hide_quickbar(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(QUICKBAR_LABEL) {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}
