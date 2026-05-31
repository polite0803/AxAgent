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
    let re = regex::Regex::new(r#"(?:(?:[A-Za-z]:[/\\])|/)[^\s"'<>\]\)}，。；：！？、]+"#).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut paths = Vec::new();
    for cap in re.captures_iter(text) {
        let p = cap[0].to_string();
        let cleaned = p.trim_end_matches(|c: char| c == '.' || c == ',' || c == ';' || c == ':');
        if seen.insert(cleaned.to_string()) {
            paths.push(cleaned.to_string());
        }
    }
    paths
}

pub fn process_media_attachments(text: &str) -> (String, Vec<MediaAttachment>) {
    let audio_as_voice = text.contains("[[audio_as_voice]]");
    let as_document = text.contains("[[as_document]]");

    let cleaned = text
        .replace("[[audio_as_voice]]", "")
        .replace("[[as_document]]", "");
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

        attachments.push(MediaAttachment {
            path: path_str,
            media_type,
            delivery_mode,
        });
    }

    (cleaned, attachments)
}
