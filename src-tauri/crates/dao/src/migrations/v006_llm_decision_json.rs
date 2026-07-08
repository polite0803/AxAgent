//! v006 — `stock_analyses` 加 `llm_decision_json` 列
//!
//! 用途:启用"方案 D — 双向并存"后,LLM 决策节点 `llm-decision-maker` 跑完
//! 把自由发挥的决策 JSON 独立存档到这一列,前端可展示"LLM 视角 vs 公式视角"
//! 对比面板。`decision_json` 仍以 portfolio-mgr 公式决策为主。
//!
//! 与 `node_results_snapshot` 的区别:
//!   - `node_results_snapshot` 存整个工作流的 NodeOutput 字典
//!   - `llm_decision_json` 只存 LLM 决策节点的精简 JSON(action / positionPct /
//!     confidence / reasoning),便于前端直接渲染 + 计算 agreement_score
//!
//! 字段类型 TEXT:与 `decision_json` / `node_results_snapshot` 保持一致,
//! 写入路径用 `serde_json::to_string`,读取路径用 `serde_json::from_str`。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute_unprepared("ALTER TABLE stock_analyses ADD COLUMN llm_decision_json TEXT")
        .await?;
    Ok(())
}
