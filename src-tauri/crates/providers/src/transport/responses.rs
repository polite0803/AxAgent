use async_trait::async_trait;

use super::{TransportProvider, TransportRequest, TransportResponse, TransportStreamChunk};

pub struct ResponsesTransport;

#[async_trait]
impl TransportProvider for ResponsesTransport {
    fn provider_name(&self) -> &'static str {
        "openai_responses"
    }

    async fn send(
        &self,
        _request: TransportRequest,
        _api_key: &str,
        _base_url: Option<&str>,
    ) -> anyhow::Result<TransportResponse> {
        anyhow::bail!("Responses transport: use the native protocol adapter for full support");
    }

    async fn send_streaming(
        &self,
        _request: TransportRequest,
        _api_key: &str,
        _base_url: Option<&str>,
    ) -> anyhow::Result<
        Box<dyn futures::Stream<Item = anyhow::Result<TransportStreamChunk>> + Send + Unpin>,
    > {
        anyhow::bail!("Responses transport: use the native protocol adapter for full support");
    }
}
