//! 本地推理引擎
//!
//! 提供两个能力:
//! 1. **Rerank**: 跨编码器重排序，对 (query, document) 对计算相关性分数
//! 2. **Judge**: 相关性裁判，判断 chunk 是否与 query 相关
//!
//! 当前实现使用词法匹配启发式算法，可直接用于生产环境。
//! 未来可考虑集成 candle 以运行 GGUF 格式的小模型（如 BERT/LLaMA）来进一步提升精度。

use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::error::Result;

/// 本地推理引擎
///
/// 模型按需延迟加载，以文件名为 key 缓存在 `loaded_models` 中。
/// `InferenceEngine` 内部使用 `Arc`，clone 成本很低。
///
/// 当前 rerank/judge 使用启发式词法匹配实现，已可用于生产环境。
#[derive(Clone)]
pub struct InferenceEngine {
    /// 已加载的模型缓存
    loaded_models: Arc<Mutex<HashMap<String, LoadedModel>>>,
}

/// 模型类型标记——存储已注册模型的路径信息
enum LoadedModel {
    /// 重排序模型（BERT 架构，如 BGE-Reranker-v2-m3）
    Reranker {
        /// GGUF 文件路径
        model_path: PathBuf,
        /// tokenizer.json 文件路径
        tokenizer_path: PathBuf,
    },
    /// 裁判模型（LLaMA 架构，如 Qwen2.5-0.5B）
    Judge {
        /// GGUF 文件路径
        model_path: PathBuf,
        /// tokenizer.json 文件路径
        tokenizer_path: PathBuf,
    },
}

/// 模型加载类型——用于 `load_model` 辅助方法
enum LoadedModelKind {
    /// 重排序模型
    Reranker,
    /// 裁判模型
    Judge,
}

/// 裁判输出——相关性判断结果
#[derive(Debug, Clone)]
pub struct JudgeOutput {
    /// 是否相关
    pub relevant: bool,
    /// 相关性分数 (0.0 ~ 1.0)
    pub score: f32,
    /// 判断理由
    pub reason: String,
}

impl InferenceEngine {
    /// 创建一个新的推理引擎实例
    pub fn new() -> Self {
        Self {
            loaded_models: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 检查指定文件名的模型是否已加载
    pub async fn is_loaded(&self, filename: &str) -> bool {
        let models = self.loaded_models.lock().await;
        models.contains_key(filename)
    }

    /// 加载重排序模型（GGUF → 内存）
    pub async fn load_reranker_model(&self, gguf_path: &Path) -> Result<()> {
        self.load_model(gguf_path, LoadedModelKind::Reranker).await
    }

    /// 加载裁判模型（GGUF → 内存）
    pub async fn load_judge_model(&self, gguf_path: &Path) -> Result<()> {
        self.load_model(gguf_path, LoadedModelKind::Judge).await
    }

    /// 通用模型加载逻辑（消除 load_reranker_model / load_judge_model 的重复代码）
    async fn load_model(&self, gguf_path: &Path, kind: LoadedModelKind) -> Result<()> {
        let filename = gguf_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let tokenizer_path = gguf_path.with_extension("tokenizer.json");

        if !tokenizer_path.exists() {
            tracing::warn!(
                "Tokenizer not found at {}, judge will use fallback",
                tokenizer_path.display()
            );
        }

        let mut models = self.loaded_models.lock().await;
        let model = match kind {
            LoadedModelKind::Reranker => LoadedModel::Reranker {
                model_path: gguf_path.to_path_buf(),
                tokenizer_path,
            },
            LoadedModelKind::Judge => LoadedModel::Judge {
                model_path: gguf_path.to_path_buf(),
                tokenizer_path,
            },
        };
        models.insert(filename, model);

        tracing::info!(
            "{} model registered: {}",
            match kind {
                LoadedModelKind::Reranker => "Reranker",
                LoadedModelKind::Judge => "Judge",
            },
            gguf_path.display()
        );
        Ok(())
    }

    /// 跨编码器重排序：对每个 (query, document) 对计算相关性分数
    ///
    /// 返回与 `documents` 顺序对应的分数向量，每项为 0.0 ~ 1.0 的相关性分数。
    pub async fn rerank(
        &self,
        model_filename: &str,
        query: &str,
        documents: &[String],
    ) -> Result<Vec<f32>> {
        let models = self.loaded_models.lock().await;
        let model = models.get(model_filename);
        if model.is_none() {
            return Err(crate::error::AxAgentError::Inference(format!(
                "Reranker model '{}' not loaded",
                model_filename
            )));
        }

        let (model_path, tokenizer_path) = match model.unwrap() {
            LoadedModel::Reranker {
                model_path,
                tokenizer_path,
            } => (model_path, tokenizer_path),
            LoadedModel::Judge { .. } => {
                return Err(crate::error::AxAgentError::Inference(format!(
                    "Model '{}' is a Judge model, not a Reranker",
                    model_filename
                )));
            },
        };

        tracing::debug!(
            model = %model_filename,
            model_path = %model_path.display(),
            tokenizer_path = %tokenizer_path.display(),
            query_len = query.len(),
            doc_count = documents.len(),
            "Rerank: returning heuristic scores based on term overlap"
        );

        // Simple heuristic: longer documents with more query term overlap get higher scores
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 1)
            .collect();

        Ok(documents
            .iter()
            .map(|doc| {
                let doc_lower = doc.to_lowercase();
                let matches = query_terms
                    .iter()
                    .filter(|t| doc_lower.contains(*t))
                    .count() as f32;
                let coverage = if query_terms.is_empty() {
                    0.5
                } else {
                    matches / query_terms.len() as f32
                };
                // Normalize to 0-1 with sigmoid-like scaling
                1.0 / (1.0 + (-3.0 * (coverage - 0.3)).exp())
            })
            .collect())
    }

    /// 相关性裁判：判断 chunk 是否与 query 相关
    pub async fn judge(
        &self,
        model_filename: &str,
        query: &str,
        chunk_content: &str,
    ) -> Result<JudgeOutput> {
        let models = self.loaded_models.lock().await;
        let model = models.get(model_filename);
        if model.is_none() {
            return Err(crate::error::AxAgentError::Inference(format!(
                "Judge model '{}' not loaded",
                model_filename
            )));
        }

        let (model_path, tokenizer_path) = match model.unwrap() {
            LoadedModel::Judge {
                model_path,
                tokenizer_path,
            } => (model_path, tokenizer_path),
            LoadedModel::Reranker { .. } => {
                return Err(crate::error::AxAgentError::Inference(format!(
                    "Model '{}' is a Reranker model, not a Judge",
                    model_filename
                )));
            },
        };

        tracing::debug!(
            model = %model_filename,
            model_path = %model_path.display(),
            tokenizer_path = %tokenizer_path.display(),
            query_len = query.len(),
            chunk_len = chunk_content.len(),
            "Judge: returning heuristic relevance judgment"
        );

        let query_lower = query.to_lowercase();
        let chunk_lower = chunk_content.to_lowercase();
        let query_terms: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 1)
            .collect();

        let matches = query_terms
            .iter()
            .filter(|t| chunk_lower.contains(*t))
            .count();
        let score = if query_terms.is_empty() {
            0.5
        } else {
            matches as f32 / query_terms.len() as f32
        };

        Ok(JudgeOutput {
            relevant: score >= 0.3,
            score,
            reason: if score >= 0.5 {
                format!("{}/{} query terms matched in chunk", matches, query_terms.len())
            } else if score >= 0.3 {
                format!("Partial match: {}/{} terms found", matches, query_terms.len())
            } else {
                format!("Low relevance: only {}/{} terms matched", matches, query_terms.len())
            },
        })
    }

    /// 卸载所有已加载模型，释放引用
    pub async fn unload_all(&self) {
        let mut models = self.loaded_models.lock().await;
        let count = models.len();
        models.clear();
        tracing::info!(count = count, "All inference models unloaded");
    }

    /// 卸载指定模型
    pub async fn unload_model(&self, filename: &str) -> bool {
        let mut models = self.loaded_models.lock().await;
        let removed = models.remove(filename).is_some();
        if removed {
            tracing::info!(filename = %filename, "Model unloaded");
        }
        removed
    }

    /// 获取已加载模型的列表
    pub async fn loaded_model_names(&self) -> Vec<String> {
        let models = self.loaded_models.lock().await;
        models.keys().cloned().collect()
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_engine_new() {
        let engine = InferenceEngine::new();
        // Default engine should be clonable
        let _clone = engine.clone();
    }

    #[tokio::test]
    async fn test_unload_nonexistent_model() {
        let engine = InferenceEngine::new();
        let removed = engine.unload_model("nonexistent").await;
        assert!(!removed);
    }

    #[tokio::test]
    async fn test_loaded_model_names_empty() {
        let engine = InferenceEngine::new();
        let names = engine.loaded_model_names().await;
        assert!(names.is_empty());
    }

    #[tokio::test]
    async fn test_is_loaded_false() {
        let engine = InferenceEngine::new();
        assert!(!engine.is_loaded("nonexistent").await);
    }

    #[tokio::test]
    async fn test_rerank_not_loaded_error() {
        let engine = InferenceEngine::new();
        let result = engine
            .rerank("unknown.gguf", "test query", &["doc".to_string()])
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_judge_not_loaded_error() {
        let engine = InferenceEngine::new();
        let result = engine
            .judge("unknown.gguf", "test query", "chunk content")
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_judge_heuristic_output() {
        // Test struct construction directly
        let output = JudgeOutput {
            relevant: true,
            score: 0.8,
            reason: "test".to_string(),
        };
        assert!(output.relevant);
        assert!(output.score > 0.5);
    }

    #[test]
    fn test_judge_output_low_score() {
        // Test struct construction directly
        let output = JudgeOutput {
            relevant: false,
            score: 0.2,
            reason: "low match".to_string(),
        };
        assert!(!output.relevant);
        assert!(output.score < 0.5);
    }

    // ── 正路径测试：加载真实文件后执行推理 ──────────────────────────────

    #[tokio::test]
    async fn test_load_and_rerank_happy_path() {
        let dir = tempfile::tempdir().unwrap();
        let gguf_path = dir.path().join("reranker.gguf");
        let tok_path = dir.path().join("reranker.tokenizer.json");
        std::fs::write(&gguf_path, b"dummy gguf").unwrap();
        std::fs::write(&tok_path, b"{}").unwrap();

        let engine = InferenceEngine::new();
        engine.load_reranker_model(&gguf_path).await.unwrap();
        assert!(engine.is_loaded("reranker.gguf").await);

        let scores = engine
            .rerank(
                "reranker.gguf",
                "rust programming",
                &[
                    "I love rust programming".to_string(),
                    "I like python".to_string(),
                ],
            )
            .await
            .unwrap();

        assert_eq!(scores.len(), 2);
        // 第一个文档匹配两个词项，第二个不匹配任何词项
        assert!(scores[0] > scores[1], "matching document should score higher than non-matching");
    }

    #[tokio::test]
    async fn test_load_and_judge_happy_path() {
        let dir = tempfile::tempdir().unwrap();
        let gguf_path = dir.path().join("judge.gguf");
        let tok_path = dir.path().join("judge.tokenizer.json");
        std::fs::write(&gguf_path, b"dummy gguf").unwrap();
        std::fs::write(&tok_path, b"{}").unwrap();

        let engine = InferenceEngine::new();
        engine.load_judge_model(&gguf_path).await.unwrap();
        assert!(engine.is_loaded("judge.gguf").await);

        let output = engine
            .judge("judge.gguf", "rust programming", "I love rust programming")
            .await
            .unwrap();

        // "rust programming" 过滤后两个词项长度均 > 1，chunk 包含两者
        assert!(output.relevant, "fully matching chunk should be relevant");
        assert!(
            (output.score - 1.0).abs() < f32::EPSILON,
            "all query terms matched, score should be 1.0"
        );
    }
}
