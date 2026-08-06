// SPDX-License-Identifier: AGPL-3.0-only

//! 客户服务实现 — SeaORM CRUD

use async_trait::async_trait;
use sea_orm::QuerySelect;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use std::str::FromStr;

use axagent_harness::util_fns::{gen_id, now_ts};
use axagent_opc_entities::opc_customers;
use axagent_opc_types::{
    CreateCustomerInput, Customer, CustomerFilter, CustomerService, CustomerSource, CustomerStatus,
    OpcError, OpcResult, UpdateCustomerInput,
};

/// 默认客户服务实现
pub struct DefaultCustomerService {
    pub db: DatabaseConnection,
}

impl DefaultCustomerService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

// ── Entity ↔ DTO 转换 ─────────────────────────────────────────────

fn entity_to_dto(e: opc_customers::Model) -> OpcResult<Customer> {
    let tags: Vec<String> = serde_json::from_str(&e.tags_json).unwrap_or_default();
    let source = e.source.as_deref().map(|s| match s {
        "referral" => CustomerSource::Referral,
        "website" => CustomerSource::Website,
        "social_media" => CustomerSource::SocialMedia,
        "marketplace" => CustomerSource::Marketplace,
        "direct" => CustomerSource::Direct,
        other => CustomerSource::Other(other.to_string()),
    });
    let status = CustomerStatus::from_str(&e.status).unwrap_or(CustomerStatus::Lead);

    Ok(Customer {
        id: e.id,
        name: e.name,
        email: e.email,
        phone: e.phone,
        company: e.company,
        source,
        tags,
        notes: e.notes,
        total_revenue: e.total_revenue,
        invoice_count: e.invoice_count,
        status,
        created_at: e.created_at,
        updated_at: e.updated_at,
    })
}

fn source_to_str(s: &CustomerSource) -> String {
    match s {
        CustomerSource::Referral => "referral".into(),
        CustomerSource::Website => "website".into(),
        CustomerSource::SocialMedia => "social_media".into(),
        CustomerSource::Marketplace => "marketplace".into(),
        CustomerSource::Direct => "direct".into(),
        CustomerSource::Other(v) => v.clone(),
    }
}

// ── Service 实现 ───────────────────────────────────────────────────

#[async_trait]
impl CustomerService for DefaultCustomerService {
    async fn create_customer(&self, input: CreateCustomerInput) -> OpcResult<Customer> {
        let id = gen_id();
        let now = now_ts();

        opc_customers::ActiveModel {
            id: Set(id.clone()),
            name: Set(input.name),
            email: Set(input.email),
            phone: Set(input.phone),
            company: Set(input.company),
            source: Set(input.source.as_ref().map(source_to_str)),
            tags_json: Set(serde_json::to_string(&input.tags).unwrap_or_else(|_| "[]".into())),
            notes: Set(input.notes),
            total_revenue: Set(0.0),
            invoice_count: Set(0),
            status: Set(CustomerStatus::Lead.as_str().to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.db)
        .await
        .map_err(|e| OpcError::Database(e.to_string()))?;

        self.get_customer(&id).await
    }

    async fn get_customer(&self, id: &str) -> OpcResult<Customer> {
        let entity = opc_customers::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("customer {id}")))?;

        entity_to_dto(entity)
    }

    async fn list_customers(&self, filter: CustomerFilter) -> OpcResult<Vec<Customer>> {
        let mut query =
            opc_customers::Entity::find().order_by_desc(opc_customers::Column::CreatedAt);

        if let Some(status) = &filter.status {
            query = query.filter(opc_customers::Column::Status.eq(status.as_str()));
        }
        if let Some(search) = &filter.search {
            query = query.filter(
                sea_orm::Condition::any()
                    .add(opc_customers::Column::Name.contains(search))
                    .add(opc_customers::Column::Email.contains(search))
                    .add(opc_customers::Column::Company.contains(search)),
            );
        }
        if let Some(limit) = filter.limit {
            query = query.limit(limit as u64);
        }
        if let Some(offset) = filter.offset {
            query = query.offset(offset as u64);
        }

        let entities = query.all(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        entities.into_iter().map(entity_to_dto).collect()
    }

    async fn update_customer(&self, id: &str, input: UpdateCustomerInput) -> OpcResult<Customer> {
        let entity = opc_customers::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?
            .ok_or_else(|| OpcError::NotFound(format!("customer {id}")))?;

        let mut am: opc_customers::ActiveModel = entity.into();
        am.updated_at = Set(now_ts());

        if let Some(name) = input.name {
            am.name = Set(name);
        }
        if let Some(email) = input.email {
            am.email = Set(email);
        }
        if let Some(phone) = input.phone {
            am.phone = Set(phone);
        }
        if let Some(company) = input.company {
            am.company = Set(company);
        }
        if let Some(tags) = input.tags {
            am.tags_json = Set(serde_json::to_string(&tags).unwrap_or_else(|_| "[]".into()));
        }
        if let Some(notes) = input.notes {
            am.notes = Set(notes);
        }

        am.update(&self.db).await.map_err(|e| OpcError::Database(e.to_string()))?;
        self.get_customer(id).await
    }

    async fn delete_customer(&self, id: &str) -> OpcResult<()> {
        let result = opc_customers::Entity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(OpcError::NotFound(format!("customer {id}")));
        }
        Ok(())
    }

    async fn find_by_email(&self, email: &str) -> OpcResult<Option<Customer>> {
        let entity = opc_customers::Entity::find()
            .filter(opc_customers::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| OpcError::Database(e.to_string()))?;

        match entity {
            Some(e) => entity_to_dto(e).map(Some),
            None => Ok(None),
        }
    }
}
