//! 本地推理引擎
//!
//! 1. Rerank: 跨编码器重排序（当前启发式，candle 0.9+ 可支持 BERT GGUF）
//! 2. Judge: LLaMA 相关性裁判（真实 candle 推理 + 启发式回退）
//!
//! 非 Android 平台通过 candle 0.8 + tokenizers 0.21 运行真实 LLM 推理。
//! 每个模型在独立线程中运行（candle 张量 !Send），通过 channel 通信。

use async_trait::async_trait;
use axagent_harness::InferenceEngine as InferenceEngineTrait;
use axagent_harness::core_error::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
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
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
        }
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
        };

        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::Builder::new()
            .name(format!("inf-{}", filename))
            .spawn(move || {
                worker_main(&gguf, &tok, kind, kind_label, receiver);
            })
            .map_err(|e| {
                axagent_harness::core_error::AxAgentError::Inference(format!("spawn thread: {}", e))
            })?;

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
                    .map_err(|e| {
                        axagent_harness::core_error::AxAgentError::Inference(format!("send: {}", e))
                    })?;
                rx.await.map_err(|_| {
                    axagent_harness::core_error::AxAgentError::Inference("worker down".into())
                })?
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
                    .map_err(|e| {
                        axagent_harness::core_error::AxAgentError::Inference(format!("send: {}", e))
                    })?;
                rx.await.map_err(|_| {
                    axagent_harness::core_error::AxAgentError::Inference("worker down".into())
                })?
            },
            _ => Ok(heuristic_judge(query, chunk)),
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
}

// ── Worker 主循环 ──────────────────────────────────────────────────────────

fn worker_main(
    gguf: &Path,
    tok: &Path,
    kind: ModelKind,
    label: &str,
    rx: std::sync::mpsc::Receiver<WorkMsg>,
) {
    #[cfg(not(target_os = "android"))]
    let loaded = load_candle_model(gguf, tok, kind);

    #[cfg(target_os = "android")]
    let loaded: Option<()> = None;

    for msg in rx {
        match msg {
            WorkMsg::Rerank {
                query,
                documents,
                reply,
            } => {
                let scores = heuristic_rerank(&query, &documents);
                let _ = reply.send(Ok(scores));
            },
            WorkMsg::Judge {
                query,
                chunk_content,
                reply,
            } => {
                #[cfg(not(target_os = "android"))]
                let result = match &loaded {
                    Some(m) => candle_judge(m, &query, &chunk_content),
                    None => Ok(heuristic_judge(&query, &chunk_content)),
                };
                #[cfg(target_os = "android")]
                let result: Result<JudgeOutput> = Ok(heuristic_judge(&query, &chunk_content));
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
    #[allow(dead_code)]
    kind: ModelKind,
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
            Some(CandleModel {
                kind,
                model,
                tokenizer,
            })
        },
        ModelKind::Reranker => {
            tracing::info!("Reranker: heuristic mode (candle BERT GGUF requires 0.9+)");
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
            $e.map_err(|e| axagent_harness::core_error::AxAgentError::Inference(e.to_string()))?
        };
    }

    let prompt = format!(
        "<|im_start|>system\nJudge relevance. Reply ONLY YES or NO.\n<|im_end|>\n\
         <|im_start|>user\nQuery: {}\nChunk: {}\nRelevant? YES/NO:<|im_end|>\n<|im_start|>assistant\n",
        query, chunk
    );

    let dev = Device::Cpu;
    let enc = m.tokenizer.encode(prompt, true).map_err(|e| {
        axagent_harness::core_error::AxAgentError::Inference(format!("tokenize: {}", e))
    })?;
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

    let out = m.tokenizer.decode(&tokens, false).map_err(|e| {
        axagent_harness::core_error::AxAgentError::Inference(format!("decode: {}", e))
    })?;
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
    JudgeOutput {
        relevant: score >= 0.3,
        score,
        reason: format!("{}/{} terms", m, terms.len()),
    }
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
        let o = InferenceEngine::new()
            .judge("x", "rust code", "rust programming")
            .await
            .unwrap();
        assert!(o.relevant);
    }

    #[tokio::test]
    async fn test_judge_fallback_irrelevant() {
        let o = InferenceEngine::new()
            .judge("x", "rust programming", "python django")
            .await
            .unwrap();
        assert!(!o.relevant || o.score < 0.5);
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
        let o = JudgeOutput {
            relevant: true,
            score: 0.8,
            reason: "ok".into(),
        };
        assert!(o.relevant);
        assert!(o.score > 0.5);
    }
}
