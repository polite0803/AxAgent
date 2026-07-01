// SPDX-License-Identifier: AGPL-3.0-only

//! Credential persistence — CRUD over the `credentials` table.
//!
//! The `data_encrypted` column stores the full `Credential` struct as
//! an AES-256-GCM encrypted JSON blob. Encryption/decryption is handled
//! by `axagent_harness::credential::CredentialStore`, not here.

use sea_orm::*;

use axagent_entities::credentials;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::util_fns::{gen_id, now_ts};

/// A database row representing a stored credential (metadata only, no secrets).
#[derive(Debug, Clone)]
pub struct CredentialRow {
    pub id: String,
    pub name: String,
    pub credential_type: String,
    pub data_encrypted: String,
    pub created_at: i64,
    pub updated_at: i64,
}

fn row_from_entity(m: credentials::Model) -> CredentialRow {
    CredentialRow {
        id: m.id,
        name: m.name,
        credential_type: m.credential_type,
        data_encrypted: m.data_encrypted,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

/// Insert a new credential row.
pub async fn insert_credential(
    db: &DatabaseConnection,
    name: &str,
    credential_type: &str,
    data_encrypted: &str,
) -> Result<CredentialRow> {
    let id = gen_id();
    let now = now_ts();

    credentials::ActiveModel {
        id: Set(id.clone()),
        name: Set(name.to_string()),
        credential_type: Set(credential_type.to_string()),
        data_encrypted: Set(data_encrypted.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    let row = credentials::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("credential {id}")))?;
    Ok(row_from_entity(row))
}

/// Get a credential row by ID.
pub async fn get_credential(db: &DatabaseConnection, id: &str) -> Result<CredentialRow> {
    credentials::Entity::find_by_id(id)
        .one(db)
        .await?
        .map(row_from_entity)
        .ok_or_else(|| AxAgentError::NotFound(format!("credential {id}")))
}

/// List all credential rows (metadata only).
pub async fn list_credentials(db: &DatabaseConnection) -> Result<Vec<CredentialRow>> {
    let rows = credentials::Entity::find()
        .order_by_asc(credentials::Column::Name)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(row_from_entity).collect())
}

/// Update the encrypted data and timestamp for an existing credential.
pub async fn update_credential(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    credential_type: &str,
    data_encrypted: &str,
) -> Result<CredentialRow> {
    let row = credentials::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("credential {id}")))?;

    let mut am: credentials::ActiveModel = row.into();
    am.name = Set(name.to_string());
    am.credential_type = Set(credential_type.to_string());
    am.data_encrypted = Set(data_encrypted.to_string());
    am.updated_at = Set(now_ts());
    am.update(db).await?;

    let row = credentials::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("credential {id}")))?;
    Ok(row_from_entity(row))
}

/// Delete a credential by ID.
pub async fn delete_credential(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = credentials::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("credential {id}")));
    }
    Ok(())
}

/// Check if a credential exists by ID.
pub async fn credential_exists(db: &DatabaseConnection, id: &str) -> Result<bool> {
    let count = credentials::Entity::find()
        .filter(credentials::Column::Id.eq(id))
        .count(db)
        .await?;
    Ok(count > 0)
}
