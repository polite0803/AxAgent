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
    /// 显示器缩放因子 (1.0 = 100%, 2.0 = 200%)
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub struct ScreenCapture;

#[cfg(target_os = "windows")]
fn is_black_frame(image: &image::RgbaImage) -> bool {
    let pixels = image.as_raw();
    let sample_step = (pixels.len() / 4000).max(4);
    let total_samples = (pixels.len() / sample_step).min(1000);

    let mut black_count = 0usize;
    let mut sampled = 0usize;
    for chunk in pixels.chunks(sample_step) {
        if sampled >= total_samples {
            break;
        }
        if chunk.len() >= 4 && chunk[0] < 10 && chunk[1] < 10 && chunk[2] < 10 {
            black_count += 1;
        }
        sampled += 1;
    }

    sampled > 0 && (black_count as f64 / sampled as f64) > 0.95
}

#[cfg(target_os = "windows")]
fn gdi_capture_monitor(monitor_index: u32) -> Result<(image::RgbaImage, f64)> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;

    let monitors = xcap::Monitor::all()?;
    let monitor = monitors
        .get(monitor_index as usize)
        .ok_or_else(|| anyhow::anyhow!("Monitor {} not found", monitor_index))?;

    let x = monitor.x()?;
    let y = monitor.y()?;
    let width = monitor.width()? as i32;
    let height = monitor.height()? as i32;
    let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;

    unsafe {
        let hwnd = GetDesktopWindow();
        let hdc = GetDC(Some(hwnd));
        let mem_dc = CreateCompatibleDC(Some(hdc));
        let bitmap = CreateCompatibleBitmap(hdc, width, height);
        SelectObject(mem_dc, bitmap.into());

        let result = BitBlt(mem_dc, 0, 0, width, height, Some(hdc), x, y, SRCCOPY);

        if result.is_err() {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(Some(hwnd), hdc);
            let _ = DeleteObject(bitmap.into());
            anyhow::bail!("GDI BitBlt 失败: {result:?}");
        }

        let buffer_size = (width * height * 4) as usize;
        let mut buffer = vec![0u8; buffer_size];
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0,
                biSizeImage: buffer_size as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr().cast()),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(Some(hwnd), hdc);
        let _ = DeleteObject(bitmap.into());

        // BGRA → RGBA
        for pixel in buffer.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }

        let image = image::RgbaImage::from_raw(width as u32, height as u32, buffer)
            .ok_or_else(|| anyhow::anyhow!("无法从 GDI 缓冲区创建图像"))?;

        Ok((image, scale_factor))
    }
}

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
            let scale_factor = full.scale_factor;
            let mut full_image = self.base64_to_image(&full.image_base64)?;
            let cropped =
                crop_image(&mut full_image, region.x, region.y, region.width, region.height)?;
            let base64 = self.image_to_base64(&cropped)?;
            Ok(ScreenCaptureResult {
                image_base64: base64,
                width: region.width,
                height: region.height,
                monitor_index: 0,
                captured_at: chrono::Utc::now().to_rfc3339(),
                scale_factor,
            })
        }
    }

    #[allow(unused_variables)]
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

        let scale_factor = monitor.scale_factor().unwrap_or(1.0) as f64;

        // 尝试 WGC (Windows.Graphics.Capture)，失败或黑帧时回退到 GDI
        let image = match monitor.capture_image() {
            Ok(img) if !is_black_frame(&img) => img,
            Ok(_) => {
                tracing::warn!("WGC 截图疑似黑帧 (DRM/GPU 覆盖层)，回退到 GDI BitBlt");
                gdi_capture_monitor(monitor_index)?.0
            },
            Err(e) => {
                tracing::warn!("WGC 截图失败: {e}，回退到 GDI BitBlt");
                gdi_capture_monitor(monitor_index)?.0
            },
        };

        let width = image.width();
        let height = image.height();
        let base64 = self.image_to_base64(&image)?;

        Ok(ScreenCaptureResult {
            image_base64: base64,
            width,
            height,
            monitor_index,
            captured_at: chrono::Utc::now().to_rfc3339(),
            scale_factor,
        })
    }

    #[cfg(target_os = "windows")]
    async fn capture_windows_region(&self, region: CaptureRegion) -> Result<ScreenCaptureResult> {
        let full = self.capture_windows_full(0).await?;
        let scale_factor = full.scale_factor;
        let mut full_image = self.base64_to_image(&full.image_base64)?;
        let cropped = crop_image(&mut full_image, region.x, region.y, region.width, region.height)?;
        let base64 = self.image_to_base64(&cropped)?;

        Ok(ScreenCaptureResult {
            image_base64: base64,
            width: region.width,
            height: region.height,
            monitor_index: 0,
            captured_at: chrono::Utc::now().to_rfc3339(),
            scale_factor,
        })
    }

    #[cfg(target_os = "windows")]
    async fn capture_windows_by_title(&self, window_title: &str) -> Result<ScreenCaptureResult> {
        use xcap::Window;

        let windows = Window::all()?;
        let window = windows
            .iter()
            .find(|w| w.title().map(|t| t.contains(window_title)).unwrap_or(false))
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
            scale_factor: 1.0,
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
            scale_factor: 1.0,
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
            scale_factor: 1.0,
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
