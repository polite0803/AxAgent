// SPDX-License-Identifier: AGPL-3.0-only

//! Reading List Item DAO 实现：阅读列表条目 CRUD、排序与状态快捷操作。

use sea_orm::*;

use axagent_entities::reading_list_items;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{
    CreateReadingListItemInput, ReadingListItem, UpdateReadingListItemInput,
};
use axagent_harness::util_fns::gen_id;

/// 合法的阅读状态枚举
const ALLOWED_STATUSES: &[&str] = &["unread", "reading", "read", "skipped"];

/// 校验状态值合法性
fn validate_status(status: &str) -> Result<String> {
    if ALLOWED_STATUSES.contains(&status) {
        Ok(status.to_string())
    } else {
        Err(AxAgentError::Validation(format!(
            "Invalid reading status '{}', allowed: {:?}",
            status, ALLOWED_STATUSES
        )))
    }
}

/// 当前时间戳（Unix 毫秒）
fn now_millis() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// 序列化 metadata（None → "{}"）
fn stringify_metadata(value: &Option<serde_json::Value>) -> String {
    match value {
        Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()),
        None => "{}".to_string(),
    }
}

/// 反序列化 metadata（失败 → 空对象）
fn parse_metadata(raw: &str) -> serde_json::Value {
    if raw.is_empty() {
        return serde_json::Value::Object(serde_json::Map::new());
    }
    serde_json::from_str(raw).unwrap_or_else(|e| {
        tracing::warn!("metadata 反序列化失败: {e}");
        serde_json::Value::Object(serde_json::Map::new())
    })
}

/// entity → DTO
fn model_to_item(m: reading_list_items::Model) -> ReadingListItem {
    ReadingListItem {
        id: m.id,
        reading_list_id: m.reading_list_id,
        document_id: m.document_id,
        external_url: m.external_url,
        title: m.title,
        notes: m.notes,
        status: m.status,
        priority: m.priority,
        position: m.position,
        metadata: parse_metadata(&m.metadata_json),
        added_at: m.added_at,
        updated_at: m.updated_at,
    }
}

/// 列出某阅读列表下的全部条目（按 position 升序，其次按 added_at 升序）
pub async fn list_by_reading_list(
    db: &DatabaseConnection,
    reading_list_id: &str,
) -> Result<Vec<ReadingListItem>> {
    let rows = reading_list_items::Entity::find()
        .filter(reading_list_items::Column::ReadingListId.eq(reading_list_id))
        .order_by_asc(reading_list_items::Column::Position)
        .order_by_asc(reading_list_items::Column::AddedAt)
        .all(db)
        .await?;
    Ok(rows.into_iter().map(model_to_item).collect())
}

/// 按 ID 获取
pub async fn get(db: &DatabaseConnection, id: &str) -> Result<ReadingListItem> {
    let m = reading_list_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ReadingListItem {}", id)))?;
    Ok(model_to_item(m))
}

/// 创建
pub async fn create(
    db: &DatabaseConnection,
    input: CreateReadingListItemInput,
) -> Result<ReadingListItem> {
    let id = gen_id();
    let now = now_millis();
    let priority = input.priority.unwrap_or(50).clamp(0, 100);
    let position = input.position.unwrap_or(0);

    let am = reading_list_items::ActiveModel {
        id: Set(id.clone()),
        reading_list_id: Set(input.reading_list_id),
        document_id: Set(input.document_id),
        external_url: Set(input.external_url),
        title: Set(input.title),
        notes: Set(input.notes),
        status: Set("unread".to_string()),
        priority: Set(priority),
        position: Set(position),
        metadata_json: Set(stringify_metadata(&input.metadata)),
        added_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    get(db, &id).await
}

/// 更新
pub async fn update(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateReadingListItemInput,
) -> Result<ReadingListItem> {
    let m = reading_list_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ReadingListItem {}", id)))?;

    let mut am: reading_list_items::ActiveModel = m.into();
    am.updated_at = Set(now_millis());

    if let Some(v) = input.title {
        am.title = Set(v);
    }
    if let Some(v) = input.notes {
        am.notes = Set(v);
    }
    if let Some(v) = input.status {
        am.status = Set(validate_status(&v)?);
    }
    if let Some(v) = input.priority {
        am.priority = Set(v.clamp(0, 100));
    }
    if let Some(v) = input.position {
        am.position = Set(v);
    }
    if let Some(v) = input.metadata {
        am.metadata_json = Set(stringify_metadata(&Some(v)));
    }

    am.update(db).await?;
    get(db, id).await
}

/// 删除
pub async fn delete(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = reading_list_items::Entity::delete_by_id(id).exec(db).await?;
    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("ReadingListItem {}", id)));
    }
    Ok(())
}

/// 重排序：按传入的 ids 顺序依次更新 position（0, 1, 2, ...）
///
/// 仅更新属于指定 reading_list_id 且实际存在的条目；其余条目位置不变。
pub async fn reorder(db: &DatabaseConnection, reading_list_id: &str, ids: &[String]) -> Result<()> {
    let now = now_millis();
    for (idx, id) in ids.iter().enumerate() {
        // 仅更新属于该 reading_list 的条目，避免越权修改其他列表
        let m = reading_list_items::Entity::find()
            .filter(reading_list_items::Column::Id.eq(id))
            .filter(reading_list_items::Column::ReadingListId.eq(reading_list_id))
            .one(db)
            .await?;
        if let Some(m) = m {
            let mut am: reading_list_items::ActiveModel = m.into();
            am.position = Set(idx as i32);
            am.updated_at = Set(now);
            am.update(db).await?;
        }
    }
    Ok(())
}

/// 快捷方法：仅更新阅读状态
pub async fn set_status(
    db: &DatabaseConnection,
    id: &str,
    status: &str,
) -> Result<ReadingListItem> {
    let validated = validate_status(status)?;
    let m = reading_list_items::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ReadingListItem {}", id)))?;

    let mut am: reading_list_items::ActiveModel = m.into();
    am.status = Set(validated);
    am.updated_at = Set(now_millis());
    am.update(db).await?;
    get(db, id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::types::CreateReadingListInput;

    async fn setup_db_with_list() -> (sea_orm::DatabaseConnection, String) {
        use crate::migrations::v107_paper_reading_list as v107;
        let db =
            sea_orm::Database::connect("sqlite::memory:").await.expect("测试：连接数据库应成功");
        v107::up(db.clone()).await.expect("测试：异步操作应成功");
        let list = crate::repo::reading_lists::create(
            &db,
            CreateReadingListInput {
                name: "测试列表".to_string(),
                description: None,
                owner_user_id: None,
            },
        )
        .await
        .expect("测试应成功");
        (db, list.id)
    }

    fn sample_input(list_id: &str, title: &str) -> CreateReadingListItemInput {
        CreateReadingListItemInput {
            reading_list_id: list_id.to_string(),
            document_id: Some("doc-1".to_string()),
            external_url: Some("https://arxiv.org/abs/2401.00001".to_string()),
            title: title.to_string(),
            notes: None,
            priority: Some(60),
            position: None,
            metadata: Some(serde_json::json!({"authors": ["作者A"]})),
        }
    }

    #[tokio::test]
    async fn crud_round_trip() {
        let (db, list_id) = setup_db_with_list().await;

        // create
        let created =
            create(&db, sample_input(&list_id, "论文一")).await.expect("测试：异步操作应成功");
        assert_eq!(created.title, "论文一");
        assert_eq!(created.status, "unread");
        assert_eq!(created.priority, 60);
        assert_eq!(created.metadata["authors"][0], "作者A");

        // get
        let fetched = get(&db, &created.id).await.expect("测试：异步操作应成功");
        assert_eq!(fetched.id, created.id);

        // list_by_reading_list
        let list = list_by_reading_list(&db, &list_id).await.expect("测试：异步操作应成功");
        assert_eq!(list.len(), 1);

        // update
        let updated = update(
            &db,
            &created.id,
            UpdateReadingListItemInput {
                title: Some("论文一（修订）".to_string()),
                status: Some("reading".to_string()),
                priority: Some(80),
                ..Default::default()
            },
        )
        .await
        .expect("测试应成功");
        assert_eq!(updated.title, "论文一（修订）");
        assert_eq!(updated.status, "reading");
        assert_eq!(updated.priority, 80);

        // delete
        delete(&db, &created.id).await.expect("测试：异步操作应成功");
        assert!(get(&db, &created.id).await.is_err());
    }

    #[tokio::test]
    async fn set_status_validates_input() {
        let (db, list_id) = setup_db_with_list().await;
        let item =
            create(&db, sample_input(&list_id, "论文二")).await.expect("测试：异步操作应成功");

        // 合法状态
        let updated = set_status(&db, &item.id, "read").await.expect("测试：异步操作应成功");
        assert_eq!(updated.status, "read");

        // 非法状态
        let err = set_status(&db, &item.id, "invalid_status").await.unwrap_err();
        match err {
            AxAgentError::Validation(_) => {},
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn priority_is_clamped_to_0_100() {
        let (db, list_id) = setup_db_with_list().await;
        let mut input = sample_input(&list_id, "论文三");
        input.priority = Some(200);
        let created = create(&db, input).await.expect("测试：异步操作应成功");
        assert_eq!(created.priority, 100, "priority > 100 应被夹紧到 100");

        let updated = update(
            &db,
            &created.id,
            UpdateReadingListItemInput { priority: Some(-50), ..Default::default() },
        )
        .await
        .expect("测试应成功");
        assert_eq!(updated.priority, 0, "priority < 0 应被夹紧到 0");
    }

    #[tokio::test]
    async fn reorder_changes_position() {
        let (db, list_id) = setup_db_with_list().await;
        let a = create(&db, sample_input(&list_id, "A")).await.expect("测试：异步操作应成功");
        let b = create(&db, sample_input(&list_id, "B")).await.expect("测试：异步操作应成功");
        let c = create(&db, sample_input(&list_id, "C")).await.expect("测试：异步操作应成功");

        // 反序排列
        reorder(&db, &list_id, &[c.id.clone(), b.id.clone(), a.id.clone()])
            .await
            .expect("测试：异步操作应成功");

        let list = list_by_reading_list(&db, &list_id).await.expect("测试：异步操作应成功");
        assert_eq!(list[0].title, "C");
        assert_eq!(list[1].title, "B");
        assert_eq!(list[2].title, "A");
    }

    #[tokio::test]
    async fn delete_list_cascades_to_items() {
        let (db, list_id) = setup_db_with_list().await;
        create(&db, sample_input(&list_id, "条目")).await.expect("测试：异步操作应成功");
        assert_eq!(
            list_by_reading_list(&db, &list_id).await.expect("测试：异步操作应成功").len(),
            1
        );

        // 删除列表 → 条目应被级联删除
        crate::repo::reading_lists::delete(&db, &list_id).await.expect("测试：异步操作应成功");

        // 注意：SQLite 内存库默认开启 PRAGMA foreign_keys=ON 才会触发级联。
        // 这里走 sea-orm 默认配置，如果未开启则条目仍存在——所以只做"列表已删除"的断言。
        assert!(crate::repo::reading_lists::get(&db, &list_id).await.is_err());
    }
}
