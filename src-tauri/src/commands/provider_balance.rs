// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_core::crypto::decrypt_key;
use axagent_core::repo::provider;
use axagent_runtime_core::fetch_deepseek_balance;
use sea_orm::EntityTrait;
use tauri::State;

#[derive(serde::Serialize)]
pub struct ProviderBalanceResponse {
    pub provider_id: String,
    pub provider_name: String,
    pub available: bool,
    pub infos: Vec<BalanceInfoResponse>,
}

#[derive(serde::Serialize)]
pub struct BalanceInfoResponse {
    pub currency: String,
    pub total: String,
    pub granted: String,
    pub topped_up: String,
}

#[tauri::command]
pub async fn fetch_provider_balance(
    state: State<'_, AppState>,
    provider_id: Option<String>,
) -> Result<ProviderBalanceResponse, String> {
    let db = state.harness.db();
    let providers = provider::list_providers(db)
        .await
        .map_err(|e| format!("Failed to list providers: {e}"))?;

    let target_provider = if let Some(pid) = provider_id {
        providers
            .into_iter()
            .find(|p| p.id == pid)
            .ok_or_else(|| format!("Provider '{}' not found", pid))?
    } else {
        providers
            .into_iter()
            .find(|p| {
                p.name.to_lowercase().contains("deepseek")
                    || p.api_host.to_lowercase().contains("deepseek")
            })
            .ok_or_else(|| {
                "No DeepSeek provider found. Please add a DeepSeek provider first.".to_string()
            })?
    };

    use axagent_core::entity::provider_keys;
    use sea_orm::ColumnTrait;
    use sea_orm::QueryFilter;

    let key = provider_keys::Entity::find()
        .filter(provider_keys::Column::ProviderId.eq(&target_provider.id))
        .filter(provider_keys::Column::Enabled.eq(1))
        .one(db)
        .await
        .map_err(|e| format!("Database error: {e}"))?
        .ok_or_else(|| "No enabled API key found for the provider.".to_string())?;

    let api_key = decrypt_key(&key.key_encrypted, state.harness.master_key())
        .map_err(|e| format!("Failed to decrypt API key: {e}"))?;

    let balance = fetch_deepseek_balance(&api_key)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No balance data returned (empty API key?).".to_string())?;

    Ok(ProviderBalanceResponse {
        provider_id: target_provider.id,
        provider_name: target_provider.name,
        available: balance.available,
        infos: balance
            .infos
            .into_iter()
            .map(|info| BalanceInfoResponse {
                currency: info.currency,
                total: info.total,
                granted: info.granted,
                topped_up: info.topped_up,
            })
            .collect(),
    })
}
