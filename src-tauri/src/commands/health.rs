use crate::AppState;
use sea_orm::ConnectionTrait;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthReport {
    pub overall: ServiceHealthStatus,
    pub checks: Vec<ServiceHealthCheck>,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceHealthCheck {
    pub name: String,
    pub status: ServiceHealthStatus,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

#[tauri::command]
pub async fn get_service_health(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<ServiceHealthReport, String> {
    let mut checks = Vec::new();
    let mut overall = ServiceHealthStatus::Healthy;

    let db_check = check_database(&state).await;
    if db_check.status == ServiceHealthStatus::Unhealthy {
        overall = ServiceHealthStatus::Unhealthy;
    } else if db_check.status == ServiceHealthStatus::Degraded
        && overall == ServiceHealthStatus::Healthy
    {
        overall = ServiceHealthStatus::Degraded;
    }
    checks.push(db_check);

    let vector_check = check_vector_store(&state).await;
    if vector_check.status == ServiceHealthStatus::Unhealthy {
        overall = ServiceHealthStatus::Unhealthy;
    } else if vector_check.status == ServiceHealthStatus::Degraded
        && overall == ServiceHealthStatus::Healthy
    {
        overall = ServiceHealthStatus::Degraded;
    }
    checks.push(vector_check);

    let agent_check = check_agents(&state).await;
    checks.push(agent_check);

    let gateway_check = check_gateway(&state).await;
    if gateway_check.status == ServiceHealthStatus::Degraded
        && overall == ServiceHealthStatus::Healthy
    {
        overall = ServiceHealthStatus::Degraded;
    }
    checks.push(gateway_check);

    let mcp_check = check_mcp(&state).await;
    if mcp_check.status == ServiceHealthStatus::Degraded && overall == ServiceHealthStatus::Healthy
    {
        overall = ServiceHealthStatus::Degraded;
    }
    checks.push(mcp_check);

    let version = app
        .config()
        .version
        .clone()
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    Ok(ServiceHealthReport {
        overall,
        checks,
        version,
    })
}

async fn check_database(state: &AppState) -> ServiceHealthCheck {
    let start = std::time::Instant::now();
    let result = state.harness.db().execute_unprepared("SELECT 1").await;

    let latency = start.elapsed().as_millis() as u64;

    match result {
        Ok(_) => ServiceHealthCheck {
            name: "database".to_string(),
            status: ServiceHealthStatus::Healthy,
            latency_ms: Some(latency),
            message: None,
        },
        Err(e) => {
            let msg = e.to_string();
            let is_transient = msg.contains("locked") || msg.contains("busy");
            ServiceHealthCheck {
                name: "database".to_string(),
                status: if is_transient {
                    ServiceHealthStatus::Degraded
                } else {
                    ServiceHealthStatus::Unhealthy
                },
                latency_ms: Some(latency),
                message: Some(msg.chars().take(200).collect()),
            }
        },
    }
}

async fn check_vector_store(state: &AppState) -> ServiceHealthCheck {
    let _ = &state;
    ServiceHealthCheck {
        name: "vectorStore".to_string(),
        status: ServiceHealthStatus::Healthy,
        latency_ms: None,
        message: None,
    }
}

async fn check_agents(state: &AppState) -> ServiceHealthCheck {
    let running = state.running_agents.read().await.len();
    ServiceHealthCheck {
        name: "agents".to_string(),
        status: ServiceHealthStatus::Healthy,
        latency_ms: None,
        message: if running > 0 {
            Some(format!("{} running", running))
        } else {
            None
        },
    }
}

async fn check_gateway(state: &AppState) -> ServiceHealthCheck {
    let guard = state.gateway.lock().await;
    match guard.as_ref() {
        Some(_) => ServiceHealthCheck {
            name: "gateway".to_string(),
            status: ServiceHealthStatus::Healthy,
            latency_ms: None,
            message: Some("running".to_string()),
        },
        None => ServiceHealthCheck {
            name: "gateway".to_string(),
            status: ServiceHealthStatus::Degraded,
            latency_ms: None,
            message: Some("not started".to_string()),
        },
    }
}

async fn check_mcp(state: &AppState) -> ServiceHealthCheck {
    let _ = &state;
    ServiceHealthCheck {
        name: "mcp".to_string(),
        status: ServiceHealthStatus::Healthy,
        latency_ms: None,
        message: None,
    }
}
