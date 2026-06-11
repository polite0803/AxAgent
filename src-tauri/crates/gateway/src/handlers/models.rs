use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde_json::json;
use std::collections::{HashMap, HashSet};

use axagent_harness::types::ProviderConfig;

use crate::handlers::error::error_response;
use crate::server::GatewayAppState;

/// GET /v1/models — list enabled models from all enabled providers.
///
/// Model IDs are emitted as plain `model_id` when globally unique across all
/// enabled providers, or as `provider_slug/model_id` when the same `model_id`
/// exists on more than one enabled provider (collision).  The legacy
/// `provider_uuid:model_id` format is **no longer emitted**.
///
/// Results are sorted deterministically: primary key is the displayed model ID
/// (lexicographic), secondary key is the provider name (tiebreaker for the rare
/// case of identical display IDs across multiple providers).
pub async fn list_models(State(state): State<GatewayAppState>) -> impl IntoResponse {
    let providers = match state.adapter.providers().list_providers().await {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": { "message": e.to_string() } })),
            )
                .into_response();
        },
    };

    let display_map = build_model_display_map(&providers);

    let mut models: Vec<serde_json::Value> = Vec::new();
    for provider in providers.iter().filter(|p| p.enabled) {
        for model in provider.models.iter().filter(|m| m.enabled) {
            let key = (provider.id.clone(), model.model_id.clone());
            let display_id = display_map
                .get(&key)
                .cloned()
                .unwrap_or_else(|| model.model_id.clone());
            models.push(json!({
                "id": display_id,
                "object": "model",
                "created": provider.created_at,
                "owned_by": provider.name,
            }));
        }
    }

    // Deterministic ordering: display ID first, provider name as tiebreaker.
    models.sort_by(|a, b| {
        let id_a = a["id"].as_str().unwrap_or("");
        let id_b = b["id"].as_str().unwrap_or("");
        let ob_a = a["owned_by"].as_str().unwrap_or("");
        let ob_b = b["owned_by"].as_str().unwrap_or("");
        id_a.cmp(id_b).then(ob_a.cmp(ob_b))
    });

    Json(json!({
        "object": "list",
        "data": models,
    }))
    .into_response()
}

// ── Model-name helpers ────────────────────────────────────────────────────────

/// Derive a stable, URL-safe slug from a provider's human-readable name.
///
/// Rules: lowercase, runs of non-alphanumeric characters become a single `-`,
/// leading/trailing `-` are stripped.  E.g. "OpenAI (EU)" → `"openai-eu"`.
pub(crate) fn provider_slug(name: &str) -> String {
    let raw: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    raw.split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Build a `provider_internal_id → public_id` map for all enabled providers.
///
/// The public ID is the name slug (see [`provider_slug`]).  When two or more
/// enabled providers share the same base slug (e.g. `"OpenAI"` and `"Open AI"`
/// both normalise to `"openai"`), a numeric suffix is appended (`-2`, `-3`, …)
/// in **internal-ID–sorted order** so the result is unique and deterministic.
pub(crate) fn build_provider_public_id_map(
    providers: &[ProviderConfig],
) -> HashMap<String, String> {
    // Group enabled providers by their base slug.
    let mut slug_groups: HashMap<String, Vec<String>> = HashMap::new();
    for p in providers.iter().filter(|p| p.enabled) {
        slug_groups
            .entry(provider_slug(&p.name))
            .or_default()
            .push(p.id.clone());
    }

    let mut map = HashMap::new();
    for (base_slug, mut ids) in slug_groups {
        if ids.len() == 1 {
            map.insert(ids.remove(0), base_slug);
        } else {
            // Stable tie-breaking by internal ID (lexicographic).
            ids.sort();
            for (i, id) in ids.into_iter().enumerate() {
                let public_id = if i == 0 {
                    base_slug.clone()
                } else {
                    format!("{}-{}", base_slug, i + 1)
                };
                map.insert(id, public_id);
            }
        }
    }
    map
}

/// Build a `(provider_internal_id, model_id) → display_id` map for all
/// enabled models across all enabled providers.
///
/// Display rules:
/// - If a `model_id` is **globally unique** across enabled providers → emit bare `model_id`.
/// - If the same `model_id` appears on **multiple** enabled providers → emit
///   `public_provider_id/model_id` using the ID from [`build_provider_public_id_map`].
pub(crate) fn build_model_display_map(
    providers: &[ProviderConfig],
) -> HashMap<(String, String), String> {
    let public_id_map = build_provider_public_id_map(providers);

    // Count how many enabled providers expose each model_id.
    let mut model_id_counts: HashMap<String, usize> = HashMap::new();
    for provider in providers.iter().filter(|p| p.enabled) {
        for model in provider.models.iter().filter(|m| m.enabled) {
            *model_id_counts.entry(model.model_id.clone()).or_default() += 1;
        }
    }

    let mut map = HashMap::new();
    for provider in providers.iter().filter(|p| p.enabled) {
        let public_id = public_id_map.get(&provider.id).cloned().unwrap_or_default();
        for model in provider.models.iter().filter(|m| m.enabled) {
            let count = *model_id_counts.get(&model.model_id).unwrap_or(&0);
            let display_id = if count > 1 {
                format!("{}/{}", public_id, model.model_id)
            } else {
                model.model_id.clone()
            };
            map.insert((provider.id.clone(), model.model_id.clone()), display_id);
        }
    }
    map
}

// ── Model-field parsing ───────────────────────────────────────────────────────

/// Result of parsing the `model` field from a chat completion request.
pub(crate) struct ParsedModel {
    /// Provider hint, if present (public ID from `/` separator).
    pub(crate) provider_hint: Option<String>,
    /// The bare model identifier (right-hand side, or whole string if no separator).
    pub(crate) model_id: String,
}

/// Parse the `model` field of a chat completion request.
///
/// Accepted formats:
/// 1. `provider_public_id/model_id`  — preferred namespaced form; only
///    recognised when the left segment is a **known** public provider ID.
///    This prevents misparsing native model IDs that contain `/` (e.g.
///    `"accounts/fireworks/models/qwen3"`).
/// 2. `model_id`                     — bare; resolved by unique match across providers
pub(crate) fn parse_model_field(model: &str, known_public_ids: &HashSet<String>) -> ParsedModel {
    if let Some((left, right)) = model.split_once('/')
        && known_public_ids.contains(left)
    {
        return ParsedModel {
            provider_hint: Some(left.to_string()),
            model_id: right.to_string(),
        };
    }
    ParsedModel {
        provider_hint: None,
        model_id: model.to_string(),
    }
}

/// Resolve the `ProviderConfig` and canonical `model_id` string from a parsed
/// model field.
///
/// - Slug hint (`/`): match enabled provider by its public ID (from the map),
///   verify model exists.
/// - No hint: scan all enabled providers for an enabled model with that ID;
///   succeed only when exactly one provider has it — otherwise error with a
///   helpful message asking the caller to use the `provider/model` form.
pub(crate) fn resolve_provider_for_model(
    providers: &[ProviderConfig],
    public_id_map: &HashMap<String, String>,
    parsed: &ParsedModel,
) -> Result<(ProviderConfig, String), axum::response::Response> {
    let enabled: Vec<&ProviderConfig> = providers.iter().filter(|p| p.enabled).collect();

    match &parsed.provider_hint {
        Some(hint) => {
            let provider_opt = enabled
                .iter()
                .find(|p| public_id_map.get(&p.id) == Some(hint));

            let provider = provider_opt.ok_or_else(|| {
                error_response(StatusCode::NOT_FOUND, &format!("Provider '{}' not found", hint))
            })?;

            if !provider
                .models
                .iter()
                .any(|m| m.enabled && m.model_id == parsed.model_id)
            {
                return Err(error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Model '{}' not found on provider '{}'", parsed.model_id, hint),
                ));
            }

            Ok(((*provider).clone(), parsed.model_id.clone()))
        },
        None => {
            // Bare model_id: find matching enabled providers.
            let matching: Vec<&&ProviderConfig> = enabled
                .iter()
                .filter(|p| {
                    p.models
                        .iter()
                        .any(|m| m.enabled && m.model_id == parsed.model_id)
                })
                .collect();

            match matching.len() {
                0 => Err(error_response(
                    StatusCode::NOT_FOUND,
                    &format!("Model '{}' not found", parsed.model_id),
                )),
                1 => Ok(((*matching[0]).clone(), parsed.model_id.clone())),
                _ => {
                    let provider_names: Vec<&str> = matching
                        .iter()
                        .filter_map(|p| public_id_map.get(&p.id).map(|s| s.as_str()))
                        .collect();
                    Err(error_response(
                        StatusCode::CONFLICT,
                        &format!(
                            "Model '{}' is available on multiple providers: {}. Please specify a provider using the 'provider/model' format (e.g. '{}/{}')",
                            parsed.model_id,
                            provider_names.join(", "),
                            provider_names.first().unwrap_or(&"provider"),
                            parsed.model_id
                        ),
                    ))
                },
            }
        },
    }
}
