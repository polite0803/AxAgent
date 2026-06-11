//! HTTP handlers for the Axum gateway, split by endpoint family.
//!
//! ## Layout
//!
//! - [`error`]      — common error response helper, `record_log!` macro,
//!   `provider_type_to_str` mapper, and the shared Hermes provider resolver.
//! - [`health`]     — `GET /health` and `GET /health/detailed`.
//! - [`responses`]  — `GET` / `DELETE` for `/v1/responses/{id}`.
//! - [`jobs`]       — twelve `/api/jobs*` endpoints (list/create/get/update/delete/
//!   pause/resume/trigger + run lifecycle + schedule + enable/disable).
//! - [`models`]     — `GET /v1/models` plus the model-name slug helpers used by
//!   `chat_completions` (model-field parsing, public-id map, etc.).
//! - [`chat`]       — `POST /v1/chat/completions` and its streaming / non-streaming
//!   body builders.
//! - [`streaming`]  — re-export of the streaming helpers from `chat` for callers
//!   that prefer the family-specific module path.
//!
//! External callers should keep importing via `crate::handlers::xxx` — the
//! `pub use` re-exports below preserve the original module surface.

pub mod chat;
pub mod error;
pub mod health;
pub mod jobs;
pub mod models;
pub mod responses;
pub mod streaming;

// Re-exports to preserve the original `crate::handlers::xxx` paths.
#[allow(unused_imports)]
pub(crate) use chat::{chat_completions, handle_non_stream, handle_stream};
pub use health::{detailed_health_check, health_check};
pub use jobs::{
    cancel_run, create_job, delete_job, disable_job, enable_job, get_job, get_job_schedule,
    get_run, get_run_logs, list_jobs, list_runs, pause_job, resume_job, retry_run, trigger_job,
    trigger_run, update_job, update_job_schedule,
};
pub use models::list_models;
pub use responses::{delete_response, get_response};
// These items are `pub(crate)` in their submodules; re-export them at the
// same crate-private level so other modules in the gateway crate can keep
// referring to them as `crate::handlers::xxx` (or simply via the
// `crate::handlers::error::*` / `crate::handlers::models::*` paths).
#[allow(unused_imports)]
pub(crate) use error::{error_response, provider_type_to_str, resolve_hermes_provider_context};
#[allow(unused_imports)]
pub(crate) use models::{
    build_model_display_map, build_provider_public_id_map, parse_model_field, provider_slug,
    resolve_provider_for_model, ParsedModel,
};
