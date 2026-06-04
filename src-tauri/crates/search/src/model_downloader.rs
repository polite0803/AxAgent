use std::path::{Path, PathBuf};

use axagent_harness::core_error::Result;

/// 模型下载管理器——从 HuggingFace Hub 或自定义 URL 下载 GGUF 模型文件
#[derive(Debug, Clone)]
pub struct ModelDownloader {
    cache_dir: PathBuf,
}

/// 预定义模型清单
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PresetModel {
    pub filename: String,
    pub hf_repo: Option<String>,
    pub direct_url: Option<String>,
    pub sha256: String,
    pub model_type: PresetModelType,
    pub display_name: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum PresetModelType {
    Reranker,
    Judge,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalModelInfo {
    pub name: String,
    pub file_path: String,
    pub size_bytes: u64,
    pub downloaded_at: String,
    pub sha256: String,
    pub model_type: PresetModelType,
    pub is_downloaded: bool,
}

impl ModelDownloader {
    /// 使用默认缓存路径创建下载管理器（~/.axagent/models/）
    pub fn new() -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".axagent")
            .join("models");
        Self { cache_dir }
    }

    /// 使用指定缓存路径创建下载管理器
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// 返回缓存目录路径
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// 返回预定义的模型清单
    pub fn preset_models() -> Vec<PresetModel> {
        vec![
            PresetModel {
                filename: "bge-reranker-v2-m3.Q4_K_M.gguf".to_string(),
                hf_repo: Some("gpustack/bge-reranker-v2-m3-GGUF".to_string()),
                direct_url: None,
                sha256: String::new(),
                model_type: PresetModelType::Reranker,
                display_name: "BGE-Reranker-v2-m3 (Q4_K_M)".to_string(),
                size_bytes: 316_000_000,
            },
            PresetModel {
                filename: "qwen2.5-0.5b.Q4_K_M.gguf".to_string(),
                hf_repo: Some("Qwen/Qwen2.5-0.5B-GGUF".to_string()),
                direct_url: None,
                sha256: String::new(),
                model_type: PresetModelType::Judge,
                display_name: "Qwen2.5 0.5B (Q4_K_M)".to_string(),
                size_bytes: 400_000_000,
            },
        ]
    }

    /// 确保指定模型已下载，返回模型文件的路径
    pub async fn ensure_model(&self, preset: &PresetModel) -> Result<PathBuf> {
        let model_path = self.cache_dir.join(&preset.filename);
        if model_path.exists() {
            if !preset.sha256.is_empty() {
                let actual = Self::sha256_file(&model_path)?;
                if actual == preset.sha256 {
                    tracing::info!(name = %preset.filename, "Model already cached");
                    return Ok(model_path);
                }
                tracing::warn!(
                    name = %preset.filename,
                    "Cached model hash mismatch, re-downloading"
                );
                tokio::fs::remove_file(&model_path).await.ok();
            } else {
                return Ok(model_path);
            }
        }

        // 优先 HuggingFace Hub
        if let Some(repo) = &preset.hf_repo {
            match self
                .download_from_hf(repo, &preset.filename, &preset.sha256)
                .await
            {
                Ok(path) => return Ok(path),
                Err(e) => {
                    tracing::warn!("HF download failed: {}, trying direct URL", e);
                },
            }
        }

        // 回退到直链
        if let Some(url) = &preset.direct_url {
            self.download_direct(&preset.filename, url, &preset.sha256)
                .await
        } else {
            Err(axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                "No download source for {}",
                preset.filename
            )))
        }
    }

    /// 从 HuggingFace Hub 下载模型文件（通过直链下载，无需 hf-hub）
    #[cfg(not(target_os = "android"))]
    async fn download_from_hf(
        &self,
        repo: &str,
        filename: &str,
        expected_sha256: &str,
    ) -> Result<PathBuf> {
        let url = format!("https://huggingface.co/{repo}/resolve/main/{filename}");
        self.download_direct(filename, &url, expected_sha256).await
    }

    #[cfg(target_os = "android")]
    async fn download_from_hf(
        &self,
        _repo: &str,
        _filename: &str,
        _expected_sha256: &str,
    ) -> Result<PathBuf> {
        Err(axagent_harness::core_error::AxAgentError::ModelDownload(
            "HuggingFace Hub is not available on Android".to_string(),
        ))
    }

    /// 从直链下载模型文件（支持断点续传）
    async fn download_direct(
        &self,
        filename: &str,
        url: &str,
        expected_sha256: &str,
    ) -> Result<PathBuf> {
        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| {
                axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                    "Failed to create cache dir: {}",
                    e
                ))
            })?;

        let model_path = self.cache_dir.join(filename);
        let tmp_path = model_path.with_extension("download");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3600))
            .build()
            .map_err(|e| {
                axagent_harness::core_error::AxAgentError::ModelDownload(format!("HTTP client error: {}", e))
            })?;

        let mut request = client.get(url);
        let has_partial = tmp_path.exists();
        if has_partial && let Ok(meta) = tokio::fs::metadata(&tmp_path).await {
            let range = format!("bytes={}-", meta.len());
            request = request.header("Range", range);
            tracing::info!(
                filename = %filename,
                bytes = meta.len(),
                "Resuming download"
            );
        }

        let response = request.send().await.map_err(|e| {
            axagent_harness::core_error::AxAgentError::ModelDownload(format!("Download failed: {}", e))
        })?;

        let status = response.status();

        // 检查服务器是否支持断点续传（206 Partial Content）
        if has_partial && status != reqwest::StatusCode::PARTIAL_CONTENT {
            tracing::warn!(
                filename = %filename,
                "Server does not support resume, restarting download"
            );
            tokio::fs::remove_file(&tmp_path).await.ok();
        }

        if !status.is_success() {
            return Err(axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                "HTTP {} for {}",
                status, url
            )));
        }

        // 以追加模式打开（续传）或创建新文件
        let mut file = if tmp_path.exists() {
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&tmp_path)
                .await
                .map_err(|e| {
                    axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                        "Cannot open temp file: {}",
                        e
                    ))
                })?
        } else {
            tokio::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp_path)
                .await
                .map_err(|e| {
                    axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                        "Cannot open temp file: {}",
                        e
                    ))
                })?
        };

        // 流式写入响应体，避免内存爆满
        use futures::StreamExt;
        use tokio::io::AsyncWriteExt;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                axagent_harness::core_error::AxAgentError::ModelDownload(format!("Read response: {}", e))
            })?;
            file.write_all(&chunk).await.map_err(|e| {
                axagent_harness::core_error::AxAgentError::ModelDownload(format!("Write temp file: {}", e))
            })?;
        }

        tokio::fs::rename(&tmp_path, &model_path)
            .await
            .map_err(|e| {
                axagent_harness::core_error::AxAgentError::ModelDownload(format!("Rename temp file: {}", e))
            })?;

        // SHA256 完整性校验
        if !expected_sha256.is_empty() {
            let actual = Self::sha256_file(&model_path)?;
            if actual != expected_sha256 {
                tokio::fs::remove_file(&model_path).await.ok();
                return Err(axagent_harness::core_error::AxAgentError::ModelIntegrity {
                    expected: expected_sha256.to_string(),
                    actual,
                });
            }
        }

        tracing::info!(filename = %filename, "Model downloaded and verified");
        Ok(model_path)
    }

    /// 列出所有模型（含下载状态）
    pub fn list_all_models(&self) -> Vec<LocalModelInfo> {
        ModelDownloader::preset_models()
            .into_iter()
            .map(|p| {
                let path = self.cache_dir.join(&p.filename);
                let is_downloaded = path.exists();
                let meta = std::fs::metadata(&path).ok();
                LocalModelInfo {
                    name: p.display_name.clone(),
                    file_path: path.to_string_lossy().to_string(),
                    size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(p.size_bytes),
                    downloaded_at: if is_downloaded {
                        meta.and_then(|m| m.modified().ok())
                            .map(|t| {
                                chrono::DateTime::<chrono::Utc>::from(t)
                                    .format("%Y-%m-%d %H:%M")
                                    .to_string()
                            })
                            .unwrap_or_default()
                    } else {
                        String::new()
                    },
                    sha256: if is_downloaded {
                        Self::sha256_file(&path).unwrap_or_default()
                    } else {
                        String::new()
                    },
                    model_type: p.model_type,
                    is_downloaded,
                }
            })
            .collect()
    }

    /// 移除缓存的模型文件
    pub fn remove_model(&self, filename: &str) -> Result<()> {
        if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
            return Err(axagent_harness::core_error::AxAgentError::Validation(
                "Filename must not contain path separators or traversal".to_string(),
            ));
        }
        let path = self.cache_dir.join(filename);
        let canonical_base = self
            .cache_dir
            .canonicalize()
            .map_err(axagent_harness::core_error::AxAgentError::Io)?;
        if path.exists() {
            let canonical_path = path
                .canonicalize()
                .map_err(axagent_harness::core_error::AxAgentError::Io)?;
            if !canonical_path.starts_with(&canonical_base) {
                return Err(axagent_harness::core_error::AxAgentError::Validation(
                    "Path traversal detected".to_string(),
                ));
            }
            std::fs::remove_file(&path).map_err(axagent_harness::core_error::AxAgentError::Io)?;
        }
        Ok(())
    }

    /// 计算文件的 SHA256 哈希（流式读取，避免一次性加载到内存）
    pub fn sha256_file(path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let file = std::fs::File::open(path).map_err(axagent_harness::core_error::AxAgentError::Io)?;
        let mut reader = std::io::BufReader::new(file);
        let mut hasher = Sha256::new();
        std::io::copy(&mut reader, &mut hasher).map_err(axagent_harness::core_error::AxAgentError::Io)?;
        Ok(hex::encode(hasher.finalize()))
    }
}

impl Default for ModelDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_preset_models_not_empty() {
        let models = ModelDownloader::preset_models();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model_type, PresetModelType::Reranker);
        assert_eq!(models[1].model_type, PresetModelType::Judge);
    }

    #[test]
    fn test_list_all_models_shows_all() {
        let tmp = TempDir::new().unwrap();
        let dl = ModelDownloader::with_cache_dir(tmp.path().to_path_buf());
        let models = dl.list_all_models();
        assert_eq!(models.len(), 2);
        assert!(!models[0].is_downloaded);
        assert!(!models[1].is_downloaded);
    }

    #[test]
    fn test_remove_nonexistent_model() {
        let tmp = TempDir::new().unwrap();
        let dl = ModelDownloader::with_cache_dir(tmp.path().to_path_buf());
        let result = dl.remove_model("nonexistent.gguf");
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
