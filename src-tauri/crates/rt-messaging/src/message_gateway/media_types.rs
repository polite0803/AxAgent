// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    Native,
    Voice,
    Document,
}

impl DeliveryMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryMode::Native => "native",
            DeliveryMode::Voice => "voice",
            DeliveryMode::Document => "document",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Audio,
    Video,
    Document,
}

impl MediaType {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaType::Image => "image",
            MediaType::Audio => "audio",
            MediaType::Video => "video",
            MediaType::Document => "document",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAttachment {
    pub path: String,
    pub media_type: MediaType,
    pub delivery_mode: DeliveryMode,
}

fn detect_media_type(ext: &str) -> Option<MediaType> {
    match ext.to_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "bmp" | "ico" | "tiff" | "tif" => {
            Some(MediaType::Image)
        },
        "mp3" | "wav" | "ogg" | "flac" | "aac" | "m4a" | "wma" => Some(MediaType::Audio),
        "mp4" | "webm" | "avi" | "mkv" | "mov" | "wmv" | "flv" => Some(MediaType::Video),
        "pdf" | "docx" | "xlsx" | "pptx" | "doc" | "xls" | "ppt" | "odt" | "ods" | "odp" => {
            Some(MediaType::Document)
        },
        _ => None,
    }
}

fn extract_absolute_paths(text: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"(?:(?:[A-Za-z]:[/\\])|/)[^\s"'<>\]\)}，。；：！？、]+"#)
        .expect("static regex");
    let mut seen = std::collections::HashSet::new();
    let mut paths = Vec::new();
    for cap in re.captures_iter(text) {
        let p = cap[0].to_string();
        let cleaned = p.trim_end_matches(['.', ',', ';', ':']);
        if seen.insert(cleaned.to_string()) {
            paths.push(cleaned.to_string());
        }
    }
    paths
}

pub fn process_media_attachments(text: &str) -> (String, Vec<MediaAttachment>) {
    let audio_as_voice = text.contains("[[audio_as_voice]]");
    let as_document = text.contains("[[as_document]]");

    let cleaned = text.replace("[[audio_as_voice]]", "").replace("[[as_document]]", "");
    let cleaned = cleaned.trim().to_string();

    let paths = extract_absolute_paths(text);
    let mut attachments = Vec::new();

    for path_str in paths {
        let path = std::path::Path::new(&path_str);
        let ext = match path.extension().and_then(|e| e.to_str()) {
            Some(e) => e,
            None => continue,
        };
        let media_type = match detect_media_type(ext) {
            Some(mt) => mt,
            None => continue,
        };
        if !path.is_file() {
            continue;
        }

        let delivery_mode = if as_document {
            DeliveryMode::Document
        } else if audio_as_voice && media_type == MediaType::Audio {
            DeliveryMode::Voice
        } else {
            DeliveryMode::Native
        };

        attachments.push(MediaAttachment { path: path_str, media_type, delivery_mode });
    }

    (cleaned, attachments)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_type_as_str_mapping() {
        assert_eq!(MediaType::Image.as_str(), "image");
        assert_eq!(MediaType::Audio.as_str(), "audio");
        assert_eq!(MediaType::Video.as_str(), "video");
        assert_eq!(MediaType::Document.as_str(), "document");
    }

    #[test]
    fn delivery_mode_as_str_mapping() {
        assert_eq!(DeliveryMode::Native.as_str(), "native");
        assert_eq!(DeliveryMode::Voice.as_str(), "voice");
        assert_eq!(DeliveryMode::Document.as_str(), "document");
    }

    #[test]
    fn detect_media_type_covers_all_categories() {
        // 图片
        for ext in ["png", "JPG", "jpeg", "gif", "svg", "webp", "bmp", "ico", "tiff", "tif"] {
            assert_eq!(detect_media_type(ext), Some(MediaType::Image), "ext={ext}");
        }
        // 音频
        for ext in ["mp3", "wav", "ogg", "flac", "aac", "m4a", "wma"] {
            assert_eq!(detect_media_type(ext), Some(MediaType::Audio), "ext={ext}");
        }
        // 视频
        for ext in ["mp4", "webm", "avi", "mkv", "mov", "wmv", "flv"] {
            assert_eq!(detect_media_type(ext), Some(MediaType::Video), "ext={ext}");
        }
        // 文档
        for ext in ["pdf", "docx", "xlsx", "pptx", "doc", "xls", "ppt", "odt", "ods", "odp"] {
            assert_eq!(detect_media_type(ext), Some(MediaType::Document), "ext={ext}");
        }
    }

    #[test]
    fn detect_media_type_is_case_insensitive_and_rejects_unknown() {
        assert_eq!(detect_media_type("PNG"), Some(MediaType::Image));
        assert_eq!(detect_media_type("Mp4"), Some(MediaType::Video));
        assert_eq!(detect_media_type("exe"), None);
        assert_eq!(detect_media_type(""), None);
        assert_eq!(detect_media_type("txt"), None);
    }

    #[test]
    fn extract_absolute_paths_unix_and_windows() {
        let text = "见 /home/user/a.png 与 C:\\Users\\me\\b.pdf 两个文件";
        let paths = extract_absolute_paths(text);
        assert!(paths.iter().any(|p| p == "/home/user/a.png"), "{paths:?}");
        assert!(paths.iter().any(|p| p == "C:\\Users\\me\\b.pdf"), "{paths:?}");
    }

    #[test]
    fn extract_absolute_paths_dedup_and_trailing_punct() {
        // 同一路径出现两次 + 结尾中文句号应被剥离且去重
        let text = "文件在 /tmp/x.mp3。再看 /tmp/x.mp3";
        let paths = extract_absolute_paths(text);
        assert_eq!(paths.len(), 1, "应去重: {paths:?}");
        assert_eq!(paths[0], "/tmp/x.mp3");
    }

    #[test]
    fn extract_absolute_paths_ignores_bare_filename() {
        // 无路径分隔符的裸文件名不应被提取（正则要求以 / 或 盘符:\ 开头）
        let paths = extract_absolute_paths("这是 report.png 一个纯文件名");
        assert!(paths.is_empty(), "裸文件名不应被提取: {paths:?}");
    }

    #[test]
    fn process_media_strips_markers_and_cleans_text() {
        let (cleaned, atts) =
            process_media_attachments("你好[[audio_as_voice]] 世界[[as_document]]");
        assert!(!cleaned.contains("[[audio_as_voice]]"));
        assert!(!cleaned.contains("[[as_document]]"));
        assert_eq!(cleaned, "你好 世界");
        // 无真实文件路径 → 无附件
        assert!(atts.is_empty());
    }

    #[test]
    fn process_media_skips_nonexistent_files() {
        // 路径合法且扩展名可识别，但文件不存在 → 应被过滤
        let (_cleaned, atts) = process_media_attachments("看 /nonexistent/definitely/x.png");
        assert!(atts.is_empty());
    }

    #[test]
    fn process_media_detects_real_file_delivery_mode() {
        // 创建真实临时文件以走通 is_file() 分支，验证 delivery_mode 逻辑
        let dir = std::env::temp_dir().join(format!("axtest-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let audio = dir.join("clip.mp3");
        std::fs::write(&audio, b"fake").unwrap();
        let audio_str = audio.to_string_lossy().replace('\\', "/");

        // 默认 → Native
        let (_c, atts) = process_media_attachments(&format!("发送 {audio_str}"));
        assert_eq!(atts.len(), 1, "{atts:?}");
        assert_eq!(atts[0].media_type, MediaType::Audio);
        assert_eq!(atts[0].delivery_mode, DeliveryMode::Native);

        // [[audio_as_voice]] + 音频 → Voice
        let (_c, atts) = process_media_attachments(&format!("[[audio_as_voice]] 发送 {audio_str}"));
        assert_eq!(atts[0].delivery_mode, DeliveryMode::Voice);

        // [[as_document]] 优先 → Document
        let (_c, atts) = process_media_attachments(&format!(
            "[[as_document]][[audio_as_voice]] 发送 {audio_str}"
        ));
        assert_eq!(atts[0].delivery_mode, DeliveryMode::Document);

        std::fs::remove_dir_all(&dir).ok();
    }
}
