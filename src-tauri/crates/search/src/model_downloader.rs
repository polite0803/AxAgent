// SPDX-License-Identifier: AGPL-3.0-only

use std::path::{Path, PathBuf};

use axagent_harness::core_error::Result;

/// 模型下载管理器——从 HuggingFace Hub 或自定义 URL 下载 GGUF 模型文件
#[derive(Debug, Clone)]
pub struct ModelDownloader {
    cache_dir: PathBuf,
}

/// 下载进度回调：`(downloaded_bytes, total_bytes)`。
/// `total_bytes` 未知时为 0（分块传输等场景）。
/// `Sync` 必须：async 下载 future 会在持有回调引用的状态下跨 await 移动。
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

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
    /// 稀疏神经编码器（BGE-M3 等），输出 (token_id, weight) 列表。
    /// 用于多引擎 RAG 的 sparse 检索路径。
    SparseEncoder,
    /// 稠密向量模型（bge-m3 等），供知识库/向量检索主链路使用。
    Embedding,
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
        let cache_dir =
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".axagent").join("models");
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
            // BGE-M3 sparse encoder（与 bge-reranker-v2-m3 同源 BERT 架构）。
            // 输出 token 级激活权重，用于 sparse neural 检索路径。
            // tokenizer 复用 bge-m3 tokenizer.json（需单独下载）。
            PresetModel {
                filename: "bge-m3-sparse.Q4_K_M.gguf".to_string(),
                hf_repo: Some("gpustack/bge-m3-GGUF".to_string()),
                direct_url: None,
                sha256: String::new(),
                model_type: PresetModelType::SparseEncoder,
                display_name: "BGE-M3 Sparse Encoder (Q4_K_M)".to_string(),
                size_bytes: 280_000_000,
            },
            // BGE-M3 稠密向量模型（Q5_K_M），知识库 embedding 主链路。
            PresetModel {
                filename: "bge-m3.Q5_K_M.gguf".to_string(),
                hf_repo: Some("gpustack/bge-m3-GGUF".to_string()),
                direct_url: None,
                sha256: String::new(),
                model_type: PresetModelType::Embedding,
                display_name: "BGE-M3 Embedding (Q5_K_M)".to_string(),
                size_bytes: 467_663_008,
            },
        ]
    }

    /// 按 filename 查找预设模型。
    ///
    /// 调用方传入 `model_filename`（如 `"bge-m3-sparse.Q4_K_M.gguf"`），
    /// 命中则返回 `PresetModel` 引用，否则 `None`。
    /// 用于注册/加载 sparse encoder 等可选模型，避免硬编码模型路径。
    pub fn find_preset(filename: &str) -> Option<PresetModel> {
        Self::preset_models().into_iter().find(|m| m.filename == filename)
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
            match self.download_from_hf(repo, &preset.filename, &preset.sha256, None).await {
                Ok(path) => return Ok(path),
                Err(e) => {
                    tracing::warn!("HF download failed: {}, trying direct URL", e);
                },
            }
        }

        // 回退到直链
        if let Some(url) = &preset.direct_url {
            self.download_direct(&preset.filename, url, &preset.sha256, None).await
        } else {
            Err(axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                "No download source for {}",
                preset.filename
            )))
        }
    }

    /// 下载 GGUF 模型，带进度回调。
    ///
    /// - `hf_repo` 非空时按 `{hf_endpoint}/{repo}/resolve/main/{filename}` 构造 URL
    ///   （`hf_endpoint` 可传 `https://hf-mirror.com` 等国内镜像）。
    /// - `direct_url` 非空时优先使用直链。
    /// - `on_progress` 每写入一个网络块回调一次 `(downloaded_bytes, total_bytes)`，
    ///   `total_bytes` 为 0 表示长度未知。
    pub async fn download_with_progress(
        &self,
        filename: &str,
        hf_repo: Option<&str>,
        direct_url: Option<&str>,
        hf_endpoint: &str,
        expected_sha256: &str,
        on_progress: Option<ProgressCallback>,
    ) -> Result<PathBuf> {
        if let Some(url) = direct_url.filter(|u| !u.is_empty()) {
            return self
                .download_direct(filename, url, expected_sha256, on_progress.as_ref())
                .await;
        }
        if let Some(repo) = hf_repo.filter(|r| !r.is_empty()) {
            let endpoint = if hf_endpoint.is_empty() {
                "https://huggingface.co"
            } else {
                hf_endpoint.trim_end_matches('/')
            };
            let url = format!("{endpoint}/{repo}/resolve/main/{filename}");
            return self
                .download_direct(filename, &url, expected_sha256, on_progress.as_ref())
                .await;
        }
        Err(axagent_harness::core_error::AxAgentError::ModelDownload(
            "No download source provided".to_string(),
        ))
    }

    /// 从 HuggingFace Hub 下载模型文件（通过直链下载，无需 hf-hub）
    #[cfg(not(target_os = "android"))]
    async fn download_from_hf(
        &self,
        repo: &str,
        filename: &str,
        expected_sha256: &str,
        on_progress: Option<&ProgressCallback>,
    ) -> Result<PathBuf> {
        let url = format!("https://huggingface.co/{repo}/resolve/main/{filename}");
        self.download_direct(filename, &url, expected_sha256, on_progress).await
    }

    #[cfg(target_os = "android")]
    async fn download_from_hf(
        &self,
        _repo: &str,
        _filename: &str,
        _expected_sha256: &str,
        _on_progress: Option<&ProgressCallback>,
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
        on_progress: Option<&ProgressCallback>,
    ) -> Result<PathBuf> {
        tokio::fs::create_dir_all(&self.cache_dir).await.map_err(|e| {
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
                axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                    "HTTP client error: {}",
                    e
                ))
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
            axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                "Download failed: {}",
                e
            ))
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
            tokio::fs::OpenOptions::new().append(true).open(&tmp_path).await.map_err(|e| {
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
        // 总大小：续传时 = 已写入字节 + 响应剩余长度；否则 = 响应 Content-Length。
        // 必须先于 bytes_stream() 读取（该方法会 move response）。
        let total_hint = response.content_length().unwrap_or(0);
        let mut stream = response.bytes_stream();
        let base_len = tokio::fs::metadata(&tmp_path).await.map(|m| m.len()).unwrap_or(0);
        let mut downloaded = base_len;
        if let Some(cb) = on_progress {
            cb(downloaded, base_len.saturating_add(total_hint));
        }
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                    "Read response: {}",
                    e
                ))
            })?;
            file.write_all(&chunk).await.map_err(|e| {
                axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                    "Write temp file: {}",
                    e
                ))
            })?;
            downloaded = downloaded.saturating_add(chunk.len() as u64);
            if let Some(cb) = on_progress {
                cb(downloaded, base_len.saturating_add(total_hint));
            }
        }

        tokio::fs::rename(&tmp_path, &model_path).await.map_err(|e| {
            axagent_harness::core_error::AxAgentError::ModelDownload(format!(
                "Rename temp file: {}",
                e
            ))
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
        let canonical_base =
            self.cache_dir.canonicalize().map_err(axagent_harness::core_error::AxAgentError::Io)?;
        if path.exists() {
            let canonical_path =
                path.canonicalize().map_err(axagent_harness::core_error::AxAgentError::Io)?;
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
        use std::io::Read;
        let file =
            std::fs::File::open(path).map_err(axagent_harness::core_error::AxAgentError::Io)?;
        let mut reader = std::io::BufReader::new(file);
        let mut hasher = Sha256::new();
        // sha2 0.11 起 Sha256 不再直接实现 io::Write，改为手动分块读取更新（保持流式，避免整文件入内存）。
        let mut buf = [0u8; 8192];
        loop {
            let n = reader.read(&mut buf).map_err(axagent_harness::core_error::AxAgentError::Io)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
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
        assert_eq!(models.len(), 4);
        assert_eq!(models[0].model_type, PresetModelType::Reranker);
        assert_eq!(models[1].model_type, PresetModelType::Judge);
        assert_eq!(models[2].model_type, PresetModelType::SparseEncoder);
        assert_eq!(models[3].model_type, PresetModelType::Embedding);
    }

    #[test]
    fn test_list_all_models_shows_all() {
        let tmp = TempDir::new().expect("测试：new 应成功");
        let dl = ModelDownloader::with_cache_dir(tmp.path().to_path_buf());
        let models = dl.list_all_models();
        assert_eq!(models.len(), 4);
        assert!(!models[0].is_downloaded);
        assert!(!models[1].is_downloaded);
        assert!(!models[2].is_downloaded);
        assert!(!models[3].is_downloaded);
    }

    #[test]
    fn test_find_preset_sparse_encoder() {
        // 按文件名查找 sparse encoder 预设（注册式查找，非硬编码路径）
        let m = ModelDownloader::find_preset("bge-m3-sparse.Q4_K_M.gguf");
        assert!(m.is_some());
        assert_eq!(m.expect("测试应成功").model_type, PresetModelType::SparseEncoder);
    }

    #[test]
    fn test_find_preset_unknown_returns_none() {
        assert!(ModelDownloader::find_preset("nonexistent.gguf").is_none());
    }

    #[test]
    fn test_remove_nonexistent_model() {
        let tmp = TempDir::new().expect("测试：new 应成功");
        let dl = ModelDownloader::with_cache_dir(tmp.path().to_path_buf());
        let result = dl.remove_model("nonexistent.gguf");
        assert!(result.is_ok());
    }

    #[test]
    fn test_sha256_file() {
        use std::io::Write;
        let tmp = TempDir::new().expect("测试：new 应成功");
        let path = tmp.path().join("test.bin");
        let mut f = std::fs::File::create(&path).expect("测试应成功");
        f.write_all(b"hello world").expect("测试：write_all 应成功");
        let hash = ModelDownloader::sha256_file(&path).expect("测试：sha256_file 应成功");
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }
}
