use sea_orm::*;

use crate::entity::gateway_link_activities;
use crate::entity::gateway_link_policies;
use crate::entity::gateway_links;
use crate::error::{AxAgentError, HealthCheckError, Result};
use crate::types::{
    CreateGatewayLinkInput, GatewayLink, GatewayLinkActivity, GatewayLinkModelSync,
    GatewayLinkPolicy, GatewayLinkSkillSync, SaveGatewayLinkPolicyInput,
};
use crate::utils::{gen_id, now_ts};

/// Build an authenticated reqwest client for a gateway link endpoint.
fn build_link_client(endpoint: &str, api_key: Option<&str>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    // Set a reasonable timeout for gateway link requests
    builder = builder
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(5));
    // We set the auth header per-request, so no default headers needed here
    let _ = (endpoint, api_key); // used per-request
    builder.build().unwrap_or_default()
}

/// Make an authenticated GET request to a gateway link endpoint.
async fn link_get(endpoint: &str, api_key: Option<&str>, path: &str) -> Result<String> {
    let client = build_link_client(endpoint, api_key);
    let url = format!("{}{}", endpoint.trim_end_matches('/'), path);
    let mut req = client.get(&url);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("Request to {} failed: {}", url, e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AxAgentError::Gateway(format!(
            "Gateway link error {} from {}: {}",
            status, url, text
        )));
    }
    resp.text()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("Read error from {}: {}", url, e)))
}

/// Make an authenticated POST request with JSON body to a gateway link endpoint.
async fn link_post_json(
    endpoint: &str,
    api_key: Option<&str>,
    path: &str,
    body: &serde_json::Value,
) -> Result<String> {
    let client = build_link_client(endpoint, api_key);
    let url = format!("{}{}", endpoint.trim_end_matches('/'), path);
    let mut req = client.post(&url).json(body);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("POST to {} failed: {}", url, e)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AxAgentError::Gateway(format!(
            "Gateway link error {} from {}: {}",
            status, url, text
        )));
    }
    resp.text()
        .await
        .map_err(|e| AxAgentError::Gateway(format!("Read error from {}: {}", url, e)))
}

fn link_from_entity(m: gateway_links::Model) -> GatewayLink {
    GatewayLink {
        id: m.id,
        name: m.name,
        link_type: m.link_type,
        endpoint: m.endpoint,
        api_key_id: m.api_key_id,
        enabled: m.enabled != 0,
        status: m.status,
        error_message: m.error_message,
        auto_sync_models: m.auto_sync_models != 0,
        auto_sync_skills: m.auto_sync_skills != 0,
        last_sync_at: m.last_sync_at,
        latency_ms: m.latency_ms,
        version: m.version,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn policy_from_entity(m: gateway_link_policies::Model) -> GatewayLinkPolicy {
    GatewayLinkPolicy {
        id: m.id,
        link_id: m.link_id,
        route_strategy: m.route_strategy,
        model_fallback_enabled: m.model_fallback_enabled != 0,
        global_rpm: m.global_rpm,
        per_model_rpm: m.per_model_rpm,
        token_limit_per_minute: m.token_limit_per_minute,
        key_rotation_strategy: m.key_rotation_strategy,
        key_failover_enabled: m.key_failover_enabled != 0,
    }
}

fn activity_from_entity(m: gateway_link_activities::Model) -> GatewayLinkActivity {
    GatewayLinkActivity {
        id: m.id,
        link_id: m.link_id,
        activity_type: m.activity_type,
        description: m.description,
        created_at: m.created_at,
    }
}

pub async fn list_gateway_links(db: &DatabaseConnection) -> Result<Vec<GatewayLink>> {
    let rows = gateway_links::Entity::find()
        .order_by_desc(gateway_links::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(link_from_entity).collect())
}

pub async fn get_gateway_link(db: &DatabaseConnection, id: &str) -> Result<Option<GatewayLink>> {
    let row = gateway_links::Entity::find_by_id(id).one(db).await?;
    Ok(row.map(link_from_entity))
}

pub async fn create_gateway_link(
    db: &DatabaseConnection,
    input: &CreateGatewayLinkInput,
) -> Result<GatewayLink> {
    let id = gen_id();
    let now = now_ts();

    gateway_links::ActiveModel {
        id: Set(id.clone()),
        name: Set(input.name.clone()),
        link_type: Set(input.link_type.clone()),
        endpoint: Set(input.endpoint.clone()),
        api_key_id: Set(input.api_key_id.clone()),
        enabled: Set(1),
        status: Set("disconnected".to_string()),
        error_message: Set(None),
        auto_sync_models: Set(if input.auto_sync_models.unwrap_or(false) {
            1
        } else {
            0
        }),
        auto_sync_skills: Set(if input.auto_sync_skills.unwrap_or(false) {
            1
        } else {
            0
        }),
        last_sync_at: Set(None),
        latency_ms: Set(None),
        version: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    let row = gateway_links::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", id)))?;

    Ok(link_from_entity(row))
}

pub async fn delete_gateway_link(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = gateway_links::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("GatewayLink {}", id)));
    }
    Ok(())
}

pub async fn toggle_gateway_link(db: &DatabaseConnection, id: &str, enabled: bool) -> Result<()> {
    let row = gateway_links::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", id)))?;

    let mut am: gateway_links::ActiveModel = row.into();
    am.enabled = Set(if enabled { 1 } else { 0 });
    am.updated_at = Set(now_ts());
    am.update(db).await?;
    Ok(())
}

pub async fn update_gateway_link_status(
    db: &DatabaseConnection,
    id: &str,
    status: &str,
    error_message: Option<&str>,
    latency_ms: Option<i64>,
    version: Option<&str>,
) -> Result<GatewayLink> {
    let row = gateway_links::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", id)))?;

    let mut am: gateway_links::ActiveModel = row.into();
    am.status = Set(status.to_string());
    am.error_message = Set(error_message.map(|s| s.to_string()));
    am.latency_ms = Set(latency_ms);
    am.version = Set(version.map(|s| s.to_string()));
    am.updated_at = Set(now_ts());
    am.update(db).await?;

    let updated = gateway_links::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", id)))?;
    Ok(link_from_entity(updated))
}

pub async fn update_gateway_link_sync_settings(
    db: &DatabaseConnection,
    id: &str,
    auto_sync_models: bool,
    auto_sync_skills: bool,
) -> Result<GatewayLink> {
    let row = gateway_links::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", id)))?;

    let mut am: gateway_links::ActiveModel = row.into();
    am.auto_sync_models = Set(if auto_sync_models { 1 } else { 0 });
    am.auto_sync_skills = Set(if auto_sync_skills { 1 } else { 0 });
    am.updated_at = Set(now_ts());
    am.update(db).await?;

    let updated = gateway_links::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", id)))?;
    Ok(link_from_entity(updated))
}

/// Connect to a remote gateway link: health-check the endpoint, update status to connected.
pub async fn connect_gateway_link(
    db: &DatabaseConnection,
    link_id: &str,
    api_key: Option<&str>,
) -> Result<GatewayLink> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;

    if link.enabled == 0 {
        return Err(AxAgentError::Gateway(format!(
            "Gateway link {} is disabled",
            link_id
        )));
    }

    // Attempt to reach the remote gateway's health endpoint
    let start = std::time::Instant::now();
    match link_get(&link.endpoint, api_key, "/health").await {
        Ok(body) => {
            let latency_ms = start.elapsed().as_millis() as i64;

            // Try to extract version from the health response
            let version = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("version").cloned())
                .and_then(|v| v.as_str().map(|s| s.to_string()));

            let now = now_ts();
            let mut am: gateway_links::ActiveModel = link.into();
            am.status = Set("connected".to_string());
            am.error_message = Set(None);
            am.latency_ms = Set(Some(latency_ms));
            am.version = Set(version);
            am.updated_at = Set(now);
            am.update(db).await?;

            add_activity(
                db,
                link_id,
                "connect",
                Some(&format!("Connected ({}ms)", latency_ms)),
            )
            .await?;

            let updated = gateway_links::Entity::find_by_id(link_id)
                .one(db)
                .await?
                .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;
            Ok(link_from_entity(updated))
        },
        Err(e) => {
            let now = now_ts();
            let mut am: gateway_links::ActiveModel = link.into();
            am.status = Set("error".to_string());
            am.error_message = Set(Some(e.to_string()));
            am.updated_at = Set(now);
            am.update(db).await?;

            add_activity(
                db,
                link_id,
                "connect_failed",
                Some(&format!("Connection failed: {}", e)),
            )
            .await?;

            let updated = gateway_links::Entity::find_by_id(link_id)
                .one(db)
                .await?
                .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;
            Ok(link_from_entity(updated))
        },
    }
}

/// Disconnect from a remote gateway link: update status to disconnected.
pub async fn disconnect_gateway_link(
    db: &DatabaseConnection,
    link_id: &str,
) -> Result<GatewayLink> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;

    let now = now_ts();
    let mut am: gateway_links::ActiveModel = link.into();
    am.status = Set("disconnected".to_string());
    am.error_message = Set(None);
    am.latency_ms = Set(None);
    am.updated_at = Set(now);
    am.update(db).await?;

    add_activity(db, link_id, "disconnect", Some("Disconnected")).await?;

    let updated = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;
    Ok(link_from_entity(updated))
}

pub async fn get_gateway_link_model_syncs(
    db: &DatabaseConnection,
    link_id: &str,
) -> Result<Vec<GatewayLinkModelSync>> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;

    if link.status != "connected" {
        return Ok(vec![]);
    }

    let providers = crate::repo::provider::list_providers(db).await?;
    let mut result = Vec::new();

    for provider in &providers {
        if !provider.enabled {
            continue;
        }
        let models = crate::repo::provider::list_models_for_provider(db, &provider.id).await?;
        for model in &models {
            if !model.enabled {
                continue;
            }
            result.push(GatewayLinkModelSync {
                model_id: model.model_id.clone(),
                provider_name: provider.name.clone(),
                sync_status: "not_selected".to_string(),
                last_sync_at: None,
            });
        }
    }

    Ok(result)
}

/// Push selected models to the remote gateway via POST /v1/models/sync.
pub async fn push_gateway_link_models(
    db: &DatabaseConnection,
    link_id: &str,
    model_ids: &[String],
    api_key: Option<&str>,
) -> Result<()> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;

    if link.status != "connected" {
        return Err(AxAgentError::Gateway(format!(
            "Gateway link {} is not connected",
            link_id
        )));
    }

    // Build the model payload from local providers
    let providers = crate::repo::provider::list_providers(db).await?;
    let mut models_to_push = Vec::new();
    for provider in &providers {
        if !provider.enabled {
            continue;
        }
        let models = crate::repo::provider::list_models_for_provider(db, &provider.id).await?;
        for model in &models {
            if !model.enabled {
                continue;
            }
            if model_ids.contains(&model.model_id) {
                models_to_push.push(serde_json::json!({
                    "model_id": model.model_id,
                    "name": model.name,
                    "provider_name": provider.name,
                }));
            }
        }
    }

    let body = serde_json::json!({ "models": models_to_push });
    link_post_json(&link.endpoint, api_key, "/v1/models/sync", &body).await?;

    let now = now_ts();
    let mut am: gateway_links::ActiveModel = link.into();
    am.last_sync_at = Set(Some(now));
    am.updated_at = Set(now);
    am.update(db).await?;

    add_activity(
        db,
        link_id,
        "push_models",
        Some(&format!(
            "Pushed {} models to gateway",
            models_to_push.len()
        )),
    )
    .await?;
    Ok(())
}

/// Sync all enabled models to the remote gateway via POST /v1/models/sync.
pub async fn sync_all_gateway_link_models(
    db: &DatabaseConnection,
    link_id: &str,
    api_key: Option<&str>,
) -> Result<()> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;

    if link.status != "connected" {
        return Err(AxAgentError::Gateway(format!(
            "Gateway link {} is not connected",
            link_id
        )));
    }

    // Build the full model payload
    let providers = crate::repo::provider::list_providers(db).await?;
    let mut all_models = Vec::new();
    for provider in &providers {
        if !provider.enabled {
            continue;
        }
        let models = crate::repo::provider::list_models_for_provider(db, &provider.id).await?;
        for model in &models {
            if !model.enabled {
                continue;
            }
            all_models.push(serde_json::json!({
                "model_id": model.model_id,
                "name": model.name,
                "provider_name": provider.name,
            }));
        }
    }

    let body = serde_json::json!({ "models": all_models });
    link_post_json(&link.endpoint, api_key, "/v1/models/sync", &body).await?;

    let now = now_ts();
    let mut am: gateway_links::ActiveModel = link.into();
    am.last_sync_at = Set(Some(now));
    am.updated_at = Set(now);
    am.update(db).await?;

    add_activity(
        db,
        link_id,
        "sync_models",
        Some(&format!(
            "Synced all {} models to gateway",
            all_models.len()
        )),
    )
    .await?;
    Ok(())
}

/// Fetch skill syncs from the remote gateway via GET /v1/skills.
pub async fn get_gateway_link_skill_syncs(
    db: &DatabaseConnection,
    link_id: &str,
    api_key: Option<&str>,
) -> Result<Vec<GatewayLinkSkillSync>> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;

    if link.status != "connected" {
        return Ok(vec![]);
    }

    // Fetch skills from the remote gateway
    match link_get(&link.endpoint, api_key, "/v1/skills").await {
        Ok(body) => {
            // Try to parse as an array of skill objects
            if let Ok(skills_json) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(arr) = skills_json.as_array() {
                    return Ok(arr
                        .iter()
                        .map(|s| GatewayLinkSkillSync {
                            skill_name: s
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            skill_version: s
                                .get("version")
                                .and_then(|v| v.as_str())
                                .map(|v| v.to_string()),
                            sync_status: "synced".to_string(),
                            last_sync_at: Some(now_ts()),
                        })
                        .collect());
                }
            }
            Ok(vec![])
        },
        Err(_) => Ok(vec![]),
    }
}

/// Push selected skills to the remote gateway via POST /v1/skills/sync.
pub async fn push_gateway_link_skills(
    db: &DatabaseConnection,
    link_id: &str,
    skill_names: &[String],
    api_key: Option<&str>,
) -> Result<()> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;

    if link.status != "connected" {
        return Err(AxAgentError::Gateway(format!(
            "Gateway link {} is not connected",
            link_id
        )));
    }

    let body = serde_json::json!({ "skills": skill_names });
    link_post_json(&link.endpoint, api_key, "/v1/skills/sync", &body).await?;

    let now = now_ts();
    let mut am: gateway_links::ActiveModel = link.into();
    am.last_sync_at = Set(Some(now));
    am.updated_at = Set(now);
    am.update(db).await?;

    add_activity(
        db,
        link_id,
        "push_skills",
        Some(&format!("Pushed {} skills to gateway", skill_names.len())),
    )
    .await?;
    Ok(())
}

/// Sync all skills to the remote gateway via POST /v1/skills/sync.
pub async fn sync_all_gateway_link_skills(
    db: &DatabaseConnection,
    link_id: &str,
    api_key: Option<&str>,
) -> Result<()> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLink {}", link_id)))?;

    if link.status != "connected" {
        return Err(AxAgentError::Gateway(format!(
            "Gateway link {} is not connected",
            link_id
        )));
    }

    let body = serde_json::json!({ "sync_all": true });
    link_post_json(&link.endpoint, api_key, "/v1/skills/sync", &body).await?;

    let now = now_ts();
    let mut am: gateway_links::ActiveModel = link.into();
    am.last_sync_at = Set(Some(now));
    am.updated_at = Set(now);
    am.update(db).await?;

    add_activity(
        db,
        link_id,
        "sync_skills",
        Some("All skills synced to gateway"),
    )
    .await?;
    Ok(())
}

pub async fn get_gateway_link_policy(
    db: &DatabaseConnection,
    link_id: &str,
) -> Result<Option<GatewayLinkPolicy>> {
    let row = gateway_link_policies::Entity::find()
        .filter(gateway_link_policies::Column::LinkId.eq(link_id))
        .one(db)
        .await?;
    Ok(row.map(policy_from_entity))
}

pub async fn save_gateway_link_policy(
    db: &DatabaseConnection,
    link_id: &str,
    input: &SaveGatewayLinkPolicyInput,
) -> Result<GatewayLinkPolicy> {
    let existing = gateway_link_policies::Entity::find()
        .filter(gateway_link_policies::Column::LinkId.eq(link_id))
        .one(db)
        .await?;

    match existing {
        Some(row) => {
            let mut am: gateway_link_policies::ActiveModel = row.into();
            if let Some(ref v) = input.route_strategy {
                am.route_strategy = Set(v.clone());
            }
            if let Some(v) = input.model_fallback_enabled {
                am.model_fallback_enabled = Set(if v { 1 } else { 0 });
            }
            if let Some(ref v) = input.global_rpm {
                am.global_rpm = Set(*v);
            }
            if let Some(ref v) = input.per_model_rpm {
                am.per_model_rpm = Set(*v);
            }
            if let Some(ref v) = input.token_limit_per_minute {
                am.token_limit_per_minute = Set(*v);
            }
            if let Some(ref v) = input.key_rotation_strategy {
                am.key_rotation_strategy = Set(v.clone());
            }
            if let Some(v) = input.key_failover_enabled {
                am.key_failover_enabled = Set(if v { 1 } else { 0 });
            }
            let updated = am.update(db).await?;
            Ok(policy_from_entity(updated))
        },
        None => {
            let id = gen_id();
            gateway_link_policies::ActiveModel {
                id: Set(id.clone()),
                link_id: Set(link_id.to_string()),
                route_strategy: Set(input
                    .route_strategy
                    .clone()
                    .unwrap_or_else(|| "round_robin".to_string())),
                model_fallback_enabled: Set(if input.model_fallback_enabled.unwrap_or(false) {
                    1
                } else {
                    0
                }),
                global_rpm: Set(input.global_rpm.unwrap_or(None)),
                per_model_rpm: Set(input.per_model_rpm.unwrap_or(None)),
                token_limit_per_minute: Set(input.token_limit_per_minute.unwrap_or(None)),
                key_rotation_strategy: Set(input
                    .key_rotation_strategy
                    .clone()
                    .unwrap_or_else(|| "sequential".to_string())),
                key_failover_enabled: Set(if input.key_failover_enabled.unwrap_or(false) {
                    1
                } else {
                    0
                }),
            }
            .insert(db)
            .await?;

            let row = gateway_link_policies::Entity::find_by_id(&id)
                .one(db)
                .await?
                .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLinkPolicy {}", id)))?;
            Ok(policy_from_entity(row))
        },
    }
}

pub async fn get_gateway_link_activities(
    db: &DatabaseConnection,
    link_id: &str,
) -> Result<Vec<GatewayLinkActivity>> {
    let rows = gateway_link_activities::Entity::find()
        .filter(gateway_link_activities::Column::LinkId.eq(link_id))
        .order_by_desc(gateway_link_activities::Column::CreatedAt)
        .limit(50)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(activity_from_entity).collect())
}

async fn add_activity(
    db: &DatabaseConnection,
    link_id: &str,
    activity_type: &str,
    description: Option<&str>,
) -> Result<GatewayLinkActivity> {
    let id = gen_id();
    let now = now_ts();

    gateway_link_activities::ActiveModel {
        id: Set(id.clone()),
        link_id: Set(link_id.to_string()),
        activity_type: Set(activity_type.to_string()),
        description: Set(description.map(|s| s.to_string())),
        created_at: Set(now),
    }
    .insert(db)
    .await?;

    let row = gateway_link_activities::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayLinkActivity {}", id)))?;
    Ok(activity_from_entity(row))
}

pub struct GatewayLinkTimeouts {
    pub health_check: std::time::Duration,
    pub connect: std::time::Duration,
    pub sync: std::time::Duration,
    pub default: std::time::Duration,
}

impl Default for GatewayLinkTimeouts {
    fn default() -> Self {
        Self {
            health_check: std::time::Duration::from_secs(3),
            connect: std::time::Duration::from_secs(5),
            sync: std::time::Duration::from_secs(30),
            default: std::time::Duration::from_secs(10),
        }
    }
}

fn build_health_check_client(
    timeouts: &GatewayLinkTimeouts,
    endpoint: &str,
    api_key: Option<&str>,
) -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    builder = builder
        .timeout(timeouts.health_check)
        .connect_timeout(timeouts.health_check);
    let _ = (endpoint, api_key);
    builder.build().unwrap_or_default()
}

pub async fn check_gateway_health(
    db: &DatabaseConnection,
    link_id: &str,
    api_key: Option<&str>,
) -> std::result::Result<u64, HealthCheckError> {
    let link = gateway_links::Entity::find_by_id(link_id)
        .one(db)
        .await
        .map_err(|e| HealthCheckError::Permanent(format!("Database error: {}", e)))?;

    let link = match link {
        Some(l) => l,
        None => {
            return Err(HealthCheckError::Permanent(format!(
                "GatewayLink {} not found",
                link_id
            )))
        },
    };
    let timeouts = GatewayLinkTimeouts::default();
    let client = build_health_check_client(&timeouts, &link.endpoint, api_key);
    let url = format!("{}/health", link.endpoint.trim_end_matches('/'));

    let start = std::time::Instant::now();
    let mut req = client.get(&url);
    if let Some(key) = api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }

    let resp = req.send().await.map_err(|e| {
        if e.is_timeout() || e.is_connect() {
            HealthCheckError::Network(format!("Connection failed: {}", e))
        } else {
            HealthCheckError::Network(format!("Health check to {} failed: {}", url, e))
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().await.unwrap_or_default();
        return Err(HealthCheckError::from_status(status, &text));
    }

    let latency_ms = start.elapsed().as_millis() as u64;
    Ok(latency_ms)
}

pub async fn connect_gateway_link_with_retry(
    db: &DatabaseConnection,
    link_id: &str,
    api_key: Option<&str>,
    max_retries: u32,
) -> Result<GatewayLink> {
    let mut backoff = ExponentialBackoff::new(1000, 60000);
    let mut last_error = None;

    for attempt in 0..max_retries {
        match connect_gateway_link(db, link_id, api_key).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries - 1 {
                    let delay = backoff.next_delay();
                    tracing::warn!(
                        "Gateway link {} connection attempt {}/{} failed, retrying in {:?}: {}",
                        link_id,
                        attempt + 1,
                        max_retries,
                        delay,
                        last_error.as_ref().expect("last_error was just set")
                    );
                    tokio::time::sleep(delay).await;
                }
            },
        }
    }

    Err(last_error.expect("last_error must be set if loop completed without returning"))
}

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    base_delay_ms: u64,
    max_delay_ms: u64,
    current_attempt: u32,
}

impl ExponentialBackoff {
    pub fn new(base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            base_delay_ms,
            max_delay_ms,
            current_attempt: 0,
        }
    }

    pub fn next_delay(&mut self) -> std::time::Duration {
        let delay = std::cmp::min(
            self.base_delay_ms * 2u64.pow(self.current_attempt.min(10)),
            self.max_delay_ms,
        );
        self.current_attempt += 1;
        std::time::Duration::from_millis(delay)
    }

    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }
}
