//! HuggingFace 扫描器
//! 通过 HuggingFace API 采集模型和应用的需求信号

use super::marketplace_scanner::{MarketplaceScanner, RawLead};
use async_trait::async_trait;

/// HuggingFace 扫描器
pub struct HuggingFaceScanner {
    http: reqwest::Client,
    api_token: Option<String>,
}

impl HuggingFaceScanner {
    pub fn new() -> Self {
        let http = reqwest::Client::new();
        let api_token = std::env::var("HF_API_TOKEN").ok();
        Self { http, api_token }
    }

    /// 构建模型搜索 URL
    fn build_models_search_url(query: &str, limit: u32) -> String {
        format!(
            "https://huggingface.co/api/models?search={}&limit={}&sort=downloads&direction=-1",
            query, limit
        )
    }

    /// 构建 Space 搜索 URL
    fn build_spaces_search_url(query: &str, limit: u32) -> String {
        format!(
            "https://huggingface.co/api/spaces?search={}&limit={}&sort=likes&direction=-1",
            query, limit
        )
    }

    /// 构建请求头
    fn build_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Some(ref token) = self.api_token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// 需求相关标签
    fn demand_tags() -> Vec<&'static str> {
        vec![
            "text-generation",
            "translation",
            "summarization",
            "question-answering",
            "text-classification",
            "image-generation",
            "image-to-text",
            "text-to-image",
            "audio",
            "speech",
            "voice",
            "object-detection",
            "image-segmentation",
            "tabular",
            "time-series",
            "reinforcement-learning",
            "robotics",
            "text-to-video",
            "video-classification",
        ]
    }

    /// 检查模型是否与需求相关
    fn is_demand_model(
        model_id: &str,
        pipeline_tag: &str,
        tags: &[String],
        downloads: u64,
    ) -> bool {
        let demand_keywords = [
            "api",
            "integration",
            "deploy",
            "production",
            "real-time",
            "streaming",
            "low-latency",
            "custom",
            "fine-tune",
            "fine tuning",
            "finetune",
            "financial",
            "trading",
            "stock",
            "multi-modal",
            "multimodal",
            "agent",
            "agentic",
            "autonomous",
            "embedding",
            "embeddings",
            "vector",
        ];

        let full_text = format!("{} {} {}", model_id, pipeline_tag, tags.join(" ")).to_lowercase();
        let has_demand_keyword = demand_keywords.iter().any(|kw| full_text.contains(kw));

        let demand_tags_set: std::collections::HashSet<&str> =
            Self::demand_tags().into_iter().collect();
        let has_demand_tag =
            tags.iter().any(|t| demand_tags_set.contains(t.to_lowercase().as_str()));

        // 下载量大的热门模型也视为有价值信号
        let is_popular = downloads >= 100_000;

        has_demand_keyword || has_demand_tag || is_popular
    }

    /// 提取模型趋势描述
    fn extract_model_trend(
        model_id: &str,
        pipeline_tag: &str,
        downloads: u64,
        likes: u64,
    ) -> Option<String> {
        let tag = pipeline_tag.to_lowercase();

        let trend_map: Vec<(&str, &str)> = vec![
            ("text-generation", "文本生成"),
            ("translation", "翻译模型"),
            ("summarization", "摘要生成"),
            ("question-answering", "问答系统"),
            ("image-generation", "图像生成"),
            ("audio", "音频处理"),
            ("speech", "语音识别"),
            ("tabular", "表格预测"),
            ("time-series", "时间序列预测"),
            ("reinforcement-learning", "强化学习"),
        ];

        let trend_type = trend_map.iter().find(|(k, _)| tag.contains(k));
        let trend_label = trend_type.map(|(_, v)| *v).unwrap_or("通用 AI 模型");

        if downloads > 1_000_000 {
            Some(format!(
                "[热门] {} | {} | {} 次下载 | {} 点赞",
                trend_label, model_id, downloads, likes
            ))
        } else if downloads > 100_000 {
            Some(format!("[流行] {} | {} | {} 次下载", trend_label, model_id, downloads))
        } else {
            None
        }
    }
}

impl Default for HuggingFaceScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MarketplaceScanner for HuggingFaceScanner {
    fn platform(&self) -> &'static str {
        "huggingface"
    }

    async fn search(&self, q: &str) -> Result<Vec<RawLead>, String> {
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let mut leads = Vec::new();
        let headers = self.build_headers();

        // 搜索模型
        let models_url = Self::build_models_search_url(q, 30);
        tracing::info!(query = q, "[HuggingFaceScanner] 搜索模型");

        let models_response = self
            .http
            .get(&models_url)
            .headers(headers.clone())
            .send()
            .await
            .map_err(|e| format!("HuggingFace API 请求失败: {}", e))?;

        if models_response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err("HuggingFace API 速率限制".to_string());
        }

        if models_response.status().is_success() {
            let models: Vec<serde_json::Value> =
                models_response.json().await.map_err(|e| format!("响应解析失败: {}", e))?;

            for model in &models {
                let model_id = model["id"].as_str().unwrap_or("").to_string();
                let pipeline_tag = model["pipeline_tag"].as_str().unwrap_or("").to_string();
                let downloads = model["downloads"].as_i64().unwrap_or(0) as u64;
                let likes = model["likes"].as_i64().unwrap_or(0) as u64;

                let tags: Vec<String> = model["tags"]
                    .as_array()
                    .map(|arr| {
                        arr.iter().filter_map(|t| t.as_str().map(|s| s.to_string())).collect()
                    })
                    .unwrap_or_default();

                if Self::is_demand_model(&model_id, &pipeline_tag, &tags, downloads)
                    && let Some(trend) =
                        Self::extract_model_trend(&model_id, &pipeline_tag, downloads, likes)
                {
                    let url = format!("https://huggingface.co/{}", model_id);
                    let mut snapshot = model.clone();
                    snapshot["_extracted_trend"] = serde_json::json!(trend);
                    snapshot["_extracted_source"] = serde_json::json!("huggingface_model");

                    leads.push(RawLead {
                        platform: "huggingface".to_string(),
                        title: format!("Model: {}", model_id),
                        description: trend,
                        url,
                        price_text: None,
                        contact: None,
                        snapshot,
                    });
                }
            }
        }

        // 搜索 Spaces
        let spaces_url = Self::build_spaces_search_url(q, 20);
        tracing::info!(query = q, "[HuggingFaceScanner] 搜索 Spaces");

        if let Ok(spaces_response) = self.http.get(&spaces_url).headers(headers).send().await
            && spaces_response.status().is_success()
        {
            let spaces: Vec<serde_json::Value> = spaces_response.json().await.unwrap_or_default();

            for space in &spaces {
                let space_id = space["id"].as_str().unwrap_or("").to_string();
                let likes = space["likes"].as_i64().unwrap_or(0) as u64;
                let space_type = space["sdk"].as_str().unwrap_or("unknown").to_string();

                if likes >= 50 {
                    let url = format!("https://huggingface.co/{}", space_id);
                    let mut snapshot = space.clone();
                    snapshot["_extracted_source"] = serde_json::json!("huggingface_space");

                    leads.push(RawLead {
                        platform: "huggingface".to_string(),
                        title: format!("Space: {}", space_id),
                        description: format!(
                            "[Space] {} | {} | {} 点赞",
                            space_id, space_type, likes
                        ),
                        url,
                        price_text: None,
                        contact: None,
                        snapshot,
                    });
                }
            }
        }

        tracing::info!(query = q, filtered = leads.len(), "[HuggingFaceScanner] 搜索完成");

        Ok(leads)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let scanner = HuggingFaceScanner::new();
        assert_eq!(scanner.platform(), "huggingface");
    }

    #[test]
    fn test_build_search_url() {
        let url = HuggingFaceScanner::build_models_search_url("llm", 10);
        assert!(url.contains("huggingface.co"));
        assert!(url.contains("models"));
    }

    #[test]
    fn test_is_demand_model() {
        // 热门模型
        assert!(HuggingFaceScanner::is_demand_model(
            "llama-3",
            "text-generation",
            &["transformers".to_string()],
            1_000_000
        ));

        // 需求标签
        assert!(HuggingFaceScanner::is_demand_model(
            "custom-model",
            "",
            &["time-series".to_string()],
            100
        ));

        // 需求关键词
        assert!(HuggingFaceScanner::is_demand_model("finetune-model", "", &[], 50));

        // 不相关且不热门
        assert!(!HuggingFaceScanner::is_demand_model(
            "unknown-model",
            "",
            &["other".to_string()],
            10
        ));
    }

    #[test]
    fn test_extract_model_trend() {
        // 热门模型
        let trend = HuggingFaceScanner::extract_model_trend(
            "llama-3",
            "text-generation",
            5_000_000,
            100_000,
        );
        assert!(trend.is_some());
        assert!(trend.unwrap().contains("热门"));

        // 流行模型
        let trend = HuggingFaceScanner::extract_model_trend("bert-base", "", 200_000, 50_000);
        assert!(trend.is_some());
        assert!(trend.unwrap().contains("流行"));

        // 小众模型
        let trend = HuggingFaceScanner::extract_model_trend("small-model", "", 5_000, 100);
        assert!(trend.is_none());

        // 翻译模型
        let trend = HuggingFaceScanner::extract_model_trend(
            "translate-model",
            "translation",
            500_000,
            20_000,
        );
        assert!(trend.is_some());
        assert!(trend.unwrap().contains("翻译"));
    }

    #[tokio::test]
    async fn test_search_without_token() {
        let scanner = HuggingFaceScanner::new();
        let result = scanner.search("llm").await;
        assert!(result.is_ok());
    }
}
