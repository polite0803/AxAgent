// SPDX-License-Identifier: AGPL-3.0-only

//! WebDAV 远程存储实现
//!
//! 基于 HTTP PROPFIND/PUT/DELETE 等方法实现 WebDAV 协议操作。

use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Response};

use axagent_harness::device_sync::{RemoteFileInfo, RemoteStorage, RemoteStorageConfig};

use crate::utils::{base64_encode, extract_xml_value, parse_http_date};

/// WebDAV 存储实现
pub struct WebdavStorage {
    config: RemoteStorageConfig,
    client: Client,
}

impl WebdavStorage {
    /// 创建 WebDAV 存储实例
    pub fn new(config: RemoteStorageConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create WebDAV client: {}", e))?;

        Ok(Self { config, client })
    }

    /// 构建认证头
    fn build_auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // Basic Auth
        let credentials = format!(
            "Basic {}",
            base64_encode(&format!(
                "{}:{}",
                self.config.credentials.access_key, self.config.credentials.secret_key
            ))
        );

        if let Ok(value) = HeaderValue::from_str(&credentials) {
            headers.insert(HeaderName::from_static("authorization"), value);
        }

        headers
    }

    /// 构建完整 URL
    fn build_url(&self, path: &str) -> String {
        let base = self.config.endpoint.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{}/{}", base, path)
    }

    /// 发送 WebDAV 请求
    async fn send_request(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        extra_headers: Option<HeaderMap>,
    ) -> Result<Response, String> {
        let url = self.build_url(path);
        let headers = self.build_auth_headers();

        let mut request = match method {
            "GET" => self.client.get(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PROPFIND" => self.client.request(
                reqwest::Method::from_bytes(b"PROPFIND").expect("WebDAV：PROPFIND 方法无效"),
                &url,
            ),
            "MKCOL" => self.client.request(
                reqwest::Method::from_bytes(b"MKCOL").expect("WebDAV：MKCOL 方法无效"),
                &url,
            ),
            _ => return Err(format!("Unsupported method: {}", method)),
        };

        for (key, value) in headers.iter() {
            request = request.header(key, value);
        }

        if let Some(extra) = extra_headers {
            for (key, value) in extra.iter() {
                request = request.header(key, value);
            }
        }

        if let Some(data) = body {
            request = request.body(data.to_vec());
        }

        request.send().await.map_err(|e| format!("WebDAV request failed: {}", e))
    }

    /// 解析 PROPFIND 响应
    fn parse_propfind_response(xml_body: &str) -> Vec<RemoteFileInfo> {
        let mut files = Vec::new();

        // 简单的 XML 解析（避免引入额外依赖）
        let entries: Vec<&str> = xml_body.split("<D:response>").collect();

        for entry in entries.iter().skip(1) {
            let path = extract_xml_value(entry, "D:href").unwrap_or_default();
            let display_name = extract_xml_value(entry, "D:displayname").unwrap_or_default();
            let content_length = extract_xml_value(entry, "D:getcontentlength")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            let last_modified = extract_xml_value(entry, "D:getlastmodified")
                .map(|v| parse_http_date(&v))
                .unwrap_or(0);
            let content_type = extract_xml_value(entry, "D:getcontenttype");

            if !path.is_empty() {
                // 去掉前缀路径
                let clean_path =
                    path.strip_prefix('/').map(|s| s.to_string()).unwrap_or_else(|| path.clone());

                files.push(RemoteFileInfo {
                    path: clean_path,
                    size: content_length,
                    last_modified,
                    content_type,
                });
            } else if !display_name.is_empty() {
                // 使用 display_name 作为备用
                files.push(RemoteFileInfo {
                    path: display_name,
                    size: content_length,
                    last_modified,
                    content_type,
                });
            }
        }

        files
    }
}

#[async_trait]
impl RemoteStorage for WebdavStorage {
    async fn upload(&self, path: &str, data: &[u8]) -> Result<(), String> {
        let full_path = format!("{}/{}", self.config.bucket_or_path, path);

        // 确保目录存在
        if let Some(dir) = path.rsplit_once('/') {
            let dir_path = format!("{}/{}", self.config.bucket_or_path, dir.0);
            let _ = self.send_request("MKCOL", &dir_path, None, None).await; // 忽略错误（目录可能已存在）
        }

        self.send_request("PUT", &full_path, Some(data), None).await?;

        Ok(())
    }

    async fn download(&self, path: &str) -> Result<Vec<u8>, String> {
        let full_path = format!("{}/{}", self.config.bucket_or_path, path);

        let response = self.send_request("GET", &full_path, None, None).await?;

        if !response.status().is_success() {
            return Err(format!("Download failed: HTTP {}", response.status()));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read response body: {}", e))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<RemoteFileInfo>, String> {
        let full_path = format!("{}/{}", self.config.bucket_or_path, prefix);

        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("depth"), HeaderValue::from_static("1"));

        let response = self.send_request("PROPFIND", &full_path, None, Some(headers)).await?;

        if !response.status().is_success() {
            return Err(format!("List failed: HTTP {}", response.status()));
        }

        let body = response.text().await.map_err(|e| format!("Failed to read response: {}", e))?;

        Ok(Self::parse_propfind_response(&body))
    }

    async fn delete(&self, path: &str) -> Result<(), String> {
        let full_path = format!("{}/{}", self.config.bucket_or_path, path);

        self.send_request("DELETE", &full_path, None, None).await?;

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, String> {
        let response = self.send_request("PROPFIND", &self.config.bucket_or_path, None, None).await;

        match response {
            Ok(resp) => Ok(resp.status().is_success() || resp.status() == 405),
            Err(e) => Err(format!("Health check failed: {}", e)),
        }
    }

    fn config(&self) -> &RemoteStorageConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_propfind_response() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
    <D:href>/files/test.txt</D:href>
    <D:displayname>test.txt</D:displayname>
    <D:getcontentlength>1024</D:getcontentlength>
    <D:getlastmodified>Mon, 01 Jan 2024 00:00:00 GMT</D:getlastmodified>
    <D:getcontenttype>text/plain</D:getcontenttype>
  </D:response>
</D:multistatus>"#;

        let files = WebdavStorage::parse_propfind_response(xml);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "files/test.txt");
        assert_eq!(files[0].size, 1024);
        assert_eq!(files[0].content_type, Some("text/plain".to_string()));
    }
}
