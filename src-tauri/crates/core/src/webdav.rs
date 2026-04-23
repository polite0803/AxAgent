use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;

use crate::error::{AxAgentError, Result};

// === Types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub path: String,
    pub accept_invalid_certs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavFileInfo {
    pub file_name: String,
    pub size: i64,
    pub last_modified: String,
    pub hostname: String,
}

pub struct BackupZipContents {
    pub db_path: std::path::PathBuf,
    pub metadata: serde_json::Value,
    pub has_documents: bool,
    pub has_workspace: bool,
    pub master_key_path: Option<std::path::PathBuf>,
}

// === WebDAV Client ===

pub struct WebDavClient {
    client: Client,
    config: WebDavConfig,
}

impl WebDavClient {
    pub fn new(config: WebDavConfig) -> Result<Self> {
        let client = Client::builder()
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| AxAgentError::Gateway(format!("Failed to create HTTP client: {}", e)))?;
        Ok(Self { client, config })
    }

    fn base_url(&self) -> String {
        let host = self.config.host.trim_end_matches('/');
        let path = self.config.path.trim_matches('/');
        if path.is_empty() {
            format!("{}/", host)
        } else {
            format!("{}/{}/", host, path)
        }
    }

    fn file_url(&self, filename: &str) -> String {
        format!("{}{}", self.base_url(), filename)
    }

    /// Check connection and auto-create remote directory if missing.
    pub async fn check_connection(&self) -> Result<bool> {
        let url = self.base_url();
        let method = Method::from_bytes(b"PROPFIND")
            .map_err(|e| AxAgentError::Gateway(format!("Invalid method: {}", e)))?;

        let response = self
            .client
            .request(method, &url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Depth", "0")
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV connection failed: {}", e)))?;

        match response.status() {
            StatusCode::MULTI_STATUS | StatusCode::OK => Ok(true),
            StatusCode::NOT_FOUND => {
                self.mkdir().await?;
                Ok(true)
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => Err(AxAgentError::Gateway(
                "WebDAV authentication failed".to_string(),
            )),
            status => Err(AxAgentError::Gateway(format!(
                "WebDAV error: HTTP {}",
                status
            ))),
        }
    }

    /// Create the remote directory tree.
    async fn mkdir(&self) -> Result<()> {
        let host = self.config.host.trim_end_matches('/');
        let path = self.config.path.trim_matches('/');
        if path.is_empty() {
            return Ok(());
        }

        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let mut current = String::new();

        for part in parts {
            current = if current.is_empty() {
                part.to_string()
            } else {
                format!("{}/{}", current, part)
            };

            let url = format!("{}/{}/", host, current);
            let method = Method::from_bytes(b"MKCOL")
                .map_err(|e| AxAgentError::Gateway(format!("Invalid method: {}", e)))?;

            let response = self
                .client
                .request(method, &url)
                .basic_auth(&self.config.username, Some(&self.config.password))
                .send()
                .await
                .map_err(|e| AxAgentError::Gateway(format!("WebDAV MKCOL failed: {}", e)))?;

            // CREATED=success, METHOD_NOT_ALLOWED=already exists
            match response.status() {
                StatusCode::CREATED | StatusCode::OK | StatusCode::METHOD_NOT_ALLOWED => {}
                status => {
                    return Err(AxAgentError::Gateway(format!(
                        "WebDAV mkdir failed for '{}': HTTP {}",
                        current, status
                    )));
                }
            }
        }
        Ok(())
    }

    /// List axagent backup .zip files in the remote directory.
    pub async fn list_files(&self) -> Result<Vec<WebDavFileInfo>> {
        run_after_directory_ready(
            || self.check_connection(),
            || async {
                let url = self.base_url();
                let method = Method::from_bytes(b"PROPFIND")
                    .map_err(|e| AxAgentError::Gateway(format!("Invalid method: {}", e)))?;

                let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:resourcetype/>
  </D:prop>
</D:propfind>"#;

                let response = self
                    .client
                    .request(method, &url)
                    .basic_auth(&self.config.username, Some(&self.config.password))
                    .header("Depth", "1")
                    .header("Content-Type", "application/xml; charset=utf-8")
                    .body(body)
                    .send()
                    .await
                    .map_err(|e| AxAgentError::Gateway(format!("WebDAV PROPFIND failed: {}", e)))?;

                if response.status() != StatusCode::MULTI_STATUS && !response.status().is_success()
                {
                    return Err(AxAgentError::Gateway(format!(
                        "WebDAV list failed: HTTP {}",
                        response.status()
                    )));
                }

                let text = response
                    .text()
                    .await
                    .map_err(|e| AxAgentError::Gateway(format!("Failed to read response: {}", e)))?;

                parse_propfind_response(&text)
            },
        )
        .await
    }

    /// Upload a local file to the remote directory.
    pub async fn upload_file(&self, filename: &str, local_path: &Path) -> Result<()> {
        run_after_directory_ready(
            || self.check_connection(),
            || async {
                let data = std::fs::read(local_path)
                    .map_err(|e| AxAgentError::Gateway(format!("Failed to read file: {}", e)))?;
                let url = self.file_url(filename);

                let response = self
                    .client
                    .put(&url)
                    .basic_auth(&self.config.username, Some(&self.config.password))
                    .header("Content-Type", "application/octet-stream")
                    .body(data)
                    .send()
                    .await
                    .map_err(|e| AxAgentError::Gateway(format!("WebDAV upload failed: {}", e)))?;

                match response.status() {
                    StatusCode::CREATED | StatusCode::OK | StatusCode::NO_CONTENT => Ok(()),
                    status => Err(AxAgentError::Gateway(format!(
                        "WebDAV upload failed: HTTP {}",
                        status
                    ))),
                }
            },
        )
        .await
    }

    /// Download a file from the remote directory to a local path.
    pub async fn download_file(&self, filename: &str, local_path: &Path) -> Result<()> {
        let url = self.file_url(filename);

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AxAgentError::Gateway(format!(
                "WebDAV download failed: HTTP {}",
                response.status()
            )));
        }

        let data = response
            .bytes()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("Failed to read download: {}", e)))?;

        std::fs::write(local_path, &data)
            .map_err(|e| AxAgentError::Gateway(format!("Failed to write file: {}", e)))?;
        Ok(())
    }

    /// Delete a file from the remote directory.
    pub async fn delete_file(&self, filename: &str) -> Result<()> {
        let url = self.file_url(filename);

        let response = self
            .client
            .delete(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV delete failed: {}", e)))?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
            status => Err(AxAgentError::Gateway(format!(
                "WebDAV delete failed: HTTP {}",
                status
            ))),
        }
    }
}

// === ZIP Backup Utilities ===

/// Create a backup ZIP containing the database snapshot and metadata.
pub fn create_backup_zip(
    db_path: &Path,
    documents_dir: Option<&Path>,
    workspace_dir: Option<&Path>,
    master_key_path: Option<&Path>,
    dest_zip: &Path,
    app_version: &str,
    object_counts_json: &str,
) -> Result<()> {
    let file = std::fs::File::create(dest_zip)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to create ZIP file: {}", e)))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // axagent.db
    let db_data = std::fs::read(db_path)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to read database: {}", e)))?;
    let db_checksum = format!("{:x}", Sha256::digest(&db_data));

    zip.start_file("axagent.db", options)
        .map_err(|e| AxAgentError::Gateway(format!("ZIP error: {}", e)))?;
    zip.write_all(&db_data)
        .map_err(|e| AxAgentError::Gateway(format!("ZIP write error: {}", e)))?;

    // metadata.json
    let metadata = serde_json::json!({
        "version": 1,
        "app_version": app_version,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "hostname": get_hostname(),
        "db_checksum": db_checksum,
        "include_documents": documents_dir.is_some(),
        "include_workspace": workspace_dir.is_some(),
        "object_counts": object_counts_json,
    });
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .map_err(|e| AxAgentError::Gateway(format!("JSON error: {}", e)))?;

    zip.start_file("metadata.json", SimpleFileOptions::default())
        .map_err(|e| AxAgentError::Gateway(format!("ZIP error: {}", e)))?;
    zip.write_all(metadata_json.as_bytes())
        .map_err(|e| AxAgentError::Gateway(format!("ZIP write error: {}", e)))?;

    // Optional: master.key for cross-device restore
    if let Some(key_path) = master_key_path {
        if key_path.exists() {
            let key_data = std::fs::read(key_path)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to read master.key: {}", e)))?;
            zip.start_file("master.key", options)
                .map_err(|e| AxAgentError::Gateway(format!("ZIP error: {}", e)))?;
            zip.write_all(&key_data)
                .map_err(|e| AxAgentError::Gateway(format!("ZIP write error: {}", e)))?;
        }
    }

    // Optional: documents/ directory
    if let Some(docs_dir) = documents_dir {
        if docs_dir.exists() {
            add_directory_to_zip(&mut zip, docs_dir, "documents", options)?;
        }
    }

    // Optional: workspace/ directory
    if let Some(ws_dir) = workspace_dir {
        if ws_dir.exists() {
            add_directory_to_zip(&mut zip, ws_dir, "workspace", options)?;
        }
    }

    zip.finish()
        .map_err(|e| AxAgentError::Gateway(format!("ZIP finalize error: {}", e)))?;
    Ok(())
}

/// Extract a backup ZIP and return its contents.
pub fn extract_backup_zip(zip_path: &Path, dest_dir: &Path) -> Result<BackupZipContents> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to open ZIP: {}", e)))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AxAgentError::Gateway(format!("Invalid ZIP file: {}", e)))?;

    std::fs::create_dir_all(dest_dir)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to create temp dir: {}", e)))?;

    let mut db_path = None;
    let mut metadata = None;
    let mut has_documents = false;
    let mut has_workspace = false;
    let mut master_key_path = None;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AxAgentError::Gateway(format!("ZIP read error: {}", e)))?;
        let name = entry.name().to_string();

        if name.contains("..") {
            continue; // path traversal protection
        }

        if name == "axagent.db" {
            let path = dest_dir.join("axagent.db");
            let mut outfile = std::fs::File::create(&path)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to extract db: {}", e)))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to extract db: {}", e)))?;
            db_path = Some(path);
        } else if name == "metadata.json" {
            let mut contents = String::new();
            entry
                .read_to_string(&mut contents)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to read metadata: {}", e)))?;
            metadata = Some(
                serde_json::from_str::<serde_json::Value>(&contents)
                    .map_err(|e| AxAgentError::Gateway(format!("Invalid metadata JSON: {}", e)))?,
            );
        } else if name == "master.key" {
            let path = dest_dir.join("master.key");
            let mut outfile = std::fs::File::create(&path)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to extract master.key: {}", e)))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to extract master.key: {}", e)))?;
            master_key_path = Some(path);
        } else if name.starts_with("documents/") && !entry.is_dir() {
            has_documents = true;
            let path = dest_dir.join(&name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let mut outfile = std::fs::File::create(&path)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to extract file: {}", e)))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to extract file: {}", e)))?;
        } else if name.starts_with("workspace/") && !entry.is_dir() {
            has_workspace = true;
            let path = dest_dir.join(&name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let mut outfile = std::fs::File::create(&path)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to extract file: {}", e)))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to extract file: {}", e)))?;
        }
    }

    Ok(BackupZipContents {
        db_path: db_path.ok_or_else(|| AxAgentError::Gateway("No axagent.db in backup ZIP".into()))?,
        metadata: metadata
            .ok_or_else(|| AxAgentError::Gateway("No metadata.json in backup ZIP".into()))?,
        has_documents,
        has_workspace,
        master_key_path,
    })
}

/// Verify the checksum of an extracted database against metadata.
pub fn verify_db_checksum(db_path: &Path, expected_checksum: &str) -> Result<bool> {
    let data = std::fs::read(db_path)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to read db for checksum: {}", e)))?;
    let actual = format!("{:x}", Sha256::digest(&data));
    Ok(actual == expected_checksum)
}

/// Generate a backup filename with timestamp and hostname.
pub fn generate_backup_filename() -> String {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let hostname = get_hostname();
    format!("axagent-backup-{}.{}.zip", timestamp, hostname)
}

/// Parse hostname from a backup filename.
pub fn parse_hostname_from_filename(filename: &str) -> String {
    // Format: axagent-backup-YYYYMMDD_HHMMSS.hostname.zip
    let name = filename.trim_end_matches(".zip");
    if let Some(rest) = name.strip_prefix("axagent-backup-") {
        // rest = "YYYYMMDD_HHMMSS.hostname"
        if let Some(dot_pos) = rest.find('.') {
            return rest[dot_pos + 1..].to_string();
        }
    }
    "unknown".to_string()
}

pub fn documents_sync_root() -> std::path::PathBuf {
    crate::storage_paths::documents_root()
}

pub fn sync_status_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

async fn run_after_directory_ready<T, Check, CheckFut, Action, ActionFut>(
    check: Check,
    action: Action,
) -> Result<T>
where
    Check: FnOnce() -> CheckFut,
    CheckFut: Future<Output = Result<bool>>,
    Action: FnOnce() -> ActionFut,
    ActionFut: Future<Output = Result<T>>,
{
    check().await?;
    action().await
}

// === Internal Helpers ===

fn get_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn add_directory_to_zip<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    dir: &Path,
    prefix: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    let mut files = Vec::new();
    collect_files(dir, &mut files)?;

    for file_path in files {
        let rel = file_path
            .strip_prefix(dir)
            .map_err(|e| AxAgentError::Gateway(format!("Path error: {}", e)))?;
        let zip_path = format!("{}/{}", prefix, rel.to_string_lossy());

        zip.start_file(&zip_path, options)
            .map_err(|e| AxAgentError::Gateway(format!("ZIP error: {}", e)))?;
        let data = std::fs::read(&file_path)
            .map_err(|e| AxAgentError::Gateway(format!("Read error: {}", e)))?;
        zip.write_all(&data)
            .map_err(|e| AxAgentError::Gateway(format!("ZIP write error: {}", e)))?;
    }
    Ok(())
}

fn collect_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to read directory: {}", e)))?
    {
        let entry = entry.map_err(|e| AxAgentError::Gateway(format!("Dir entry error: {}", e)))?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

/// Parse WebDAV PROPFIND XML response to extract file information.
fn parse_propfind_response(xml: &str) -> Result<Vec<WebDavFileInfo>> {
    let mut files = Vec::new();
    let response_blocks = split_xml_responses(xml);

    for block in response_blocks {
        let lower_block = block.to_lowercase();
        // Skip collections (directories)
        if lower_block.contains("<d:collection") || lower_block.contains("<collection") {
            continue;
        }

        let href = extract_xml_value(&block, "href").unwrap_or_default();
        if href.is_empty() || href.ends_with('/') {
            continue;
        }

        let file_name = url_decode(href.split('/').last().unwrap_or(""));
        if file_name.is_empty() || !file_name.ends_with(".zip") {
            continue;
        }

        if !file_name.starts_with("axagent-backup-") {
            continue;
        }

        let size: i64 = extract_xml_value(&block, "getcontentlength")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let last_modified = extract_xml_value(&block, "getlastmodified").unwrap_or_default();
        let hostname = parse_hostname_from_filename(&file_name);

        files.push(WebDavFileInfo {
            file_name,
            size,
            last_modified,
            hostname,
        });
    }

    // Newest first (filenames contain timestamps)
    files.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(files)
}

fn split_xml_responses(xml: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let lower = xml.to_lowercase();

    let tag_patterns = ["d:response", "response"];
    for tag in &tag_patterns {
        let open1 = format!("<{}>", tag);
        let open2 = format!("<{} ", tag);
        let close = format!("</{}>", tag);

        let mut pos = 0;
        while pos < lower.len() {
            let start = lower[pos..]
                .find(&open1)
                .or_else(|| lower[pos..].find(&open2));
            if let Some(s) = start {
                let abs_start = pos + s;
                if let Some(end) = lower[abs_start..].find(&close) {
                    let abs_end = abs_start + end + close.len();
                    blocks.push(xml[abs_start..abs_end].to_string());
                    pos = abs_end;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if !blocks.is_empty() {
            break;
        }
    }
    blocks
}

fn extract_xml_value(xml: &str, tag_local_name: &str) -> Option<String> {
    let lower = xml.to_lowercase();
    let tag = tag_local_name.to_lowercase();

    let patterns = [
        (format!("<d:{}>", tag), format!("</d:{}>", tag)),
        (format!("<{}>", tag), format!("</{}>", tag)),
    ];

    for (open, close) in &patterns {
        if let Some(start) = lower.find(open) {
            let content_start = start + open.len();
            if let Some(end) = lower[content_start..].find(close) {
                return Some(xml[content_start..content_start + end].trim().to_string());
            }
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h1 = (bytes[i + 1] as char).to_digit(16);
            let h2 = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h1), Some(h2)) = (h1, h2) {
                result.push((h1 * 16 + h2) as u8 as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn run_after_directory_ready_checks_before_action() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let check_events = events.clone();
        let action_events = events.clone();

        let result = run_after_directory_ready(
            move || async move {
                check_events.lock().unwrap().push("check");
                Ok(true)
            },
            move || async move {
                action_events.lock().unwrap().push("action");
                Ok::<_, AxAgentError>("done")
            },
        )
        .await;

        assert!(matches!(result, Ok("done")));
        assert_eq!(*events.lock().unwrap(), vec!["check", "action"]);
    }

    #[tokio::test]
    async fn run_after_directory_ready_skips_action_when_check_fails() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let check_events = events.clone();
        let action_events = events.clone();

        let result: Result<&'static str> = run_after_directory_ready(
            move || async move {
                check_events.lock().unwrap().push("check");
                Err(AxAgentError::Gateway("probe failed".into()))
            },
            move || async move {
                action_events.lock().unwrap().push("action");
                Ok("done")
            },
        )
        .await;

        assert!(result.is_err(), "check failures must stop the action");
        assert_eq!(*events.lock().unwrap(), vec!["check"]);
    }

    #[test]
    fn documents_sync_root_matches_documents_root() {
        assert_eq!(
            documents_sync_root(),
            crate::storage_paths::documents_root()
        );
    }

    #[test]
    fn sync_status_timestamp_is_rfc3339() {
        let timestamp = sync_status_timestamp();
        assert!(
            chrono::DateTime::parse_from_rfc3339(&timestamp).is_ok(),
            "sync status timestamps should be RFC3339 so the frontend can render them directly, got: {timestamp}"
        );
    }
}
