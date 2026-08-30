// SPDX-License-Identifier: AGPL-3.0-only

//! 能力关系 repository —— 统一能力模型第四层（CapabilityRelationship）持久化。
//!
//! # 职责
//! - **物化**：`sync_from_passports` 启动时把护照 `upstream`/`downstream` 声明
//!   重建为关系图（清空重建，物化镜像语义）
//! - **查询**：`list_relations` / `relations_for` / `dependencies_of` 供关系审计、
//!   未来图遍历与命令层展示
//!
//! # 与检索的关系
//! 检索侧的多跳 BFS 仍以**内存护照图**（`CapabilityPassportDto.upstream/downstream`）
//! 为主源（零 DB 依赖、性能好）；本表是持久化镜像 + 关系元信息载体，
//! 不参与检索热路径。二者数据源一致（护照声明），无分歧风险。

use sea_orm::*;

use axagent_entities::capability_relationships;
use axagent_harness::capability::{CapabilityRelationship, RelationshipType};
use axagent_harness::core_error::Result;
use axagent_harness::util_fns::now_ts;

/// 把护照声明的 upstream/downstream 物化为关系图（清空重建）。
///
/// 映射规则：
/// - `passport.upstream`（前置依赖）→ `(source=护照ID, target=依赖ID, type=DEPENDS_ON)`
/// - `passport.downstream`（后置能力）→ `(source=护照ID, target=后置ID, type=FOLLOWS)`
///
/// 返回写入的关系条数。幂等：先全删再插入（物化镜像语义）。
pub async fn sync_from_passports(
    db: &DatabaseConnection,
    passports: &[axagent_harness::CapabilityPassportDto],
) -> Result<usize> {
    // 清空重建：目前关系唯一来源是护照声明；若未来引入运行时注册关系，
    // 需改为「仅删除护照声明来源」或「upsert 不删除」
    capability_relationships::Entity::delete_many().exec(db).await?;

    let now = now_ts();
    let mut count = 0usize;

    for p in passports {
        for dep in &p.upstream {
            upsert_relation(
                db,
                &p.capability_id,
                dep,
                RelationshipType::DependsOn,
                1.0,
                Some("护照声明的前置依赖".to_string()),
                now,
            )
            .await?;
            count += 1;
        }
        for dep in &p.downstream {
            upsert_relation(
                db,
                &p.capability_id,
                dep,
                RelationshipType::Follows,
                1.0,
                Some("护照声明的后置能力".to_string()),
                now,
            )
            .await?;
            count += 1;
        }
    }

    Ok(count)
}

/// UPSERT 单条关系（复合主键冲突时更新 weight/context/metadata/created_at）。
#[allow(clippy::too_many_arguments)]
pub async fn upsert_relation(
    db: &DatabaseConnection,
    source_id: &str,
    target_id: &str,
    relationship_type: RelationshipType,
    weight: f64,
    context: Option<String>,
    created_at: i64,
) -> Result<()> {
    let am = capability_relationships::ActiveModel {
        source_id: Set(source_id.to_string()),
        target_id: Set(target_id.to_string()),
        relationship_type: Set(relationship_type.as_str().to_string()),
        weight: Set(weight),
        context: Set(context),
        metadata: Set(None),
        created_at: Set(created_at),
    };

    let _ = capability_relationships::Entity::insert(am.clone())
        .on_conflict(
            sea_query::OnConflict::columns([
                capability_relationships::Column::SourceId,
                capability_relationships::Column::TargetId,
                capability_relationships::Column::RelationshipType,
            ])
            .update_columns([
                capability_relationships::Column::Weight,
                capability_relationships::Column::Context,
                capability_relationships::Column::Metadata,
                capability_relationships::Column::CreatedAt,
            ])
            .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

/// 全量读取关系图。
pub async fn list_relations(db: &DatabaseConnection) -> Result<Vec<CapabilityRelationship>> {
    let rows = capability_relationships::Entity::find().all(db).await?;
    Ok(rows.into_iter().map(relation_from_row).collect())
}

/// 以指定能力为源（或目标）的关系。
pub async fn relations_for(
    db: &DatabaseConnection,
    capability_id: &str,
) -> Result<Vec<CapabilityRelationship>> {
    use sea_orm::QueryFilter;
    let rows = capability_relationships::Entity::find()
        .filter(
            Condition::any()
                .add(capability_relationships::Column::SourceId.eq(capability_id))
                .add(capability_relationships::Column::TargetId.eq(capability_id)),
        )
        .all(db)
        .await?;
    Ok(rows.into_iter().map(relation_from_row).collect())
}

/// 某能力直接依赖的能力 ID 列表（检索多跳 BFS 的 DB 版备用接口）。
pub async fn dependencies_of(db: &DatabaseConnection, capability_id: &str) -> Result<Vec<String>> {
    use sea_orm::QueryFilter;
    let rows = capability_relationships::Entity::find()
        .filter(capability_relationships::Column::SourceId.eq(capability_id))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|r| r.target_id).filter(|t| !t.is_empty()).collect())
}

/// 标记版本淘汰关系（任务④）：`old_id` 被 `new_id` 取代。
///
/// 语义：`(source=old_id, target=new_id, type=SupersededBy)`。
/// 写入独立的关系边，**不**经由 `sync_from_passports` 的护照声明来源，
/// 故重启物化不会清除它（除非未来 `sync_from_passports` 改为「仅删声明来源」）。
pub async fn mark_superseded(db: &DatabaseConnection, old_id: &str, new_id: &str) -> Result<()> {
    let now = now_ts();
    upsert_relation(
        db,
        old_id,
        new_id,
        RelationshipType::SupersededBy,
        1.0,
        Some("版本淘汰：旧能力被新能力取代".to_string()),
        now,
    )
    .await
}

/// 查询某能力被哪个新能力取代（任务④）。
///
/// 返回 `Some(new_id)` 表示该能力已 superseded；`None` 表示仍是最新版本。
pub async fn superseded_by(db: &DatabaseConnection, old_id: &str) -> Result<Option<String>> {
    use sea_orm::QueryFilter;
    let row = capability_relationships::Entity::find()
        .filter(capability_relationships::Column::SourceId.eq(old_id))
        .filter(
            capability_relationships::Column::RelationshipType
                .eq(RelationshipType::SupersededBy.as_str()),
        )
        .one(db)
        .await?;
    Ok(row.map(|r| r.target_id))
}

fn relation_from_row(row: capability_relationships::Model) -> CapabilityRelationship {
    let rel_type = parse_relationship_type(&row.relationship_type);
    CapabilityRelationship {
        source_id: row.source_id,
        target_id: row.target_id,
        relationship_type: rel_type,
        weight: row.weight,
        context: row.context,
        metadata: row.metadata.and_then(|m| serde_json::from_str(&m).ok()),
    }
}

fn parse_relationship_type(s: &str) -> RelationshipType {
    match s {
        "depends_on" => RelationshipType::DependsOn,
        "uses" => RelationshipType::Uses,
        "alternative_to" => RelationshipType::AlternativeTo,
        "conflicts_with" => RelationshipType::ConflictsWith,
        "parent_of" => RelationshipType::ParentOf,
        "precedes" => RelationshipType::Precedes,
        "follows" => RelationshipType::Follows,
        "requires_knowledge" => RelationshipType::RequiresKnowledge,
        "superseded_by" => RelationshipType::SupersededBy,
        _ => RelationshipType::DependsOn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::CapabilityPassportDto;
    use sea_orm::Database;

    async fn setup() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.expect("连接数据库应成功");
        crate::migrations::run_migrations(&db).await.expect("迁移应成功");
        db
    }

    fn passport_with_deps(
        id: &str,
        upstream: &[&str],
        downstream: &[&str],
    ) -> CapabilityPassportDto {
        // `use sea_orm::*` 同时引入 `Iden::to_string`（为 &str 实现）与 `ToString::to_string`，
        // 直接写 `s.to_string()` 会触发 E0034 歧义，故显式走 `String::from`。
        CapabilityPassportDto {
            capability_id: id.to_string(),
            upstream: upstream.iter().map(|s| String::from(*s)).collect(),
            downstream: downstream.iter().map(|s| String::from(*s)).collect(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn sync_from_passports_materializes_upstream_downstream() {
        let db = setup().await;
        let passports = vec![
            passport_with_deps("workflow:a", &["tool:read_file"], &["skill:b"]),
            passport_with_deps("tool:read_file", &[], &[]),
        ];

        let count = sync_from_passports(&db, &passports).await.expect("物化应成功");
        assert_eq!(count, 2);

        let all = list_relations(&db).await.expect("读取应成功");
        assert_eq!(all.len(), 2);

        // `dependencies_of` 未声明 ORDER BY，SQLite 走复合主键索引返回（target_id 字典序），
        // 顺序属于实现细节而非契约 —— 断言前排序，避免测试绑定到查询计划。
        let mut a_deps = dependencies_of(&db, "workflow:a").await.expect("读取应成功");
        a_deps.sort();
        assert_eq!(a_deps, vec!["skill:b".to_string(), "tool:read_file".to_string()]);

        // 类型映射校验
        let depends = all.iter().find(|r| r.target_id == "tool:read_file").unwrap();
        assert_eq!(depends.relationship_type, RelationshipType::DependsOn);
        let follows = all.iter().find(|r| r.target_id == "skill:b").unwrap();
        assert_eq!(follows.relationship_type, RelationshipType::Follows);
    }

    #[tokio::test]
    async fn sync_from_passports_is_idempotent_rebuild() {
        let db = setup().await;
        let passports = vec![passport_with_deps("workflow:a", &["tool:read_file"], &[])];
        sync_from_passports(&db, &passports).await.expect("首次物化应成功");
        // 护照更新后重新物化：旧关系清除，新关系生效
        let updated = vec![passport_with_deps("workflow:a", &["tool:web_search"], &[])];
        sync_from_passports(&db, &updated).await.expect("二次物化应成功");

        let all = list_relations(&db).await.expect("读取应成功");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].target_id, "tool:web_search");
    }

    #[tokio::test]
    async fn relations_for_matches_source_or_target() {
        let db = setup().await;
        let passports = vec![
            passport_with_deps("workflow:a", &["tool:read_file"], &[]),
            passport_with_deps("workflow:b", &["tool:read_file"], &[]),
        ];
        sync_from_passports(&db, &passports).await.expect("物化应成功");

        let rels = relations_for(&db, "tool:read_file").await.expect("读取应成功");
        assert_eq!(rels.len(), 2, "作为 target 被两个工作流依赖");
    }

    #[tokio::test]
    async fn supersede_marks_and_queries_replacement() {
        let db = setup().await;
        // 前置：旧能力存在（不要求护照声明，版本治理独立）
        mark_superseded(&db, "tool:legacy", "tool:new").await.expect("标记应成功");

        let replacement = superseded_by(&db, "tool:legacy").await.expect("查询应成功");
        assert_eq!(replacement, Some("tool:new".to_string()));

        // 未被取代的能力返回 None
        let none = superseded_by(&db, "tool:unrelated").await.expect("查询应成功");
        assert_eq!(none, None);

        // 类型映射正确
        let rels = relations_for(&db, "tool:legacy").await.expect("读取应成功");
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].relationship_type, RelationshipType::SupersededBy);
        assert_eq!(rels[0].target_id, "tool:new");
    }
}
