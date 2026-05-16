use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::future::Future;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use zip::write::SimpleFileOptions;

use crate::error::{AxAgentError, Result};

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

pub struct WebDavClient {
    client: Client,
    config: WebDavConfig,
    mkdir_cache: Mutex<HashSet<String>>,
}

impl WebDavClient {
    pub fn new(config: WebDavConfig) -> Result<Self> {
        let client = Client::builder()
            .danger_accept_invalid_certs(config.accept_invalid_certs)
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| AxAgentError::Gateway(format!("Failed to create HTTP client: {}", e)))?;
        Ok(Self {
            client,
            config,
            mkdir_cache: Mutex::new(HashSet::new()),
        })
    }

    pub fn config(&self) -> &WebDavConfig {
        &self.config
    }

    pub fn base_url(&self) -> String {
        let host = self.config.host.trim_end_matches('/');
        let path = self.config.path.trim_matches('/');
        if path.is_empty() {
            format!("{}/", host)
        } else {
            format!("{}/{}/", host, path)
        }
    }

    pub fn file_url(&self, filename: &str) -> String {
        format!("{}{}", self.base_url(), filename)
    }

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
                self.ensure_dir().await?;
                Ok(true)
            },
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(AxAgentError::Gateway("WebDAV authentication failed".to_string()))
            },
            status => Err(AxAgentError::Gateway(format!("WebDAV error: HTTP {}", status))),
        }
    }

    pub async fn ensure_dir(&self) -> Result<()> {
        let host = self.config.host.trim_end_matches('/');
        let path = self.config.path.trim_matches('/');
        if path.is_empty() {
            return Ok(());
        }

        {
            let cache = self.mkdir_cache.lock().unwrap();
            if cache.contains(path) {
                return Ok(());
            }
        }

        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let mut current = String::new();

        for part in parts {
            current = if current.is_empty() {
                part.to_string()
            } else {
                format!("{}/{}", current, part)
            };

            {
                let cache = self.mkdir_cache.lock().unwrap();
                if cache.contains(&current) {
                    continue;
                }
            }

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

            match response.status() {
                StatusCode::CREATED | StatusCode::OK | StatusCode::METHOD_NOT_ALLOWED => {},
                status => {
                    return Err(AxAgentError::Gateway(format!(
                        "WebDAV mkdir failed for '{}': HTTP {}",
                        current, status
                    )));
                },
            }

            self.mkdir_cache.lock().unwrap().insert(current.clone());
        }
        Ok(())
    }

    pub async fn ensure_parent_dir(&self, key: &str) -> Result<()> {
        if let Some(parent) = key.rfind('/') {
            let parent_path = &key[..parent];
            let host = self.config.host.trim_end_matches('/');
            let base_path = self.config.path.trim_matches('/');

            let full_parent = if base_path.is_empty() {
                parent_path.to_string()
            } else {
                format!("{}/{}", base_path, parent_path)
            };

            {
                let cache = self.mkdir_cache.lock().unwrap();
                if cache.contains(&full_parent) {
                    return Ok(());
                }
            }

            let parts: Vec<&str> = full_parent.split('/').filter(|p| !p.is_empty()).collect();
            let mut current = String::new();

            for part in parts {
                current = if current.is_empty() {
                    part.to_string()
                } else {
                    format!("{}/{}", current, part)
                };

                {
                    let cache = self.mkdir_cache.lock().unwrap();
                    if cache.contains(&current) {
                        continue;
                    }
                }

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

                match response.status() {
                    StatusCode::CREATED | StatusCode::OK | StatusCode::METHOD_NOT_ALLOWED => {},
                    status => {
                        return Err(AxAgentError::Gateway(format!(
                            "WebDAV mkdir failed for '{}': HTTP {}",
                            current, status
                        )));
                    },
                }

                self.mkdir_cache.lock().unwrap().insert(current.clone());
            }
        }
        Ok(())
    }

    pub async fn propfind(&self, url: &str, depth: &str) -> Result<String> {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:getetag/>
    <D:resourcetype/>
  </D:prop>
</D:propfind>"#;

        let method = Method::from_bytes(b"PROPFIND")
            .map_err(|e| AxAgentError::Gateway(format!("Invalid method: {}", e)))?;

        let response = self
            .client
            .request(method, url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Depth", depth)
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV PROPFIND failed: {}", e)))?;

        if response.status() == StatusCode::FORBIDDEN && depth == "infinity" {
            return Err(AxAgentError::Gateway("Depth infinity not supported".to_string()));
        }

        if response.status() != StatusCode::MULTI_STATUS && !response.status().is_success() {
            return Err(AxAgentError::Gateway(format!(
                "WebDAV PROPFIND failed: HTTP {}",
                response.status()
            )));
        }

        response
            .text()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("Failed to read response: {}", e)))
    }

    pub async fn get_raw(&self, key: &str) -> Result<(Vec<u8>, Option<String>, Option<String>)> {
        let url = self.file_url(key);
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

        let etag = response
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let last_modified = response
            .headers()
            .get("last-modified")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let data = response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| AxAgentError::Gateway(format!("Failed to read response: {}", e)))?;

        Ok((data, etag, last_modified))
    }

    pub async fn put_raw(
        &self,
        key: &str,
        data: &[u8],
        if_match: Option<&str>,
    ) -> Result<Option<String>> {
        self.ensure_parent_dir(key).await?;

        let url = self.file_url(key);
        let mut req = self
            .client
            .put(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Content-Type", "application/octet-stream")
            .body(data.to_vec());

        if let Some(etag) = if_match {
            req = req.header("If-Match", etag);
        }

        let response = req
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV upload failed: {}", e)))?;

        if response.status() == StatusCode::PRECONDITION_FAILED {
            return Err(AxAgentError::Gateway(
                "WebDAV precondition failed: ETag mismatch".to_string(),
            ));
        }

        match response.status() {
            StatusCode::CREATED | StatusCode::OK | StatusCode::NO_CONTENT => {},
            status => {
                return Err(AxAgentError::Gateway(format!(
                    "WebDAV upload failed: HTTP {}",
                    status
                )));
            },
        }

        let etag = response
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        Ok(etag)
    }

    pub async fn delete_raw(&self, key: &str, if_match: Option<&str>) -> Result<()> {
        let url = self.file_url(key);
        let mut req = self
            .client
            .delete(&url)
            .basic_auth(&self.config.username, Some(&self.config.password));

        if let Some(etag) = if_match {
            req = req.header("If-Match", etag);
        }

        let response = req
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV delete failed: {}", e)))?;

        if response.status() == StatusCode::PRECONDITION_FAILED {
            return Err(AxAgentError::Gateway(
                "WebDAV precondition failed: ETag mismatch".to_string(),
            ));
        }

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
            status => Err(AxAgentError::Gateway(format!("WebDAV delete failed: HTTP {}", status))),
        }
    }

    pub async fn head_raw(&self, key: &str) -> Result<(Option<String>, Option<String>, i64)> {
        let url = self.file_url(key);
        let response = self
            .client
            .head(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV HEAD failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AxAgentError::NotFound(format!("WebDAV object not found: {}", key)));
        }

        let etag = response
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let last_modified = response
            .headers()
            .get("last-modified")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let size = response
            .headers()
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        Ok((etag, last_modified, size))
    }

    pub async fn list_files(&self) -> Result<Vec<WebDavFileInfo>> {
        run_after_directory_ready(
            || self.check_connection(),
            || async {
                let url = self.base_url();
                let text = self.propfind(&url, "1").await?;
                parse_propfind_response(&text)
            },
        )
        .await
    }

    pub async fn upload_file(&self, filename: &str, local_path: &Path) -> Result<()> {
        run_after_directory_ready(
            || self.check_connection(),
            || async {
                let data = std::fs::read(local_path)
                    .map_err(|e| AxAgentError::Gateway(format!("Failed to read file: {}", e)))?;
                self.put_raw(filename, &data, None).await?;
                Ok(())
            },
        )
        .await
    }

    pub async fn download_file(&self, filename: &str, local_path: &Path) -> Result<()> {
        let (data, _, _) = self.get_raw(filename).await?;
        std::fs::write(local_path, &data)
            .map_err(|e| AxAgentError::Gateway(format!("Failed to write file: {}", e)))?;
        Ok(())
    }

    pub async fn delete_file(&self, filename: &str) -> Result<()> {
        self.delete_raw(filename, None).await
    }

    pub async fn list_recursive(
        &self,
        prefix: &str,
    ) -> Result<Vec<crate::cloud_storage::StorageObjectMeta>> {
        let url = if prefix.is_empty() {
            self.base_url()
        } else {
            format!("{}{}", self.base_url(), prefix)
        };

        match self.propfind(&url, "infinity").await {
            Ok(text) => {
                let objects = parse_propfind_responses_for_sync(&text, prefix)?;
                Ok(objects)
            },
            Err(_) => self.list_recursive_iterative(prefix).await,
        }
    }

    async fn list_recursive_iterative(
        &self,
        prefix: &str,
    ) -> Result<Vec<crate::cloud_storage::StorageObjectMeta>> {
        let mut all_objects = Vec::new();
        let mut dirs_to_visit = vec![prefix.to_string()];

        while let Some(dir) = dirs_to_visit.pop() {
            let url = if dir.is_empty() {
                self.base_url()
            } else {
                format!("{}{}", self.base_url(), dir)
            };

            let text = self.propfind(&url, "1").await?;
            let (files, subdirs) = parse_propfind_responses_with_dirs(&text, &dir)?;

            all_objects.extend(files);
            dirs_to_visit.extend(subdirs);
        }

        Ok(all_objects)
    }
}

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

    let db_data = std::fs::read(db_path)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to read database: {}", e)))?;
    let db_checksum = format!("{:x}", Sha256::digest(&db_data));

    zip.start_file("axagent.db", options)
        .map_err(|e| AxAgentError::Gateway(format!("ZIP error: {}", e)))?;
    zip.write_all(&db_data)
        .map_err(|e| AxAgentError::Gateway(format!("ZIP write error: {}", e)))?;

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

    if let Some(key_path) = master_key_path {
        if key_path.exists() {
            let key_data = std::fs::read(key_path)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to read master.key: {}", e)))?;
            let encrypted_key = crate::crypto::encrypt_backup_key(&key_data);
            zip.start_file("master.key.enc", options)
                .map_err(|e| AxAgentError::Gateway(format!("ZIP error: {}", e)))?;
            zip.write_all(&encrypted_key)
                .map_err(|e| AxAgentError::Gateway(format!("ZIP write error: {}", e)))?;
        }
    }

    if let Some(docs_dir) = documents_dir {
        if docs_dir.exists() {
            add_directory_to_zip(&mut zip, docs_dir, "documents", options)?;
        }
    }

    if let Some(ws_dir) = workspace_dir {
        if ws_dir.exists() {
            add_directory_to_zip(&mut zip, ws_dir, "workspace", options)?;
        }
    }

    zip.finish()
        .map_err(|e| AxAgentError::Gateway(format!("ZIP finalize error: {}", e)))?;
    Ok(())
}

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

        if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
            continue;
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
        } else if name == "master.key.enc" {
            let mut enc_data = Vec::new();
            entry.read_to_end(&mut enc_data).map_err(|e| {
                AxAgentError::Gateway(format!("Failed to read master.key.enc: {}", e))
            })?;
            let key_data = crate::crypto::decrypt_backup_key(&enc_data).map_err(|e| {
                AxAgentError::Gateway(format!("Failed to decrypt master.key: {}", e))
            })?;
            let path = dest_dir.join("master.key");
            let mut outfile = std::fs::File::create(&path).map_err(|e| {
                AxAgentError::Gateway(format!("Failed to extract master.key: {}", e))
            })?;
            outfile
                .write_all(&key_data)
                .map_err(|e| AxAgentError::Gateway(format!("Failed to write master.key: {}", e)))?;
            master_key_path = Some(path);
        } else if name == "master.key" {
            let path = dest_dir.join("master.key");
            let mut outfile = std::fs::File::create(&path).map_err(|e| {
                AxAgentError::Gateway(format!("Failed to extract master.key: {}", e))
            })?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| {
                AxAgentError::Gateway(format!("Failed to extract master.key: {}", e))
            })?;
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
        db_path: db_path
            .ok_or_else(|| AxAgentError::Gateway("No axagent.db in backup ZIP".into()))?,
        metadata: metadata
            .ok_or_else(|| AxAgentError::Gateway("No metadata.json in backup ZIP".into()))?,
        has_documents,
        has_workspace,
        master_key_path,
    })
}

pub fn verify_db_checksum(db_path: &Path, expected_checksum: &str) -> Result<bool> {
    let data = std::fs::read(db_path)
        .map_err(|e| AxAgentError::Gateway(format!("Failed to read db for checksum: {}", e)))?;
    let actual = format!("{:x}", Sha256::digest(&data));
    Ok(actual == expected_checksum)
}

pub fn generate_backup_filename() -> String {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let hostname = get_hostname();
    format!("axagent-backup-{}.{}.zip", timestamp, hostname)
}

pub fn parse_hostname_from_filename(filename: &str) -> String {
    let name = filename.trim_end_matches(".zip");
    if let Some(rest) = name.strip_prefix("axagent-backup-") {
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

fn parse_propfind_response(xml: &str) -> Result<Vec<WebDavFileInfo>> {
    let mut files = Vec::new();
    let response_blocks = split_xml_responses(xml);

    for block in response_blocks {
        let lower_block = block.to_lowercase();
        if lower_block.contains("<d:collection") || lower_block.contains("<collection") {
            continue;
        }

        let href = extract_xml_value(&block, "href").unwrap_or_default();
        if href.is_empty() || href.ends_with('/') {
            continue;
        }

        let file_name = url_decode(href.split('/').next_back().unwrap_or(""));
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

    files.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(files)
}

fn parse_propfind_responses_for_sync(
    xml: &str,
    prefix: &str,
) -> Result<Vec<crate::cloud_storage::StorageObjectMeta>> {
    let mut files = Vec::new();
    let response_blocks = split_xml_responses(xml);
    let base_href = {
        let base = if prefix.is_empty() { "/" } else { prefix };
        base.trim_matches('/').to_string()
    };

    for block in response_blocks {
        let lower_block = block.to_lowercase();
        if lower_block.contains("<d:collection") || lower_block.contains("<collection") {
            continue;
        }

        let href = extract_xml_value(&block, "href").unwrap_or_default();
        if href.is_empty() || href.ends_with('/') {
            continue;
        }

        let decoded_href = url_decode(&href);
        let file_name = decoded_href
            .split('/')
            .rfind(|s| !s.is_empty())
            .unwrap_or("");
        if file_name.is_empty() {
            continue;
        }

        let key = extract_path_after_prefix(&decoded_href, &base_href);

        if key.is_empty() {
            continue;
        }

        let size: i64 = extract_xml_value(&block, "getcontentlength")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let last_modified = extract_xml_value(&block, "getlastmodified");
        let etag = extract_xml_value(&block, "getetag").map(|s| s.trim_matches('"').to_string());

        files.push(crate::cloud_storage::StorageObjectMeta {
            key,
            etag,
            last_modified,
            size,
        });
    }
    Ok(files)
}

fn parse_propfind_responses_with_dirs(
    xml: &str,
    prefix: &str,
) -> Result<(Vec<crate::cloud_storage::StorageObjectMeta>, Vec<String>)> {
    let mut files = Vec::new();
    let mut subdirs = Vec::new();
    let response_blocks = split_xml_responses(xml);
    let base_href = {
        let base = if prefix.is_empty() { "/" } else { prefix };
        base.trim_matches('/').to_string()
    };

    for block in response_blocks {
        let lower_block = block.to_lowercase();
        let href = extract_xml_value(&block, "href").unwrap_or_default();
        if href.is_empty() {
            continue;
        }

        let decoded_href = url_decode(&href);

        if lower_block.contains("<d:collection") || lower_block.contains("<collection") {
            if decoded_href.ends_with('/') {
                let dir_name = decoded_href.trim_end_matches('/');
                let dir_part = dir_name.split('/').next_back().unwrap_or("");
                if !dir_part.is_empty()
                    && dir_part != base_href.split('/').next_back().unwrap_or("")
                {
                    let sub_prefix = if prefix.is_empty() {
                        dir_part.to_string()
                    } else {
                        format!("{}/{}", prefix, dir_part)
                    };
                    subdirs.push(sub_prefix);
                }
            }
            continue;
        }

        let file_name = decoded_href
            .split('/')
            .rfind(|s| !s.is_empty())
            .unwrap_or("");
        if file_name.is_empty() {
            continue;
        }

        let key = extract_path_after_prefix(&decoded_href, &base_href);
        if key.is_empty() {
            continue;
        }

        let size: i64 = extract_xml_value(&block, "getcontentlength")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let last_modified = extract_xml_value(&block, "getlastmodified");
        let etag = extract_xml_value(&block, "getetag").map(|s| s.trim_matches('"').to_string());

        files.push(crate::cloud_storage::StorageObjectMeta {
            key,
            etag,
            last_modified,
            size,
        });
    }
    Ok((files, subdirs))
}

fn extract_path_after_prefix(href: &str, prefix: &str) -> String {
    let prefix_lower = prefix.to_lowercase();
    let href_lower = href.to_lowercase();

    if let Some(pos) = href_lower.find(&prefix_lower) {
        let after = &href[pos + prefix.len()..];
        after.trim_start_matches('/').to_string()
    } else {
        href.trim_start_matches('/').to_string()
    }
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

pub fn extract_xml_value(xml: &str, tag_local_name: &str) -> Option<String> {
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

pub fn url_decode(s: &str) -> String {
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
        assert_eq!(documents_sync_root(), crate::storage_paths::documents_root());
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
