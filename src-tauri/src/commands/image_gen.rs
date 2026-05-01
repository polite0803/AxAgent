use axagent_providers::image_gen::{
    DallEProvider, FluxProvider, ImageGenProvider, ImageGenRequest, ImageGenResponse,
};
use tauri::command;

#[command]
pub async fn generate_image(
    prompt: String,
    negative_prompt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    steps: Option<u32>,
    seed: Option<u64>,
    model: Option<String>,
    provider: Option<String>,
    api_key: Option<String>,
) -> Result<ImageGenResponse, String> {
    let provider_name = provider.as_deref().unwrap_or("flux");

    let request = ImageGenRequest {
        prompt,
        negative_prompt,
        width,
        height,
        steps,
        seed,
        model,
        n: Some(1),
    };

    match provider_name {
        "flux" | "Flux" => {
            let api_token =
                api_key.ok_or_else(|| "API key required for Flux provider".to_string())?;
            let provider = FluxProvider::new(api_token);
            provider.generate(request).await.map_err(|e| e.to_string())
        },
        "dall-e" | "dalle" | "DALL-E" => {
            let api_key =
                api_key.ok_or_else(|| "API key required for DALL-E provider".to_string())?;
            let provider = DallEProvider::new(api_key, None);
            provider.generate(request).await.map_err(|e| e.to_string())
        },
        _ => Err(format!(
            "Unknown provider: {}. Use 'flux' or 'dall-e'",
            provider_name
        )),
    }
}
