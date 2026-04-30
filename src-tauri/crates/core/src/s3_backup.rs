use chrono::Utc;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::error::{AxAgentError, Result};

/// S3-compatible storage configuration.
/// Supports AWS S3, MinIO, Cloudflare R2, Backblaze B2, etc.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3FileInfo {
    pub key: String,
    pub size: i64,
    pub last_modified: String,
    pub etag: String,
}

// ── S3 Client with AWS Signature V4 ──────────────────────────────────

pub struct S3Client {
    client: Client,
    config: S3Config,
}

impl S3Client {
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

    #[allow(dead_code)]
    fn base_url(&self) -> String {
        if self.config.use_path_style {
            format!(
                "{}/{}",
                self.config.endpoint.trim_end_matches('/'),
                self.config.bucket
            )
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

    /// Check connectivity by listing objects (prefix="")
    pub async fn check_connection(&self) -> Result<bool> {
        match self.list_files("", 1).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// List backup files in the bucket under the configured root.
    pub async fn list_files(&self, prefix: &str, max_keys: usize) -> Result<Vec<S3FileInfo>> {
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
        parse_s3_list_response(&body)
    }

    /// Upload a file to S3.
    pub async fn upload_file(&self, key: &str, data: &[u8], content_type: &str) -> Result<()> {
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
        Ok(())
    }

    /// Download a file from S3.
    pub async fn download_file(&self, key: &str) -> Result<Vec<u8>> {
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
            return Err(AxAgentError::Gateway(format!(
                "S3 download error: {}",
                body
            )));
        }

        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| AxAgentError::Gateway(format!("S3 read body error: {}", e)))
    }

    /// Delete a file from S3.
    pub async fn delete_file(&self, key: &str) -> Result<()> {
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

    // ── AWS Signature V4 Implementation ──────────────────────────────

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
        let now = Utc::now();
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
        header_map.insert(
            "Host",
            reqwest::header::HeaderValue::from_str(&host).unwrap(),
        );
        header_map.insert(
            "X-Amz-Date",
            reqwest::header::HeaderValue::from_str(&amz_date).unwrap(),
        );
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

fn parse_s3_list_response(xml: &str) -> Result<Vec<S3FileInfo>> {
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

        files.push(S3FileInfo {
            key,
            size,
            last_modified,
            etag,
        });
    }
    Ok(files)
}
