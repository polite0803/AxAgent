use anyhow::Result;
use image::ImageEncoder;
use serde::{Deserialize, Serialize};

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

pub struct ScreenCapture;

impl ScreenCapture {
    pub fn new() -> Self {
        let temp_dir = std::env::temp_dir().join("axagent_captures");
        let _ = std::fs::create_dir_all(&temp_dir);
        Self
    }

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
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            anyhow::bail!("Unsupported platform for screen capture")
        }
    }

    pub async fn capture_region(&self, region: CaptureRegion) -> Result<ScreenCaptureResult> {
        #[cfg(target_os = "windows")]
        {
            self.capture_windows_region(region).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            let full = self.capture_full(None).await?;
            let mut full_image = self.base64_to_image(&full.image_base64)?;
            let cropped = crop_image(
                &mut full_image,
                region.x,
                region.y,
                region.width,
                region.height,
            )?;
            let base64 = self.image_to_base64(&cropped)?;
            Ok(ScreenCaptureResult {
                image_base64: base64,
                width: region.width,
                height: region.height,
                monitor_index: 0,
                captured_at: chrono::Utc::now().to_rfc3339(),
            })
        }
    }

    pub async fn capture_window(&self, window_title: &str) -> Result<ScreenCaptureResult> {
        #[cfg(target_os = "windows")]
        {
            self.capture_windows_by_title(window_title).await
        }
        #[cfg(not(target_os = "windows"))]
        {
            anyhow::bail!("Window capture not yet supported on this platform")
        }
    }

    #[cfg(target_os = "windows")]
    async fn capture_windows_full(&self, monitor_index: u32) -> Result<ScreenCaptureResult> {
        use xcap::Monitor;

        let monitors = Monitor::all()?;
        let monitor = monitors
            .get(monitor_index as usize)
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

    #[cfg(target_os = "windows")]
    async fn capture_windows_region(&self, region: CaptureRegion) -> Result<ScreenCaptureResult> {
        let full = self.capture_windows_full(0).await?;
        let mut full_image = self.base64_to_image(&full.image_base64)?;
        let cropped = crop_image(
            &mut full_image,
            region.x,
            region.y,
            region.width,
            region.height,
        )?;
        let base64 = self.image_to_base64(&cropped)?;

        Ok(ScreenCaptureResult {
            image_base64: base64,
            width: region.width,
            height: region.height,
            monitor_index: 0,
            captured_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    #[cfg(target_os = "windows")]
    async fn capture_windows_by_title(&self, window_title: &str) -> Result<ScreenCaptureResult> {
        use xcap::Window;

        let windows = Window::all()?;
        let window = windows
            .iter()
            .find(|w| w.title().contains(window_title))
            .ok_or_else(|| anyhow::anyhow!("Window '{}' not found", window_title))?;

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

    #[cfg(target_os = "macos")]
    async fn capture_macos_full(&self, _monitor_index: u32) -> Result<ScreenCaptureResult> {
        let output = tokio::process::Command::new("screencapture")
            .args(["-x", "/tmp/axagent_capture.png"])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("screencapture failed");
        }

        let img = image::open("/tmp/axagent_capture.png")?;
        let width = img.width();
        let height = img.height();
        let rgba = img.to_rgba8();
        let base64 = self.image_to_base64(&rgba)?;

        Ok(ScreenCaptureResult {
            image_base64: base64,
            width,
            height,
            monitor_index: 0,
            captured_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    #[cfg(target_os = "linux")]
    async fn capture_linux_full(&self, _monitor_index: u32) -> Result<ScreenCaptureResult> {
        let output = tokio::process::Command::new("import")
            .args(["-window", "root", "/tmp/axagent_capture.png"])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!("import (ImageMagick) failed");
        }

        let img = image::open("/tmp/axagent_capture.png")?;
        let width = img.width();
        let height = img.height();
        let rgba = img.to_rgba8();
        let base64 = self.image_to_base64(&rgba)?;

        Ok(ScreenCaptureResult {
            image_base64: base64,
            width,
            height,
            monitor_index: 0,
            captured_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn image_to_base64(&self, image: &image::RgbaImage) -> Result<String> {
        use base64::Engine;
        let mut png_data = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_data);
        encoder.write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&png_data))
    }

    fn base64_to_image(&self, base64_str: &str) -> Result<image::RgbaImage> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(base64_str)?;
        let img = image::load_from_memory(&bytes)?;
        Ok(img.to_rgba8())
    }
}

fn crop_image(
    img: &mut image::RgbaImage,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) -> Result<image::RgbaImage> {
    let (img_w, img_h) = (img.width() as i32, img.height() as i32);
    let x0 = x.max(0) as u32;
    let y0 = y.max(0) as u32;
    let x1 = (x + w as i32).min(img_w) as u32;
    let y1 = (y + h as i32).min(img_h) as u32;
    Ok(image::imageops::crop(img, x0, y0, x1 - x0, y1 - y0).to_image())
}

impl Default for ScreenCapture {
    fn default() -> Self {
        Self::new()
    }
}
