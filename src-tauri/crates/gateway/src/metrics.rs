use axum::{http::StatusCode, response::IntoResponse};
use prometheus::Encoder;

pub async fn metrics_handler() -> impl IntoResponse {
    let metric_families = prometheus::gather();
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        tracing::error!("Failed to encode metrics: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to encode metrics",
        )
            .into_response();
    }

    match String::from_utf8(buffer) {
        Ok(output) => (StatusCode::OK, axum::response::Html(output)).into_response(),
        Err(e) => {
            tracing::error!("Failed to convert metrics to string: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to convert metrics",
            )
                .into_response()
        },
    }
}
