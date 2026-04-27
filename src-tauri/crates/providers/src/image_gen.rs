use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageGenError {
    #[error("API request failed: {0}")]
    ApiError(#[from] reqwest::Error),
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Timeout waiting for prediction")]
    Timeout,
    #[error("Prediction failed: {0}")]
    PredictionFailed(String),
}

pub type Result<T> = std::result::Result<T, ImageGenError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub steps: Option<u32>,
    pub seed: Option<u64>,
    pub model: Option<String>,
    pub n: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenResponse {
    pub images: Vec<GeneratedImage>,
    pub model_used: String,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub url: Option<String>,
    pub base64: Option<String>,
    pub width: u32,
    pub height: u32,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenModelInfo {
    pub name: String,
    pub provider: String,
    pub supported_sizes: Vec<String>,
}

#[async_trait]
pub trait ImageGenProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn generate(&self, request: ImageGenRequest) -> Result<ImageGenResponse>;
}

pub struct FluxProvider {
    api_token: String,
    client: Client,
}

impl FluxProvider {
    pub fn new(api_token: String) -> Self {
        Self {
            api_token,
            client: Client::new(),
        }
    }
}

#[derive(Serialize)]
struct ReplicatePrediction {
    version: String,
    input: ReplicateInput,
}

#[derive(Serialize)]
struct ReplicateInput {
    prompt: String,
    negative_prompt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    num_inference_steps: Option<u32>,
    seed: Option<u64>,
}

#[derive(Deserialize)]
struct ReplicateResponse {
    id: String,
    status: String,
    output: Option<Vec<String>>,
}

#[async_trait]
impl ImageGenProvider for FluxProvider {
    fn name(&self) -> &str {
        "flux"
    }

    async fn generate(&self, request: ImageGenRequest) -> Result<ImageGenResponse> {
        let start = std::time::Instant::now();

        let prediction = ReplicatePrediction {
            version: "black-forest-labs/flux-schnell".to_string(),
            input: ReplicateInput {
                prompt: request.prompt,
                negative_prompt: request.negative_prompt,
                width: request.width.or(Some(1024)),
                height: request.height.or(Some(1024)),
                num_inference_steps: request.steps.or(Some(4)),
                seed: request.seed,
            },
        };

        let resp = self
            .client
            .post("https://api.replicate.com/v1/predictions")
            .header("Authorization", format!("Token {}", self.api_token))
            .json(&prediction)
            .send()
            .await?;

        let mut replicate_resp: ReplicateResponse = resp.json().await?;

        let poll_url = format!(
            "https://api.replicate.com/v1/predictions/{}",
            replicate_resp.id
        );
        for _ in 0..60 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let poll_resp = self
                .client
                .get(&poll_url)
                .header("Authorization", format!("Token {}", self.api_token))
                .send()
                .await?;
            replicate_resp = poll_resp.json().await?;
            if replicate_resp.status == "succeeded" || replicate_resp.status == "failed" {
                break;
            }
        }

        let images = replicate_resp
            .output
            .unwrap_or_default()
            .into_iter()
            .map(|url| GeneratedImage {
                url: Some(url),
                base64: None,
                width: request.width.unwrap_or(1024),
                height: request.height.unwrap_or(1024),
                seed: request.seed,
            })
            .collect();

        Ok(ImageGenResponse {
            images,
            model_used: "flux-schnell".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }
}

pub struct DallEProvider {
    api_key: String,
    base_url: String,
    client: Client,
}

impl DallEProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ImageGenProvider for DallEProvider {
    fn name(&self) -> &str {
        "dall-e"
    }

    async fn generate(&self, request: ImageGenRequest) -> Result<ImageGenResponse> {
        let start = std::time::Instant::now();

        let size = format!(
            "{}x{}",
            request.width.unwrap_or(1024),
            request.height.unwrap_or(1024)
        );

        let body = serde_json::json!({
            "model": request.model.as_deref().unwrap_or("dall-e-3"),
            "prompt": request.prompt,
            "n": request.n.unwrap_or(1),
            "size": size,
            "quality": "standard",
            "response_format": "b64_json"
        });

        let resp = self
            .client
            .post(format!("{}/images/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        #[derive(Deserialize)]
        struct DallEResponse {
            data: Vec<DallEImage>,
        }
        #[derive(Deserialize)]
        struct DallEImage {
            b64_json: Option<String>,
            url: Option<String>,
            #[allow(dead_code)]
            revised_prompt: Option<String>,
        }

        let dalle_resp: DallEResponse = resp.json().await?;

        let images = dalle_resp
            .data
            .into_iter()
            .map(|img| GeneratedImage {
                url: img.url,
                base64: img.b64_json,
                width: request.width.unwrap_or(1024),
                height: request.height.unwrap_or(1024),
                seed: None,
            })
            .collect();

        Ok(ImageGenResponse {
            images,
            model_used: "dall-e-3".to_string(),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }
}
