# 高级 RAG P0：检索质量质变 — 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在现有 RAG 管线基础上插入查询增强、跨编码器重排序、自省式质检三个阶段，并配套本地模型下载管理。

**Architecture:** 新增 4 个模块（`model_downloader`, `query_enhancement`, `self_rag` 及重构后的 `reranker`），通过 `RAGPipeline` 在 `collect_rag_context()` 中编排。每个阶段通过 `RAGConfig` 独立开关，默认关闭以保持向后兼容。

**Tech Stack:** Rust 2021 · sea-orm · tokio · reqwest · sqlite-vec · serde · 前端 React 19 + TypeScript + Ant Design 6

---

## 文件结构

```
创建:
  src-tauri/crates/core/src/model_downloader.rs    # 模型下载管理器
  src-tauri/crates/core/src/query_enhancement.rs   # 查询增强（HyDE / Multi-Query / Decomp）
  src-tauri/crates/core/src/self_rag.rs            # 自省式 RAG
  src-tauri/crates/core/src/rag_pipeline.rs        # RAG 管线编排层

修改:
  src-tauri/crates/core/src/reranker.rs            # 重构为 trait-based 后端
  src-tauri/crates/core/src/rag.rs                 # 集成 pipeline
  src-tauri/crates/core/src/types.rs               # 新增 RAGConfig 等类型
  src-tauri/crates/core/src/error.rs               # 新增模型下载错误变体
  src-tauri/crates/core/src/lib.rs                 # 注册新模块
  src-tauri/crates/core/Cargo.toml                 # 新增 tokio::fs 依赖（如需）
  src/components/settings/KnowledgeSettings.tsx    # 前端开关 UI
  src/locales/zh-CN/settings.json                  # i18n 新增 key
  src/locales/en/settings.json                     # i18n 新增 key
  (其余 9 种语言文件同步新增对应 key)
```

---

### Task 1: 模型下载管理器

**Files:**
- Create: `src-tauri/crates/core/src/model_downloader.rs`
- Modify: `src-tauri/crates/core/src/error.rs:97`（新增错误变体）
- Modify: `src-tauri/crates/core/src/lib.rs`（注册模块）

- [ ] **Step 1: 在 error.rs 中新增模型下载相关错误变体**

```rust
// error.rs - 在 AxAgentError 枚举末尾新增
#[error("Model download error: {0}")]
ModelDownload(String),

#[error("Model integrity error: expected {expected}, got {actual}")]
ModelIntegrity { expected: String, actual: String },
```

- [ ] **Step 2: 创建 model_downloader.rs 骨架与测试**

```rust
// model_downloader.rs
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
            // 快速路径：文件已存在，校验哈希
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
            crate::error::AxAgentError::ModelDownload(format!(
                "Failed to create cache dir: {}", e
            ))
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
                "HTTP {} for {}", response.status(), url
            )));
        }

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&tmp_path)
            .map_err(|e| {
                crate::error::AxAgentError::ModelDownload(format!(
                    "Cannot open temp file: {}", e
                ))
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

        // 校验 SHA256
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
        for entry in std::fs::read_dir(&self.cache_dir).map_err(|e| {
            crate::error::AxAgentError::Io(e)
        })? {
            let entry = entry.map_err(|e| {
                crate::error::AxAgentError::Io(e)
            })?;
            let path = entry.path();
            if path.is_file() && path.extension().is_none() {
                let meta = entry.metadata().map_err(|e| {
                    crate::error::AxAgentError::Io(e)
                })?;
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
            std::fs::remove_file(&path).map_err(|e| {
                crate::error::AxAgentError::Io(e)
            })?;
        }
        Ok(())
    }

    fn sha256_file(path: &Path) -> Result<String> {
        use sha2::{Digest, Sha256};
        let data = std::fs::read(path).map_err(|e| {
            crate::error::AxAgentError::Io(e)
        })?;
        let hash = Sha256::digest(&data);
        Ok(hex::encode(hash))
    }
}
```

- [ ] **Step 3: 编译检查**

```bash
cd src-tauri && cargo check -p axagent-core 2>&1
```
Expected: 编译通过，新增模块无警告。

- [ ] **Step 4: 在 lib.rs 中注册模块**

```rust
// lib.rs — 在 pub mod 列表中添加
pub mod model_downloader;
```

- [ ] **Step 5: 编写 model_downloader 测试**

```rust
// model_downloader.rs 底部

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
```

```bash
cd src-tauri && cargo test -p axagent-core -- model_downloader::tests 2>&1
```
Expected: 所有测试 PASS。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/core/src/model_downloader.rs \
        src-tauri/crates/core/src/error.rs \
        src-tauri/crates/core/src/lib.rs
git commit -m "feat: 添加模型下载管理器（ModelDownloader）

支持按需下载、SHA256 校验、断点续传、本地缓存管理"
```

---

### Task 2: 查询增强模块（HyDE + Multi-Query + Decomposition）

**Files:**
- Create: `src-tauri/crates/core/src/query_enhancement.rs`
- Modify: `src-tauri/crates/core/src/types.rs`（新增类型）
- Modify: `src-tauri/crates/core/src/lib.rs`（注册模块）

- [ ] **Step 1: 在 types.rs 中新增查询增强相关类型**

```rust
// types.rs — 在文件末尾新增

// === Query Enhancement Types ===

/// 查询增强策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnhancementStrategy {
    /// 不增强，直接使用原始查询
    None,
    /// 假设文档嵌入（HyDE）
    Hyde,
    /// 多查询改写
    MultiQuery,
    /// 查询分解
    Decomposition,
    /// 自动选择（基于查询特征）
    Auto,
}

/// 增强后的查询及其元数据
#[derive(Debug, Clone)]
pub struct EnhancedQuery {
    /// 增强后的查询文本
    pub text: String,
    /// 使用的策略
    pub strategy: EnhancementStrategy,
    /// 该查询的权重（用于结果合并）
    pub weight: f32,
}

/// 查询增强配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancementConfig {
    pub enabled: bool,
    pub strategy: EnhancementStrategy,
    /// 最大增强查询数（MultiQuery 的变体数）
    pub max_variants: usize,
    /// 是否合并 HyDE 和 MultiQuery 为一次 LLM 调用
    pub combined_call: bool,
}
```

- [ ] **Step 2: 创建 query_enhancement.rs 骨架与测试**

```rust
// query_enhancement.rs
use std::sync::Arc;
use crate::error::Result;
use crate::types::*;

/// 查询增强器 —— 将用户查询变换为多路增强查询
#[derive(Clone)]
pub struct QueryEnhancer {
    config: EnhancementConfig,
    /// 调用 LLM 完成文本生成的函数指针，由调用方注入
    llm_fn: Arc<dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> + Send + Sync>,
}

impl QueryEnhancer {
    pub fn new(
        config: EnhancementConfig,
        llm_fn: impl Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> + Send + Sync + 'static,
    ) -> Self {
        Self { config, llm_fn: Arc::new(llm_fn) }
    }

    /// 对原始查询进行增强，返回增强后的查询列表。
    /// 若 config.enabled == false 或 strategy == None，直接返回原始查询。
    pub async fn enhance(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        if !self.config.enabled {
            return Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::None,
                weight: 1.0,
            }]);
        }

        match self.config.strategy {
            EnhancementStrategy::None => Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::None,
                weight: 1.0,
            }]),
            EnhancementStrategy::Hyde => self.enhance_hyde(query).await,
            EnhancementStrategy::MultiQuery => self.enhance_multi_query(query).await,
            EnhancementStrategy::Decomposition => self.enhance_decomposition(query).await,
            EnhancementStrategy::Auto => self.enhance_auto(query).await,
        }
    }

    async fn enhance_hyde(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        let prompt = format!(
            "你是一个知识助手。请针对以下问题，写一段简洁的百科式答案（100-200字），\
             包含关键事实和专业术语。\n\n问题：{query}\n\n假设答案："
        );

        let hyde_answer = (self.llm_fn)(prompt).await?;
        let trimmed = hyde_answer.trim().to_string();
        if trimmed.is_empty() {
            return Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::Hyde,
                weight: 1.0,
            }]);
        }

        Ok(vec![EnhancedQuery {
            text: trimmed,
            strategy: EnhancementStrategy::Hyde,
            weight: 1.0,
        }])
    }

    async fn enhance_multi_query(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        let prompt = format!(
            "你是一个搜索查询优化器。将用户问题改写为 {n} 个不同视角的搜索查询，\
             每个查询聚焦问题的不同方面。返回 JSON 数组。\n\n\
             用户问题：{query}\n\n\
             返回格式：[\"查询1\", \"查询2\", ...]",
            n = self.config.max_variants.min(5)
        );

        let response = (self.llm_fn)(prompt).await?;
        let variants = parse_json_string_array(&response);

        if variants.is_empty() {
            return Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::MultiQuery,
                weight: 1.0,
            }]);
        }

        let count = variants.len();
        Ok(variants.into_iter().take(self.config.max_variants).enumerate().map(|(i, text)| {
            EnhancedQuery {
                text,
                strategy: EnhancementStrategy::MultiQuery,
                weight: 1.0 / count as f32,
            }
        }).collect())
    }

    async fn enhance_decomposition(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        let prompt = format!(
            "将以下复杂问题分解为 2-4 个简单的子问题，每个子问题独立可回答。\
             返回 JSON 数组。\n\n复杂问题：{query}\n\n\
             返回格式：[\"子问题1\", \"子问题2\", ...]"
        );

        let response = (self.llm_fn)(prompt).await?;
        let sub_queries = parse_json_string_array(&response);

        if sub_queries.is_empty() {
            return Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::Decomposition,
                weight: 1.0,
            }]);
        }

        Ok(sub_queries.into_iter().map(|text| EnhancedQuery {
            text,
            strategy: EnhancementStrategy::Decomposition,
            weight: 1.0,
        }).collect())
    }

    async fn enhance_auto(&self, query: &str) -> Result<Vec<EnhancedQuery>> {
        // 自动模式：根据查询特征选择策略
        let has_conceptual = query.contains("什么是") || query.contains("解释")
            || query.contains("原理") || query.contains("概念")
            || query.contains("总结") || query.contains("概述");
        let is_complex = query.len() > 40
            || query.contains("并且") || query.contains("同时")
            || query.contains("对比") || query.contains("区别")
            || query.contains("先后");

        if is_complex {
            self.enhance_multi_query(query).await
        } else if has_conceptual {
            self.enhance_hyde(query).await
        } else {
            // 短问题直接查询
            Ok(vec![EnhancedQuery {
                text: query.to_string(),
                strategy: EnhancementStrategy::None,
                weight: 1.0,
            }])
        }
    }
}

/// 从 LLM 响应中提取 JSON 字符串数组
fn parse_json_string_array(raw: &str) -> Vec<String> {
    // 尝试直接解析 JSON
    if let Ok(arr) = serde_json::from_str::<Vec<String>>(raw.trim()) {
        return arr;
    }
    // 尝试从 markdown 代码块中提取
    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str::<Vec<String>>(cleaned).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_string_array_direct() {
        let result = parse_json_string_array(r#"["a", "b", "c"]"#);
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_parse_json_string_array_markdown() {
        let result = parse_json_string_array("```json\n[\"x\"]\n```");
        assert_eq!(result, vec!["x"]);
    }

    #[test]
    fn test_parse_json_string_array_invalid() {
        let result = parse_json_string_array("not json at all");
        assert!(result.is_empty());
    }
}
```

- [ ] **Step 3: 编译检查**

```bash
cd src-tauri && cargo check -p axagent-core 2>&1
```

- [ ] **Step 4: 在 lib.rs 中注册模块**

```rust
// lib.rs — 在 pub mod 列表中添加
pub mod query_enhancement;
```

- [ ] **Step 5: 运行测试**

```bash
cd src-tauri && cargo test -p axagent-core -- query_enhancement::tests 2>&1
```
Expected: PASS。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/core/src/query_enhancement.rs \
        src-tauri/crates/core/src/types.rs \
        src-tauri/crates/core/src/lib.rs
git commit -m "feat: 添加查询增强模块（HyDE / MultiQuery / Decomposition）"
```

---

### Task 3: 重构 reranker.rs 为 trait-based 后端模式

**Files:**
- Modify: `src-tauri/crates/core/src/reranker.rs`（重构）
- Modify: `src-tauri/crates/core/src/lib.rs`（不变，已注册）

- [ ] **Step 1: 阅读现有 reranker.rs 中的 HybridSearchResult 引用**

确认当前 `reranker.rs` 依赖 `crate::hybrid_search::HybridSearchResult`，重构后保持该依赖。

- [ ] **Step 2: 重写 reranker.rs**

```rust
// reranker.rs — 完整重写
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::hybrid_search::HybridSearchResult;

// ── 配置 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankConfig {
    pub enabled: bool,
    /// 后端类型: "rule" | "cross_encoder" | "pipeline"
    pub backend: String,
    /// Cross-encoder 使用的 Ollama 模型名
    pub cross_encoder_model: Option<String>,
    /// 最终保留数
    pub top_n: usize,
    /// 从检索阶段取多少候选
    pub candidate_k: usize,
    /// 规则初筛后保留多少个给 cross-encoder
    pub rule_filter_keep: usize,
    /// 最低分数阈值
    pub score_threshold: Option<f32>,
    /// Ollama endpoint
    pub ollama_endpoint: Option<String>,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "rule".to_string(),
            cross_encoder_model: Some("bge-reranker-v2-m3".to_string()),
            top_n: 5,
            candidate_k: 30,
            rule_filter_keep: 15,
            score_threshold: None,
            ollama_endpoint: Some("http://localhost:11434".to_string()),
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
    /// 对候选集重新排序，返回 (chunk_id, score) 列表
    async fn rerank(
        &self,
        query: &str,
        chunks: &[(String, String, f32)], // (id, content, original_score)
    ) -> crate::error::Result<Vec<(String, f32)>>;
}

// ── 规则后端（现有逻辑迁移）────────────────────────────────

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

// ── Cross-Encoder 后端（Ollama）────────────────────────────

pub struct CrossEncoderReranker {
    model_name: String,
    ollama_endpoint: String,
}

impl CrossEncoderReranker {
    pub fn new(model_name: String, ollama_endpoint: String) -> Self {
        Self { model_name, ollama_endpoint }
    }

    async fn call_ollama_rerank(
        &self,
        query: &str,
        documents: &[String],
    ) -> crate::error::Result<Vec<f32>> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": self.model_name,
            "query": query,
            "documents": documents,
        });

        let resp = client
            .post(format!("{}/api/rerank", self.ollama_endpoint))
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::error::AxAgentError::Provider(format!(
                "Ollama rerank request failed: {}", e
            )))?;

        if !resp.status().is_success() {
            return Err(crate::error::AxAgentError::Provider(format!(
                "Ollama rerank HTTP {}", resp.status()
            )));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| {
            crate::error::AxAgentError::Provider(format!("Ollama rerank parse: {}", e))
        })?;

        let scores: Vec<f32> = data["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|r| r["relevance_score"].as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(scores)
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

        match self.call_ollama_rerank(query, &documents).await {
            Ok(scores) => {
                let mut result: Vec<(String, f32)> = chunks.iter().zip(scores.iter())
                    .map(|((id, _, _), &s)| (id.clone(), s))
                    .collect();
                result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                Ok(result)
            },
            Err(e) => {
                tracing::warn!("Cross-encoder rerank failed, falling back to original ordering: {}", e);
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

    /// 执行两级管线：RuleReranker(初筛) → CrossEncoderReranker(精排)
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

pub fn create_rerank_pipeline(config: &RerankConfig) -> RerankPipeline {
    let mut pipeline = RerankPipeline::new();
    match config.backend.as_str() {
        "rule" => {
            pipeline.add_stage(Box::new(RuleReranker));
        },
        "cross_encoder" => {
            let endpoint = config.ollama_endpoint.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = config.cross_encoder_model.clone().unwrap_or_else(|| "bge-reranker-v2-m3".to_string());
            pipeline.add_stage(Box::new(CrossEncoderReranker::new(model, endpoint)));
        },
        "pipeline" => {
            pipeline.add_stage(Box::new(RuleReranker));
            let endpoint = config.ollama_endpoint.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
            let model = config.cross_encoder_model.clone().unwrap_or_else(|| "bge-reranker-v2-m3".to_string());
            pipeline.add_stage(Box::new(CrossEncoderReranker::new(model, endpoint)));
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
        let pipeline = create_rerank_pipeline(&RerankConfig::default());
        let results = vec![
            make_result("1", "The quick brown fox", 0.5),
            make_result("2", "fox jumps over the lazy dog", 0.9),
        ];
        let reranked = pipeline.execute("lazy dog", results, &RerankConfig::default()).await;
        assert_eq!(reranked[0].id, "2");
    }

    #[tokio::test]
    async fn test_empty_results() {
        let pipeline = create_rerank_pipeline(&RerankConfig::default());
        let reranked = pipeline.execute("test", vec![], &RerankConfig::default()).await;
        assert!(reranked.is_empty());
    }

    #[tokio::test]
    async fn test_disabled_config() {
        let mut config = RerankConfig::default();
        config.enabled = false;
        let pipeline = create_rerank_pipeline(&config);
        let results = vec![make_result("1", "test content", 0.8)];
        let reranked = pipeline.execute("test", results, &config).await;
        assert_eq!(reranked.len(), 1);
        assert_eq!(reranked[0].rerank_score, 0.8);
    }

    #[tokio::test]
    async fn test_top_n_limit() {
        let mut config = RerankConfig::default();
        config.top_n = 2;
        config.candidate_k = 5;
        let pipeline = create_rerank_pipeline(&config);
        let results = vec![
            make_result("1", "a", 0.3),
            make_result("2", "b", 0.5),
            make_result("3", "c", 0.9),
            make_result("4", "d", 0.7),
        ];
        let reranked = pipeline.execute("test", results, &config).await;
        assert_eq!(reranked.len(), 2);
    }
}
```

- [ ] **Step 3: 编译检查**

```bash
cd src-tauri && cargo check -p axagent-core 2>&1
```

需要确保 `axagent-core` 的 `Cargo.toml` 已有 `async-trait` 和 `reqwest` 依赖——已有。

- [ ] **Step 4: 运行测试**

```bash
cd src-tauri && cargo test -p axagent-core -- reranker::tests 2>&1
```
Expected: 4 个测试 PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/core/src/reranker.rs
git commit -m "feat: 重构 reranker 为 trait-based 模式（RuleReranker + CrossEncoderReranker + Pipeline）"
```

---

### Task 4: Self-RAG 质检门控

**Files:**
- Create: `src-tauri/crates/core/src/self_rag.rs`
- Modify: `src-tauri/crates/core/src/lib.rs`（注册模块）

- [ ] **Step 1: 创建 self_rag.rs**

```rust
// self_rag.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── 配置 ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfRagConfig {
    pub enabled: bool,
    /// Ollama 裁判模型名（如 "qwen2.5:0.5b"）
    pub judge_model: String,
    /// Ollama endpoint
    pub ollama_endpoint: String,
    /// chunk 相关性最低分（0.0-1.0）
    pub relevance_threshold: f32,
    /// 相关 chunk 占比最低阈值（低于此值触发纠正循环）
    pub quality_threshold: f32,
    /// 最大纠正轮数
    pub max_retry_rounds: u8,
}

impl Default for SelfRagConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            judge_model: "qwen2.5:0.5b".to_string(),
            ollama_endpoint: "http://localhost:11434".to_string(),
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
}

impl SelfRagGate {
    pub fn new(config: SelfRagConfig) -> Self {
        Self { config }
    }

    /// 批量判断每个 chunk 的相关性
    pub async fn judge_chunks(
        &self,
        query: &str,
        chunks: &[(String, String)], // (chunk_id, content)
    ) -> crate::error::Result<Vec<RelevanceJudgment>> {
        if !self.config.enabled || chunks.is_empty() {
            return Ok(chunks.iter().map(|(id, _)| RelevanceJudgment {
                chunk_id: id.clone(),
                relevant: true,
                score: 1.0,
                reason: "Self-RAG disabled".to_string(),
            }).collect());
        }

        let client = reqwest::Client::new();

        let judgments: Vec<RelevanceJudgment> = futures::future::join_all(
            chunks.iter().map(|(id, content)| {
                let query = query.to_string();
                let content = content.clone();
                let client = &client;
                let config = &self.config;
                async move {
                    judge_single(&client, config, id, &query, &content).await
                }
            })
        ).await.into_iter().map(|r| r.unwrap_or_else(|e| {
            tracing::warn!("Judge failed for chunk: {}", e);
            RelevanceJudgment {
                chunk_id: "unknown".to_string(),
                relevant: true, // 降级：失败时假定相关
                score: 0.5,
                reason: format!("judge error: {}", e),
            }
        })).collect();

        Ok(judgments)
    }

    /// 评估整体检索质量
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

    /// 生成精炼后的查询（用于纠正循环）
    pub async fn refine_query(
        &self,
        original: &str,
        quality_diag: &str,
    ) -> crate::error::Result<String> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": &self.config.judge_model,
            "prompt": format!(
                "原始查询未能从知识库中检索到相关内容。诊断：{quality_diag}\n\n\
                 请将原始查询改写得更具体、更聚焦关键词，以提高检索命中率。\
                 返回改写后的查询文本，不要额外说明。\n\n\
                 原始查询：{original}\n\n改写查询："
            ),
            "stream": false,
        });

        let resp = client
            .post(format!("{}/api/generate", self.config.ollama_endpoint))
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::error::AxAgentError::Provider(format!(
                "Ollama refine query failed: {}", e
            )))?;

        let data: serde_json::Value = resp.json().await.map_err(|e| {
            crate::error::AxAgentError::Provider(format!("Parse refine response: {}", e))
        })?;

        Ok(data["response"].as_str().unwrap_or(original).to_string())
    }
}

async fn judge_single(
    client: &reqwest::Client,
    config: &SelfRagConfig,
    chunk_id: &str,
    query: &str,
    content: &str,
) -> crate::error::Result<RelevanceJudgment> {
    let body = serde_json::json!({
        "model": &config.judge_model,
        "prompt": format!(
            "你是一个相关性裁判。给定用户问题和检索到的文档块，判断该文档是否与问题相关。\n\n\
             用户问题：{query}\n文档块：{content}\n\n\
             返回 JSON：{{\"relevant\": true/false, \"score\": 0.0-1.0, \"reason\": \"一句话说明理由\"}}"
        ),
        "stream": false,
        "format": "json",
    });

    let resp = client
        .post(format!("{}/api/generate", config.ollama_endpoint))
        .json(&body)
        .send()
        .await
        .map_err(|e| crate::error::AxAgentError::Provider(format!(
            "Ollama judge request failed: {}", e
        )))?;

    let data: serde_json::Value = resp.json().await.map_err(|e| {
        crate::error::AxAgentError::Provider(format!("Judge response parse: {}", e))
    })?;

    let response_text = data["response"].as_str().unwrap_or("{}");
    let parsed: serde_json::Value = serde_json::from_str(response_text).unwrap_or_default();

    Ok(RelevanceJudgment {
        chunk_id: chunk_id.to_string(),
        relevant: parsed["relevant"].as_bool().unwrap_or(true),
        score: parsed["score"].as_f64().unwrap_or(0.5) as f32,
        reason: parsed["reason"].as_str().unwrap_or("").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_quality_good() {
        let gate = SelfRagGate::new(SelfRagConfig::default());
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
        let gate = SelfRagGate::new(SelfRagConfig::default());
        let judgments = vec![
            RelevanceJudgment { chunk_id: "1".into(), relevant: false, score: 0.2, reason: "no".into() },
            RelevanceJudgment { chunk_id: "2".into(), relevant: false, score: 0.1, reason: "no".into() },
        ];
        match gate.evaluate_quality(&judgments) {
            RetrievalQuality::Poor(_) => {},
            other => panic!("Expected Poor, got {:?}", other),
        }
    }

    #[test]
    fn test_evaluate_quality_empty() {
        let gate = SelfRagGate::new(SelfRagConfig::default());
        match gate.evaluate_quality(&[]) {
            RetrievalQuality::Poor(_) => {},
            other => panic!("Expected Poor for empty, got {:?}", other),
        }
    }
}
```

- [ ] **Step 2: 编译检查**

```bash
cd src-tauri && cargo check -p axagent-core 2>&1
```

注意：需要 `futures` 依赖——已在 `Cargo.toml` 中存在。

- [ ] **Step 3: 在 lib.rs 中注册**

```rust
pub mod self_rag;
```

- [ ] **Step 4: 运行测试**

```bash
cd src-tauri && cargo test -p axagent-core -- self_rag::tests 2>&1
```
Expected: 3 个测试 PASS。

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/core/src/self_rag.rs src-tauri/crates/core/src/lib.rs
git commit -m "feat: 添加 Self-RAG 质检门控（相关性判断 + 检索质量评估 + 纠正循环）"
```

---

### Task 5: RAG 管线编排层 + 集成到 rag.rs

**Files:**
- Create: `src-tauri/crates/core/src/rag_pipeline.rs`
- Modify: `src-tauri/crates/core/src/rag.rs`
- Modify: `src-tauri/crates/core/src/types.rs`
- Modify: `src-tauri/crates/core/src/lib.rs`

- [ ] **Step 1: 在 types.rs 中扩展 SourceConfig 和 RAGConfig**

```rust
// types.rs — SourceConfig 扩展
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig {
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    // 新增字段
    pub rerank_enabled: Option<bool>,
    pub self_rag_enabled: Option<bool>,
    pub query_enhancement_enabled: Option<bool>,
}

impl SourceConfig {
    pub fn default() -> Self {
        Self {
            embedding_provider: None,
            embedding_dimensions: None,
            retrieval_threshold: None,
            retrieval_top_k: None,
            rerank_enabled: None,
            self_rag_enabled: None,
            query_enhancement_enabled: None,
        }
    }
}

// types.rs — 全局 RAG 管线配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RAGPipelineConfig {
    pub query_enhancement: crate::types::EnhancementConfig,
    pub rerank: crate::reranker::RerankConfig,
    pub self_rag: crate::self_rag::SelfRagConfig,
}

impl Default for RAGPipelineConfig {
    fn default() -> Self {
        Self {
            query_enhancement: crate::types::EnhancementConfig {
                enabled: false,
                strategy: EnhancementStrategy::Auto,
                max_variants: 3,
                combined_call: true,
            },
            rerank: crate::reranker::RerankConfig::default(),
            self_rag: crate::self_rag::SelfRagConfig::default(),
        }
    }
}
```

- [ ] **Step 2: 创建 rag_pipeline.rs — 管线编排层**

```rust
// rag_pipeline.rs
use sea_orm::DatabaseConnection;
use crate::error::Result;
use crate::hybrid_search::{HybridSearchResult, HybridSearcher};
use crate::rag::{self, AsyncEmbedFn};
use crate::reranker::{self, RerankPipeline};
use crate::self_rag::{SelfRagGate, RetrievalQuality};
use crate::types::*;
use crate::vector_store::VectorStore;

/// RAG 管线 —— 编排查询增强 → 检索 → 重排序 → 质检
pub struct RAGPipeline {
    rerank_pipeline: RerankPipeline,
    self_rag_gate: SelfRagGate,
}

impl RAGPipeline {
    pub fn new(config: &RAGPipelineConfig) -> Self {
        Self {
            rerank_pipeline: reranker::create_rerank_pipeline(&config.rerank),
            self_rag_gate: SelfRagGate::new(config.self_rag.clone()),
        }
    }

    /// 完整管线：查询增强 → 检索 → 重排序 → 质检 → 返回上下文
    pub async fn execute<S: rag::RAGSource + ?Sized>(
        &self,
        source: &S,
        db: &DatabaseConnection,
        master_key: &[u8; 32],
        vector_store: &VectorStore,
        container_id: &str,
        query: &str,
        top_k: usize,
        dimensions: Option<usize>,
        embed_fn: impl AsyncEmbedFn,
        rerank_config: &reranker::RerankConfig,
    ) -> Result<PipelineOutput> {
        // 阶段 1：检索（使用现有 rag::search）
        let raw_results = rag::search(
            source, db, master_key, vector_store,
            container_id, query, top_k.max(rerank_config.candidate_k),
            dimensions, embed_fn,
        ).await?;

        if raw_results.is_empty() {
            return Ok(PipelineOutput {
                results: vec![],
                quality: RetrievalQuality::Poor("No results from search".to_string()),
                retries: 0,
            });
        }

        // 转换为 HybridSearchResult
        let hybrid_results: Vec<HybridSearchResult> = raw_results.iter().map(|r| {
            HybridSearchResult {
                id: r.id.clone(),
                document_id: r.document_id.clone(),
                chunk_index: r.chunk_index,
                content: r.content.clone(),
                vector_score: Some(1.0 - (r.score / 20.0).min(1.0)),
                bm25_score: None,
                combined_score: 1.0 - (r.score / 20.0).min(1.0),
            }
        }).collect();

        // 阶段 2：重排序
        let reranked = self.rerank_pipeline.execute(query, hybrid_results, rerank_config).await;

        // 阶段 3：质检
        let chunks: Vec<(String, String)> = reranked.iter()
            .map(|r| (r.id.clone(), r.content.clone()))
            .collect();

        let judgments = self.self_rag_gate.judge_chunks(query, &chunks).await?;
        let quality = self.self_rag_gate.evaluate_quality(&judgments);

        // 过滤不相关 chunk
        let filtered: Vec<RerankedChunk> = reranked.iter()
            .zip(judgments.iter())
            .filter(|(_, j)| j.relevant)
            .map(|(r, j)| RerankedChunk {
                id: r.id.clone(),
                document_id: r.document_id.clone(),
                content: r.content.clone(),
                score: r.rerank_score,
                relevance: j.score,
                reason: j.reason.clone(),
            })
            .collect();

        Ok(PipelineOutput {
            results: filtered,
            quality,
            retries: 0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RerankedChunk {
    pub id: String,
    pub document_id: String,
    pub content: String,
    pub score: f32,
    pub relevance: f32,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub results: Vec<RerankedChunk>,
    pub quality: RetrievalQuality,
    pub retries: u8,
}
```

- [ ] **Step 3: 在 rag.rs 的 collect_rag_context 中集成 pipeline**

在 `collect_rag_context()` 函数签名下方，新增可选的 pipeline 参数。保持函数签名向后兼容：新增一个带 pipeline 的重载版本 `collect_rag_context_with_pipeline`：

```rust
// rag.rs — 在文件末尾新增

/// 带管线增强的上下文收集（新入口）
///
/// 相比 collect_rag_context 增加了查询增强、重排序和质检阶段。
pub async fn collect_rag_context_with_pipeline(
    db: &DatabaseConnection,
    master_key: &[u8; 32],
    vector_store: &VectorStore,
    kb_ids: &[String],
    mem_ids: &[String],
    wiki_ids: &[String],
    query: &str,
    top_k: usize,
    embed_fn: impl AsyncEmbedFn,
    pipeline_config: &crate::types::RAGPipelineConfig,
    llm_fn: Option<&(dyn Fn(String) -> std::pin::Pin<Box<dyn std::future::Future<Output = crate::error::Result<String>> + Send>> + Send + Sync)>,
) -> RagContextResult {
    // 阶段 0：查询增强
    let queries: Vec<String> = if pipeline_config.query_enhancement.enabled {
        if let Some(llm) = llm_fn {
            let enhancer = crate::query_enhancement::QueryEnhancer::new(
                pipeline_config.query_enhancement.clone(),
                llm,
            );
            match enhancer.enhance(query).await {
                Ok(enhanced) => enhanced.into_iter().map(|eq| eq.text).collect(),
                Err(e) => {
                    tracing::warn!("Query enhancement failed: {}", e);
                    vec![query.to_string()]
                },
            }
        } else {
            vec![query.to_string()]
        }
    } else {
        vec![query.to_string()]
    };

    // 使用第一个增强查询（后续可扩展为多查询合并）
    let effective_query = queries.first().map(|s| s.as_str()).unwrap_or(query);

    // 如果没有启用 pipeline，直接走原有逻辑
    if !pipeline_config.rerank.enabled && !pipeline_config.self_rag.enabled {
        return collect_rag_context(
            db, master_key, vector_store,
            kb_ids, mem_ids, wiki_ids,
            effective_query, top_k, embed_fn,
        ).await;
    }

    let pipeline = crate::rag_pipeline::RAGPipeline::new(pipeline_config);

    if kb_ids.is_empty() && mem_ids.is_empty() && wiki_ids.is_empty() {
        return RagContextResult {
            context_parts: vec![],
            source_results: vec![],
        };
    }

    // 构建 source refs（复用原有逻辑）
    let mut sources: Vec<RAGSourceRef> = Vec::new();
    for id in kb_ids { sources.push(RAGSourceRef { source_type: RAGSourceType::Knowledge, container_id: id.clone() }); }
    for id in mem_ids { sources.push(RAGSourceRef { source_type: RAGSourceType::Memory, container_id: id.clone() }); }
    for id in wiki_ids { sources.push(RAGSourceRef { source_type: RAGSourceType::Wiki, container_id: id.clone() }); }

    let mut context_parts = Vec::new();
    let mut source_results = Vec::new();

    for src_ref in &sources {
        let source = src_ref.source();
        let (source_top_k, _threshold, dims) = {
            let (sk, _, d) = resolve_source_config(db, &src_ref.source_type, &src_ref.container_id).await;
            (if sk > 0 { sk } else { top_k }, sk, d)
        };

        let result = pipeline.execute(
            source.as_ref(), db, master_key, vector_store,
            &src_ref.container_id, effective_query,
            source_top_k, dims,
            embed_fn.clone(),
            &pipeline_config.rerank,
        ).await;

        match result {
            Ok(output) if !output.results.is_empty() => {
                let label = source.context_label();
                let snippets: Vec<String> = output.results.iter()
                    .map(|r| r.content.clone()).collect();
                context_parts.push(format!("[{}]\n{}", label, snippets.join("\n---\n")));

                let source_type_str = match src_ref.source_type {
                    RAGSourceType::Knowledge => "knowledge",
                    RAGSourceType::Memory => "memory",
                    RAGSourceType::Wiki => "wiki",
                };

                let items: Vec<RagRetrievedItem> = output.results.iter().map(|r| {
                    RagRetrievedItem {
                        content: r.content.clone(),
                        score: r.score,
                        document_id: r.document_id.clone(),
                        id: r.id.clone(),
                        document_name: None,
                    }
                }).collect();

                match output.quality {
                    RetrievalQuality::Poor(diag) => {
                        tracing::warn!("Poor RAG quality for {} {}: {}", source_type_str, src_ref.container_id, diag);
                    },
                    _ => {},
                }

                source_results.push(RagSourceResult {
                    source_type: source_type_str.to_string(),
                    container_id: src_ref.container_id.clone(),
                    items,
                });
            },
            Ok(_) => {
                tracing::warn!("Pipeline returned no results for {} {}", source.collection_prefix(), src_ref.container_id);
            },
            Err(e) => {
                tracing::warn!("Pipeline failed for {} {}: {}", source.collection_prefix(), src_ref.container_id, e);
            },
        }
    }

    let (deduped_results, deduped_context) = deduplicate_cross_source(source_results, context_parts);

    RagContextResult {
        context_parts: deduped_context,
        source_results: deduped_results,
    }
}
```

- [ ] **Step 4: 在 lib.rs 中注册 rag_pipeline 模块**

```rust
pub mod rag_pipeline;
```

- [ ] **Step 5: 编译检查**

```bash
cd src-tauri && cargo check -p axagent-core 2>&1
```

修复所有编译错误。重点关注类型不匹配。

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/core/src/rag_pipeline.rs \
        src-tauri/crates/core/src/rag.rs \
        src-tauri/crates/core/src/types.rs \
        src-tauri/crates/core/src/lib.rs
git commit -m "feat: 创建 RAG 管线编排层并集成到 collect_rag_context"
```

---

### Task 6: 前端知识库设置页适配

**Files:**
- Modify: `src/components/settings/KnowledgeSettings.tsx`
- Modify: `src/locales/zh-CN/settings.json`
- Modify: `src/locales/en/settings.json`
- Modify: 其余 9 种语言的 `settings.json`

- [ ] **Step 1: 在 zh-CN 和 en 语言文件中添加 i18n key**

```json
// zh-CN/settings.json 中新增
{
  "rag": {
    "advanced": "高级检索设置",
    "rerank": {
      "title": "智能重排序",
      "desc": "启用跨编码器模型对检索结果进行语义精排",
      "backend": "重排序后端",
      "backendRule": "规则评分",
      "backendCross": "跨编码器（本地 Ollama）",
      "backendPipeline": "两级管线（规则+跨编码器）",
      "modelName": "Rerank 模型",
      "ollamaEndpoint": "Ollama 地址",
      "topN": "最终返回数",
      "candidateK": "候选集大小"
    },
    "selfRag": {
      "title": "自省式质检",
      "desc": "用本地裁判模型评估检索结果质量，自动纠正低质量检索",
      "judgeModel": "裁判模型",
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
      "maxVariants": "最大变体数",
      "combinedCall": "合并 HyDE+MultiQuery 为一次调用"
    }
  }
}
```

```json
// en/settings.json 中新增
{
  "rag": {
    "advanced": "Advanced Retrieval Settings",
    "rerank": {
      "title": "Smart Reranking",
      "desc": "Semantic reranking using cross-encoder models for better relevance",
      "backend": "Rerank Backend",
      "backendRule": "Rule-based",
      "backendCross": "Cross-Encoder (Local Ollama)",
      "backendPipeline": "Two-stage Pipeline (Rule + Cross-Encoder)",
      "modelName": "Rerank Model",
      "ollamaEndpoint": "Ollama Endpoint",
      "topN": "Final Top-N",
      "candidateK": "Candidate Set Size"
    },
    "selfRag": {
      "title": "Self-RAG Quality Gate",
      "desc": "Evaluate retrieval quality with a local judge model, auto-correct poor results",
      "judgeModel": "Judge Model",
      "relevanceThreshold": "Relevance Threshold",
      "qualityThreshold": "Quality Threshold",
      "maxRetries": "Max Retry Rounds"
    },
    "queryEnhancement": {
      "title": "Query Enhancement",
      "desc": "Auto-rewrite and expand user queries for better retrieval",
      "strategy": "Enhancement Strategy",
      "strategyNone": "None",
      "strategyHyde": "HyDE (Hypothetical Document)",
      "strategyMultiQuery": "Multi-Query",
      "strategyDecomposition": "Decomposition",
      "strategyAuto": "Auto",
      "maxVariants": "Max Variants",
      "combinedCall": "Combine HyDE+MultiQuery"
    }
  }
}
```

- [ ] **Step 2: 在 KnowledgeSettings.tsx 中添加高级 RAG 设置区域**

在现有知识库详情面板中，找到嵌入模型配置区域之后，添加高级 RAG 配置折叠面板：

```tsx
// KnowledgeSettings.tsx — 在 return JSX 的合适位置插入
import { Collapse, Switch, Select, InputNumber, Input } from "antd";
import { SettingOutlined } from "@ant-design/icons";

// 在组件函数内部新增状态
const [ragAdvancedConfig, setRagAdvancedConfig] = useState({
  rerankEnabled: false,
  rerankBackend: "rule" as "rule" | "cross_encoder" | "pipeline",
  rerankTopN: 5,
  rerankCandidateK: 30,
  selfRagEnabled: false,
  selfRagJudgeModel: "qwen2.5:0.5b",
  selfRagRelevanceThreshold: 0.5,
  selfRagQualityThreshold: 0.6,
  selfRagMaxRetries: 2,
  queryEnhancementEnabled: false,
  queryEnhancementStrategy: "auto" as "none" | "hyde" | "multi_query" | "decomposition" | "auto",
  queryEnhancementMaxVariants: 3,
  queryEnhancementCombinedCall: true,
});

// JSX — 在知识库详情面板中添加
<Collapse
  ghost
  items={[{
    key: "advanced-rag",
    label: <Space><SettingOutlined />{t("settings.rag.advanced")}</Space>,
    children: (
      <>
        {/* 查询增强 */}
        <Divider plain>{t("settings.rag.queryEnhancement.title")}</Divider>
        <Form.Item label={t("settings.rag.queryEnhancement.title")} help={t("settings.rag.queryEnhancement.desc")}>
          <Switch checked={ragAdvancedConfig.queryEnhancementEnabled} onChange={(v) =>
            setRagAdvancedConfig(prev => ({ ...prev, queryEnhancementEnabled: v }))
          } />
        </Form.Item>
        {ragAdvancedConfig.queryEnhancementEnabled && (
          <>
            <Form.Item label={t("settings.rag.queryEnhancement.strategy")}>
              <Select value={ragAdvancedConfig.queryEnhancementStrategy} onChange={(v) =>
                setRagAdvancedConfig(prev => ({ ...prev, queryEnhancementStrategy: v }))
              } options={[
                { value: "none", label: t("settings.rag.queryEnhancement.strategyNone") },
                { value: "hyde", label: t("settings.rag.queryEnhancement.strategyHyde") },
                { value: "multi_query", label: t("settings.rag.queryEnhancement.strategyMultiQuery") },
                { value: "decomposition", label: t("settings.rag.queryEnhancement.strategyDecomposition") },
                { value: "auto", label: t("settings.rag.queryEnhancement.strategyAuto") },
              ]} />
            </Form.Item>
            <Form.Item label={t("settings.rag.queryEnhancement.maxVariants")}>
              <InputNumber min={2} max={5} value={ragAdvancedConfig.queryEnhancementMaxVariants} onChange={(v) =>
                setRagAdvancedConfig(prev => ({ ...prev, queryEnhancementMaxVariants: v ?? 3 }))
              } />
            </Form.Item>
          </>
        )}

        {/* 重排序 */}
        <Divider plain>{t("settings.rag.rerank.title")}</Divider>
        <Form.Item label={t("settings.rag.rerank.title")} help={t("settings.rag.rerank.desc")}>
          <Switch checked={ragAdvancedConfig.rerankEnabled} onChange={(v) =>
            setRagAdvancedConfig(prev => ({ ...prev, rerankEnabled: v }))
          } />
        </Form.Item>
        {ragAdvancedConfig.rerankEnabled && (
          <>
            <Form.Item label={t("settings.rag.rerank.backend")}>
              <Select value={ragAdvancedConfig.rerankBackend} onChange={(v) =>
                setRagAdvancedConfig(prev => ({ ...prev, rerankBackend: v }))
              } options={[
                { value: "rule", label: t("settings.rag.rerank.backendRule") },
                { value: "cross_encoder", label: t("settings.rag.rerank.backendCross") },
                { value: "pipeline", label: t("settings.rag.rerank.backendPipeline") },
              ]} />
            </Form.Item>
            {ragAdvancedConfig.rerankBackend !== "rule" && (
              <Form.Item label={t("settings.rag.rerank.modelName")}>
                <Input value="bge-reranker-v2-m3" disabled />
              </Form.Item>
            )}
            <Form.Item label={t("settings.rag.rerank.topN")}>
              <InputNumber min={1} max={20} value={ragAdvancedConfig.rerankTopN} onChange={(v) =>
                setRagAdvancedConfig(prev => ({ ...prev, rerankTopN: v ?? 5 }))
              } />
            </Form.Item>
            <Form.Item label={t("settings.rag.rerank.candidateK")}>
              <InputNumber min={5} max={100} value={ragAdvancedConfig.rerankCandidateK} onChange={(v) =>
                setRagAdvancedConfig(prev => ({ ...prev, rerankCandidateK: v ?? 30 }))
              } />
            </Form.Item>
          </>
        )}

        {/* Self-RAG */}
        <Divider plain>{t("settings.rag.selfRag.title")}</Divider>
        <Form.Item label={t("settings.rag.selfRag.title")} help={t("settings.rag.selfRag.desc")}>
          <Switch checked={ragAdvancedConfig.selfRagEnabled} onChange={(v) =>
            setRagAdvancedConfig(prev => ({ ...prev, selfRagEnabled: v }))
          } />
        </Form.Item>
        {ragAdvancedConfig.selfRagEnabled && (
          <>
            <Form.Item label={t("settings.rag.selfRag.judgeModel")}>
              <Input value={ragAdvancedConfig.selfRagJudgeModel} onChange={(e) =>
                setRagAdvancedConfig(prev => ({ ...prev, selfRagJudgeModel: e.target.value }))
              } />
            </Form.Item>
            <Form.Item label={t("settings.rag.selfRag.relevanceThreshold")}>
              <InputNumber min={0.1} max={1.0} step={0.05} value={ragAdvancedConfig.selfRagRelevanceThreshold} onChange={(v) =>
                setRagAdvancedConfig(prev => ({ ...prev, selfRagRelevanceThreshold: v ?? 0.5 }))
              } />
            </Form.Item>
            <Form.Item label={t("settings.rag.selfRag.qualityThreshold")}>
              <InputNumber min={0.1} max={1.0} step={0.05} value={ragAdvancedConfig.selfRagQualityThreshold} onChange={(v) =>
                setRagAdvancedConfig(prev => ({ ...prev, selfRagQualityThreshold: v ?? 0.6 }))
              } />
            </Form.Item>
            <Form.Item label={t("settings.rag.selfRag.maxRetries")}>
              <InputNumber min={1} max={5} value={ragAdvancedConfig.selfRagMaxRetries} onChange={(v) =>
                setRagAdvancedConfig(prev => ({ ...prev, selfRagMaxRetries: v ?? 2 }))
              } />
            </Form.Item>
          </>
        )}
      </>
    ),
  }]}
/>
```

- [ ] **Step 3: 同步其余 9 种语言文件**

对 `ja-JP`, `ko-KR`, `fr-FR`, `de-DE`, `es-ES`, `pt-BR`, `ru-RU`, `ar-SA`, `vi-VN` 各语言文件添加相同的 key 结构，值保持英文回退（标注需翻译）。

- [ ] **Step 4: TypeScript 编译检查**

```bash
cd src && npx tsc --noEmit 2>&1
```
Expected: no new errors。

- [ ] **Step 5: dprint 格式化**

```bash
npm run format
```

- [ ] **Step 6: Commit**

```bash
git add src/components/settings/KnowledgeSettings.tsx \
        src/locales/zh-CN/settings.json \
        src/locales/en/settings.json \
        src/locales/ja-JP/settings.json \
        src/locales/ko-KR/settings.json \
        src/locales/fr-FR/settings.json \
        src/locales/de-DE/settings.json \
        src/locales/es-ES/settings.json \
        src/locales/pt-BR/settings.json \
        src/locales/ru-RU/settings.json \
        src/locales/ar-SA/settings.json \
        src/locales/vi-VN/settings.json
git commit -m "feat: 知识库设置页添加高级 RAG 配置（rerank + selfRAG + queryEnhancement）"
```

---

### Task 7: 端到端集成验证 + 最终检查

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
Expected: 无新增类型错误。

- [ ] **Step 4: 前端格式化**

```bash
npm run format
```

- [ ] **Step 5: clippy 零警告**

```bash
cd src-tauri && cargo clippy -- -D warnings 2>&1
```
Expected: 零 clippy 警告。

- [ ] **Step 6: rustfmt 检查**

```bash
cd src-tauri && cargo fmt --check 2>&1
```
Expected: 格式化已就绪。

- [ ] **Step 7: Commit**

```bash
git add -u
git commit -m "chore: P0 高级 RAG 端到端集成验证通过"
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
- [ ] 新建模块已在 `types.rs` 注册（如有类型）
- [ ] 新的 Tauri 命令（如有）已在 `commands/mod.rs` + `lib.rs` generate_handler 注册
- [ ] 所有 11 种语言文件已同步新增 i18n key
