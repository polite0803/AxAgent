// SPDX-License-Identifier: AGPL-3.0-only

//! S3 兼容存储实现
//!
//! 使用 HTTP 请求实现 S3 协议的基本操作（PUT/GET/DELETE/LIST），
//! 支持 AWS S3、MinIO、Cloudflare R2 等兼容存储服务。

use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Response};

use axagent_harness::device_sync::{RemoteFileInfo, RemoteStorage, RemoteStorageConfig};

use crate::utils::{extract_xml_value, guess_content_type, parse_iso8601_date};

/// S3 存储实现
pub struct S3Storage {
    config: RemoteStorageConfig,
    client: Client,
}

impl S3Storage {
    /// 创建 S3 存储实例
    pub fn new(config: RemoteStorageConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create S3 client: {}", e))?;

        Ok(Self { config, client })
    }

    /// 构建 AWS 认证头（简化版）
    fn build_auth_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        let auth_value = format!(
            "AWS_ACCESS_KEY_ID={};AWS_SECRET_ACCESS_KEY={}",
            self.config.credentials.access_key, self.config.credentials.secret_key
        );

        if let Ok(value) = HeaderValue::from_str(&auth_value) {
            headers.insert(HeaderName::from_static("x-amz-auth"), value);
        }

        headers.insert(
            HeaderName::from_static("x-amz-content-sha256"),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let now = chrono::Utc::now();
        let date_str = now.format("%Y%m%dT%H%M%SZ").to_string();
        if let Ok(value) = HeaderValue::from_str(&date_str) {
            headers.insert(HeaderName::from_static("x-amz-date"), value);
        }

        headers
    }

    /// 构建 S3 请求 URL（路径风格）
    fn build_url(&self, path: &str) -> String {
        let endpoint = self.config.endpoint.trim_end_matches('/');
        let bucket = &self.config.bucket_or_path;
        let key = path.trim_start_matches('/');
        format!("{}/{}/{}", endpoint, bucket, key)
    }

    /// 发送 S3 请求
    async fn send_request(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
        content_type: Option<&str>,
    ) -> Result<Response, String> {
        let url = self.build_url(path);
        let headers = Self::build_auth_headers(self);

        let mut request = match method {
            "GET" => self.client.get(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => return Err(format!("Unsupported S3 method: {}", method)),
        };

        for (key, value) in headers.iter() {
            request = request.header(key, value);
        }

        if let Some(ct) = content_type
            && let Ok(value) = HeaderValue::from_str(ct)
        {
            request = request.header(HeaderName::from_static("content-type"), value);
        }

        if let Some(data) = body {
            request = request.body(data.to_vec());
        }

        request.send().await.map_err(|e| format!("S3 request failed: {}", e))
    }

    /// 解析 S3 ListObjects 响应
    fn parse_list_response(xml_body: &str) -> Vec<RemoteFileInfo> {
        let mut files = Vec::new();

        let contents_blocks: Vec<&str> = xml_body.split("<Contents>").collect();

        for block in contents_blocks.iter().skip(1) {
            let key = extract_xml_value(block, "Key").unwrap_or_default();
            let size =
                extract_xml_value(block, "Size").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
            let last_modified = extract_xml_value(block, "LastModified")
                .map(|v| parse_iso8601_date(&v))
                .unwrap_or(0);
            let content_type = extract_xml_value(block, "ContentType");

            if !key.is_empty() {
                files.push(RemoteFileInfo { path: key, size, last_modified, content_type });
            }
        }

        files
    }
}

#[async_trait]
impl RemoteStorage for S3Storage {
    async fn upload(&self, path: &str, data: &[u8]) -> Result<(), String> {
        let content_type = guess_content_type(path);

        let response = self.send_request("PUT", path, Some(data), Some(content_type)).await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("S3 upload failed: HTTP {} - {}", status, body));
        }

        Ok(())
    }

    async fn download(&self, path: &str) -> Result<Vec<u8>, String> {
        let response = self.send_request("GET", path, None, None).await?;

        if !response.status().is_success() {
            return Err(format!("S3 download failed: HTTP {}", response.status()));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read S3 response: {}", e))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<RemoteFileInfo>, String> {
        let url = format!(
            "{}/{}/?prefix={}&list-type=2",
            self.config.endpoint.trim_end_matches('/'),
            self.config.bucket_or_path,
            prefix
        );

        let headers = Self::build_auth_headers(self);

        let mut request = self.client.get(&url);
        for (key, value) in headers.iter() {
            request = request.header(key, value);
        }

        let response =
            request.send().await.map_err(|e| format!("S3 list request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("S3 list failed: HTTP {} - {}", status, body));
        }

        let body =
            response.text().await.map_err(|e| format!("Failed to read S3 list response: {}", e))?;

        Ok(Self::parse_list_response(&body))
    }

    async fn delete(&self, path: &str) -> Result<(), String> {
        let response = self.send_request("DELETE", path, None, None).await?;

        if !response.status().is_success() && response.status() != 204 {
            return Err(format!("S3 delete failed: HTTP {}", response.status()));
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<bool, String> {
        let url = format!(
            "{}/{}/",
            self.config.endpoint.trim_end_matches('/'),
            self.config.bucket_or_path
        );

        let headers = Self::build_auth_headers(self);

        let mut request = self.client.get(&url);
        for (key, value) in headers.iter() {
            request = request.header(key, value);
        }

        let response =
            request.send().await.map_err(|e| format!("S3 health check failed: {}", e))?;

        Ok(response.status().is_success()
            || response.status() == 204
            || response.status() == 301
            || response.status() == 403)
    }

    fn config(&self) -> &RemoteStorageConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list_response() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
  <Contents>
    <Key>sync/device1/changes.json</Key>
    <LastModified>2024-01-01T00:00:00Z</LastModified>
    <Size>2048</Size>
    <ContentType>application/json</ContentType>
  </Contents>
  <Contents>
    <Key>sync/device2/state.json</Key>
    <LastModified>2024-01-02T12:00:00Z</LastModified>
    <Size>1024</Size>
  </Contents>
</ListBucketResult>"#;

        let files = S3Storage::parse_list_response(xml);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "sync/device1/changes.json");
        assert_eq!(files[0].size, 2048);
        assert_eq!(files[0].content_type, Some("application/json".to_string()));
        assert_eq!(files[1].path, "sync/device2/state.json");
        assert_eq!(files[1].size, 1024);
    }
}
