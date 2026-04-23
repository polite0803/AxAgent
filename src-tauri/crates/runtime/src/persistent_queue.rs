use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "gateway_message_queue")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub payload_type: String,
    pub payload: String,
    pub status: String,
    #[sea_orm(default_value = "0")]
    pub retry_count: i32,
    pub max_retries: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: Option<i64>,
    pub correlation_id: Option<String>,
    pub reply_to: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: uuid_v4(),
            from_agent: String::new(),
            to_agent: String::new(),
            payload_type: "text".to_string(),
            payload: String::new(),
            status: "pending".to_string(),
            retry_count: 0,
            max_retries: 3,
            created_at: now_ts(),
            updated_at: now_ts(),
            expires_at: None,
            correlation_id: None,
            reply_to: None,
        }
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let random: u128 = (timestamp as u128) << 64 | (rand_u64() as u128);
    format!("{:032x}", random)
}

fn rand_u64() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish()
}