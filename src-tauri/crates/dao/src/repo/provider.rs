use sea_orm::sea_query::Expr;
use sea_orm::*;

use axagent_entities::{models, provider_keys, providers};
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::*;
use axagent_harness::util_fns::{gen_id, now_ts};

fn parse_provider_type(s: &str) -> ProviderType {
    match s {
        "openai" => ProviderType::OpenAI,
        "openai_responses" => ProviderType::OpenAIResponses,
        "anthropic" => ProviderType::Anthropic,
        "gemini" => ProviderType::Gemini,
        "openclaw" => ProviderType::OpenClaw,
        "hermes" => ProviderType::Hermes,
        "ollama" => ProviderType::Ollama,
        _ => ProviderType::OpenClaw, // fallback to OpenClaw for unknown types
    }
}

fn provider_type_str(pt: &ProviderType) -> &'static str {
    match pt {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    }
}

fn key_from_entity(m: provider_keys::Model) -> ProviderKey {
    ProviderKey {
        id: m.id,
        provider_id: m.provider_id,
        key_encrypted: m.key_encrypted,
        key_prefix: m.key_prefix,
        enabled: m.enabled != 0,
        last_validated_at: m.last_validated_at,
        last_error: m.last_error,
        rotation_index: m.rotation_index as u32,
        created_at: m.created_at,
    }
}

fn model_from_entity(m: models::Model) -> Model {
    Model {
        provider_id: m.provider_id,
        model_id: m.model_id,
        name: m.name,
        group_name: m.group_name,
        model_type: m.model_type.parse().unwrap_or_default(),
        capabilities: serde_json::from_str(&m.capabilities).unwrap_or_default(),
        max_tokens: m.max_tokens.map(|v| v as u32),
        max_output_tokens: None,
        enabled: m.enabled != 0,
        param_overrides: m
            .param_overrides
            .and_then(|s| serde_json::from_str(&s).ok()),
        input_price_per_mtok: m.input_price_per_mtok,
        output_price_per_mtok: m.output_price_per_mtok,
    }
}

fn provider_from_entity(
    row: providers::Model,
    keys: Vec<ProviderKey>,
    models: Vec<Model>,
) -> ProviderConfig {
    ProviderConfig {
        id: row.id,
        name: row.name,
        provider_type: parse_provider_type(&row.provider_type),
        api_host: row.api_host,
        api_path: row.api_path,
        enabled: row.enabled != 0,
        models,
        keys,
        proxy_config: row.proxy_config.and_then(|s| serde_json::from_str(&s).ok()),
        custom_headers: row.custom_headers,
        icon: row.icon,
        builtin_id: row.builtin_id,
        sort_order: row.sort_order,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

// --- Provider CRUD ---

pub async fn list_providers(db: &DatabaseConnection) -> Result<Vec<ProviderConfig>> {
    let rows = providers::Entity::find()
        .order_by_asc(providers::Column::SortOrder)
        .order_by_desc(providers::Column::CreatedAt)
        .all(db)
        .await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let id = row.id.clone();
        let keys = list_keys_for_provider(db, &id).await?;
        let models = list_models_for_provider(db, &id).await?;
        result.push(provider_from_entity(row, keys, models));
    }
    Ok(result)
}

/// 解析系统默认 LLM 调用配置：第一个启用的 provider + 第一个启用的 key + 第一个启用的 model。
/// 工作流 Llm/Agent 执行器调用此函数自动获取模型，无需每个节点手动配置。
pub async fn resolve_default_provider(
    db: &DatabaseConnection,
) -> std::result::Result<(ProviderConfig, ProviderKey, String), String> {
    let providers = list_providers(db).await.map_err(|e| e.to_string())?;
    let prov = providers
        .into_iter()
        .find(|p| p.enabled)
        .ok_or_else(|| "无可用 LLM provider".to_string())?;
    let key = prov
        .keys
        .iter()
        .find(|k| k.enabled)
        .cloned()
        .ok_or_else(|| "provider 无可用 API key".to_string())?;
    let model = prov
        .models
        .iter()
        .find(|m| m.enabled)
        .map(|m| m.model_id.clone())
        .ok_or_else(|| "provider 无可用模型".to_string())?;
    Ok((prov, key, model))
}

/// 基于项目设置解析默认 LLM 配置。
/// 优先使用 AppSettings 中配置的 default_provider_id + default_model_id，
/// 若未配置则回退到 resolve_default_provider（首个启用的 provider）。
pub async fn resolve_project_default(
    db: &DatabaseConnection,
) -> std::result::Result<(ProviderConfig, ProviderKey, String), String> {
    let settings = crate::repo::settings::get_settings(db)
        .await
        .map_err(|e| format!("读取项目设置失败: {e}"))?;

    if let (Some(ref pid), Some(ref mid)) =
        (settings.default_provider_id, settings.default_model_id)
    {
        let providers = list_providers(db).await.map_err(|e| e.to_string())?;
        if let Some(prov) = providers.into_iter().find(|p| p.id == *pid && p.enabled) {
            let key = prov
                .keys
                .iter()
                .find(|k| k.enabled)
                .cloned()
                .ok_or_else(|| "项目默认 provider 无可用 API key".to_string())?;
            let model_exists = prov.models.iter().any(|m| m.model_id == *mid && m.enabled);
            if model_exists {
                return Ok((prov, key, mid.clone()));
            }
        }
    }

    resolve_default_provider(db).await
}

/// 统一的工作流节点模型解析函数。
///
/// 优先级：
/// 1. node_model — 节点配置中显式指定的模型（最高优先级）
/// 2. session_model / session_provider_id — 会话/工作流级覆盖
/// 3. profile_suggested_provider — Agent Profile 建议的 provider（仅 Agent）
/// 4. 项目默认模型（AppSettings.default_model_id + default_provider_id）
///
/// 返回 (ProviderConfig, ProviderKey, model_id)
pub async fn resolve_model_for_node(
    db: &DatabaseConnection,
    node_model: Option<&str>,
    session_model: Option<&str>,
    session_provider_id: Option<&str>,
    profile_suggested_provider: Option<&str>,
) -> std::result::Result<(ProviderConfig, ProviderKey, String), String> {
    let effective_provider_id = session_provider_id.or(profile_suggested_provider);

    if let Some(pid) = effective_provider_id {
        let providers = list_providers(db).await.map_err(|e| e.to_string())?;
        if let Some(prov) = providers.into_iter().find(|p| p.id == pid && p.enabled) {
            let key = prov
                .keys
                .iter()
                .find(|k| k.enabled)
                .cloned()
                .ok_or_else(|| format!("provider '{}' 无可用 API key", pid))?;
            let default_model = prov
                .models
                .iter()
                .find(|m| m.enabled)
                .map(|m| m.model_id.clone())
                .unwrap_or_default();
            let model = node_model
                .or(session_model)
                .unwrap_or(&default_model)
                .to_string();
            return Ok((prov, key, model));
        }
    }

    let (prov, key, default_model) = resolve_project_default(db).await?;
    let model = node_model
        .or(session_model)
        .unwrap_or(&default_model)
        .to_string();
    Ok((prov, key, model))
}

pub async fn get_provider(db: &DatabaseConnection, id: &str) -> Result<ProviderConfig> {
    let row = providers::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Provider {}", id)))?;

    let keys = list_keys_for_provider(db, &row.id).await?;
    let models = list_models_for_provider(db, &row.id).await?;
    Ok(provider_from_entity(row, keys, models))
}

pub async fn create_provider(
    db: &DatabaseConnection,
    input: CreateProviderInput,
) -> Result<ProviderConfig> {
    let id = gen_id();
    let now = now_ts();

    providers::ActiveModel {
        id: Set(id.clone()),
        name: Set(input.name),
        provider_type: Set(provider_type_str(&input.provider_type).to_string()),
        api_host: Set(input.api_host),
        api_path: Set(input.api_path),
        enabled: Set(if input.enabled { 1 } else { 0 }),
        proxy_config: Set(None),
        custom_headers: Set(None),
        icon: Set(None),
        builtin_id: Set(input.builtin_id),
        sort_order: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    get_provider(db, &id).await
}

pub async fn update_provider(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateProviderInput,
) -> Result<ProviderConfig> {
    let existing = get_provider(db, id).await?;
    let now = now_ts();

    let name = input.name.unwrap_or(existing.name);
    let api_host = input.api_host.unwrap_or(existing.api_host);
    let enabled = input.enabled.unwrap_or(existing.enabled);
    let provider_type = input.provider_type.unwrap_or(existing.provider_type);
    let proxy_json = match input.proxy_config {
        Some(ref pc) => Some(serde_json::to_string(pc).unwrap_or_default()),
        None => existing
            .proxy_config
            .map(|pc| serde_json::to_string(&pc).unwrap_or_default()),
    };

    let row = providers::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Provider {}", id)))?;

    let mut am: providers::ActiveModel = row.into();
    am.name = Set(name);
    am.api_host = Set(api_host);
    am.provider_type = Set(provider_type_str(&provider_type).to_string());
    am.enabled = Set(if enabled { 1 } else { 0 });
    am.proxy_config = Set(proxy_json);
    if let Some(api_path) = input.api_path {
        am.api_path = Set(api_path);
    }
    if let Some(sort_order) = input.sort_order {
        am.sort_order = Set(sort_order);
    }
    if let Some(custom_headers) = input.custom_headers {
        am.custom_headers = Set(custom_headers);
    }
    if let Some(icon) = input.icon {
        am.icon = Set(icon);
    }
    am.updated_at = Set(now);
    am.update(db).await?;

    get_provider(db, id).await
}

pub async fn delete_provider(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = providers::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("Provider {}", id)));
    }
    Ok(())
}

pub async fn toggle_provider(db: &DatabaseConnection, id: &str, enabled: bool) -> Result<()> {
    let row = providers::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Provider {}", id)))?;

    // 启用前检查：必须有至少一个已验证通过且无错误的 API Key
    if enabled {
        let valid_key_count = provider_keys::Entity::find()
            .filter(provider_keys::Column::ProviderId.eq(id))
            .filter(provider_keys::Column::Enabled.eq(1))
            .filter(provider_keys::Column::LastValidatedAt.is_not_null())
            .filter(provider_keys::Column::LastError.is_null())
            .count(db)
            .await?;
        if valid_key_count == 0 {
            return Err(AxAgentError::Validation(
                "Provider cannot be enabled: no validated API key. Please add and validate an API key first.".to_string(),
            ));
        }
    }

    let mut am: providers::ActiveModel = row.into();
    am.enabled = Set(if enabled { 1 } else { 0 });
    am.updated_at = Set(now_ts());
    am.update(db).await?;

    Ok(())
}

/// 检查 provider 是否还有有效 key（enabled + last_validated_at 不为空 + last_error 为空），
/// 如果没有且 provider 为启用状态，则自动禁用 provider。
async fn auto_disable_provider_if_no_valid_keys(
    db: &DatabaseConnection,
    provider_id: &str,
) -> Result<()> {
    let valid_key_count = provider_keys::Entity::find()
        .filter(provider_keys::Column::ProviderId.eq(provider_id))
        .filter(provider_keys::Column::Enabled.eq(1))
        .filter(provider_keys::Column::LastValidatedAt.is_not_null())
        .filter(provider_keys::Column::LastError.is_null())
        .count(db)
        .await?;

    if valid_key_count == 0 {
        // 没有有效 key，自动禁用 provider
        if let Some(row) = providers::Entity::find_by_id(provider_id).one(db).await? {
            if row.enabled != 0 {
                let mut am: providers::ActiveModel = row.into();
                am.enabled = Set(0);
                am.updated_at = Set(now_ts());
                am.update(db).await?;
            }
        }
    }

    Ok(())
}

// --- Provider Key CRUD ---

pub async fn reorder_providers(db: &DatabaseConnection, provider_ids: &[String]) -> Result<()> {
    for (i, id) in provider_ids.iter().enumerate() {
        providers::Entity::update_many()
            .col_expr(providers::Column::SortOrder, Expr::value(i as i32))
            .col_expr(
                providers::Column::UpdatedAt,
                Expr::value(axagent_harness::util_fns::now_ts()),
            )
            .filter(providers::Column::Id.eq(id))
            .exec(db)
            .await?;
    }
    Ok(())
}

// --- Provider Key CRUD (continued) ---

pub async fn list_keys_for_provider(
    db: &DatabaseConnection,
    provider_id: &str,
) -> Result<Vec<ProviderKey>> {
    let rows = provider_keys::Entity::find()
        .filter(provider_keys::Column::ProviderId.eq(provider_id))
        .order_by_asc(provider_keys::Column::RotationIndex)
        .all(db)
        .await?;

    Ok(rows.into_iter().map(key_from_entity).collect())
}

pub async fn add_provider_key(
    db: &DatabaseConnection,
    provider_id: &str,
    key_encrypted: &str,
    key_prefix: &str,
) -> Result<ProviderKey> {
    let id = gen_id();
    let now = now_ts();

    let max_idx = provider_keys::Entity::find()
        .filter(provider_keys::Column::ProviderId.eq(provider_id))
        .select_only()
        .column_as(provider_keys::Column::RotationIndex.max(), "m")
        .into_tuple::<Option<i32>>()
        .one(db)
        .await?
        .flatten();
    let rotation_index = max_idx.unwrap_or(-1) + 1;

    provider_keys::ActiveModel {
        id: Set(id.clone()),
        provider_id: Set(provider_id.to_string()),
        key_encrypted: Set(key_encrypted.to_string()),
        key_prefix: Set(key_prefix.to_string()),
        enabled: Set(1),
        last_validated_at: Set(None),
        last_error: Set(None),
        rotation_index: Set(rotation_index),
        created_at: Set(now),
    }
    .insert(db)
    .await?;

    let row = provider_keys::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ProviderKey {}", id)))?;
    Ok(key_from_entity(row))
}

pub async fn update_provider_key(
    db: &DatabaseConnection,
    key_id: &str,
    key_encrypted: &str,
    key_prefix: &str,
) -> Result<ProviderKey> {
    let row = provider_keys::Entity::find_by_id(key_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ProviderKey {}", key_id)))?;

    let mut am: provider_keys::ActiveModel = row.into();
    am.key_encrypted = Set(key_encrypted.to_string());
    am.key_prefix = Set(key_prefix.to_string());
    am.last_validated_at = Set(None);
    am.last_error = Set(None);
    am.update(db).await?;

    get_provider_key(db, key_id).await
}

pub async fn delete_provider_key(db: &DatabaseConnection, key_id: &str) -> Result<()> {
    // 先查询 key 获取所属 provider_id
    let key = provider_keys::Entity::find_by_id(key_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ProviderKey {}", key_id)))?;
    let provider_id = key.provider_id.clone();

    let result = provider_keys::Entity::delete_by_id(key_id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("ProviderKey {}", key_id)));
    }

    // 删除 key 后检查 provider 是否还有有效 key，没有则自动禁用
    auto_disable_provider_if_no_valid_keys(db, &provider_id).await?;

    Ok(())
}

pub async fn toggle_provider_key(
    db: &DatabaseConnection,
    key_id: &str,
    enabled: bool,
) -> Result<()> {
    let row = provider_keys::Entity::find_by_id(key_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ProviderKey {}", key_id)))?;

    let provider_id = row.provider_id.clone();
    let mut am: provider_keys::ActiveModel = row.into();
    am.enabled = Set(if enabled { 1 } else { 0 });
    am.update(db).await?;

    // 禁用 key 后检查 provider 是否还有有效 key，没有则自动禁用
    if !enabled {
        auto_disable_provider_if_no_valid_keys(db, &provider_id).await?;
    }

    Ok(())
}

pub async fn get_provider_key(db: &DatabaseConnection, key_id: &str) -> Result<ProviderKey> {
    let row = provider_keys::Entity::find_by_id(key_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ProviderKey {}", key_id)))?;
    Ok(key_from_entity(row))
}

pub async fn get_active_key(db: &DatabaseConnection, provider_id: &str) -> Result<ProviderKey> {
    let row = provider_keys::Entity::find()
        .filter(provider_keys::Column::ProviderId.eq(provider_id))
        .filter(provider_keys::Column::Enabled.eq(1))
        .order_by_asc(provider_keys::Column::RotationIndex)
        .one(db)
        .await?
        .ok_or_else(|| {
            AxAgentError::NotFound(format!("No active key for provider {}", provider_id))
        })?;
    Ok(key_from_entity(row))
}

pub async fn update_key_validation(
    db: &DatabaseConnection,
    key_id: &str,
    valid: bool,
) -> Result<()> {
    if let Some(row) = provider_keys::Entity::find_by_id(key_id).one(db).await? {
        let mut am: provider_keys::ActiveModel = row.into();
        if valid {
            // 验证成功：记录时间戳，清除历史错误
            am.last_validated_at = Set(Some(now_ts()));
            am.last_error = Set(None);
        } else {
            // 验证失败：只记录错误，不设置 last_validated_at
            // 这样 toggle_provider 的 is_not_null 检查才能正确排除无效 Key
            am.last_error = Set(Some("Validation failed".to_string()));
        }
        am.update(db).await?;
    }
    Ok(())
}

pub async fn get_enabled_keys(
    db: &DatabaseConnection,
    provider_id: &str,
) -> Result<Vec<ProviderKey>> {
    let rows = provider_keys::Entity::find()
        .filter(provider_keys::Column::ProviderId.eq(provider_id))
        .filter(provider_keys::Column::Enabled.eq(1))
        .order_by_asc(provider_keys::Column::RotationIndex)
        .all(db)
        .await?;

    Ok(rows.into_iter().map(key_from_entity).collect())
}

pub async fn update_rotation_index(
    db: &DatabaseConnection,
    key_id: &str,
    index: u32,
) -> Result<()> {
    if let Some(row) = provider_keys::Entity::find_by_id(key_id).one(db).await? {
        let mut am: provider_keys::ActiveModel = row.into();
        am.rotation_index = Set(index as i32);
        am.update(db).await?;
    }
    Ok(())
}

// --- Model CRUD ---

pub async fn list_models_for_provider(
    db: &DatabaseConnection,
    provider_id: &str,
) -> Result<Vec<Model>> {
    let rows = models::Entity::find()
        .filter(models::Column::ProviderId.eq(provider_id))
        .order_by_asc(models::Column::Name)
        .all(db)
        .await?;

    Ok(rows.into_iter().map(model_from_entity).collect())
}

pub async fn get_model(
    db: &DatabaseConnection,
    provider_id: &str,
    model_id: &str,
) -> Result<Model> {
    let row = models::Entity::find_by_id((provider_id.to_string(), model_id.to_string()))
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Model {}/{}", provider_id, model_id)))?;

    Ok(model_from_entity(row))
}

pub async fn save_models(
    db: &DatabaseConnection,
    provider_id: &str,
    input_models: &[Model],
) -> Result<()> {
    let provider_id = provider_id.to_string();

    // Deduplicate: keep the last occurrence of each model_id
    let mut seen = std::collections::HashSet::new();
    let mut deduped: Vec<Model> = Vec::with_capacity(input_models.len());
    for model in input_models.iter().rev() {
        if seen.insert(model.model_id.clone()) {
            deduped.push(model.clone());
        }
    }
    deduped.reverse();

    db.transaction::<_, _, sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            models::Entity::delete_many()
                .filter(models::Column::ProviderId.eq(&provider_id))
                .exec(txn)
                .await?;

            for model in &deduped {
                let capabilities =
                    serde_json::to_string(&model.capabilities).unwrap_or_else(|_| "[]".to_string());
                let param_overrides = model
                    .param_overrides
                    .as_ref()
                    .map(|po| serde_json::to_string(po).unwrap_or_else(|_| "null".to_string()));

                models::ActiveModel {
                    provider_id: Set(provider_id.clone()),
                    model_id: Set(model.model_id.clone()),
                    name: Set(model.name.clone()),
                    group_name: Set(model.group_name.clone()),
                    model_type: Set(model.model_type.to_string()),
                    capabilities: Set(capabilities),
                    max_tokens: Set(model.max_tokens.map(|v| v as i64)),
                    enabled: Set(if model.enabled { 1 } else { 0 }),
                    param_overrides: Set(param_overrides),
                    input_price_per_mtok: Set(model.input_price_per_mtok),
                    output_price_per_mtok: Set(model.output_price_per_mtok),
                }
                .insert(txn)
                .await?;
            }

            Ok(())
        })
    })
    .await?;

    Ok(())
}

pub async fn toggle_model(
    db: &DatabaseConnection,
    provider_id: &str,
    model_id: &str,
    enabled: bool,
) -> Result<Model> {
    let row = models::Entity::find_by_id((provider_id.to_string(), model_id.to_string()))
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Model {}/{}", provider_id, model_id)))?;

    let mut am: models::ActiveModel = row.into();
    am.enabled = Set(if enabled { 1 } else { 0 });
    am.update(db).await?;

    get_model(db, provider_id, model_id).await
}

pub async fn update_model_params(
    db: &DatabaseConnection,
    provider_id: &str,
    model_id: &str,
    overrides: ModelParamOverrides,
) -> Result<Model> {
    let param_json = serde_json::to_string(&overrides).unwrap_or_default();

    let row = models::Entity::find_by_id((provider_id.to_string(), model_id.to_string()))
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Model {}/{}", provider_id, model_id)))?;

    let mut am: models::ActiveModel = row.into();
    am.param_overrides = Set(Some(param_json));
    am.update(db).await?;

    get_model(db, provider_id, model_id).await
}

// --- Built-in Provider Merge ---

/// Merge built-in provider definitions with database records.
/// Built-in providers without a DB row appear as virtual providers (enabled=false, no keys/models).
/// Built-in providers with a DB row use the DB values (user overrides).
/// Custom providers (builtin_id=NULL) are appended after built-ins.
pub async fn list_providers_merged(db: &DatabaseConnection) -> Result<Vec<ProviderConfig>> {
    let db_providers = list_providers(db).await?;
    let builtins = crate::db::get_builtin_providers();

    let mut result = Vec::new();
    let mut added_ids = std::collections::HashSet::new();

    for bp in &builtins {
        if let Some(db_prov) = db_providers
            .iter()
            .find(|p| p.builtin_id.as_deref() == Some(bp.builtin_id))
        {
            result.push(db_prov.clone());
            added_ids.insert(db_prov.id.clone());
        } else {
            // Check if there's a custom provider with matching name/provider_type
            let existing_custom = db_providers.iter().find(|p| {
                p.builtin_id.is_none() && p.name == bp.name && p.provider_type == bp.provider_type
            });
            if let Some(custom_prov) = existing_custom {
                // Use the custom provider (don't add virtual)
                result.push(custom_prov.clone());
                added_ids.insert(custom_prov.id.clone());
            } else {
                // Add virtual built-in provider
                let now = now_ts();
                let default_models: Vec<Model> = bp
                    .models
                    .iter()
                    .map(|(model_id, name, caps, max_tokens)| Model {
                        provider_id: format!("builtin_{}", bp.builtin_id),
                        model_id: String::from(*model_id),
                        name: String::from(*name),
                        group_name: None,
                        model_type: ModelType::detect(model_id),
                        capabilities: caps.clone(),
                        max_tokens: *max_tokens,
                        max_output_tokens: None,
                        enabled: true,
                        param_overrides: None,
                        input_price_per_mtok: None,
                        output_price_per_mtok: None,
                    })
                    .collect();

                result.push(ProviderConfig {
                    id: format!("builtin_{}", bp.builtin_id),
                    name: String::from(bp.name),
                    provider_type: bp.provider_type.clone(),
                    api_host: String::from(bp.api_host),
                    api_path: None,
                    enabled: false,
                    models: default_models,
                    keys: vec![],
                    proxy_config: None,
                    custom_headers: None,
                    icon: None,
                    builtin_id: Some(String::from(bp.builtin_id)),
                    sort_order: 0,
                    created_at: now,
                    updated_at: now,
                });
            }
        }
    }

    // Append custom providers (no builtin_id, not already added)
    for p in &db_providers {
        if !added_ids.contains(&p.id) {
            result.push(p.clone());
        }
    }

    // Sort: enabled first (by sort_order), then disabled (by sort_order)
    result.sort_by(|a, b| {
        b.enabled
            .cmp(&a.enabled)
            .then(a.sort_order.cmp(&b.sort_order))
    });

    Ok(result)
}

/// Materialize a virtual built-in provider into the database.
/// Called when a user first modifies a built-in provider that has no DB record.
/// Returns the new real provider ID.
pub async fn ensure_builtin_provider(db: &DatabaseConnection, builtin_id: &str) -> Result<String> {
    let existing = providers::Entity::find()
        .filter(providers::Column::BuiltinId.eq(builtin_id))
        .one(db)
        .await?;

    if let Some(row) = existing {
        return Ok(row.id);
    }

    let builtins = crate::db::get_builtin_providers();
    let bp = builtins
        .iter()
        .find(|b| b.builtin_id == builtin_id)
        .ok_or_else(|| AxAgentError::NotFound(format!("Built-in provider {}", builtin_id)))?;

    let prov = create_provider(
        db,
        CreateProviderInput {
            name: String::from(bp.name),
            provider_type: bp.provider_type.clone(),
            api_host: String::from(bp.api_host),
            api_path: None,
            enabled: false,
            builtin_id: Some(String::from(builtin_id)),
        },
    )
    .await?;

    let models: Vec<Model> = bp
        .models
        .iter()
        .map(|(model_id, name, caps, max_tokens)| Model {
            provider_id: prov.id.clone(),
            model_id: String::from(*model_id),
            name: String::from(*name),
            group_name: None,
            model_type: ModelType::detect(model_id),
            capabilities: caps.clone(),
            max_tokens: *max_tokens,
            max_output_tokens: None,
            enabled: true,
            param_overrides: None,
            input_price_per_mtok: None,
            output_price_per_mtok: None,
        })
        .collect();

    save_models(db, &prov.id, &models).await?;

    Ok(prov.id)
}

/// Resolve a provider ID that might be a virtual builtin ID (e.g., "builtin_openai").
/// If virtual, materializes the provider into DB and returns the real ID.
/// If already a real ID, returns it unchanged.
pub async fn resolve_provider_id(db: &DatabaseConnection, id: &str) -> Result<String> {
    if let Some(builtin_id) = id.strip_prefix("builtin_") {
        ensure_builtin_provider(db, builtin_id).await
    } else {
        Ok(id.to_string())
    }
}
