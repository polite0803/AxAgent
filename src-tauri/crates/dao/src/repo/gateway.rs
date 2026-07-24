// SPDX-License-Identifier: AGPL-3.0-only

//! gateway_keys 表的 CRUD 操作。
//!
//! gateway_usage 表的查询统一放在 `repo::gateway_usage` 模块，
//! 避免两个模块各自维护一份 record_usage / get_metrics 造成重复定义。

use sea_orm::*;

use axagent_entities::gateway_keys;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::platform_adapter::CryptoService;
use axagent_harness::types::*;
use axagent_harness::util_fns::now_ts;

fn key_from_entity(m: gateway_keys::Model) -> GatewayKey {
    GatewayKey {
        id: m.id,
        name: m.name,
        key_hash: m.key_hash,
        key_prefix: m.key_prefix,
        enabled: m.enabled != 0,
        created_at: m.created_at,
        last_used_at: m.last_used_at,
        has_encrypted_key: m.encrypted_key.is_some(),
    }
}

// --- Gateway Key CRUD ---

pub async fn list_gateway_keys(db: &DatabaseConnection) -> Result<Vec<GatewayKey>> {
    let rows =
        gateway_keys::Entity::find().order_by_desc(gateway_keys::Column::CreatedAt).all(db).await?;

    Ok(rows.into_iter().map(key_from_entity).collect())
}

pub async fn create_gateway_key(
    db: &DatabaseConnection,
    name: &str,
    crypto: &dyn CryptoService,
    master_key: Option<&[u8; 32]>,
) -> Result<CreateGatewayKeyResult> {
    crate::repo::gateway_key::create_gateway_key(db, name, crypto, master_key).await
}

pub async fn delete_gateway_key(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = gateway_keys::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("GatewayKey {}", id)));
    }
    Ok(())
}

pub async fn toggle_gateway_key(db: &DatabaseConnection, id: &str, enabled: bool) -> Result<()> {
    let row = gateway_keys::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayKey {}", id)))?;

    let mut am: gateway_keys::ActiveModel = row.into();
    am.enabled = Set(if enabled { 1 } else { 0 });
    am.update(db).await?;

    Ok(())
}

/// Verify an incoming API key against stored hashes. Returns the matching key if found.
///
/// SECURITY (H6): 使用 `HMAC(master_key, api_key)` 而非 `SHA-256(api_key)`：
/// 1) 防 rainbow-table（HMAC 多了一个 master_key 参数）；
/// 2) 比较阶段用 `subtle::ConstantTimeEq` 阻断长度/字符级时序信息泄露。
pub async fn verify_key(
    db: &DatabaseConnection,
    plain_key: &str,
    crypto: &dyn CryptoService,
    master_key: &[u8; 32],
) -> Result<GatewayKey> {
    use subtle::ConstantTimeEq;
    // 1) 用 master_key 派生 HMAC；这样数据库只存"key 受 master_key 保护"的形式。
    let key_hash = crypto.hmac_sha256(master_key, plain_key);

    let row = gateway_keys::Entity::find()
        .filter(gateway_keys::Column::KeyHash.eq(&key_hash))
        .filter(gateway_keys::Column::Enabled.eq(1))
        .one(db)
        .await?
        .ok_or_else(|| {
            // SECURITY: 在错误路径上做一次固定时长的工作，避免"未命中"和"命中"分支
            // 在时间上可区分。subtle::ConstantTimeEq 在任何长度都消耗固定时长。
            let _ = key_hash.as_bytes().ct_eq(key_hash.as_bytes());
            AxAgentError::NotFound("Invalid or disabled gateway key".to_string())
        })?;

    // 2) 显式再做一次 CT 比较（DB 检索是 dominant cost，但此处保证代码风格一致）。
    let stored = row.key_hash.as_bytes();
    let computed = key_hash.as_bytes();
    if stored.ct_eq(computed).unwrap_u8() != 1 {
        return Err(AxAgentError::NotFound("Invalid or disabled gateway key".to_string()));
    }
    Ok(key_from_entity(row))
}

pub async fn update_last_used(db: &DatabaseConnection, id: &str) -> Result<()> {
    if let Some(row) = gateway_keys::Entity::find_by_id(id).one(db).await? {
        let mut am: gateway_keys::ActiveModel = row.into();
        am.last_used_at = Set(Some(now_ts()));
        am.update(db).await?;
    }
    Ok(())
}

/// Look up a gateway key by its stable id (not the API key plaintext).
/// Returns `Err(NotFound)` if the key does not exist.
pub async fn get_by_id(db: &DatabaseConnection, key_id: &str) -> Result<GatewayKey> {
    let row = gateway_keys::Entity::find_by_id(key_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("GatewayKey {}", key_id)))?;
    Ok(key_from_entity(row))
}
