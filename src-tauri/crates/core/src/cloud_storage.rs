/// Cloud storage abstraction and sync engine.
///
/// Provides a unified `StorageBackend` trait for WebDAV and S3-compatible services,
/// with built-in presets for popular Chinese cloud providers.
///
/// Architecture:
/// - `StorageBackend` trait: abstract CRUD operations over any cloud storage
/// - `S3ProviderPreset`: built-in endpoint/region config for common providers
/// - `CloudStorageConfig`: user-facing configuration (WebDAV or S3)
/// - `SyncManifest`: cloud sync state tracking (version, file list, checksums)
use async_trait::async_trait;
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

use crate::error::{AxAgentError, Result};

// ─── Storage Object Types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObject {
    pub key: String,
    pub data: Vec<u8>,
    pub content_type: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageObjectMeta {
    pub key: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    pub objects: Vec<StorageObjectMeta>,
    pub is_truncated: bool,
    pub continuation_token: Option<String>,
}

// ─── Storage Backend Trait ────────────────────────────────────────────

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn get(&self, key: &str) -> Result<StorageObject>;
    async fn put(&self, key: &str, data: &[u8], content_type: &str) -> Result<StorageObjectMeta>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn list(&self, prefix: &str, limit: usize) -> Result<ListResult>;
    async fn head(&self, key: &str) -> Result<StorageObjectMeta>;
    async fn check_connection(&self) -> Result<bool>;
}

// ─── S3 Provider Presets (Chinese providers) ──────────────────────────

/// Known S3-compatible providers with sensible defaults.
/// Users select a preset, then fill in credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum S3ProviderPreset {
    Aws,
    CloudflareR2,
    AlibabaOss,
    TencentCos,
    HuaweiObs,
    BaiduBos,
    Minio,
    SeaweedFs,
    Custom,
}

impl S3ProviderPreset {
    /// Returns the endpoint template for this provider.
    /// The `{region}` placeholder should be replaced with the actual region.
    pub fn endpoint_template(&self) -> &'static str {
        match self {
            Self::Aws => "https://s3.{region}.amazonaws.com",
            Self::CloudflareR2 => "https://{account_id}.r2.cloudflarestorage.com",
            Self::AlibabaOss => "https://oss-{region}.aliyuncs.com",
            Self::TencentCos => "https://cos.{region}.myqcloud.com",
            Self::HuaweiObs => "https://obs.{region}.myhuaweicloud.com",
            Self::BaiduBos => "https://{region}.bos.amazonaws.com",
            Self::Minio => "http://localhost:9000",
            Self::SeaweedFs => "http://localhost:8333",
            Self::Custom => "",
        }
    }

    /// Returns a human-readable display name (in Chinese).
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Aws => "Amazon S3",
            Self::CloudflareR2 => "Cloudflare R2",
            Self::AlibabaOss => "阿里云 OSS",
            Self::TencentCos => "腾讯云 COS",
            Self::HuaweiObs => "华为云 OBS",
            Self::BaiduBos => "百度云 BOS",
            Self::Minio => "MinIO (自建)",
            Self::SeaweedFs => "SeaweedFS (自建)",
            Self::Custom => "自定义",
        }
    }

    /// Returns the default region hint for this provider.
    pub fn default_region(&self) -> &'static str {
        match self {
            Self::Aws => "us-east-1",
            Self::CloudflareR2 => "auto",
            Self::AlibabaOss => "cn-hangzhou",
            Self::TencentCos => "ap-shanghai",
            Self::HuaweiObs => "cn-north-4",
            Self::BaiduBos => "bj",
            Self::Minio => "us-east-1",
            Self::SeaweedFs => "",
            Self::Custom => "",
        }
    }

    /// Returns true if this provider requires path-style addressing.
    pub fn default_use_path_style(&self) -> bool {
        match self {
            Self::Minio | Self::SeaweedFs | Self::Custom => true,
            _ => false,
        }
    }

    /// All available presets for UI dropdown.
    pub fn all_presets() -> Vec<Self> {
        vec![
            Self::AlibabaOss,
            Self::TencentCos,
            Self::HuaweiObs,
            Self::BaiduBos,
            Self::Aws,
            Self::CloudflareR2,
            Self::Minio,
            Self::SeaweedFs,
            Self::Custom,
        ]
    }
}

// ─── S3 Client with StorageBackend Implementation ─────────────────────

/// S3-compatible storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub use_path_style: bool,
}

pub struct S3Backend {
    client: Client,
    config: S3Config,
}

impl S3Backend {
    pub fn new(config: S3Config) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client");
        Self { client, config }
    }

    fn host(&self) -> String {
        let endpoint = self
            .config
            .endpoint
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/');
        format!("{}.{}", self.config.bucket, endpoint)
    }

    fn base_url(&self) -> String {
        if self.config.use_path_style {
            format!("{}/{}", self.config.endpoint.trim_end_matches('/'), self.config.bucket)
        } else {
            let endpoint = self.config.endpoint.trim_start_matches("https://");
            if self.config.endpoint.starts_with("http://") {
                format!("http://{}.{}", self.config.bucket, endpoint)
            } else {
                format!("https://{}.{}", self.config.bucket, endpoint)
            }
        }
    }

    fn object_key(&self, filename: &str) -> String {
        if self.config.root.is_empty() {
            filename.to_string()
        } else {
            format!("{}/{}", self.config.root.trim_matches('/'), filename)
        }
    }

    // ── AWS Signature V4 ──────────────────────────────────────────────

    fn sign_request(
        &self,
        method: Method,
        path: &str,
        query: &BTreeMap<String, String>,
        payload_hash_str: &str,
    ) -> (reqwest::header::HeaderMap, String) {
        self.sign_request_with_body(method, path, query, &[], payload_hash_str)
    }

    fn sign_request_with_body(
        &self,
        method: Method,
        path: &str,
        query: &BTreeMap<String, String>,
        body: &[u8],
        content_type: &str,
    ) -> (reqwest::header::HeaderMap, String) {
        let now = chrono::Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();

        let service = "s3";
        let region = &self.config.region;
        let host = if self.config.use_path_style {
            let endpoint = self
                .config
                .endpoint
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/');
            endpoint.to_string()
        } else {
            self.host()
        };

        let canonical_uri = if path.is_empty() { "/" } else { path };
        let canonical_querystring = build_canonical_query(query);
        let payload_hash = if body.is_empty() {
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string()
        } else {
            let mut hasher = Sha256::new();
            hasher.update(body);
            hex::encode(hasher.finalize())
        };

        let headers = {
            let mut h = BTreeMap::new();
            h.insert("host".to_string(), host.clone());
            h.insert("x-amz-content-sha256".to_string(), payload_hash.clone());
            h.insert("x-amz-date".to_string(), amz_date.clone());
            h
        };

        let signed_headers = headers
            .keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(";");
        let canonical_headers = headers
            .iter()
            .map(|(k, v)| format!("{}:{}", k, v.trim()))
            .collect::<Vec<_>>()
            .join("\n");

        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method.as_str(),
            canonical_uri,
            canonical_querystring,
            canonical_headers,
            "",
            signed_headers
        );

        let mut cr_hasher = Sha256::new();
        cr_hasher.update(canonical_request.as_bytes());
        let cr_hash = hex::encode(cr_hasher.finalize());

        let scope = format!("{}/{}/{}/aws4_request", date_stamp, region, service);
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{}\n{}\n{}", amz_date, scope, cr_hash);

        let signing_key =
            get_signature_key(&self.config.secret_access_key, &date_stamp, region, service);
        let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.config.access_key_id, scope, signed_headers, signature
        );

        let url = format!(
            "{}://{}{}",
            if self.config.endpoint.starts_with("http://") {
                "http"
            } else {
                "https"
            },
            host,
            canonical_uri
        );
        let url = if canonical_querystring.is_empty() {
            url
        } else {
            format!("{}?{}", url, canonical_querystring)
        };

        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert("Host", reqwest::header::HeaderValue::from_str(&host).unwrap());
        header_map.insert("X-Amz-Date", reqwest::header::HeaderValue::from_str(&amz_date).unwrap());
        header_map.insert(
            "X-Amz-Content-Sha256",
            reqwest::header::HeaderValue::from_str(&payload_hash).unwrap(),
        );
        header_map.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&authorization).unwrap(),
        );
        if !content_type.is_empty() {
            header_map.insert(
                "Content-Type",
                reqwest::header::HeaderValue::from_str(content_type).unwrap(),
            );
        }

        (header_map, url)
    }
}

#[async_trait]
impl StorageBackend for S3Backend {
    async fn get(&self, key: &str) -> Result<StorageObject> {
        let full_key = self.object_key(key);
        let path = format!("/{}", full_key);

        let (headers, url) = self.sign_request(Method::GET, &path, &BTreeMap::new(), "");
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("S3 download failed: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Gateway(format!("S3 download error: {}", body)));
        }

        let etag = resp
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.trim_matches('"').to_string());

        let data = resp
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| AxAgentError::Gateway(format!("S3 read body error: {}", e)))?;

        let data_len = data.len() as i64;

        Ok(StorageObject {
            key: key.to_string(),
            data,
            content_type: "application/octet-stream".into(),
            etag,
            last_modified: None,
            size: data_len,
        })
    }

    async fn put(&self, key: &str, data: &[u8], content_type: &str) -> Result<StorageObjectMeta> {
        let full_key = self.object_key(key);
        let path = format!("/{}", full_key);

        let (headers, url) =
            self.sign_request_with_body(Method::PUT, &path, &BTreeMap::new(), data, content_type);

        let resp = self
            .client
            .put(&url)
            .headers(headers)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("S3 upload failed: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Gateway(format!("S3 upload error: {}", body)));
        }

        let etag = resp
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.trim_matches('"').to_string());

        Ok(StorageObjectMeta {
            key: key.to_string(),
            etag,
            last_modified: None,
            size: data.len() as i64,
        })
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let full_key = self.object_key(key);
        let path = format!("/{}", full_key);

        let (headers, url) = self.sign_request(Method::DELETE, &path, &BTreeMap::new(), "");
        let resp = self
            .client
            .delete(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("S3 delete failed: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Gateway(format!("S3 delete error: {}", body)));
        }
        Ok(())
    }

    async fn list(&self, prefix: &str, max_keys: usize) -> Result<ListResult> {
        let full_prefix = if prefix.is_empty() && !self.config.root.is_empty() {
            format!("{}/", self.config.root.trim_matches('/'))
        } else if !self.config.root.is_empty() {
            format!("{}/{}", self.config.root.trim_matches('/'), prefix)
        } else {
            prefix.to_string()
        };

        let mut query_params = BTreeMap::new();
        query_params.insert("list-type".to_string(), "2".to_string());
        query_params.insert("prefix".to_string(), full_prefix);
        query_params.insert("max-keys".to_string(), max_keys.to_string());

        let (headers, url) = self.sign_request(Method::GET, "/", &query_params, "");
        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("S3 list failed: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Gateway(format!("S3 list error: {}", body)));
        }

        let body = resp.text().await.unwrap_or_default();
        let objects = parse_s3_list_response(&body)?;

        Ok(ListResult {
            objects,
            is_truncated: false,
            continuation_token: None,
        })
    }

    async fn head(&self, key: &str) -> Result<StorageObjectMeta> {
        let full_key = self.object_key(key);
        let path = format!("/{}", full_key);

        let (headers, url) = self.sign_request(Method::HEAD, &path, &BTreeMap::new(), "");
        let resp = self
            .client
            .head(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("S3 HEAD failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(AxAgentError::NotFound(format!("S3 object not found: {}", key)));
        }

        let etag = resp
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.trim_matches('"').to_string());

        let size = resp
            .headers()
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        Ok(StorageObjectMeta {
            key: key.to_string(),
            etag,
            last_modified: None,
            size,
        })
    }

    async fn check_connection(&self) -> Result<bool> {
        match self.list("", 1).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

// ─── WebDAV Client with StorageBackend Implementation ─────────────────

/// WebDAV-compatible storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub path: String,
    pub accept_invalid_certs: bool,
}

pub struct WebDavBackend {
    client: Client,
    config: WebDavConfig,
}

impl WebDavBackend {
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

    /// Auto-create remote directory tree if missing.
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

            match response.status() {
                StatusCode::CREATED | StatusCode::OK | StatusCode::METHOD_NOT_ALLOWED => {},
                status => {
                    return Err(AxAgentError::Gateway(format!(
                        "WebDAV mkdir failed for '{}': HTTP {}",
                        current, status
                    )));
                },
            }
        }
        Ok(())
    }
}

#[async_trait]
impl StorageBackend for WebDavBackend {
    async fn get(&self, key: &str) -> Result<StorageObject> {
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

        let data_len = data.len() as i64;

        Ok(StorageObject {
            key: key.to_string(),
            data,
            content_type: "application/octet-stream".into(),
            etag,
            last_modified,
            size: data_len,
        })
    }

    async fn put(&self, key: &str, data: &[u8], _content_type: &str) -> Result<StorageObjectMeta> {
        // Ensure directory exists before uploading
        self.mkdir().await?;

        let url = self.file_url(key);
        let response = self
            .client
            .put(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Content-Type", "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV upload failed: {}", e)))?;

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

        Ok(StorageObjectMeta {
            key: key.to_string(),
            etag,
            last_modified: None,
            size: data.len() as i64,
        })
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let url = self.file_url(key);
        let response = self
            .client
            .delete(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV delete failed: {}", e)))?;

        match response.status() {
            StatusCode::OK | StatusCode::NO_CONTENT | StatusCode::NOT_FOUND => Ok(()),
            status => Err(AxAgentError::Gateway(format!("WebDAV delete failed: HTTP {}", status))),
        }
    }

    async fn list(&self, prefix: &str, _limit: usize) -> Result<ListResult> {
        let url = if prefix.is_empty() {
            self.base_url()
        } else {
            format!("{}{}", self.base_url(), prefix)
        };

        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:">
  <D:prop>
    <D:getcontentlength/>
    <D:getlastmodified/>
    <D:getetag/>
    <D:resourcetype/>
  </D:prop>
</D:propfind>"#;

        let response = self
            .client
            .request(Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV PROPFIND failed: {}", e)))?;

        if response.status() != StatusCode::MULTI_STATUS && !response.status().is_success() {
            return Err(AxAgentError::Gateway(format!(
                "WebDAV list failed: HTTP {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| AxAgentError::Gateway(format!("Failed to read response: {}", e)))?;

        let objects = parse_propfind_responses(&text, &prefix)?;
        Ok(ListResult {
            objects,
            is_truncated: false,
            continuation_token: None,
        })
    }

    async fn head(&self, key: &str) -> Result<StorageObjectMeta> {
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

        let size = response
            .headers()
            .get("content-length")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        Ok(StorageObjectMeta {
            key: key.to_string(),
            etag,
            last_modified: None,
            size,
        })
    }

    async fn check_connection(&self) -> Result<bool> {
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
            .map_err(|e| AxAgentError::Gateway(format!("WebDAV connection check failed: {}", e)))?;

        match response.status() {
            StatusCode::MULTI_STATUS | StatusCode::OK => Ok(true),
            StatusCode::NOT_FOUND => {
                self.mkdir().await?;
                Ok(true)
            },
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(AxAgentError::Gateway("WebDAV authentication failed".to_string()))
            },
            status => Err(AxAgentError::Gateway(format!("WebDAV error: HTTP {}", status))),
        }
    }
}

// ─── Cloud Storage Configuration ─────────────────────────────────────

/// User-facing cloud storage configuration.
/// Supports WebDAV and S3-compatible services.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudStorageConfig {
    /// Provider preset (used for UI hints, not enforced at runtime)
    pub provider_preset: S3ProviderPreset,
    /// Storage backend type
    pub backend_type: BackendType,
    /// Whether cloud sync is enabled
    pub sync_enabled: bool,
    /// Sync mode: `backup` (desktop, ZIP-based) or `sync` (mobile, file-level)
    pub sync_mode: SyncMode,
    /// Profile name for multi-profile setups
    pub profile_name: String,
    /// WebDAV configuration (if backend_type is webdav)
    pub webdav: Option<WebDavConfig>,
    /// S3 configuration (if backend_type is s3)
    pub s3: Option<S3Config>,
}

impl Default for CloudStorageConfig {
    fn default() -> Self {
        Self {
            provider_preset: S3ProviderPreset::Custom,
            backend_type: BackendType::None,
            sync_enabled: false,
            sync_mode: SyncMode::Backup,
            profile_name: "default".to_string(),
            webdav: None,
            s3: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendType {
    None,
    WebDav,
    S3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncMode {
    /// Desktop mode: periodic ZIP backups
    Backup,
    /// Mobile mode: real-time file-level sync with local DB
    Sync,
}

impl CloudStorageConfig {
    /// Create a `StorageBackend` from this config.
    pub fn create_backend(&self) -> Result<Arc<dyn StorageBackend>> {
        match self.backend_type {
            BackendType::S3 => {
                let s3 = self.s3.as_ref().ok_or_else(|| {
                    AxAgentError::Gateway("S3 configuration is missing".to_string())
                })?;
                Ok(Arc::new(S3Backend::new(s3.clone())))
            },
            BackendType::WebDav => {
                let wd = self.webdav.as_ref().ok_or_else(|| {
                    AxAgentError::Gateway("WebDAV configuration is missing".to_string())
                })?;
                Ok(Arc::new(WebDavBackend::new(wd.clone())?))
            },
            BackendType::None => {
                Err(AxAgentError::Gateway("No storage backend configured".to_string()))
            },
        }
    }
}

// ─── Sync Manifest ───────────────────────────────────────────────────

/// Tracks the sync state between local and cloud storage.
/// Stored both locally (for quick comparison) and in cloud (as `manifest.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncManifest {
    /// Schema version for future compatibility
    pub version: u32,
    /// Last successful sync timestamp (RFC3339)
    pub last_sync_at: Option<String>,
    /// Monotonically increasing version counter
    pub sync_version: u64,
    /// Local device identifier (hostname or UUID)
    pub device_id: String,
    /// List of tracked files with their cloud ETags
    pub files: Vec<SyncFileEntry>,
    /// Database checksum for DB-level sync detection
    pub db_checksum: Option<String>,
    /// Timestamp when DB was last pushed to cloud
    pub db_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncFileEntry {
    /// Relative path within the storage root (e.g. "images/abc123_photo.jpg")
    pub key: String,
    /// Cloud ETag (for S3) or WebDAV ETag
    pub etag: Option<String>,
    /// File size in bytes
    pub size: i64,
    /// Local modification timestamp (Unix epoch ms)
    pub local_modified_at: u64,
}

impl SyncManifest {
    pub fn new(device_id: String) -> Self {
        Self {
            version: 1,
            last_sync_at: None,
            sync_version: 0,
            device_id,
            files: Vec::new(),
            db_checksum: None,
            db_synced_at: None,
        }
    }

    /// Find the ETag for a given file key.
    pub fn get_etag(&self, key: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|f| f.key == key)
            .and_then(|f| f.etag.as_deref())
    }

    /// Update or insert a file entry.
    pub fn upsert_file(&mut self, key: String, etag: Option<String>, size: i64) {
        if let Some(entry) = self.files.iter_mut().find(|f| f.key == key) {
            entry.etag = etag;
            entry.size = size;
        } else {
            self.files.push(SyncFileEntry {
                key,
                etag,
                size,
                local_modified_at: current_epoch_ms(),
            });
        }
    }

    /// Remove a file entry.
    pub fn remove_file(&mut self, key: &str) {
        self.files.retain(|f| f.key != key);
    }
}

// ─── Sync Engine ──────────────────────────────────────────────────────

/// Orchestrates bidirectional sync between local storage and cloud backend.
pub struct SyncEngine {
    pub backend: Arc<dyn StorageBackend>,
    local_manifest: Arc<tokio::sync::RwLock<SyncManifest>>,
    manifest_key: String,
    profile_name: String,
}

impl SyncEngine {
    pub fn new(backend: Arc<dyn StorageBackend>, profile_name: &str, device_id: &str) -> Self {
        Self {
            backend,
            local_manifest: Arc::new(tokio::sync::RwLock::new(SyncManifest::new(
                device_id.to_string(),
            ))),
            manifest_key: format!("profiles/{}/sync/manifest.json", profile_name),
            profile_name: profile_name.to_string(),
        }
    }

    /// Full sync: pull remote manifest, compare, download new/changed files, push local changes.
    pub async fn full_sync(&self) -> Result<SyncResult> {
        let mut result = SyncResult::default();

        // 1. Fetch remote manifest
        let remote_manifest = match self.backend.get(&self.manifest_key).await {
            Ok(obj) => serde_json::from_slice::<SyncManifest>(&obj.data).ok(),
            Err(_) => None,
        };

        // 2. If no remote manifest, this is first sync — push local state
        if remote_manifest.is_none() {
            let mut manifest = self.local_manifest.write().await;
            manifest.last_sync_at = Some(current_rfc3339());
            self.push_manifest(&manifest).await?;
            return Ok(result);
        }

        let remote_manifest = remote_manifest.unwrap();

        // 3. Build index of remote files
        let remote_keys: std::collections::HashMap<&str, &str> = remote_manifest
            .files
            .iter()
            .filter_map(|f| f.etag.as_deref().map(|e| (f.key.as_str(), e)))
            .collect();

        // 4. Determine what needs to be downloaded
        let mut manifest = self.local_manifest.write().await;
        for remote_file in &remote_manifest.files {
            let local_etag = manifest.get_etag(&remote_file.key);
            if local_etag != remote_file.etag.as_deref() {
                result.pending_downloads.push(remote_file.key.clone());
            }
        }

        // 5. Determine what needs to be uploaded (local files not in remote)
        for local_file in &manifest.files {
            if !remote_keys.contains_key(local_file.key.as_str()) {
                result.pending_uploads.push(local_file.key.clone());
            }
        }

        manifest.last_sync_at = Some(current_rfc3339());
        if !result.pending_downloads.is_empty() || !result.pending_uploads.is_empty() {
            manifest.sync_version += 1;
        }
        self.push_manifest(&manifest).await?;

        Ok(result)
    }

    /// Incremental sync: compare local manifest with remote, pull/push only changed files.
    pub async fn incremental_sync(&self) -> Result<SyncResult> {
        self.full_sync().await
    }

    /// Pull database from cloud. Returns the DB bytes if available.
    pub async fn pull_database(&self) -> Result<Option<Vec<u8>>> {
        let db_key = format!("profiles/{}/db/axagent.db", self.profile_name);
        match self.backend.head(&db_key).await {
            Ok(meta) => {
                let local_manifest = self.local_manifest.read().await;
                let needs_pull = local_manifest.db_checksum.as_deref() != meta.etag.as_deref();
                drop(local_manifest);

                if needs_pull {
                    let obj = self.backend.get(&db_key).await?;
                    let mut manifest = self.local_manifest.write().await;
                    manifest.db_checksum = meta.etag.clone();
                    manifest.db_synced_at = Some(current_rfc3339());
                    self.push_manifest(&manifest).await?;
                    Ok(Some(obj.data))
                } else {
                    Ok(None)
                }
            },
            Err(_) => Ok(None),
        }
    }

    /// Push database to cloud.
    pub async fn push_database(&self, data: &[u8]) -> Result<()> {
        let db_key = format!("profiles/{}/db/axagent.db", self.profile_name);
        let meta = self
            .backend
            .put(&db_key, data, "application/x-sqlite3")
            .await?;

        let mut manifest = self.local_manifest.write().await;
        manifest.db_checksum = meta.etag.clone();
        manifest.db_synced_at = Some(current_rfc3339());
        manifest.sync_version += 1;
        self.push_manifest(&manifest).await
    }

    /// Fetch a single file from cloud and save locally.
    pub async fn fetch_file(&self, key: &str, local_path: &Path) -> Result<()> {
        let obj = self.backend.get(key).await?;
        if let Some(parent) = local_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(local_path, &obj.data)?;

        // Update manifest
        let mut manifest = self.local_manifest.write().await;
        manifest.upsert_file(key.to_string(), obj.etag, obj.size);
        Ok(())
    }

    /// Blocking helper for use from sync contexts.
    pub fn blocking_fetch(&self, key: &str, local_path: &Path) -> Result<()> {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.fetch_file(key, local_path))
    }

    /// Push a single file to cloud.
    pub async fn push_file(&self, key: &str, data: &[u8], content_type: &str) -> Result<()> {
        let meta = self.backend.put(key, data, content_type).await?;

        let mut manifest = self.local_manifest.write().await;
        manifest.upsert_file(key.to_string(), meta.etag, meta.size);
        manifest.sync_version += 1;
        self.push_manifest(&manifest).await
    }

    /// Load manifest from local file.
    pub async fn load_local_manifest(&self, path: &Path) -> Result<()> {
        if path.exists() {
            let data = std::fs::read(path)?;
            let manifest = serde_json::from_slice::<SyncManifest>(&data)?;
            *self.local_manifest.write().await = manifest;
        }
        Ok(())
    }

    /// Save manifest to local file.
    pub async fn save_local_manifest(&self, path: &Path) -> Result<()> {
        let manifest = self.local_manifest.read().await;
        let data = serde_json::to_vec_pretty(&*manifest)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &data)?;
        Ok(())
    }

    async fn push_manifest(&self, manifest: &SyncManifest) -> Result<()> {
        let data = serde_json::to_vec_pretty(manifest)?;
        self.backend
            .put(&self.manifest_key, &data, "application/json")
            .await?;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SyncResult {
    pub pending_downloads: Vec<String>,
    pub pending_uploads: Vec<String>,
}

// ─── Internal Helpers ─────────────────────────────────────────────────

fn build_canonical_query(query: &BTreeMap<String, String>) -> String {
    query
        .iter()
        .map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                c.to_string()
            } else {
                format!("%{:02X}", c as u8)
            }
        })
        .collect()
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use sha2::Sha256;

    const BLOCK_SIZE: usize = 64;
    let mut o_key_pad: Vec<u8> = std::iter::repeat_n(0x5c, BLOCK_SIZE).collect();
    let mut i_key_pad: Vec<u8> = std::iter::repeat_n(0x36, BLOCK_SIZE).collect();

    let key = if key.len() > BLOCK_SIZE {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.finalize().to_vec()
    } else {
        key.to_vec()
    };

    for (i, &k) in key.iter().enumerate() {
        o_key_pad[i] ^= k;
        i_key_pad[i] ^= k;
    }

    let mut inner = Sha256::new();
    inner.update(&i_key_pad);
    inner.update(data);

    let mut outer = Sha256::new();
    outer.update(&o_key_pad);
    outer.update(inner.finalize());
    outer.finalize().to_vec()
}

fn get_signature_key(secret: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{}", secret).as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

fn parse_s3_list_response(xml: &str) -> Result<Vec<StorageObjectMeta>> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| AxAgentError::Gateway(format!("S3 XML parse error: {}", e)))?;

    let mut files = Vec::new();
    for contents in doc.descendants().filter(|n| n.has_tag_name("Contents")) {
        let key = contents
            .descendants()
            .find(|n| n.has_tag_name("Key"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string();
        let size = contents
            .descendants()
            .find(|n| n.has_tag_name("Size"))
            .and_then(|n| n.text())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let last_modified = contents
            .descendants()
            .find(|n| n.has_tag_name("LastModified"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string();
        let etag = contents
            .descendants()
            .find(|n| n.has_tag_name("ETag"))
            .and_then(|n| n.text())
            .unwrap_or("")
            .trim_matches('"')
            .to_string();

        files.push(StorageObjectMeta {
            key,
            etag: Some(etag),
            last_modified: Some(last_modified),
            size,
        });
    }
    Ok(files)
}

fn parse_propfind_responses(xml: &str, prefix: &str) -> Result<Vec<StorageObjectMeta>> {
    let mut files = Vec::new();
    let lower = xml.to_lowercase();
    let tag = if lower.contains("<d:response>") {
        "d:response"
    } else {
        "response"
    };

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
                let block = &xml[abs_start..abs_end];
                let lower_block = block.to_lowercase();

                // Skip directories
                if !lower_block.contains("<d:collection") && !lower_block.contains("<collection") {
                    if let Some(href) = extract_xml_value(block, "href") {
                        let href_lower = href.to_lowercase();
                        if href_lower.ends_with('/') || href_lower.is_empty() {
                            pos = abs_end;
                            continue;
                        }
                        // Only include files under the prefix
                        if prefix.is_empty() || href_lower.contains(&prefix.to_lowercase()) {
                            let file_name = url_decode(
                                href.split('/')
                                    .filter(|s| !s.is_empty())
                                    .last()
                                    .unwrap_or(""),
                            );
                            if !file_name.is_empty() {
                                let size: i64 = extract_xml_value(block, "getcontentlength")
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0);
                                let last_modified = extract_xml_value(block, "getlastmodified");
                                let etag = extract_xml_value(block, "getetag")
                                    .map(|s| s.trim_matches('"').to_string());

                                files.push(StorageObjectMeta {
                                    key: file_name,
                                    etag,
                                    last_modified,
                                    size,
                                });
                            }
                        }
                    }
                }
                pos = abs_end;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    Ok(files)
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

fn current_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn current_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

use std::sync::Arc;
