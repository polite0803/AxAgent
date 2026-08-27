// SPDX-License-Identifier: AGPL-3.0-only

//! workflow_tools 表仓储 —— 工作流运行时工具的持久化 CRUD。
//!
//! 职责边界（与 `generated_tools`（进化引擎证据归档，全局 append-only）
//! 区分）：本表按工作流归属，承载运行时发现/LLM 生成工具的完整生命周期
//! （pending → active → disabled 状态机、使用统计、启停），供
//! `start_workflow_execution` 启动时加载并 `register_runtime_tool`。

use axagent_entities::workflow_tools;
use axagent_harness::core_error::Result;
use sea_orm::sea_query::Expr;
use sea_orm::*;

pub const STATUS_PENDING: &str = "pending";
pub const STATUS_ACTIVE: &str = "active";
pub const STATUS_DISABLED: &str = "disabled";

pub const TYPE_RHAI_SCRIPT: &str = "rhai_script";
pub const TYPE_WORKFLOW_DAG: &str = "workflow_dag";
pub const TYPE_LLM_FUNCTION: &str = "llm_function";

/// 按工作流 + 状态批量查询（启动注册时使用，只取 active）
pub async fn list_by_workflow(
    db: &DatabaseConnection,
    workflow_id: &str,
    status: Option<&str>,
) -> Result<Vec<workflow_tools::Model>> {
    let mut query =
        workflow_tools::Entity::find().filter(workflow_tools::Column::WorkflowId.eq(workflow_id));
    if let Some(s) = status {
        query = query.filter(workflow_tools::Column::Status.eq(s));
    }
    Ok(query.order_by(workflow_tools::Column::CreatedAt, Order::Asc).all(db).await?)
}

/// 查询单个工具（按工作流内工具名）
pub async fn get_by_name(
    db: &DatabaseConnection,
    workflow_id: &str,
    tool_name: &str,
) -> Result<Option<workflow_tools::Model>> {
    Ok(workflow_tools::Entity::find()
        .filter(workflow_tools::Column::WorkflowId.eq(workflow_id))
        .filter(workflow_tools::Column::ToolName.eq(tool_name))
        .one(db)
        .await?)
}

pub async fn get_by_id(db: &DatabaseConnection, id: &str) -> Result<Option<workflow_tools::Model>> {
    Ok(workflow_tools::Entity::find_by_id(id).one(db).await?)
}

/// 插入或覆盖（(workflow_id, tool_name) 冲突时更新字段并保留统计）。
///
/// 参数为扁平化 DTO 字段（与 `repo/generated_tool.rs::insert_generated_tool` 同款模式，
/// 便于命令层直接透传），属 `too_many_arguments` 的合理设计例外。
#[allow(clippy::too_many_arguments)]
pub async fn upsert(
    db: &DatabaseConnection,
    id: &str,
    workflow_id: &str,
    tool_name: &str,
    tool_type: &str,
    description: Option<&str>,
    code: Option<&str>,
    input_schema: Option<&str>,
    source: &str,
    status: &str,
    now: i64,
) -> Result<()> {
    // 先按业务键查：命中 → 更新定义（保留 usage_count/success_rate）；未命中 → 插入
    if let Some(existing) = get_by_name(db, workflow_id, tool_name).await? {
        let mut am: workflow_tools::ActiveModel = existing.into();
        am.tool_type = Set(tool_type.to_string());
        am.description = Set(description.map(|s| s.to_string()));
        am.code = Set(code.map(|s| s.to_string()));
        am.input_schema = Set(input_schema.map(|s| s.to_string()));
        am.source = Set(source.to_string());
        am.status = Set(status.to_string());
        am.updated_at = Set(now);
        am.update(db).await?;
        return Ok(());
    }

    let model = workflow_tools::ActiveModel {
        id: Set(id.to_string()),
        workflow_id: Set(workflow_id.to_string()),
        tool_name: Set(tool_name.to_string()),
        tool_type: Set(tool_type.to_string()),
        description: Set(description.map(|s| s.to_string())),
        code: Set(code.map(|s| s.to_string())),
        input_schema: Set(input_schema.map(|s| s.to_string())),
        source: Set(source.to_string()),
        status: Set(status.to_string()),
        usage_count: Set(0),
        success_rate: Set(0.0),
        created_at: Set(now),
        updated_at: Set(now),
    };
    model.insert(db).await?;
    Ok(())
}

/// 更新状态（启停工具）
pub async fn update_status(
    db: &DatabaseConnection,
    id: &str,
    status: &str,
    now: i64,
) -> Result<bool> {
    let updated = workflow_tools::Entity::update_many()
        .col_expr(workflow_tools::Column::Status, Expr::value(status.to_string()))
        .col_expr(workflow_tools::Column::UpdatedAt, Expr::value(now))
        .filter(workflow_tools::Column::Id.eq(id))
        .exec(db)
        .await?;
    Ok(updated.rows_affected > 0)
}

/// 执行反馈回写：usage_count +1，success_rate 滚动平均
pub async fn record_execution_feedback(
    db: &DatabaseConnection,
    id: &str,
    success: bool,
    now: i64,
) -> Result<bool> {
    let Some(model) = workflow_tools::Entity::find_by_id(id).one(db).await? else {
        return Ok(false);
    };
    let new_count = model.usage_count + 1;
    // 滚动平均：new_rate = (old_rate * old_count + (success as f64)) / new_count
    let new_rate = if new_count == 0 {
        0.0
    } else {
        (model.success_rate * model.usage_count as f64 + if success { 1.0 } else { 0.0 })
            / new_count as f64
    };
    let mut am: workflow_tools::ActiveModel = model.into();
    am.usage_count = Set(new_count);
    am.success_rate = Set(new_rate);
    am.updated_at = Set(now);
    am.update(db).await?;
    Ok(true)
}

/// 执行反馈回写（按业务键定位）：同 `record_execution_feedback`，但以
/// `(workflow_id, tool_name)` 定位记录 —— 供运行时 sink 使用（工具执行时
/// 只持有工具名，无 UUID id）。
pub async fn record_execution_feedback_by_name(
    db: &DatabaseConnection,
    workflow_id: &str,
    tool_name: &str,
    success: bool,
    now: i64,
) -> Result<bool> {
    let Some(model) = get_by_name(db, workflow_id, tool_name).await? else {
        return Ok(false);
    };
    record_execution_feedback(db, &model.id, success, now).await
}

/// 删除工具（工作流删除时级联）
pub async fn delete(db: &DatabaseConnection, id: &str) -> Result<bool> {
    let Some(t) = workflow_tools::Entity::find_by_id(id).one(db).await? else {
        return Ok(false);
    };
    t.delete(db).await?;
    Ok(true)
}

/// 按工作流批量删除（级联清理）
pub async fn delete_by_workflow(db: &DatabaseConnection, workflow_id: &str) -> Result<u64> {
    let result = workflow_tools::Entity::delete_many()
        .filter(workflow_tools::Column::WorkflowId.eq(workflow_id))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    async fn setup() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.expect("连接内存库");
        db.execute_unprepared(
            "CREATE TABLE workflow_tools (\
             id TEXT NOT NULL PRIMARY KEY, \
             workflow_id TEXT NOT NULL, \
             tool_name TEXT NOT NULL, \
             tool_type TEXT NOT NULL, \
             description TEXT, \
             code TEXT, \
             input_schema TEXT, \
             source TEXT NOT NULL, \
             status TEXT NOT NULL, \
             usage_count INTEGER NOT NULL DEFAULT 0, \
             success_rate REAL NOT NULL DEFAULT 0, \
             created_at INTEGER NOT NULL, \
             updated_at INTEGER NOT NULL, \
             UNIQUE (workflow_id, tool_name))",
        )
        .await
        .expect("建表");
        db
    }

    #[tokio::test]
    async fn upsert_inserts_and_updates() {
        let db = setup().await;
        let now = 1_700_000_000;
        upsert(
            &db,
            "t1",
            "wf1",
            "calc",
            TYPE_RHAI_SCRIPT,
            None,
            Some("let r = input*2; r"),
            None,
            "manual",
            STATUS_ACTIVE,
            now,
        )
        .await
        .expect("首次插入");
        upsert(
            &db,
            "t1",
            "wf1",
            "calc",
            TYPE_RHAI_SCRIPT,
            Some("新版描述"),
            Some("let r = input*3; r"),
            None,
            "manual",
            STATUS_ACTIVE,
            now + 1,
        )
        .await
        .expect("冲突覆盖");

        let loaded = list_by_workflow(&db, "wf1", Some(STATUS_ACTIVE)).await.expect("查询");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].code.as_deref(), Some("let r = input*3; r"));
        assert_eq!(loaded[0].description.as_deref(), Some("新版描述"));
        // 统计保留（首次插入为 0）
        assert_eq!(loaded[0].usage_count, 0);
    }

    #[tokio::test]
    async fn status_filter_and_update() {
        let db = setup().await;
        let now = 1_700_000_000;
        upsert(
            &db,
            "t1",
            "wf1",
            "a",
            TYPE_RHAI_SCRIPT,
            None,
            None,
            None,
            "manual",
            STATUS_PENDING,
            now,
        )
        .await
        .expect("插入 pending");
        upsert(
            &db,
            "t2",
            "wf1",
            "b",
            TYPE_RHAI_SCRIPT,
            None,
            None,
            None,
            "manual",
            STATUS_ACTIVE,
            now,
        )
        .await
        .expect("插入 active");

        let active = list_by_workflow(&db, "wf1", Some(STATUS_ACTIVE)).await.expect("查询");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].tool_name, "b");

        update_status(&db, "t1", STATUS_ACTIVE, now + 1).await.expect("启用");
        let all = list_by_workflow(&db, "wf1", None).await.expect("查询全部");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].status, STATUS_ACTIVE);
    }

    #[tokio::test]
    async fn feedback_rolls_average() {
        let db = setup().await;
        let now = 1_700_000_000;
        upsert(
            &db,
            "t1",
            "wf1",
            "calc",
            TYPE_RHAI_SCRIPT,
            None,
            None,
            None,
            "manual",
            STATUS_ACTIVE,
            now,
        )
        .await
        .expect("插入");

        record_execution_feedback(&db, "t1", true, now + 1).await.expect("成功反馈");
        record_execution_feedback(&db, "t1", false, now + 2).await.expect("失败反馈");
        let t = get_by_id(&db, "t1").await.expect("查询").expect("存在");
        assert_eq!(t.usage_count, 2);
        assert!((t.success_rate - 0.5).abs() < 1e-9, "成功率应为 0.5, 实际 {}", t.success_rate);
    }

    #[tokio::test]
    async fn unique_constraint_enforced() {
        let db = setup().await;
        let now = 1_700_000_000;
        upsert(
            &db,
            "t1",
            "wf1",
            "same",
            TYPE_RHAI_SCRIPT,
            None,
            None,
            None,
            "manual",
            STATUS_ACTIVE,
            now,
        )
        .await
        .expect("插入");
        // 不同 id 但同 (workflow_id, tool_name) → 走覆盖路径不报错
        upsert(
            &db,
            "t2",
            "wf1",
            "same",
            TYPE_RHAI_SCRIPT,
            None,
            None,
            None,
            "manual",
            STATUS_PENDING,
            now,
        )
        .await
        .expect("冲突覆盖");
        let all = list_by_workflow(&db, "wf1", None).await.expect("查询");
        assert_eq!(all.len(), 1, "同工作流内工具名应唯一");
        // 跨工作流同工具名允许
        upsert(
            &db,
            "t3",
            "wf2",
            "same",
            TYPE_RHAI_SCRIPT,
            None,
            None,
            None,
            "manual",
            STATUS_ACTIVE,
            now,
        )
        .await
        .expect("跨工作流插入");
        let all2 = list_by_workflow(&db, "wf2", None).await.expect("查询");
        assert_eq!(all2.len(), 1);
    }

    #[tokio::test]
    async fn delete_and_cascade() {
        let db = setup().await;
        let now = 1_700_000_000;
        upsert(
            &db,
            "t1",
            "wf1",
            "a",
            TYPE_RHAI_SCRIPT,
            None,
            None,
            None,
            "manual",
            STATUS_ACTIVE,
            now,
        )
        .await
        .expect("插入");
        upsert(
            &db,
            "t2",
            "wf1",
            "b",
            TYPE_RHAI_SCRIPT,
            None,
            None,
            None,
            "manual",
            STATUS_ACTIVE,
            now,
        )
        .await
        .expect("插入");

        assert!(delete(&db, "t1").await.expect("删除"));
        let n = delete_by_workflow(&db, "wf1").await.expect("级联删除");
        assert_eq!(n, 1);
        assert!(list_by_workflow(&db, "wf1", None).await.expect("查询").is_empty());
    }
}
