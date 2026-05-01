use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;

use super::{
    TransportProvider, TransportRequest, TransportResponse, TransportStreamChunk,
    TransportToolCall, TransportUsage,
};

pub struct ChatCompletionsTransport;

impl ChatCompletionsTransport {
    fn build_body(&self, request: &TransportRequest) -> Value {
        let mut body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| {
                let mut msg = serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                });
                if let Some(ref tc) = m.tool_calls {
                    msg["tool_calls"] = tc.clone();
                }
                if let Some(ref tci) = m.tool_call_id {
                    msg["tool_call_id"] = serde_json::Value::String(tci.clone());
                }
                msg
            }).collect::<Vec<_>>(),
            "stream": request.stream,
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(ref tools) = request.tools {
            body["tools"] = tools.clone();
        }

        body
    }

    fn parse_response(&self, json: &Value) -> anyhow::Result<TransportResponse> {
        let choices = json["choices"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;
        let first = choices
            .first()
            .ok_or_else(|| anyhow::anyhow!("Empty choices"))?;

        let content = first["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let tool_calls = first["message"]["tool_calls"].as_array().map(|arr| {
            arr.iter()
                .map(|tc| TransportToolCall {
                    id: tc["id"].as_str().unwrap_or("").to_string(),
                    name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    arguments: tc["function"]["arguments"]
                        .as_str()
                        .unwrap_or("{}")
                        .to_string(),
                })
                .collect()
        });

        let usage = json["usage"]
            .as_object()
            .map_or(TransportUsage::default(), |u| TransportUsage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            });

        Ok(TransportResponse {
            content,
            tool_calls,
            usage,
            finish_reason: first["finish_reason"].as_str().map(|s| s.to_string()),
        })
    }
}

#[async_trait]
impl TransportProvider for ChatCompletionsTransport {
    fn provider_name(&self) -> &'static str {
        "openai"
    }

    async fn send(
        &self,
        request: TransportRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> anyhow::Result<TransportResponse> {
        let url = format!(
            "{}/chat/completions",
            base_url.unwrap_or("https://api.openai.com/v1")
        );

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&self.build_body(&request))
            .send()
            .await?;

        let json: Value = resp.json().await?;
        self.parse_response(&json)
    }

    async fn send_streaming(
        &self,
        request: TransportRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> anyhow::Result<
        Box<dyn futures::Stream<Item = anyhow::Result<TransportStreamChunk>> + Send + Unpin>,
    > {
        let url = format!(
            "{}/chat/completions",
            base_url.unwrap_or("https://api.openai.com/v1")
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&self.build_body(&request))
            .send()
            .await?;

        let stream = response.bytes_stream().filter_map(|chunk| {
            let result = match chunk {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).to_string();
                    parse_sse_chunk(&text).transpose()
                },
                Err(e) => Some(Err(anyhow::anyhow!("Stream error: {}", e))),
            };
            std::future::ready(result)
        });

        Ok(Box::new(stream))
    }
}

fn parse_sse_chunk(line: &str) -> anyhow::Result<Option<TransportStreamChunk>> {
    for segment in line.split('\n') {
        let segment = segment.trim();
        if segment.is_empty() || segment == "data: [DONE]" {
            continue;
        }
        if let Some(data) = segment.strip_prefix("data: ") {
            let json: Value = serde_json::from_str(data)?;
            let choices = json["choices"].as_array();
            if let Some(choices) = choices {
                if let Some(first) = choices.first() {
                    let content = first["delta"]["content"].as_str().map(|s| s.to_string());
                    let finish_reason = first["finish_reason"].as_str().map(|s| s.to_string());
                    let usage = json["usage"].as_object().map(|u| TransportUsage {
                        prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                        completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                        total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
                    });

                    if content.is_some() || finish_reason.is_some() || usage.is_some() {
                        return Ok(Some(TransportStreamChunk {
                            content,
                            tool_calls: None,
                            finish_reason,
                            usage,
                        }));
                    }
                }
            }
        }
    }
    Ok(None)
}
