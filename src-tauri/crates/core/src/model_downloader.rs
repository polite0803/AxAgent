use std::path::{Path, PathBuf};

use crate::error::Result;

/// 模型下载管理器——按需从远程拉取模型文件到本地缓存
#[derive(Debug, Clone)]
pub struct ModelDownloader {
    cache_dir: PathBuf,
}

/// 本地已下载模型的信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalModelInfo {
    pub name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub downloaded_at: String,
    pub sha256: String,
}

impl ModelDownloader {
    /// 创建下载管理器，缓存目录默认为 ~/.axagent/models
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".axagent")
            .join("models");
        Self { cache_dir }
    }

    /// 指定自定义缓存目录
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// 确保指定模型已下载。若本地不存在则从 url 下载并校验 SHA256。
    pub async fn ensure_model(
        &self,
        name: &str,
        url: &str,
        expected_sha256: &str,
    ) -> Result<PathBuf> {
        let model_path = self.cache_dir.join(name);
        if model_path.exists() {
            let actual = Self::sha256_file(&model_path)?;
            if actual == expected_sha256 {
                tracing::info!(name = %name, "Model already cached");
                return Ok(model_path);
            }
            tracing::warn!(name = %name, "Cached model hash mismatch, re-downloading");
            std::fs::remove_file(&model_path).ok();
        }
        self.download_model(name, url, expected_sha256).await
    }

    /// 下载模型文件（支持断点续传）
    async fn download_model(
        &self,
        name: &str,
        url: &str,
        expected_sha256: &str,
    ) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Failed to create cache dir: {}", e))
        })?;

        let model_path = self.cache_dir.join(name);
        let tmp_path = model_path.with_extension("download");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .build()
            .map_err(|e| {
                crate::error::AxAgentError::ModelDownload(format!("HTTP client error: {}", e))
            })?;

        let mut request = client.get(url);
        if tmp_path.exists() {
            if let Ok(meta) = std::fs::metadata(&tmp_path) {
                let range = format!("bytes={}-", meta.len());
                request = request.header("Range", range);
                tracing::info!(name = %name, bytes = meta.len(), "Resuming download");
            }
        }

        let response = request.send().await.map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Download failed: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(crate::error::AxAgentError::ModelDownload(format!(
                "HTTP {} for {}",
                response.status(),
                url
            )));
        }

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&tmp_path)
            .map_err(|e| {
                crate::error::AxAgentError::ModelDownload(format!("Cannot open temp file: {}", e))
            })?;

        use std::io::Write;
        let bytes = response.bytes().await.map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Read response: {}", e))
        })?;
        file.write_all(&bytes).map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Write temp file: {}", e))
        })?;

        std::fs::rename(&tmp_path, &model_path).map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Rename temp file: {}", e))
        })?;

        let actual = Self::sha256_file(&model_path)?;
        if actual != expected_sha256 {
            std::fs::remove_file(&model_path).ok();
            return Err(crate::error::AxAgentError::ModelIntegrity {
                expected: expected_sha256.to_string(),
                actual,
            });
        }

        tracing::info!(name = %name, "Model downloaded and verified");
        Ok(model_path)
    }

    /// 列出所有本地已下载的模型
    pub fn list_local_models(&self) -> Result<Vec<LocalModelInfo>> {
        if !self.cache_dir.exists() {
            return Ok(vec![]);
        }
        let mut models = Vec::new();
        for entry in std::fs::read_dir(&self.cache_dir).map_err(crate::error::AxAgentError::Io)? {
            let entry = entry.map_err(crate::error::AxAgentError::Io)?;
            let path = entry.path();
            if path.is_file() && path.extension().is_none() {
                let meta = entry.metadata().map_err(crate::error::AxAgentError::Io)?;
                models.push(LocalModelInfo {
                    name: entry.file_name().to_string_lossy().to_string(),
                    file_path: path.to_string_lossy().to_string(),
                    size_bytes: meta.len(),
                    downloaded_at: chrono::Utc::now().to_rfc3339(),
                    sha256: Self::sha256_file(&path).unwrap_or_default(),
                });
            }
        }
        Ok(models)
    }

    /// 删除指定模型
    pub fn remove_model(&self, name: &str) -> Result<()> {
        let path = self.cache_dir.join(name);
        if path.exists() {
            std::fs::remove_file(&path).map_err(crate::error::AxAgentError::Io)?;
        }
        Ok(())
    }

    fn sha256_file(path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let data = std::fs::read(path).map_err(crate::error::AxAgentError::Io)?;
        let hash = Sha256::digest(&data);
        Ok(hex::encode(hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_list_empty_cache() {
        let tmp = TempDir::new().unwrap();
        let dl = ModelDownloader::with_cache_dir(tmp.path().to_path_buf());
        let models = dl.list_local_models().unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn test_remove_nonexistent_model() {
        let tmp = TempDir::new().unwrap();
        let dl = ModelDownloader::with_cache_dir(tmp.path().to_path_buf());
        let result = dl.remove_model("nonexistent");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sha256_file() {
        use std::io::Write;
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"hello world").unwrap();
        let hash = ModelDownloader::sha256_file(&path).unwrap();
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }
}
