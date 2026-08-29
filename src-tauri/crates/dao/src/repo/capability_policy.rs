// SPDX-License-Identifier: AGPL-3.0-only

//! 能力发现策略 repository —— 可注册后置过滤器规则的持久化层（Phase 3 策略对象化）。
//!
//! 策略规则为排除型 JSON，`CapabilityFilterImpl` 在过滤候选前加载启用策略执行裁剪。

use sea_orm::*;

use axagent_entities::capability_policies;
use axagent_harness::core_error::Result;
use axagent_harness::util_fns::now_ts;

/// 排除型策略规则的解码结构（rules_json 的权威 schema）
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityPolicyRules {
    /// 排除的域（如 ["ai_media"]）
    #[serde(default)]
    pub exclude_domains: Vec<String>,
    /// 排除的标签（如 ["cloud_api"]）
    #[serde(default)]
    pub exclude_tags: Vec<String>,
    /// 排除的能力 ID（如 ["tool:web_search"]）
    #[serde(default)]
    pub exclude_capability_ids: Vec<String>,
}

/// 启用策略的运行时 DTO
#[derive(Debug, Clone)]
pub struct CapabilityPolicyDto {
    pub id: String,
    pub name: String,
    pub rules: CapabilityPolicyRules,
}

/// 列出所有启用策略（按优先级升序）。
pub async fn list_enabled(db: &DatabaseConnection) -> Result<Vec<CapabilityPolicyDto>> {
    let rows = capability_policies::Entity::find()
        .filter(capability_policies::Column::Enabled.eq(true))
        .order_by_asc(capability_policies::Column::Priority)
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            serde_json::from_str::<CapabilityPolicyRules>(&r.rules_json)
                .ok()
                .map(|rules| CapabilityPolicyDto { id: r.id.clone(), name: r.name.clone(), rules })
        })
        .collect())
}

/// 创建/更新策略（upsert，幂等）。
pub async fn upsert_policy(
    db: &DatabaseConnection,
    id: &str,
    name: &str,
    description: Option<&str>,
    rules: &CapabilityPolicyRules,
    enabled: bool,
    priority: i32,
) -> Result<()> {
    let now = now_ts();
    let rules_json = serde_json::to_string(rules).unwrap_or_else(|_| "{}".to_string());
    let am = capability_policies::ActiveModel {
        id: Set(id.to_string()),
        name: Set(name.to_string()),
        description: Set(description.map(|s| s.to_string())),
        rules_json: Set(rules_json),
        enabled: Set(enabled),
        priority: Set(priority),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let _ = capability_policies::Entity::insert(am.clone())
        .on_conflict(
            sea_query::OnConflict::column(capability_policies::Column::Id)
                .update_columns([
                    capability_policies::Column::Name,
                    capability_policies::Column::Description,
                    capability_policies::Column::RulesJson,
                    capability_policies::Column::Enabled,
                    capability_policies::Column::Priority,
                    capability_policies::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    async fn setup() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.expect("连接数据库应成功");
        crate::migrations::run_migrations(&db).await.expect("迁移应成功");
        db
    }

    #[tokio::test]
    async fn upsert_and_list_enabled() {
        let db = setup().await;
        let rules = CapabilityPolicyRules {
            exclude_domains: vec!["ai_media".to_string()],
            exclude_tags: vec!["cloud_api".to_string()],
            exclude_capability_ids: vec!["tool:web_search".to_string()],
        };
        upsert_policy(&db, "p1", "内网安全策略", Some("内网禁云 API"), &rules, true, 1)
            .await
            .expect("upsert 应成功");
        // 幂等重跑
        upsert_policy(&db, "p1", "内网安全策略", Some("内网禁云 API"), &rules, true, 1)
            .await
            .expect("upsert 应成功");

        let policies = list_enabled(&db).await.expect("列表应成功");
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].id, "p1");
        assert_eq!(policies[0].rules.exclude_domains, vec!["ai_media".to_string()]);
    }

    #[tokio::test]
    async fn disabled_policy_not_listed() {
        let db = setup().await;
        upsert_policy(&db, "p2", "停用策略", None, &CapabilityPolicyRules::default(), false, 0)
            .await
            .expect("upsert 应成功");
        assert!(list_enabled(&db).await.expect("列表应成功").is_empty());
    }
}
