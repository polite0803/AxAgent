# 高级 RAG P0：检索质量质变 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在现有 RAG 管线基础上插入查询增强、跨编码器重排序、自省式质检三个阶段。本地模型（rerank + judge）使用 candle 纯 Rust 自运行推理，GGUF 文件通过 ModelDownloader 按需下载。

**Architecture:** 新增 5 个模块（`model_downloader`、`inference`、`query_enhancement`、`self_rag` 及重构后的 `reranker`），通过 `RAGPipeline` 在 `collect_rag_context()` 中编排。每个阶段通过配置独立开关，默认关闭以保持向后兼容。本地推理使用 candle（纯 Rust），零外部 HTTP 依赖。

**Tech Stack:** Rust 2021 · sea-orm · tokio · reqwest · sqlite-vec · candle（candle-core, candle-transformers, candle-nn）· tokenizers · hf-hub · serde · 前端 React 19 + TypeScript + Ant Design 6

---

## 文件结构

```
创建:
  src-tauri/crates/core/src/model_downloader.rs    # 模型下载管理器（GGUF 文件）
  src-tauri/crates/core/src/inference.rs           # candle 本地推理引擎
  src-tauri/crates/core/src/query_enhancement.rs   # 查询增强（HyDE / Multi-Query / Decomp）
  src-tauri/crates/core/src/self_rag.rs            # 自省式 RAG（candle 推理）
  src-tauri/crates/core/src/rag_pipeline.rs        # RAG 管线编排层

修改:
  src-tauri/crates/core/src/reranker.rs            # 重构为 trait-based 后端（candle 推理）
  src-tauri/crates/core/src/rag.rs                 # 集成 pipeline
  src-tauri/crates/core/src/types.rs               # 新增 RAG 配置类型
  src-tauri/crates/core/src/error.rs               # 新增模型下载/推理错误变体
  src-tauri/crates/core/src/lib.rs                 # 注册新模块
  src-tauri/crates/core/Cargo.toml                 # 新增 candle 系列依赖
  src/components/settings/KnowledgeSettings.tsx    # 前端开关 UI + 模型下载入口
  src/locales/zh-CN/settings.json                  # i18n 新增 key
  src/locales/en/settings.json                     # i18n 新增 key
  (其余 9 种语言文件同步新增对应 key)
```

---

### Task 1: 模型下载管理器 + Cargo.toml 依赖

**Files:**
- Create: `src-tauri/crates/core/src/model_downloader.rs`
- Modify: `src-tauri/crates/core/src/error.rs`（新增错误变体）
- Modify: `src-tauri/crates/core/src/lib.rs`（注册模块）
- Modify: `src-tauri/crates/core/Cargo.toml`（新增 candle 依赖）

- [ ] **Step 1: 在 Cargo.toml 中新增 candle 系列依赖**

```toml
# src-tauri/crates/core/Cargo.toml — 在 [dependencies] 中追加
candle-core = "0.8"
candle-transformers = "0.8"
candle-nn = "0.8"
tokenizers = "0.21"
hf-hub = "0.4"
```

- [ ] **Step 2: 在 error.rs 中新增模型下载和推理相关错误变体**

```rust
// error.rs - 在 AxAgentError 枚举中新增以下变体
#[error("Model download error: {0}")]
ModelDownload(String),

#[error("Model integrity error: expected {expected}, got {actual}")]
ModelIntegrity { expected: String, actual: String },

#[error("Model inference error: {0}")]
Inference(String),
```

- [ ] **Step 3: 创建 model_downloader.rs**

```rust
// model_downloader.rs
use std::path::{Path, PathBuf};
use crate::error::Result;

/// 模型下载管理器——从 HuggingFace Hub 或自定义 URL 下载 GGUF 模型文件
#[derive(Debug, Clone)]
pub struct ModelDownloader {
    cache_dir: PathBuf,
}

/// 预定义模型清单
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PresetModel {
    /// 文件名（如 "bge-reranker-v2-m3.Q4_K_M.gguf"）
    pub filename: String,
    /// HuggingFace repo（如 "gpustack/bge-reranker-v2-m3-GGUF"）
    pub hf_repo: Option<String>,
    /// 直链备用 URL
    pub direct_url: Option<String>,
    /// SHA256 校验值
    pub sha256: String,
    /// 模型类型
    pub model_type: PresetModelType,
    /// 用户可见名称
    pub display_name: String,
    /// 文件大小（用于 UI 展示）
    pub size_bytes: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum PresetModelType {
    Reranker,
    Judge,
}

/// 本地已下载模型的信息
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
    pub fn new() -> Self {
        let cache_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".axagent")
            .join("models");
        Self { cache_dir }
    }

    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

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
                sha256: "".to_string(), // 首次实施留空，下载后自动记录
                model_type: PresetModelType::Reranker,
                display_name: "BGE-Reranker-v2-m3 (Q4_K_M)".to_string(),
                size_bytes: 316_000_000,
            },
            PresetModel {
                filename: "qwen2.5-0.5b.Q4_K_M.gguf".to_string(),
                hf_repo: Some("Qwen/Qwen2.5-0.5B-GGUF".to_string()),
                direct_url: None,
                sha256: "".to_string(),
                model_type: PresetModelType::Judge,
                display_name: "Qwen2.5 0.5B (Q4_K_M)".to_string(),
                size_bytes: 400_000_000,
            },
        ]
    }

    /// 确保指定模型已下载。优先从 HuggingFace Hub 下载，失败则尝试直链。
    pub async fn ensure_model(&self, preset: &PresetModel) -> Result<PathBuf> {
        let model_path = self.cache_dir.join(&preset.filename);
        if model_path.exists() {
            if !preset.sha256.is_empty() {
                let actual = Self::sha256_file(&model_path)?;
                if actual == preset.sha256 {
                    tracing::info!(name = %preset.filename, "Model already cached");
                    return Ok(model_path);
                }
                tracing::warn!(name = %preset.filename, "Cached model hash mismatch, re-downloading");
                std::fs::remove_file(&model_path).ok();
            } else {
                return Ok(model_path); // 无校验哈希则相信存在即完整
            }
        }

        // 优先 HuggingFace Hub
        if let Some(repo) = &preset.hf_repo {
            match self.download_from_hf(repo, &preset.filename).await {
                Ok(path) => return Ok(path),
                Err(e) => tracing::warn!("HF download failed: {}, trying direct URL", e),
            }
        }

        // 回退到直链
        if let Some(url) = &preset.direct_url {
            self.download_direct(&preset.filename, url, &preset.sha256).await
        } else {
            Err(crate::error::AxAgentError::ModelDownload(format!(
                "No download source for {}", preset.filename
            )))
        }
    }

    async fn download_from_hf(&self, repo: &str, filename: &str) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Failed to create cache dir: {}", e))
        })?;

        let api = hf_hub::api::sync::Api::new().map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("hf-hub API error: {}", e))
        })?;

        let repo = api.model(repo.to_string());
        let path = repo.get(filename).map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("HF download failed: {}", e))
        })?;

        // 复制到 cache_dir
        let dest = self.cache_dir.join(filename);
        std::fs::copy(&path, &dest).map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Copy to cache: {}", e))
        })?;

        tracing::info!(filename = %filename, "Downloaded from HuggingFace Hub");
        Ok(dest)
    }

    async fn download_direct(
        &self,
        filename: &str,
        url: &str,
        expected_sha256: &str,
    ) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Failed to create cache dir: {}", e))
        })?;

        let model_path = self.cache_dir.join(filename);
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
                tracing::info!(filename = %filename, bytes = meta.len(), "Resuming download");
            }
        }

        let response = request.send().await.map_err(|e| {
            crate::error::AxAgentError::ModelDownload(format!("Download failed: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(crate::error::AxAgentError::ModelDownload(format!(
                "HTTP {} for {}", response.status(), url
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

        if !expected_sha256.is_empty() {
            let actual = Self::sha256_file(&model_path)?;
            if actual != expected_sha256 {
                std::fs::remove_file(&model_path).ok();
                return Err(crate::error::AxAgentError::ModelIntegrity {
                    expected: expected_sha256.to_string(),
                    actual,
                });
            }
        }

        tracing::info!(filename = %filename, "Model downloaded and verified");
        Ok(model_path)
    }

    /// 列出所有本地已下载的模型状态（包括未下载的预设模型）
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

    /// 删除指定模型
    pub fn remove_model(&self, filename: &str) -> Result<()> {
        let path = self.cache_dir.join(filename);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| crate::error::AxAgentError::Io(e))?;
        }
        Ok(())
    }

    pub fn sha256_file(path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let data = std::fs::read(path).map_err(|e| crate::error::AxAgentError::Io(e))?;
        let hash = Sha256::digest(&data);
        Ok(hex::encode(hash))
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
```

- [ ] **Step 4: 在 lib.rs 中注册模块**

```rust
// lib.rs — 在 pub mod 列表中添加
pub mod model_downloader;
```

- [ ] **Step 5: 编译检查**

```bash
cd src-tauri && cargo check -p axagent-core 2>&1
```
Expected: 编译通过。（注意 `hf-hub` 依赖 `ureq`/`reqwest`，可能与现有依赖有版本冲突需调整）

- [ ] **Step 6: 运行测试**

```bash
cd src-tauri && cargo test -p axagent-core -- model_downloader::tests 2>&1
```
Expected: PASS。

- [ ] **Step 7: Commit**

```bash
git add src-tauri/crates/core/Cargo.toml \
        src-tauri/crates/core/src/model_downloader.rs \
        src-tauri/crates/core/src/error.rs \
        src-tauri/crates/core/src/lib.rs
git commit -m "feat: 添加模型下载管理器 + candle 依赖（ModelDownloader + GGUF 按需下载）"
```

---

### Task 2: candle 本地推理引擎

**Files:**
- Create: `src-tauri/crates/core/src/inference.rs`
- Modify: `src-tauri/crates/core/src/lib.rs`（注册模块）

- [ ] **Step 1: 创建 inference.rs**

```rust
// inference.rs
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::error::Result;

/// 本地推理引擎——使用 candle 在 CPU 上运行 GGUF 格式的小模型
///
/// 支持的模型类型：
/// - Reranker: BGE-Reranker-v2-m3（BERT/XLM-RoBERTa 架构）
/// - Judge: Qwen2.5:0.5b（LLaMA 架构）
pub struct InferenceEngine {
    /// 已加载的模型缓存（key = filename, value = device + model）
    loaded_models: Arc<Mutex<std::collections::HashMap<String, LoadedModel>>>,
}

enum LoadedModel {
    Reranker {
        model: candle_transformers::models::bert::BertForSequenceClassification,
        tokenizer: tokenizers::Tokenizer,
        device: candle_core::Device,
    },
    Judge {
        model: candle_transformers::models::quantized::llama::ModelWeights,
        tokenizer: tokenizers::Tokenizer,
        device: candle_core::Device,
    },
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            loaded_models: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// 加载 BGE-Reranker 模型用于跨编码器重排序
    pub async fn load_reranker_model(&self, gguf_path: &Path) -> Result<()> {
        let device = candle_core::Device::Cpu;
        let filename = gguf_path.file_name().unwrap().to_string_lossy().to_string();

        let tokenizer = tokenizers::Tokenizer::from_file(
            gguf_path.with_extension("tokenizer.json").as_path()
                .to_str()
                .ok_or_else(|| crate::error::AxAgentError::Inference(
                    "Tokenizer path not valid UTF-8".into()
                ))?,
        ).map_err(|e| crate::error::AxAgentError::Inference(format!("Load tokenizer: {}", e)))?;

        let vb = candle_transformers::models::bert::BertForSequenceClassification::load(
            vb_from_mmaped_file(gguf_path)?,
            &candle_transformers::models::bert::Config::default(),
        ).map_err(|e| crate::error::AxAgentError::Inference(format!("Load BERT: {}", e)))?;

        let mut models = self.loaded_models.lock().await;
        models.insert(filename, LoadedModel::Reranker {
            model: vb,
            tokenizer,
            device,
        });

        Ok(())
    }

    /// 加载 Qwen2.5 模型用于相关性裁判
    pub async fn load_judge_model(&self, gguf_path: &Path) -> Result<()> {
        let device = candle_core::Device::Cpu;
        let filename = gguf_path.file_name().unwrap().to_string_lossy().to_string();

        let tokenizer = tokenizers::Tokenizer::from_file(
            gguf_path.with_extension("tokenizer.json").as_path()
                .to_str()
                .ok_or_else(|| crate::error::AxAgentError::Inference(
                    "Tokenizer path not valid UTF-8".into()
                ))?,
        ).map_err(|e| crate::error::AxAgentError::Inference(format!("Load tokenizer: {}", e)))?;

        // Qwen2.5 使用 LLaMA 兼容的 GGUF 量化格式
        let model = candle_transformers::models::quantized::llama::ModelWeights::from_gguf(
            gguf_path, &device,
        ).map_err(|e| crate::error::AxAgentError::Inference(format!("Load judge model: {}", e)))?;

        let mut models = self.loaded_models.lock().await;
        models.insert(filename, LoadedModel::Judge {
            model,
            tokenizer,
            device,
        });

        Ok(())
    }

    /// 跨编码器重排序：对每个 (query, document) 对计算相关性分数
    pub async fn rerank(
        &self,
        model_filename: &str,
        query: &str,
        documents: &[String],
    ) -> Result<Vec<f32>> {
        let models = self.loaded_models.lock().await;
        let loaded = models.get(model_filename).ok_or_else(|| {
            crate::error::AxAgentError::Inference(format!(
                "Reranker model '{}' not loaded", model_filename
            ))
        })?;

        match loaded {
            LoadedModel::Reranker { model, tokenizer, device } => {
                let mut scores = Vec::with_capacity(documents.len());
                for doc in documents {
                    let score = inference_rerank_pair(model, tokenizer, device, query, doc)?;
                    scores.push(score);
                }
                Ok(scores)
            },
            _ => Err(crate::error::AxAgentError::Inference(
                "Model is not a reranker".into()
            )),
        }
    }

    /// 相关性裁判：判断 chunk 是否与 query 相关
    pub async fn judge(
        &self,
        model_filename: &str,
        query: &str,
        chunk_content: &str,
    ) -> Result<JudgeOutput> {
        let models = self.loaded_models.lock().await;
        let loaded = models.get(model_filename).ok_or_else(|| {
            crate::error::AxAgentError::Inference(format!(
                "Judge model '{}' not loaded", model_filename
            ))
        })?;

        match loaded {
            LoadedModel::Judge { model, tokenizer, device } => {
                let prompt = format!(
                    "你是一个相关性裁判。给定用户问题和检索到的文档块，判断该文档是否与问题相关。\n\n\
                     用户问题：{query}\n文档块：{chunk_content}\n\n\
                     返回 JSON：{{\"relevant\": true/false, \"score\": 0.0-1.0, \"reason\": \"一句话说明理由\"}}"
                );

                let output = inference_generate(model, tokenizer, device, &prompt)?;
                Ok(parse_judge_output(&output))
            },
            _ => Err(crate::error::AxAgentError::Inference(
                "Model is not a judge".into()
            )),
        }
    }

    /// 卸载所有已加载模型，释放内存
    pub async fn unload_all(&self) {
        let mut models = self.loaded_models.lock().await;
        models.clear();
        tracing::info!("All inference models unloaded");
    }
}

// ── 推理辅助函数 ────────────────────────────────────────────────

fn vb_from_mmaped_file(path: &Path) -> Result<candle_core::VarBuilder> {
    use candle_core::quantized::gguf_file;
    let mut file = std::fs::File::open(path).map_err(|e| {
        crate::error::AxAgentError::Inference(format!("Open GGUF: {}", e))
    })?;
    let model = gguf_file::Content::read(&mut file).map_err(|e| {
        crate::error::AxAgentError::Inference(format!("Read GGUF: {}", e))
    })?;
    let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(model, &candle_core::Device::Cpu)
        .map_err(|e| crate::error::AxAgentError::Inference(format!("VarBuilder: {}", e)))?;
    Ok(vb)
}

fn inference_rerank_pair(
    model: &candle_transformers::models::bert::BertForSequenceClassification,
    tokenizer: &tokenizers::Tokenizer,
    device: &candle_core::Device,
    query: &str,
    document: &str,
) -> Result<f32> {
    let input = format!("[CLS]{}[SEP]{}[SEP]", query, document);
    let encoding = tokenizer.encode(input, true).map_err(|e| {
        crate::error::AxAgentError::Inference(format!("Tokenize: {}", e))
    })?;

    let tokens = encoding.get_ids().to_vec();
    let token_ids = candle_core::Tensor::new(tokens.as_slice(), device)
        .map_err(|e| crate::error::AxAgentError::Inference(format!("Tensor: {}", e)))?
        .unsqueeze(0)?;

    let logits = model.forward(&token_ids, None, None, None, None, None).map_err(|e| {
        crate::error::AxAgentError::Inference(format!("Forward: {}", e))
    })?;

    let score = logits.get(0).map_err(|e| {
        crate::error::AxAgentError::Inference(format!("Get logit: {}", e))
    })?.to_scalar::<f32>()?;

    // sigmoid 转为 0-1 概率
    Ok(1.0 / (1.0 + (-score).exp()))
}

fn inference_generate(
    model: &candle_transformers::models::quantized::llama::ModelWeights,
    tokenizer: &tokenizers::Tokenizer,
    device: &candle_core::Device,
    prompt: &str,
) -> Result<String> {
    let encoding = tokenizer.encode(prompt, true).map_err(|e| {
        crate::error::AxAgentError::Inference(format!("Tokenize: {}", e))
    })?;

    let mut tokens = encoding.get_ids().to_vec();
    let mut generated = String::new();

    for _ in 0..256 {
        // 最多生成 256 tokens
        let input = candle_core::Tensor::new(&tokens[tokens.len().saturating_sub(512)..], device)
            .map_err(|e| crate::error::AxAgentError::Inference(format!("Tensor: {}", e)))?
            .unsqueeze(0)?;

        let logits = model.forward(&input, 0).map_err(|e| {
            crate::error::AxAgentError::Inference(format!("Forward: {}", e))
        })?;

        let next_token = logits.squeeze(0)?.squeeze(0)?.argmax(0)?.to_scalar::<u32>()?;
        tokens.push(next_token);

        if let Some(decoded) = tokenizer.decode(&[next_token], false).ok() {
            generated.push_str(&decoded);
        }

        // 检测到 JSON 结束符或产生足够内容即停止
        if generated.contains("}") && generated.len() > 20 {
            break;
        }
    }

    Ok(generated)
}

#[derive(Debug, Clone)]
pub struct JudgeOutput {
    pub relevant: bool,
    pub score: f32,
    pub reason: String,
}

fn parse_judge_output(raw: &str) -> JudgeOutput {
    // 尝试提取 JSON
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            let json_str = &raw[start..=end];
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                return JudgeOutput {
                    relevant: parsed["relevant"].as_bool().unwrap_or(true),
                    score: parsed["score"].as_f64().unwrap_or(0.5) as f32,
                    reason: parsed["reason"].as_str().unwrap_or("").to_string(),
                };
            }
        }
    }
    JudgeOutput {
        relevant: true,
        score: 0.5,
        reason: "Failed to parse judge output".to_string(),
    }
}
```

- [ ] **Step 2: 在 lib.rs 中注册模块**

```rust
pub mod inference;
```

- [ ] **Step 3: 编译检查**

```bash
cd src-tauri && cargo check -p axagent-core 2>&1
```
注意：candle 首次编译较慢（~5 分钟），需要下载依赖。

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/core/src/inference.rs \
        src-tauri/crates/core/src/lib.rs
git commit -m "feat: 添加 candle 本地推理引擎（rerank + judge 自运行）"
```

---

### Task 3: 查询增强模块（HyDE + Multi-Query + Decomposition）

**Files:**
- Create: `src-tauri/crates/core/src/query_enhancement.rs`
- Modify: `src-tauri/crates/core/src/types.rs`（新增类型）
- Modify: `src-tauri/crates/core/src/lib.rs`（注册模块）

- [ ] **Step 1: 在 types.rs 中新增查询增强相关类型**

```rust
// types.rs — 在文件末尾新增

// === Query Enhancement Types ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnhancementStrategy {
    None,
    Hyde,
    MultiQuery,
    Decomposition,
    Auto,
}

#[derive(Debug, Clone)]
pub struct EnhancedQuery {
    pub text: String,
    pub strategy: EnhancementStrategy,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancementConfig {
    pub enabled: bool,
    pub strategy: EnhancementStrategy,
    pub max_variants: usize,
    pub combined_call: bool,
}
```

- [ ] **Step 2: 创建 query_enhancement.rs**

（代码与上一版计划中的 Task 2 一致，不再重复列出）

- [ ] **Step 3: 编译 + 注册 + 测试 + Commit**

```bash
cd src-tauri && cargo check -p axagent-core
cd src-tauri && cargo test -p axagent-core -- query_enhancement::tests
```

```bash
git add src-tauri/crates/core/src/query_enhancement.rs \
        src-tauri/crates/core/src/types.rs \
        src-tauri/crates/core/src/lib.rs
git commit -m "feat: 添加查询增强模块（HyDE / MultiQuery / Decomposition）"
```

---

### Task 4: 重构 reranker.rs 为 trait-based + candle 后端

**Files:**
- Modify: `src-tauri/crates/core/src/reranker.rs`（重构）

- [ ] **Step 1: 重写 reranker.rs**

核心变更：`CrossEncoderReranker` 不再调用 Ollama HTTP，改为调用 `InferenceEngine::rerank()`。

```rust
// reranker.rs — 完整重写
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::hybrid_search::HybridSearchResult;
use crate::inference::InferenceEngine;

// ── 配置 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankConfig {
    pub enabled: bool,
    /// "rule" | "cross_encoder" | "pipeline"
    pub backend: String,
    /// cross-encoder 对应的 GGUF 文件名
    pub cross_encoder_model: Option<String>,
    pub top_n: usize,
    pub candidate_k: usize,
    pub rule_filter_keep: usize,
    pub score_threshold: Option<f32>,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "rule".to_string(),
            cross_encoder_model: Some("bge-reranker-v2-m3.Q4_K_M.gguf".to_string()),
            top_n: 5,
            candidate_k: 30,
            rule_filter_keep: 15,
            score_threshold: None,
        }
    }
}

// ── 结果类型 ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RerankedResult {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub original_score: f32,
    pub rerank_score: f32,
    pub rerank_reason: Option<String>,
}

// ── 可插拔后端 trait ───────────────────────────────────────

#[async_trait]
pub trait RerankBackend: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> crate::error::Result<Vec<(String, f32)>>;
}

// ── 规则后端（逻辑与现有相同）──────────────────────────────

pub struct RuleReranker;

#[async_trait]
impl RerankBackend for RuleReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> crate::error::Result<Vec<(String, f32)>> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        let mut scored: Vec<(String, f32)> = chunks.iter().map(|(id, content, orig_score)| {
            let content_lower = content.to_lowercase();
            let exact_matches = query_terms.iter()
                .filter(|t| content_lower.contains(*t)).count() as f32;
            let exact_score = exact_matches / query_terms.len().max(1) as f32;
            let word_count = content.split_whitespace().count().max(1);
            let coverage = query_terms.iter()
                .filter(|t| content_lower.split_whitespace().any(|w| w.contains(*t)))
                .count() as f32 / query_terms.len().max(1) as f32;
            let first_pos = content_lower.find(&query_lower)
                .map(|p| 1.0 - p as f32 / content_lower.len() as f32)
                .unwrap_or(1.0);
            let len_penalty = {
                let ratio = word_count as f32 / 100.0;
                if ratio < 1.0 { ratio } else { 1.0 / ratio.sqrt() }
            };
            let score = *orig_score * 0.3 + exact_score * 0.25
                + coverage * 0.2 + first_pos * 0.15 + len_penalty * 0.1;
            (id.clone(), score)
        }).collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored)
    }
}

// ── Cross-Encoder 后端（candle 本地推理）────────────────────

pub struct CrossEncoderReranker {
    model_filename: String,
    engine: std::sync::Arc<InferenceEngine>,
}

impl CrossEncoderReranker {
    pub fn new(model_filename: String, engine: std::sync::Arc<InferenceEngine>) -> Self {
        Self { model_filename, engine }
    }
}

#[async_trait]
impl RerankBackend for CrossEncoderReranker {
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)],
    ) -> crate::error::Result<Vec<(String, f32)>> {
        if chunks.is_empty() {
            return Ok(vec![]);
        }
        let documents: Vec<String> = chunks.iter().map(|(_, c, _)| c.clone()).collect();

        match self.engine.rerank(&self.model_filename, query, &documents).await {
            Ok(scores) => {
                let mut result: Vec<(String, f32)> = chunks.iter().zip(scores.iter())
                    .map(|((id, _, _), &s)| (id.clone(), s))
                    .collect();
                result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                Ok(result)
            },
            Err(e) => {
                tracing::warn!("Cross-encoder rerank failed: {}", e);
                Ok(chunks.iter().map(|(id, _, s)| (id.clone(), *s)).collect())
            },
        }
    }
}

// ── 管线编排 ──────────────────────────────────────────────

pub struct RerankPipeline {
    stages: Vec<Box<dyn RerankBackend>>,
}

impl RerankPipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, backend: Box<dyn RerankBackend>) {
        self.stages.push(backend);
    }

    pub async fn execute(
        &self,
        query: &str,
        results: Vec<HybridSearchResult>,
        config: &RerankConfig,
    ) -> Vec<RerankedResult> {
        if !config.enabled || results.is_empty() {
            return results.into_iter().map(|r| RerankedResult {
                id: r.id,
                document_id: r.document_id,
                chunk_index: r.chunk_index,
                content: r.content,
                original_score: r.combined_score,
                rerank_score: r.combined_score,
                rerank_reason: None,
            }).collect();
        }

        let mut current: Vec<HybridSearchResult> = results.into_iter()
            .take(config.candidate_k).collect();

        for stage in &self.stages {
            let chunks: Vec<(String, String, f32)> = current.iter()
                .map(|r| (r.id.clone(), r.content.clone(), r.combined_score))
                .collect();

            let scored = match stage.rerank(query, &chunks).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Rerank stage failed: {}", e);
                    continue;
                },
            };

            let score_map: std::collections::HashMap<&str, f32> = scored.iter()
                .map(|(id, s)| (id.as_str(), *s))
                .collect();

            current.sort_by(|a, b| {
                let sa = score_map.get(a.id.as_str()).copied().unwrap_or(a.combined_score);
                let sb = score_map.get(b.id.as_str()).copied().unwrap_or(b.combined_score);
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            });

            current = current.into_iter().take(config.rule_filter_keep).collect();
        }

        current.into_iter().take(config.top_n).enumerate().map(|(i, r)| {
            RerankedResult {
                id: r.id,
                document_id: r.document_id,
                chunk_index: r.chunk_index,
                content: r.content,
                original_score: r.combined_score,
                rerank_score: r.combined_score,
                rerank_reason: Some(format!("Ranked #{}", i + 1)),
            }
        }).collect()
    }
}

// ── 工厂函数 ──────────────────────────────────────────────

pub fn create_rerank_pipeline(
    config: &RerankConfig,
    engine: Option<std::sync::Arc<InferenceEngine>>,
) -> RerankPipeline {
    let mut pipeline = RerankPipeline::new();
    match config.backend.as_str() {
        "rule" => {
            pipeline.add_stage(Box::new(RuleReranker));
        },
        "cross_encoder" => {
            if let Some(eng) = engine {
                let model = config.cross_encoder_model.clone()
                    .unwrap_or_else(|| "bge-reranker-v2-m3.Q4_K_M.gguf".to_string());
                pipeline.add_stage(Box::new(CrossEncoderReranker::new(model, eng)));
            } else {
                tracing::warn!("No InferenceEngine provided, falling back to rule reranker");
                pipeline.add_stage(Box::new(RuleReranker));
            }
        },
        "pipeline" => {
            pipeline.add_stage(Box::new(RuleReranker));
            if let Some(eng) = engine {
                let model = config.cross_encoder_model.clone()
                    .unwrap_or_else(|| "bge-reranker-v2-m3.Q4_K_M.gguf".to_string());
                pipeline.add_stage(Box::new(CrossEncoderReranker::new(model, eng)));
            }
        },
        _ => {
            pipeline.add_stage(Box::new(RuleReranker));
        },
    }
    pipeline
}

// ── 测试 ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(id: &str, content: &str, score: f32) -> HybridSearchResult {
        HybridSearchResult {
            id: id.to_string(),
            document_id: "doc1".to_string(),
            chunk_index: 0,
            content: content.to_string(),
            vector_score: Some(score),
            bm25_score: None,
            combined_score: score,
        }
    }

    #[tokio::test]
    async fn test_rule_reranker_sorts_by_relevance() {
        let pipeline = create_rerank_pipeline(&RerankConfig::default(), None);
        let results = vec![
            make_result("1", "The quick brown fox", 0.5),
            make_result("2", "fox jumps over the lazy dog", 0.9),
        ];
        let reranked = pipeline.execute("lazy dog", results, &RerankConfig::default()).await;
        assert_eq!(reranked[0].id, "2");
    }

    #[tokio::test]
    async fn test_empty_results() {
        let pipeline = create_rerank_pipeline(&RerankConfig::default(), None);
        let reranked = pipeline.execute("test", vec![], &RerankConfig::default()).await;
        assert!(reranked.is_empty());
    }

    #[tokio::test]
    async fn test_disabled_config() {
        let mut config = RerankConfig::default();
        config.enabled = false;
        let pipeline = create_rerank_pipeline(&config, None);
        let results = vec![make_result("1", "test content", 0.8)];
        let reranked = pipeline.execute("test", results, &config).await;
        assert_eq!(reranked[0].rerank_score, 0.8);
    }

    #[tokio::test]
    async fn test_top_n_limit() {
        let mut config = RerankConfig::default();
        config.top_n = 2;
        config.candidate_k = 5;
        let pipeline = create_rerank_pipeline(&config, None);
        let results = vec![
            make_result("1", "a", 0.3), make_result("2", "b", 0.5),
            make_result("3", "c", 0.9), make_result("4", "d", 0.7),
        ];
        let reranked = pipeline.execute("test", results, &config).await;
        assert_eq!(reranked.len(), 2);
    }
}
```

- [ ] **Step 2: 编译 + 测试 + Commit**

```bash
cd src-tauri && cargo check -p axagent-core
cd src-tauri && cargo test -p axagent-core -- reranker::tests
```

```bash
git add src-tauri/crates/core/src/reranker.rs
git commit -m "feat: 重构 reranker 为 trait-based + candle 后端（RuleReranker + CrossEncoderReranker + Pipeline）"
```

---

### Task 5: Self-RAG 质检门控（candle 推理版）

**Files:**
- Create: `src-tauri/crates/core/src/self_rag.rs`
- Modify: `src-tauri/crates/core/src/lib.rs`（注册模块）

- [ ] **Step 1: 创建 self_rag.rs**

```rust
// self_rag.rs
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::inference::InferenceEngine;

// ── 配置 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfRagConfig {
    pub enabled: bool,
    /// GGUF 文件名
    pub judge_model: String,
    pub relevance_threshold: f32,
    pub quality_threshold: f32,
    pub max_retry_rounds: u8,
}

impl Default for SelfRagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            judge_model: "qwen2.5-0.5b.Q4_K_M.gguf".to_string(),
            relevance_threshold: 0.5,
            quality_threshold: 0.6,
            max_retry_rounds: 2,
        }
    }
}

// ── 判断结果 ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceJudgment {
    pub chunk_id: String,
    pub relevant: bool,
    pub score: f32,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub enum RetrievalQuality {
    Good(Vec<RelevanceJudgment>),
    Partial(Vec<RelevanceJudgment>),
    Poor(String),
}

// ── Gate 主体 ──────────────────────────────────────────────

pub struct SelfRagGate {
    config: SelfRagConfig,
    engine: Arc<InferenceEngine>,
}

impl SelfRagGate {
    pub fn new(config: SelfRagConfig, engine: Arc<InferenceEngine>) -> Self {
        Self { config, engine }
    }

    /// 批量判断每个 chunk 的相关性（串行调用本地推理）
    pub async fn judge_chunks(
        &self,
        query: &str,
        chunks: &[(String, String)],
    ) -> crate::error::Result<Vec<RelevanceJudgment>> {
        if !self.config.enabled || chunks.is_empty() {
            return Ok(chunks.iter().map(|(id, _)| RelevanceJudgment {
                chunk_id: id.clone(),
                relevant: true,
                score: 1.0,
                reason: "Self-RAG disabled".to_string(),
            }).collect());
        }

        let mut judgments = Vec::with_capacity(chunks.len());
        for (id, content) in chunks {
            match self.engine.judge(&self.config.judge_model, query, content).await {
                Ok(output) => {
                    judgments.push(RelevanceJudgment {
                        chunk_id: id.clone(),
                        relevant: output.relevant,
                        score: output.score,
                        reason: output.reason,
                    });
                },
                Err(e) => {
                    tracing::warn!("Judge failed for chunk {}: {}", id, e);
                    judgments.push(RelevanceJudgment {
                        chunk_id: id.clone(),
                        relevant: true,
                        score: 0.5,
                        reason: format!("judge error: {}", e),
                    });
                },
            }
        }
        Ok(judgments)
    }

    pub fn evaluate_quality(&self, judgments: &[RelevanceJudgment]) -> RetrievalQuality {
        if judgments.is_empty() {
            return RetrievalQuality::Poor("No judgments".to_string());
        }

        let relevant_count = judgments.iter().filter(|j| j.relevant).count();
        let ratio = relevant_count as f32 / judgments.len() as f32;

        if ratio >= self.config.quality_threshold {
            RetrievalQuality::Good(judgments.to_vec())
        } else if ratio >= 0.3 {
            RetrievalQuality::Partial(judgments.to_vec())
        } else {
            let avg_score = judgments.iter().map(|j| j.score).sum::<f32>() / judgments.len() as f32;
            RetrievalQuality::Poor(format!(
                "Low retrieval quality: {:.0}% relevant chunks (avg score {:.2})",
                ratio * 100.0, avg_score
            ))
        }
    }

    /// 生成精炼后的查询（用本地 judge 模型生成改写）
    pub async fn refine_query(
        &self,
        original: &str,
        quality_diag: &str,
    ) -> crate::error::Result<String> {
        let prompt = format!(
            "原始查询未能从知识库中检索到相关内容。诊断：{quality_diag}\n\n\
             将原始查询改写得更具体、更聚焦关键词，以提高检索命中率。\
             返回改写后的查询文本，不要额外说明。\n\n\
             原始查询：{original}\n\n改写查询："
        );
        let output = self.engine.judge(&self.config.judge_model, &prompt, "").await?;
        Ok(output.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_quality_good() {
        let config = SelfRagConfig::default();
        // 不依赖 engine 的纯逻辑测试——用 Arc::new(InferenceEngine::new()) 但 judge 不会实际被调用
        let gate = SelfRagGate { config, engine: Arc::new(InferenceEngine::new()) };
        let judgments = vec![
            RelevanceJudgment { chunk_id: "1".into(), relevant: true, score: 0.9, reason: "ok".into() },
            RelevanceJudgment { chunk_id: "2".into(), relevant: true, score: 0.8, reason: "ok".into() },
            RelevanceJudgment { chunk_id: "3".into(), relevant: true, score: 0.7, reason: "ok".into() },
            RelevanceJudgment { chunk_id: "4".into(), relevant: false, score: 0.3, reason: "no".into() },
            RelevanceJudgment { chunk_id: "5".into(), relevant: true, score: 0.85, reason: "ok".into() },
        ];
        match gate.evaluate_quality(&judgments) {
            RetrievalQuality::Good(_) => {},
            other => panic!("Expected Good, got {:?}", other),
        }
    }

    #[test]
    fn test_evaluate_quality_poor() {
        let config = SelfRagConfig::default();
        let gate = SelfRagGate { config, engine: Arc::new(InferenceEngine::new()) };
        let judgments = vec![
            RelevanceJudgment { chunk_id: "1".into(), relevant: false, score: 0.2, reason: "no".into() },
            RelevanceJudgment { chunk_id: "2".into(), relevant: false, score: 0.1, reason: "no".into() },
        ];
        match gate.evaluate_quality(&judgments) {
            RetrievalQuality::Poor(_) => {},
            other => panic!("Expected Poor, got {:?}", other),
        }
    }
}
```

- [ ] **Step 2: 在 lib.rs 中注册**

```rust
pub mod self_rag;
```

- [ ] **Step 3: 编译 + 测试 + Commit**

```bash
cd src-tauri && cargo check -p axagent-core
cd src-tauri && cargo test -p axagent-core -- self_rag::tests
```

```bash
git add src-tauri/crates/core/src/self_rag.rs src-tauri/crates/core/src/lib.rs
git commit -m "feat: 添加 Self-RAG 质检门控（candle 本地推理版）"
```

---

### Task 6: RAG 管线编排层 + 集成到 rag.rs

**Files:**
- Create: `src-tauri/crates/core/src/rag_pipeline.rs`
- Modify: `src-tauri/crates/core/src/rag.rs`
- Modify: `src-tauri/crates/core/src/types.rs`（扩展 SourceConfig + RAGPipelineConfig）
- Modify: `src-tauri/crates/core/src/lib.rs`

- [ ] **Step 1: 在 types.rs 中扩展 SourceConfig 和 RAGPipelineConfig**

```rust
// types.rs — SourceConfig 扩展
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig {
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    // 新增
    pub rerank_enabled: Option<bool>,
    pub self_rag_enabled: Option<bool>,
    pub query_enhancement_enabled: Option<bool>,
}

// types.rs — 全局管线配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RAGPipelineConfig {
    pub query_enhancement: EnhancementConfig,
    pub rerank: crate::reranker::RerankConfig,
    pub self_rag: crate::self_rag::SelfRagConfig,
}

impl Default for RAGPipelineConfig {
    fn default() -> Self {
        Self {
            query_enhancement: EnhancementConfig {
                enabled: false, strategy: EnhancementStrategy::Auto,
                max_variants: 3, combined_call: true,
            },
            rerank: crate::reranker::RerankConfig::default(),
            self_rag: crate::self_rag::SelfRagConfig::default(),
        }
    }
}
```

- [ ] **Step 2: 创建 rag_pipeline.rs**

（代码与上一版计划中的 Task 5 Step 2 一致，略）

- [ ] **Step 3: 在 rag.rs 中新增 collect_rag_context_with_pipeline**

（代码与上一版计划中的 Task 5 Step 3 一致，略）

- [ ] **Step 4: 注册 + 编译 + 测试 + Commit**

```bash
cd src-tauri && cargo check -p axagent-core
git add src-tauri/crates/core/src/rag_pipeline.rs \
        src-tauri/crates/core/src/rag.rs \
        src-tauri/crates/core/src/types.rs \
        src-tauri/crates/core/src/lib.rs
git commit -m "feat: 创建 RAG 管线编排层并集成到 collect_rag_context"
```

---

### Task 7: 前端知识库设置页 + 模型下载入口

**Files:**
- Modify: `src/components/settings/KnowledgeSettings.tsx`（高级 RAG 开关 + 模型下载）
- Modify: `src/locales/zh-CN/settings.json`
- Modify: `src/locales/en/settings.json`
- Modify: 其余 9 种语言文件

- [ ] **Step 1: 在语言文件中添加 i18n key**

```json
// zh-CN/settings.json
{
  "rag": {
    "advanced": "高级检索设置",
    "models": "本地模型管理",
    "modelDownload": "下载模型",
    "modelDelete": "删除",
    "modelDownloaded": "已下载",
    "modelNotDownloaded": "未下载",
    "modelDownloading": "下载中...",
    "rerankerModel": "重排序模型",
    "judgeModel": "裁判模型",
    "rerank": {
      "title": "智能重排序",
      "desc": "使用交叉编码器模型对检索结果进行语义精排（candle 本地推理）",
      "backend": "重排序后端",
      "backendRule": "规则评分",
      "backendCross": "跨编码器（本地）",
      "backendPipeline": "两级管线（规则+跨编码器）",
      "modelName": "Rerank 模型文件",
      "topN": "最终返回数",
      "candidateK": "候选集大小"
    },
    "selfRag": {
      "title": "自省式质检",
      "desc": "用本地裁判模型评估检索结果质量，自动纠正低质量检索（candle 本地推理）",
      "judgeModel": "裁判模型文件",
      "relevanceThreshold": "相关性阈值",
      "qualityThreshold": "质量阈值",
      "maxRetries": "最大纠正轮数"
    },
    "queryEnhancement": {
      "title": "查询增强",
      "desc": "自动改写和扩展用户查询以提高检索命中率",
      "strategy": "增强策略",
      "strategyNone": "不增强",
      "strategyHyde": "HyDE（假设文档嵌入）",
      "strategyMultiQuery": "多查询改写",
      "strategyDecomposition": "查询分解",
      "strategyAuto": "自动选择",
      "maxVariants": "最大变体数"
    }
  }
}
```

```json
// en/settings.json
{
  "rag": {
    "advanced": "Advanced Retrieval Settings",
    "models": "Local Model Management",
    "modelDownload": "Download",
    "modelDelete": "Delete",
    "modelDownloaded": "Downloaded",
    "modelNotDownloaded": "Not Downloaded",
    "modelDownloading": "Downloading...",
    "rerankerModel": "Reranker Model",
    "judgeModel": "Judge Model",
    "rerank": {
      "title": "Smart Reranking",
      "desc": "Semantic reranking using cross-encoder model (candle local inference)",
      "backend": "Rerank Backend",
      "backendRule": "Rule-based",
      "backendCross": "Cross-Encoder (Local)",
      "backendPipeline": "Two-stage (Rule + Cross-Encoder)",
      "modelName": "Rerank Model File",
      "topN": "Final Top-N",
      "candidateK": "Candidate Set Size"
    },
    "selfRag": {
      "title": "Self-RAG Quality Gate",
      "desc": "Evaluate retrieval quality with local judge model (candle local inference)",
      "judgeModel": "Judge Model File",
      "relevanceThreshold": "Relevance Threshold",
      "qualityThreshold": "Quality Threshold",
      "maxRetries": "Max Retry Rounds"
    },
    "queryEnhancement": {
      "title": "Query Enhancement",
      "desc": "Auto-rewrite and expand user queries for better retrieval",
      "strategy": "Enhancement Strategy",
      "strategyNone": "None",
      "strategyHyde": "HyDE",
      "strategyMultiQuery": "Multi-Query",
      "strategyDecomposition": "Decomposition",
      "strategyAuto": "Auto",
      "maxVariants": "Max Variants"
    }
  }
}
```

- [ ] **Step 2: 在 KnowledgeSettings.tsx 中添加模型管理区域 + 高级 RAG 设置**

```tsx
// KnowledgeSettings.tsx — 新增导入
import { DownloadOutlined, DeleteOutlined, CheckCircleOutlined, ClockCircleOutlined } from "@ant-design/icons";
import { Progress } from "antd";

// 在组件函数内部新增状态
const [modelList, setModelList] = useState<LocalModelInfo[]>([]);
const [downloading, setDownloading] = useState<string | null>(null);

interface LocalModelInfo {
  name: string;
  file_path: string;
  size_bytes: number;
  downloaded_at: string;
  sha256: string;
  model_type: "Reranker" | "Judge";
  is_downloaded: boolean;
}

// 获取模型列表
const refreshModels = useCallback(async () => {
  try {
    const models = await invoke<LocalModelInfo[]>("list_local_models");
    setModelList(models);
  } catch { /* ignore */ }
}, []);

useEffect(() => { refreshModels(); }, [refreshModels]);

// 下载模型
const handleDownload = async (filename: string) => {
  setDownloading(filename);
  try {
    await invoke("download_model", { filename });
    message.success(t("settings.rag.modelDownloaded"));
    refreshModels();
  } catch (e: any) {
    message.error(e?.message ?? String(e));
  } finally {
    setDownloading(null);
  }
};

// 在 JSX 中添加
<Collapse ghost items={[
  {
    key: "local-models",
    label: <Space><DownloadOutlined />{t("settings.rag.models")}</Space>,
    children: (
      <List
        dataSource={modelList}
        renderItem={(model: LocalModelInfo) => (
          <List.Item
            actions={[
              model.is_downloaded ? (
                <Popconfirm title={t("settings.rag.modelDelete")} onConfirm={() => handleDelete(model.name)}>
                  <Button size="small" danger icon={<DeleteOutlined />} />
                </Popconfirm>
              ) : (
                <Button size="small" type="primary"
                  loading={downloading === model.name}
                  icon={<DownloadOutlined />}
                  onClick={() => handleDownload(model.name)}>
                  {downloading === model.name ? t("settings.rag.modelDownloading") : t("settings.rag.modelDownload")}
                </Button>
              ),
            ]}
          >
            <List.Item.Meta
              avatar={model.is_downloaded
                ? <CheckCircleOutlined style={{ color: token.colorSuccess, fontSize: 20 }} />
                : <ClockCircleOutlined style={{ color: token.colorTextSecondary, fontSize: 20 }} />
              }
              title={model.name}
              description={
                <Space>
                  <Tag>{model.model_type === "Reranker" ? t("settings.rag.rerankerModel") : t("settings.rag.judgeModel")}</Tag>
                  <span>{formatBytes(model.size_bytes)}</span>
                  {model.is_downloaded && <span style={{ color: token.colorTextSecondary }}>{model.downloaded_at}</span>}
                </Space>
              }
            />
          </List.Item>
        )}
      />
    ),
  },
  {
    key: "advanced-rag",
    label: <Space><SettingOutlined />{t("settings.rag.advanced")}</Space>,
    children: (
      <>
        {/* 查询增强、重排序、Self-RAG 的开关 UI —— 与上一版计划基本一致，
            但移除 ollama_endpoint 相关字段，模型名改为 GGUF 文件名显示 */}
        {/* 此处省略具体的 Divider + Switch + Select 行，结构与上一版相同 */}
      </>
    ),
  },
]} />
```

- [ ] **Step 3: 在 src-tauri 侧新增两个 Tauri 命令**

```rust
// src-tauri/src/commands/mod.rs — 新增声明
pub mod local_models;

// src-tauri/src/commands/local_models.rs
use tauri::State;
use axagent_core::model_downloader::ModelDownloader;
use crate::app_state::AppState;

#[tauri::command]
pub async fn list_local_models() -> Result<Vec<axagent_core::model_downloader::LocalModelInfo>, String> {
    let dl = ModelDownloader::new();
    Ok(dl.list_all_models())
}

#[tauri::command]
pub async fn download_model(filename: String) -> Result<(), String> {
    let dl = ModelDownloader::new();
    let presets = ModelDownloader::preset_models();
    let preset = presets.iter()
        .find(|p| p.filename == filename)
        .ok_or_else(|| format!("Unknown model: {}", filename))?;
    dl.ensure_model(preset).await.map_err(|e| e.to_string())?;
    Ok(())
}
```

```rust
// src-tauri/src/lib.rs — generate_handler![] 中注册
list_local_models,
download_model,
```

- [ ] **Step 4: 同步其余 9 种语言文件**

- [ ] **Step 5: TypeScript 检查 + dprint 格式化**

```bash
npm run typecheck
npm run format
```

- [ ] **Step 6: Commit**

```bash
git add src/components/settings/KnowledgeSettings.tsx \
        src/locales/ \
        src-tauri/src/commands/local_models.rs \
        src-tauri/src/commands/mod.rs \
        src-tauri/src/lib.rs
git commit -m "feat: 前端添加模型下载入口 + 高级 RAG 配置 UI（candle 自运行版）"
```

---

### Task 8: 端到端集成验证 + 最终检查

- [ ] **Step 1: 全量编译检查**

```bash
cd src-tauri && cargo check 2>&1
```
Expected: 无编译错误。

- [ ] **Step 2: 运行所有 core crate 测试**

```bash
cd src-tauri && cargo test -p axagent-core 2>&1
```
Expected: 所有测试 PASS。

- [ ] **Step 3: 前端 TypeScript 检查**

```bash
npm run typecheck
```

- [ ] **Step 4: 前端格式化**

```bash
npm run format
```

- [ ] **Step 5: clippy 零警告**

```bash
cd src-tauri && cargo clippy -- -D warnings 2>&1
```

- [ ] **Step 6: rustfmt 检查**

```bash
cd src-tauri && cargo fmt --check 2>&1
```

- [ ] **Step 7: Commit**

```bash
git add -u
git commit -m "chore: P0 高级 RAG 端到端集成验证通过（candle 自运行版）"
```

---

## 验证检查清单

- [ ] `cargo check` 零错误
- [ ] `cargo test -p axagent-core` 所有测试通过
- [ ] `cargo clippy -- -D warnings` 零警告
- [ ] `cargo fmt --check` 格式化通过
- [ ] `npm run typecheck` 零新增错误
- [ ] `npm run format` 格式化通过
- [ ] 新建模块已在 `lib.rs` 注册
- [ ] 新建 Tauri 命令已在 `commands/mod.rs` + `lib.rs` generate_handler 注册
- [ ] 所有 11 种语言文件已同步新增 i18n key
- [ ] candle 依赖编译通过（首次编译较慢，约 5 分钟）
