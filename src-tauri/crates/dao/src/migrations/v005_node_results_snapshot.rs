//! v005 — stock_analyses 加 `node_results_snapshot` 列
//!
//! 用途:工作流执行结束时,把 `engine.results` 完整序列化存到这一列。
//! `rerun_decision_only` 命令读这一列反序列化为 `HashMap<node_id,
//! CachedNodeResult>`,灌入 `RunOptions.cached_results`,引擎主循环
//! 命中缓存的节点直接标 Completed + 跳过 execute,只重跑 from 节点起的子图。
//!
//! 为什么不复用 `blackboard_snapshot`?后者只存 `params.{node_id}` 字段
//! (AgentNode 的 .params 或 CodeNode 的 .result),不是完整 NodeOutput,
//! 下游节点从 state.variables 拿不到 .result / .params 全部字段。
//!
//! 而 `node_results_snapshot` 直接存完整 `serde_json::Value` per node_id,
//! 配合 CachedNodeResult.output_var 还原原节点的输出语义。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared("ALTER TABLE stock_analyses ADD COLUMN node_results_snapshot TEXT")
        .await?;
    Ok(())
}
