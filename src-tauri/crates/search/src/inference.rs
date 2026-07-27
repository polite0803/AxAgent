// SPDX-License-Identifier: AGPL-3.0-only

//! 本地推理引擎
//!
//! 1. Rerank: 跨编码器重排序（当前启发式，candle 0.9+ 可支持 BERT GGUF）
//! 2. Judge: LLaMA 相关性裁判（真实 candle 推理 + 启发式回退）
//! 3. SparseEncoder: BGE-M3 风格的 sparse 神经编码（tokenizer-based 启发式）
//!
//! 非 Android 平台通过 candle 0.8 + tokenizers 0.21 运行真实 LLM 推理。
//! 每个模型在独立线程中运行（candle 张量 !Send），通过 channel 通信。
//!
//! SparseEncoder 通过 `ModelDownloader::find_preset(filename)` 注册式查找模型预设，
//! 而非硬编码模型路径；运行时通过 tokenizer 提取 token IDs 并计算权重（启发式），
//! 未来 candle 支持 BERT GGUF 后可平滑替换为真实推理。

use async_trait::async_trait;
use axagent_harness::InferenceEngine as InferenceEngineTrait;
use axagent_harness::SparseVectorEntry;
use axagent_harness::core_error::{AxAgentError, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;

// ── 公开类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct JudgeOutput {
    pub relevant: bool,
    pub score: f32,
    pub reason: String,
}

// ── 内部类型 ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum ModelKind {
    Reranker,
    Judge,
    SparseEncoder,
}

enum WorkMsg {
    Rerank {
        query: String,
        documents: Vec<String>,
        reply: tokio::sync::oneshot::Sender<Result<Vec<f32>>>,
    },
    Judge {
        query: String,
        chunk_content: String,
        reply: tokio::sync::oneshot::Sender<Result<JudgeOutput>>,
    },
    EmbedSparse {
        text: String,
        reply: tokio::sync::oneshot::Sender<Result<Vec<SparseVectorEntry>>>,
    },
    Shutdown,
}

struct WorkerHandle {
    sender: std::sync::mpsc::Sender<WorkMsg>,
    kind: ModelKind,
}

// ── 推理引擎 ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct InferenceEngine {
    workers: Arc<RwLock<HashMap<String, Arc<WorkerHandle>>>>,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self { workers: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn is_loaded(&self, filename: &str) -> bool {
        self.workers.read().await.contains_key(filename)
    }

    pub async fn load_reranker_model(&self, gguf_path: &Path) -> Result<()> {
        self.load_model(gguf_path, ModelKind::Reranker).await
    }

    pub async fn load_judge_model(&self, gguf_path: &Path) -> Result<()> {
        self.load_model(gguf_path, ModelKind::Judge).await
    }

    /// 加载 sparse encoder 模型（通过 ModelDownloader 注册式查找预设）。
    ///
    /// `gguf_path` 必须指向已下载的 sparse encoder GGUF 模型文件（如
    /// `bge-m3-sparse.Q4_K_M.gguf`）。tokenizer 路径自动推断为同目录下的
    /// `bge-m3.tokenizer.json`（若不存在，worker 会回退到空 tokenizer）。
    pub async fn load_sparse_encoder_model(&self, gguf_path: &Path) -> Result<()> {
        self.load_model(gguf_path, ModelKind::SparseEncoder).await
    }

    async fn load_model(&self, gguf_path: &Path, kind: ModelKind) -> Result<()> {
        let filename = gguf_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".into());
        let tokenizer_path = gguf_path.with_extension("tokenizer.json");
        let gguf = gguf_path.to_path_buf();
        let tok = tokenizer_path.clone();
        let kind_label = match kind {
            ModelKind::Reranker => "Reranker",
            ModelKind::Judge => "Judge",
            ModelKind::SparseEncoder => "SparseEncoder",
        };

        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::Builder::new()
            .name(format!("inf-{}", filename))
            .spawn(move || {
                worker_main(&gguf, &tok, kind, kind_label, receiver);
            })
            .map_err(|e| AxAgentError::Inference(format!("spawn thread: {}", e)))?;

        let mut workers = self.workers.write().await;
        workers.insert(filename, Arc::new(WorkerHandle { sender, kind }));
        tracing::info!("{kind_label} model loaded: {}", gguf_path.display());
        Ok(())
    }

    pub async fn rerank(
        &self,
        filename: &str,
        query: &str,
        documents: &[String],
    ) -> Result<Vec<f32>> {
        let h = self.workers.read().await.get(filename).cloned();
        match h {
            Some(ref h) if h.kind == ModelKind::Reranker => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                h.sender
                    .send(WorkMsg::Rerank {
                        query: query.to_string(),
                        documents: documents.to_vec(),
                        reply: tx,
                    })
                    .map_err(|e| AxAgentError::Inference(format!("send: {}", e)))?;
                rx.await.map_err(|_| AxAgentError::Inference("worker down".into()))?
            },
            _ => Ok(heuristic_rerank(query, documents)),
        }
    }

    pub async fn judge(&self, filename: &str, query: &str, chunk: &str) -> Result<JudgeOutput> {
        let h = self.workers.read().await.get(filename).cloned();
        match h {
            Some(ref h) if h.kind == ModelKind::Judge => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                h.sender
                    .send(WorkMsg::Judge {
                        query: query.to_string(),
                        chunk_content: chunk.to_string(),
                        reply: tx,
                    })
                    .map_err(|e| AxAgentError::Inference(format!("send: {}", e)))?;
                rx.await.map_err(|_| AxAgentError::Inference("worker down".into()))?
            },
            _ => Ok(heuristic_judge(query, chunk)),
        }
    }

    /// 计算 sparse 神经表示。
    ///
    /// - 若 `filename` 对应的 sparse encoder 已加载，通过 worker 推理
    /// - 否则返回空 Vec（调用方回退到 BM25 或 dense 检索）
    pub async fn embed_sparse(&self, filename: &str, text: &str) -> Result<Vec<SparseVectorEntry>> {
        let h = self.workers.read().await.get(filename).cloned();
        match h {
            Some(ref h) if h.kind == ModelKind::SparseEncoder => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                h.sender
                    .send(WorkMsg::EmbedSparse { text: text.to_string(), reply: tx })
                    .map_err(|e| AxAgentError::Inference(format!("send: {}", e)))?;
                rx.await.map_err(|_| AxAgentError::Inference("worker down".into()))?
            },
            _ => Ok(Vec::new()),
        }
    }

    pub async fn unload_model(&self, filename: &str) -> bool {
        self.workers
            .write()
            .await
            .remove(filename)
            .map(|h| {
                let _ = h.sender.send(WorkMsg::Shutdown);
            })
            .is_some()
    }

    pub async fn unload_all(&self) {
        for (_, h) in self.workers.write().await.drain() {
            let _ = h.sender.send(WorkMsg::Shutdown);
        }
    }

    pub async fn loaded_model_names(&self) -> Vec<String> {
        self.workers.read().await.keys().cloned().collect()
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceEngineTrait for InferenceEngine {
    async fn rerank(
        &self,
        model_filename: &str,
        query: &str,
        documents: &[String],
    ) -> Result<Vec<f32>> {
        self.rerank(model_filename, query, documents).await
    }

    async fn embed_sparse(
        &self,
        model_filename: &str,
        text: &str,
    ) -> Result<Vec<SparseVectorEntry>> {
        self.embed_sparse(model_filename, text).await
    }
}

// ── 全局推理引擎单例 ──────────────────────────────────────────────────────────

static GLOBAL_ENGINE: OnceLock<Arc<InferenceEngine>> = OnceLock::new();
static AUTO_LOAD_MODELS: AtomicBool = AtomicBool::new(true);

/// 设置是否在下载后自动加载模型到内存。
/// 供 `set_auto_load_models` Tauri 命令调用。
pub fn set_auto_load_models(enabled: bool) {
    AUTO_LOAD_MODELS.store(enabled, Ordering::Release);
}

/// 查询当前是否启用自动加载。
pub fn is_auto_load_models() -> bool {
    AUTO_LOAD_MODELS.load(Ordering::Acquire)
}

/// 获取或初始化全局推理引擎。
///
/// 首次调用时自动扫描 `~/.axagent/models/` 目录，将已下载的模型加载到引擎中。
/// 若 `AUTO_LOAD_MODELS` 为 false，则跳过自动加载。
/// 全局引擎与 RAG 管线共享状态，确保下载后的模型能被 pipeline 使用。
pub fn global_engine() -> Arc<InferenceEngine> {
    GLOBAL_ENGINE
        .get_or_init(|| {
            let engine = Arc::new(InferenceEngine::new());
            if AUTO_LOAD_MODELS.load(Ordering::Acquire) {
                // 后台 auto-load 已下载的模型
                let cache_dir =
                    crate::model_downloader::ModelDownloader::new().cache_dir().to_path_buf();
                let eng = engine.clone();
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        auto_load_downloaded_models(eng, &cache_dir).await;
                    });
                } else {
                    tracing::warn!("No tokio runtime: skipping auto-load of models");
                }
            } else {
                tracing::info!("Auto-load disabled: skipping model loading");
            }
            engine
        })
        .clone()
}

/// 下载模型后将其加载到全局引擎中。
///
/// 供 `download_model` Tauri 命令调用，确保下载完成后模型立即可用。
/// 若 `AUTO_LOAD_MODELS` 为 false，仅下载不加载。
pub async fn download_and_load_model(filename: &str) -> Result<()> {
    let dl = crate::model_downloader::ModelDownloader::new();
    let preset =
        crate::model_downloader::ModelDownloader::find_preset(filename).ok_or_else(|| {
            AxAgentError::ModelDownload(format!("Unknown preset model: {}", filename))
        })?;
    dl.ensure_model(&preset).await?;

    if !AUTO_LOAD_MODELS.load(Ordering::Acquire) {
        tracing::info!("Auto-load disabled: model {} downloaded but not loaded", filename);
        return Ok(());
    }

    let engine = global_engine();
    let path = dl.cache_dir().join(filename);
    match preset.model_type {
        crate::model_downloader::PresetModelType::Reranker => {
            engine.load_reranker_model(&path).await
        },
        crate::model_downloader::PresetModelType::Judge => engine.load_judge_model(&path).await,
        crate::model_downloader::PresetModelType::SparseEncoder => {
            engine.load_sparse_encoder_model(&path).await
        },
    }
}

/// 从全局引擎卸载模型，再从磁盘删除。
///
/// 供 `delete_model` Tauri 命令调用，确保删除前停止 worker 线程。
pub async fn delete_and_unload_model(filename: &str) -> Result<()> {
    // 先卸载 worker（若已加载）
    if let Some(engine) = GLOBAL_ENGINE.get() {
        let name = Path::new(filename)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| filename.to_string());
        engine.unload_model(&name).await;
    }
    // 再删除磁盘文件
    let dl = crate::model_downloader::ModelDownloader::new();
    dl.remove_model(filename)
}

/// 后台扫描 `cache_dir` 下已下载的 GGUF 文件，按预设类型加载到引擎。
async fn auto_load_downloaded_models(engine: Arc<InferenceEngine>, cache_dir: &Path) {
    use crate::model_downloader::{ModelDownloader, PresetModelType};
    for preset in ModelDownloader::preset_models() {
        let path = cache_dir.join(&preset.filename);
        if !path.exists() {
            continue;
        }
        let result = match preset.model_type {
            PresetModelType::Reranker => engine.load_reranker_model(&path).await,
            PresetModelType::Judge => engine.load_judge_model(&path).await,
            PresetModelType::SparseEncoder => engine.load_sparse_encoder_model(&path).await,
        };
        match result {
            Ok(()) => {
                tracing::info!("Auto-loaded model {} from {}", preset.filename, path.display())
            },
            Err(e) => {
                tracing::warn!("Failed to auto-load model {}: {}", preset.filename, e)
            },
        }
    }
}

// ── Worker 主循环 ──────────────────────────────────────────────────────────

// Android 平台不加载 candle 模型（gguf/tok/kind 仅在 load_candle_model 中使用），
// 因此这三个参数在 android 上是未使用的。
#[cfg_attr(target_os = "android", allow(unused_variables))]
fn worker_main(
    gguf: &Path,
    tok: &Path,
    kind: ModelKind,
    label: &str,
    rx: std::sync::mpsc::Receiver<WorkMsg>,
) {
    #[cfg(not(target_os = "android"))]
    let loaded = load_candle_model(gguf, tok, kind);

    // SparseEncoder worker 预加载 tokenizer，用于启发式 sparse 计算
    let sparse_tokenizer = if kind == ModelKind::SparseEncoder {
        match tokenizers::Tokenizer::from_file(tok) {
            Ok(t) => Some(t),
            Err(e) => {
                tracing::warn!(
                    "SparseEncoder: tokenizer load failed at {}: {} — falling back to whitespace tokenization",
                    tok.display(),
                    e
                );
                None
            },
        }
    } else {
        None
    };

    for msg in rx {
        match msg {
            WorkMsg::Rerank { query, documents, reply } => {
                let scores = heuristic_rerank(&query, &documents);
                let _ = reply.send(Ok(scores));
            },
            WorkMsg::Judge { query, chunk_content, reply } => {
                #[cfg(not(target_os = "android"))]
                let result = match &loaded {
                    Some(m) => candle_judge(m, &query, &chunk_content),
                    None => Ok(heuristic_judge(&query, &chunk_content)),
                };
                #[cfg(target_os = "android")]
                let result: Result<JudgeOutput> = Ok(heuristic_judge(&query, &chunk_content));
                let _ = reply.send(result);
            },
            WorkMsg::EmbedSparse { text, reply } => {
                let result = Ok(heuristic_sparse_encode(&text, sparse_tokenizer.as_ref()));
                let _ = reply.send(result);
            },
            WorkMsg::Shutdown => break,
        }
    }
    tracing::info!("Worker '{label}' shut down");
}

// ── Candle 模型加载 ────────────────────────────────────────────────────────

#[cfg(not(target_os = "android"))]
struct CandleModel {
    model: candle_transformers::models::quantized_llama::ModelWeights,
    tokenizer: tokenizers::Tokenizer,
}

#[cfg(not(target_os = "android"))]
fn load_candle_model(gguf: &Path, tok: &Path, kind: ModelKind) -> Option<CandleModel> {
    match kind {
        ModelKind::Judge => {
            let tokenizer = match tokenizers::Tokenizer::from_file(tok) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("tokenizer load failed: {}", e);
                    return None;
                },
            };
            let mut file = match std::fs::File::open(gguf) {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("GGUF open failed: {}", e);
                    return None;
                },
            };
            let ct = match candle_core::quantized::gguf_file::Content::read(&mut file) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("GGUF parse failed: {}", e);
                    return None;
                },
            };
            let device = candle_core::Device::Cpu;
            let model = match candle_transformers::models::quantized_llama::ModelWeights::from_gguf(
                ct, &mut file, &device,
            ) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("Model build failed: {}", e);
                    return None;
                },
            };
            tracing::info!("Loaded LLaMA judge model from {}", gguf.display());
            Some(CandleModel { model, tokenizer })
        },
        ModelKind::Reranker => {
            tracing::info!("Reranker: heuristic mode (candle BERT GGUF requires 0.9+)");
            None
        },
        ModelKind::SparseEncoder => {
            // BGE-M3 sparse 推理需要：
            // 1. candle-transformers 0.11 对 BERT/XLM-RoBERTa GGUF 的支持
            // 2. 正确提取 sparse head（Linear + sigmoid）
            // 当前 candle 0.11 不直接支持 BERT GGUF 加载，回退到 tokenizer-based 启发式
            tracing::info!(
                "SparseEncoder: heuristic mode (candle BERT GGUF support pending in 0.9+)"
            );
            None
        },
    }
}

// ── Candle 推理 ────────────────────────────────────────────────────────────

#[cfg(not(target_os = "android"))]
fn candle_judge(m: &CandleModel, query: &str, chunk: &str) -> Result<JudgeOutput> {
    use candle_core::{Device, Tensor};

    macro_rules! c {
        ($e:expr) => {
            $e.map_err(|e| AxAgentError::Inference(e.to_string()))?
        };
    }

    let prompt = format!(
        "<|im_start|>system\nJudge relevance. Reply ONLY YES or NO.\n<|im_end|>\n\
         <|im_start|>user\nQuery: {}\nChunk: {}\nRelevant? YES/NO:<|im_end|>\n<|im_start|>assistant\n",
        query, chunk
    );

    let dev = Device::Cpu;
    let enc = m
        .tokenizer
        .encode(prompt, true)
        .map_err(|e| AxAgentError::Inference(format!("tokenize: {}", e)))?;
    let ids = enc.get_ids();
    let mut input = c!(c!(Tensor::new(ids, &dev)).unsqueeze(0));
    let mut model = m.model.clone();
    let mut tokens = Vec::new();

    for _ in 0..5 {
        let pos = input.dims()[1].saturating_sub(1) + tokens.len();
        let logits = c!(model.forward(&input, pos));
        let t = c!(c!(c!(logits.get(0)).argmax(0)).to_scalar::<u32>());
        tokens.push(t);
        if t == 2 || t >= 32000 {
            break;
        }
        let tok = c!(c!(Tensor::new(&[t], &dev)).unsqueeze(0));
        input = c!(Tensor::cat(&[&input, &tok], 1));
    }

    let out = m
        .tokenizer
        .decode(&tokens, false)
        .map_err(|e| AxAgentError::Inference(format!("decode: {}", e)))?;
    let is_yes = out.to_uppercase().contains("YES");

    Ok(JudgeOutput {
        relevant: is_yes,
        score: if is_yes { 0.85 } else { 0.15 },
        reason: format!("LLM: {}", out.trim()),
    })
}

// ── 启发式回退 ────────────────────────────────────────────────────────────

fn heuristic_rerank(query: &str, documents: &[String]) -> Vec<f32> {
    let q = query.to_lowercase();
    let terms: Vec<&str> = q.split_whitespace().filter(|w| w.len() > 1).collect();
    documents
        .iter()
        .map(|doc| {
            let d = doc.to_lowercase();
            let m = terms.iter().filter(|t| d.contains(*t)).count() as f32;
            let c = if terms.is_empty() {
                0.5
            } else {
                m / terms.len() as f32
            };
            1.0 / (1.0 + (-3.0 * (c - 0.3)).exp())
        })
        .collect()
}

fn heuristic_judge(query: &str, chunk: &str) -> JudgeOutput {
    let q = query.to_lowercase();
    let c = chunk.to_lowercase();
    let terms: Vec<&str> = q.split_whitespace().filter(|w| w.len() > 1).collect();
    let m = terms.iter().filter(|t| c.contains(*t)).count();
    let score = if terms.is_empty() {
        0.5
    } else {
        m as f32 / terms.len() as f32
    };
    JudgeOutput { relevant: score >= 0.3, score, reason: format!("{}/{} terms", m, terms.len()) }
}

/// 启发式 sparse 编码：基于 tokenizer 的 lexical 稀疏表示。
///
/// 实现：
/// - 若 tokenizer 可用：用 tokenizer.encode 拿到 token IDs，对每个 unique token
///   计算权重 = `1.0 / sqrt(token_count)`（模拟 BGE-M3 sparse 的归一化风格）
/// - 若 tokenizer 不可用：用空格分词 + FNV-1a hash 映射到 token_id
///
/// 返回值：
/// - 非零 (token_id, weight) 列表
/// - 适合作为 sparse 检索路径的查询向量
fn heuristic_sparse_encode(
    text: &str,
    tokenizer: Option<&tokenizers::Tokenizer>,
) -> Vec<SparseVectorEntry> {
    if let Some(tok) = tokenizer {
        match tok.encode(text, true) {
            Ok(enc) => {
                let ids = enc.get_ids();
                if ids.is_empty() {
                    return Vec::new();
                }
                // 聚合每个 token 的出现次数
                let mut counts: HashMap<u32, u32> = HashMap::new();
                for &id in ids {
                    *counts.entry(id).or_insert(0) += 1;
                }
                let norm = (ids.len() as f32).sqrt();
                // 输出非零项，权重 = count / sqrt(total_tokens)
                let mut entries: Vec<SparseVectorEntry> = counts
                    .into_iter()
                    .map(|(token_id, count)| SparseVectorEntry {
                        token_id,
                        weight: count as f32 / norm,
                    })
                    .collect();
                // 按 token_id 排序，便于后续 cosine / dot product 优化
                entries.sort_by_key(|e| e.token_id);
                entries
            },
            Err(e) => {
                tracing::warn!(
                    "SparseEncoder: tokenizer encode failed, falling back to whitespace: {}",
                    e
                );
                whitespace_sparse_encode(text)
            },
        }
    } else {
        whitespace_sparse_encode(text)
    }
}

/// 极简 fallback：空格分词 + FNV-1a hash 作为 token_id
fn whitespace_sparse_encode(text: &str) -> Vec<SparseVectorEntry> {
    let tokens: Vec<&str> = text.split_whitespace().filter(|w| !w.is_empty()).collect();
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut counts: HashMap<u32, u32> = HashMap::new();
    for w in &tokens {
        // FNV-1a 32-bit hash → token_id
        let mut h: u32 = 0x811c9dc5;
        for b in w.as_bytes() {
            h ^= *b as u32;
            h = h.wrapping_mul(0x01000193);
        }
        *counts.entry(h).or_insert(0) += 1;
    }
    let norm = (tokens.len() as f32).sqrt();
    let mut entries: Vec<SparseVectorEntry> = counts
        .into_iter()
        .map(|(token_id, count)| SparseVectorEntry { token_id, weight: count as f32 / norm })
        .collect();
    entries.sort_by_key(|e| e.token_id);
    entries
}

// ── 测试 ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clone() {
        let e = InferenceEngine::new();
        let _ = e.clone();
    }
    #[tokio::test]
    async fn test_unload_empty() {
        assert!(!InferenceEngine::new().unload_model("x").await);
    }
    #[tokio::test]
    async fn test_names_empty() {
        assert!(InferenceEngine::new().loaded_model_names().await.is_empty());
    }
    #[tokio::test]
    async fn test_is_loaded_false() {
        assert!(!InferenceEngine::new().is_loaded("x").await);
    }

    #[tokio::test]
    async fn test_rerank_fallback() {
        let r = InferenceEngine::new()
            .rerank("x", "rust code", &["rust".into(), "python".into()])
            .await
            .unwrap();
        assert!(r[0] > r[1]);
    }

    #[tokio::test]
    async fn test_judge_fallback_relevant() {
        let o = InferenceEngine::new().judge("x", "rust code", "rust programming").await.unwrap();
        assert!(o.relevant);
    }

    #[tokio::test]
    async fn test_judge_fallback_irrelevant() {
        let o =
            InferenceEngine::new().judge("x", "rust programming", "python django").await.unwrap();
        assert!(!o.relevant || o.score < 0.5);
    }

    #[tokio::test]
    async fn test_embed_sparse_returns_empty_when_not_loaded() {
        // 未加载任何 sparse encoder 时，返回空 Vec（调用方应回退到 BM25/dense）
        let r = InferenceEngine::new().embed_sparse("x", "hello world").await.unwrap();
        assert!(r.is_empty());
    }

    #[test]
    fn test_heuristic_rerank_order() {
        // 词语至少 2 字符，单字符会被 filter(|w| w.len() > 1) 过滤
        let s = heuristic_rerank("foo bar baz", &["foo bar baz".into(), "xyz qux abc".into()]);
        assert!(s[0] > 0.85);
        assert!(s[1] < 0.5);
    }

    #[test]
    fn test_judge_output_struct() {
        let o = JudgeOutput { relevant: true, score: 0.8, reason: "ok".into() };
        assert!(o.relevant);
        assert!(o.score > 0.5);
    }

    #[test]
    fn test_whitespace_sparse_encode_basic() {
        let entries = whitespace_sparse_encode("hello world hello");
        assert_eq!(entries.len(), 2); // "hello" + "world"
        // "hello" 出现 2 次，权重 = 2 / sqrt(3)
        let hello_entry = entries
            .iter()
            .find(|e| e.weight > 0.9)
            .expect("expected a high-weight entry for repeated 'hello'");
        assert!((hello_entry.weight - 2.0_f32 / 3.0_f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn test_whitespace_sparse_encode_empty() {
        assert!(whitespace_sparse_encode("").is_empty());
        assert!(whitespace_sparse_encode("   ").is_empty());
    }

    #[test]
    fn test_whitespace_sparse_encode_sorted_by_token_id() {
        let entries = whitespace_sparse_encode("a b c");
        // 验证返回结果按 token_id 升序排列
        for w in entries.windows(2) {
            assert!(w[0].token_id <= w[1].token_id, "entries should be sorted by token_id");
        }
    }

    #[test]
    fn test_heuristic_sparse_encode_no_tokenizer_uses_whitespace() {
        let entries = heuristic_sparse_encode("alpha beta alpha", None);
        assert_eq!(entries.len(), 2); // "alpha" + "beta"
    }

    #[test]
    fn test_sparse_vector_entry_serialization() {
        let entry = SparseVectorEntry { token_id: 42, weight: 0.5 };
        let json = serde_json::to_string(&entry).unwrap();
        let decoded: SparseVectorEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, decoded);
    }
}
