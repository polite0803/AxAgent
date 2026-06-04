use serde::{Deserialize, Serialize};

// ── 配置 ──────────────────────────────────────────────────

/// SelfRagConfig 定义在 axagent-harness::rag_config
pub use axagent_harness::rag_config::SelfRagConfig;

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
    ) -> axagent_harness::core_error::Result<Vec<RelevanceJudgment>> {
        if !self.config.enabled || chunks.is_empty() {
            return Ok(chunks
                .iter()
                .map(|(id, _)| RelevanceJudgment {
                    chunk_id: id.clone(),
                    relevant: true,
                    score: 1.0,
                    reason: "Self-RAG disabled".to_string(),
                })
                .collect());
        }

        let client = reqwest::Client::new();

        let judgments: Vec<RelevanceJudgment> =
            futures::future::join_all(chunks.iter().map(|(id, content)| {
                let query = query.to_string();
                let content = content.clone();
                let config = self.config.clone();
                let client = client.clone();
                async move { judge_single(&client, &config, id, &query, &content).await }
            }))
            .await
            .into_iter()
            .map(|r| {
                r.unwrap_or_else(|e| {
                    tracing::warn!("Judge failed for chunk: {}", e);
                    RelevanceJudgment {
                        chunk_id: "unknown".to_string(),
                        relevant: true,
                        score: 0.5,
                        reason: format!("judge error: {}", e),
                    }
                })
            })
            .collect();

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
                ratio * 100.0,
                avg_score
            ))
        }
    }

    /// 生成精炼后的查询（用于纠正循环）
    pub async fn refine_query(
        &self,
        original: &str,
        quality_diag: &str,
    ) -> axagent_harness::core_error::Result<String> {
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
            .map_err(|e| {
                axagent_harness::core_error::AxAgentError::Provider(format!(
                    "Ollama refine query failed: {}",
                    e
                ))
            })?;

        let data: serde_json::Value = resp.json().await.map_err(|e| {
            axagent_harness::core_error::AxAgentError::Provider(format!(
                "Parse refine response: {}",
                e
            ))
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
) -> axagent_harness::core_error::Result<RelevanceJudgment> {
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
        .map_err(|e| {
            axagent_harness::core_error::AxAgentError::Provider(format!(
                "Ollama judge request failed: {}",
                e
            ))
        })?;

    let data: serde_json::Value = resp.json().await.map_err(|e| {
        axagent_harness::core_error::AxAgentError::Provider(format!("Judge response parse: {}", e))
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
            RelevanceJudgment {
                chunk_id: "1".into(),
                relevant: true,
                score: 0.9,
                reason: "ok".into(),
            },
            RelevanceJudgment {
                chunk_id: "2".into(),
                relevant: true,
                score: 0.8,
                reason: "ok".into(),
            },
            RelevanceJudgment {
                chunk_id: "3".into(),
                relevant: true,
                score: 0.7,
                reason: "ok".into(),
            },
            RelevanceJudgment {
                chunk_id: "4".into(),
                relevant: false,
                score: 0.3,
                reason: "no".into(),
            },
            RelevanceJudgment {
                chunk_id: "5".into(),
                relevant: true,
                score: 0.85,
                reason: "ok".into(),
            },
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
            RelevanceJudgment {
                chunk_id: "1".into(),
                relevant: false,
                score: 0.2,
                reason: "no".into(),
            },
            RelevanceJudgment {
                chunk_id: "2".into(),
                relevant: false,
                score: 0.1,
                reason: "no".into(),
            },
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
