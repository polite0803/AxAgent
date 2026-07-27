// SPDX-License-Identifier: AGPL-3.0-only

//! Reading List DAO 实现：用户收藏的论文/文档集合 CRUD 与排序。

use sea_orm::*;

use axagent_entities::reading_lists;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{CreateReadingListInput, ReadingList, UpdateReadingListInput};
use axagent_harness::util_fns::gen_id;

/// 当前时间戳（Unix 毫秒）
fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// entity → DTO
fn model_to_list(m: reading_lists::Model) -> ReadingList {
    ReadingList {
        id: m.id,
        name: m.name,
        description: m.description,
        owner_user_id: m.owner_user_id,
        status: m.status,
        sort_order: m.sort_order,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

/// 列出全部阅读列表（按 sort_order 升序，其次按 created_at 升序）
pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<ReadingList>> {
    let rows = reading_lists::Entity::find()
        .order_by_asc(reading_lists::Column::SortOrder)
        .order_by_asc(reading_lists::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(model_to_list).collect())
}

/// 按 ID 获取
pub async fn get(db: &DatabaseConnection, id: &str) -> Result<ReadingList> {
    let m = reading_lists::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ReadingList {}", id)))?;
    Ok(model_to_list(m))
}

/// 创建
pub async fn create(db: &DatabaseConnection, input: CreateReadingListInput) -> Result<ReadingList> {
    let id = gen_id();
    let now = now_millis();

    let am = reading_lists::ActiveModel {
        id: Set(id.clone()),
        name: Set(input.name),
        description: Set(input.description),
        owner_user_id: Set(input.owner_user_id),
        status: Set("active".to_string()),
        sort_order: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    get(db, &id).await
}

/// 更新
pub async fn update(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateReadingListInput,
) -> Result<ReadingList> {
    let m = reading_lists::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ReadingList {}", id)))?;

    let mut am: reading_lists::ActiveModel = m.into();
    am.updated_at = Set(now_millis());

    if let Some(v) = input.name {
        am.name = Set(v);
    }
    if let Some(v) = input.description {
        am.description = Set(v);
    }
    if let Some(v) = input.status {
        am.status = Set(v);
    }
    if let Some(v) = input.sort_order {
        am.sort_order = Set(v);
    }

    am.update(db).await?;
    get(db, id).await
}

/// 删除（外键 ON DELETE CASCADE 会自动清理 reading_list_items）
pub async fn delete(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = reading_lists::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("ReadingList {}", id)));
    }
    Ok(())
}

/// 重排序：按传入的 ids 顺序依次更新 sort_order（0, 1, 2, ...）
///
/// 不在列表中的阅读列表 sort_order 不变；不存在的 id 静默跳过。
pub async fn reorder(db: &DatabaseConnection, ids: &[String]) -> Result<()> {
    let now = now_millis();
    for (idx, id) in ids.iter().enumerate() {
        let m = reading_lists::Entity::find_by_id(id).one(db).await?;
        if let Some(m) = m {
            let mut am: reading_lists::ActiveModel = m.into();
            am.sort_order = Set(idx as i32);
            am.updated_at = Set(now);
            am.update(db).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input(name: &str) -> CreateReadingListInput {
        CreateReadingListInput {
            name: name.to_string(),
            description: Some("测试列表".to_string()),
            owner_user_id: None,
        }
    }

    #[tokio::test]
    async fn crud_round_trip() {
        use crate::migrations::v107_paper_reading_list as v107;
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        v107::up(db.clone()).await.unwrap();

        // create
        let created = create(&db, sample_input("待读")).await.unwrap();
        assert_eq!(created.name, "待读");
        assert_eq!(created.status, "active");
        assert_eq!(created.sort_order, 0);

        // get
        let fetched = get(&db, &created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);

        // list_all
        let list = list_all(&db).await.unwrap();
        assert_eq!(list.len(), 1);

        // update
        let updated = update(
            &db,
            &created.id,
            UpdateReadingListInput {
                name: Some("已读".to_string()),
                description: Some(None),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.name, "已读");
        assert_eq!(updated.description, None);

        // delete
        delete(&db, &created.id).await.unwrap();
        assert!(get(&db, &created.id).await.is_err());
    }

    #[tokio::test]
    async fn reorder_changes_sort_order() {
        use crate::migrations::v107_paper_reading_list as v107;
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        v107::up(db.clone()).await.unwrap();

        let a = create(&db, sample_input("A")).await.unwrap();
        let b = create(&db, sample_input("B")).await.unwrap();
        let c = create(&db, sample_input("C")).await.unwrap();

        // 反序排列
        reorder(&db, &[c.id.clone(), b.id.clone(), a.id.clone()]).await.unwrap();

        let list = list_all(&db).await.unwrap();
        assert_eq!(list[0].name, "C");
        assert_eq!(list[1].name, "B");
        assert_eq!(list[2].name, "A");
    }
}
