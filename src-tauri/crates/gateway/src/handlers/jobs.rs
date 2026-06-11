use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde_json::json;
use std::time::Instant;

use crate::auth::AuthenticatedKey;
use crate::handlers::error::{error_response, provider_type_to_str, record_log, resolve_hermes_provider_context};
use crate::server::GatewayAppState;

/// GET /api/jobs — list all jobs from Hermes/OpenClaw gateway
pub async fn list_jobs(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.list_jobs(&ctx).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                "/api/jobs",
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                "/api/jobs",
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to list jobs: {}", e))
        },
    }
}

/// POST /api/jobs — create a new job
pub async fn create_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Json(job_data): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    let job_data_str = serde_json::to_string(&job_data)
        .unwrap_or_else(|e| format!("{{\"error\":\"Serialization failed: {}\"}}", e));

    match adapter.create_job(&ctx, &job_data_str).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                "/api/jobs",
                None,
                &provider.id,
                201,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::CREATED)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                "/api/jobs",
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to create job: {}", e))
        },
    }
}

/// GET /api/jobs/{job_id} — get a specific job
pub async fn get_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.get_job(&ctx, &job_id).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to get job: {}", e))
        },
    }
}

/// PATCH /api/jobs/{job_id} — update a job
pub async fn update_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    axum::extract::Json(job_data): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    let job_data_str = serde_json::to_string(&job_data)
        .unwrap_or_else(|e| format!("{{\"error\":\"Serialization failed: {}\"}}", e));

    match adapter.update_job(&ctx, &job_id, &job_data_str).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "PATCH",
                &format!("/api/jobs/{}", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "PATCH",
                &format!("/api/jobs/{}", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to update job: {}", e))
        },
    }
}

/// DELETE /api/jobs/{job_id} — delete a job
pub async fn delete_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.delete_job(&ctx, &job_id).await {
        Ok(_) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "DELETE",
                &format!("/api/jobs/{}", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            Json(json!({ "deleted": true, "id": job_id })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "DELETE",
                &format!("/api/jobs/{}", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to delete job: {}", e))
        },
    }
}

/// POST /api/jobs/{job_id}/pause — pause a job
pub async fn pause_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.pause_job(&ctx, &job_id).await {
        Ok(_) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/pause", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            Json(json!({ "paused": true, "id": job_id })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/pause", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to pause job: {}", e))
        },
    }
}

/// POST /api/jobs/{job_id}/resume — resume a job
pub async fn resume_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.resume_job(&ctx, &job_id).await {
        Ok(_) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/resume", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            Json(json!({ "resumed": true, "id": job_id })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/resume", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to resume job: {}", e))
        },
    }
}

/// POST /api/jobs/{job_id}/run — trigger/run a job immediately
pub async fn trigger_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.trigger_job(&ctx, &job_id).await {
        Ok(_) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/run", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            Json(json!({ "triggered": true, "id": job_id })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/run", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );

            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to trigger job: {}", e))
        },
    }
}

/// GET /api/jobs/{job_id}/runs — list runs for a job
pub async fn list_runs(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.list_runs(&ctx, &job_id).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}/runs", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}/runs", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to list runs: {}", e))
        },
    }
}

/// POST /api/jobs/{job_id}/runs — trigger a new run
pub async fn trigger_run(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    axum::extract::Json(params): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    let params_str = serde_json::to_string(&params)
        .unwrap_or_else(|e| format!("{{\"error\":\"Serialization failed: {}\"}}", e));

    match adapter.trigger_run(&ctx, &job_id, Some(&params_str)).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/runs", job_id),
                None,
                &provider.id,
                201,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::CREATED)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/runs", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to trigger run: {}", e))
        },
    }
}

/// GET /api/jobs/{job_id}/runs/{run_id} — get a specific run
pub async fn get_run(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path((job_id, run_id)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.get_run(&ctx, &job_id, &run_id).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}/runs/{}", job_id, run_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}/runs/{}", job_id, run_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to get run: {}", e))
        },
    }
}

/// POST /api/jobs/{job_id}/runs/{run_id}/cancel — cancel a run
pub async fn cancel_run(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path((job_id, run_id)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.cancel_run(&ctx, &job_id, &run_id).await {
        Ok(_) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/runs/{}/cancel", job_id, run_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );
            Json(json!({ "cancelled": true, "job_id": job_id, "run_id": run_id })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/runs/{}/cancel", job_id, run_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to cancel run: {}", e))
        },
    }
}

/// GET /api/jobs/{job_id}/runs/{run_id}/logs — get run logs
pub async fn get_run_logs(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path((job_id, run_id)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.get_run_logs(&ctx, &job_id, &run_id).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}/runs/{}/logs", job_id, run_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}/runs/{}/logs", job_id, run_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to get run logs: {}", e))
        },
    }
}

/// POST /api/jobs/{job_id}/runs/{run_id}/retry — retry a run
pub async fn retry_run(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path((job_id, run_id)): axum::extract::Path<(String, String)>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.retry_run(&ctx, &job_id, &run_id).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/runs/{}/retry", job_id, run_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/runs/{}/retry", job_id, run_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to retry run: {}", e))
        },
    }
}

/// GET /api/jobs/{job_id}/schedule — get job schedule
pub async fn get_job_schedule(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.get_job_schedule(&ctx, &job_id).await {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}/schedule", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "GET",
                &format!("/api/jobs/{}/schedule", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to get job schedule: {}", e))
        },
    }
}

/// PUT /api/jobs/{job_id}/schedule — update job schedule
pub async fn update_job_schedule(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    axum::extract::Json(schedule): axum::extract::Json<serde_json::Value>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    let schedule_str = serde_json::to_string(&schedule)
        .unwrap_or_else(|e| format!("{{\"error\":\"Serialization failed: {}\"}}", e));

    match adapter
        .update_job_schedule(&ctx, &job_id, &schedule_str)
        .await
    {
        Ok(response_body) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "PUT",
                &format!("/api/jobs/{}/schedule", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );

            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(response_body.into())
                .unwrap_or_else(|_| {
                    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response")
                })
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "PUT",
                &format!("/api/jobs/{}/schedule", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to update job schedule: {}", e),
            )
        },
    }
}

/// POST /api/jobs/{job_id}/enable — enable a job
pub async fn enable_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.enable_job(&ctx, &job_id).await {
        Ok(_) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/enable", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );
            Json(json!({ "enabled": true, "id": job_id })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/enable", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to enable job: {}", e))
        },
    }
}

/// POST /api/jobs/{job_id}/disable — disable a job
pub async fn disable_job(
    State(state): State<GatewayAppState>,
    Extension(auth): Extension<AuthenticatedKey>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let AuthenticatedKey(gateway_key) = auth;
    let start_time = Instant::now();

    let (provider, ctx) =
        match resolve_hermes_provider_context(&state.adapter, &*state.provider_registry).await {
            Ok(r) => r,
            Err(resp) => return resp,
        };
    let adapter = match state
        .provider_registry
        .get(provider_type_to_str(&provider.provider_type))
    {
        Some(a) => a,
        None => {
            return error_response(StatusCode::BAD_GATEWAY, "No adapter available");
        },
    };

    match adapter.disable_job(&ctx, &job_id).await {
        Ok(_) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/disable", job_id),
                None,
                &provider.id,
                200,
                elapsed,
                0,
                0,
                None
            );
            Json(json!({ "disabled": true, "id": job_id })).into_response()
        },
        Err(e) => {
            let elapsed = start_time.elapsed().as_millis() as i32;
            record_log!(
                &state.adapter,
                gateway_key,
                "POST",
                &format!("/api/jobs/{}/disable", job_id),
                None,
                &provider.id,
                500,
                elapsed,
                0,
                0,
                None
            );
            error_response(StatusCode::BAD_GATEWAY, &format!("Failed to disable job: {}", e))
        },
    }
}
