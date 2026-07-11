use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use serde::Serialize;
use tauri::State;

/// 列表：荐股推荐历史记录（按 generated_at 分组，每条记录含时间/周期/股票数/风格列表）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoHistoryItem {
    pub generated_at: String,
    pub period: String,
    pub stock_count: i64,
    pub styles: String,
    pub created_at: String,
}

#[tauri::command]
pub async fn list_reco_history(
    state: State<'_, AppState>,
    style_filter: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<RecoHistoryItem>, String> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = state.harness.db();

    let mut sql = String::from(
        "SELECT generated_at, period, COUNT(*) as stock_count, \
         GROUP_CONCAT(DISTINCT style) as styles, MAX(created_at) as created_at \
         FROM reco_picks WHERE 1=1",
    );
    let mut values: Vec<sea_orm::Value> = Vec::new();

    if let Some(ref style) = style_filter {
        sql.push_str(" AND style = ?");
        values.push(style.clone().into());
    }

    sql.push_str(" GROUP BY generated_at ORDER BY generated_at DESC");

    if let Some(l) = limit {
        sql.push_str(" LIMIT ?");
        values.push((l as i64).into());
    }
    if let Some(o) = offset {
        sql.push_str(" OFFSET ?");
        values.push((o as i64).into());
    }

    let stmt = Statement::from_sql_and_values(sea_orm::DbBackend::Sqlite, sql.as_str(), values);

    let rows = db.query_all_raw(stmt).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股历史失败: {e}"))
    })?;

    let items = rows
        .iter()
        .map(|row| RecoHistoryItem {
            generated_at: row
                .try_get::<String>("", "generated_at")
                .unwrap_or_default(),
            period: row.try_get::<String>("", "period").unwrap_or_default(),
            stock_count: row.try_get::<i64>("", "stock_count").unwrap_or(0),
            styles: row.try_get::<String>("", "styles").unwrap_or_default(),
            created_at: row.try_get::<String>("", "created_at").unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    Ok(items)
}

/// 获取某次荐股/瓶颈掘金详情（按 generated_at 获取该轮所有推荐股票）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoDetailItem {
    pub id: String,
    pub generated_at: String,
    pub period: String,
    pub stock_code: String,
    pub stock_name: String,
    pub style: String,
    pub confidence: i32,
    pub synthetic: i32,
    pub seed_pool_json: Option<String>,
    pub pick_data: Option<String>,
    pub created_at: String,
}

#[tauri::command]
pub async fn get_reco_detail(
    state: State<'_, AppState>,
    generated_at: String,
    style_filter: Option<String>,
) -> Result<Vec<RecoDetailItem>, String> {
    use axagent_entities::reco_picks;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = state.harness.db();
    let mut query =
        reco_picks::Entity::find().filter(reco_picks::Column::GeneratedAt.eq(&generated_at));

    if let Some(ref style) = style_filter {
        query = query.filter(reco_picks::Column::Style.eq(style));
    }

    let items = query.all(db).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股详情失败: {e}"))
    })?;

    Ok(items
        .into_iter()
        .map(|m| RecoDetailItem {
            id: m.id,
            generated_at: m.generated_at,
            period: m.period,
            stock_code: m.stock_code,
            stock_name: m.stock_name,
            style: m.style,
            confidence: m.confidence,
            synthetic: m.synthetic,
            seed_pool_json: m.seed_pool_json,
            pick_data: m.pick_data,
            created_at: m.created_at,
        })
        .collect())
}

/// 批量删除荐股记录（按 generated_at 删除整轮推荐）
#[tauri::command]
pub async fn batch_delete_reco_history(
    state: State<'_, AppState>,
    generated_ats: Vec<String>,
) -> Result<(), String> {
    use axagent_entities::reco_picks;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = state.harness.db();
    for ts in &generated_ats {
        reco_picks::Entity::delete_many()
            .filter(reco_picks::Column::GeneratedAt.eq(ts))
            .exec(db)
            .await
            .map_err(|e| {
                ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("删除荐股记录失败: {e}"))
            })?;
    }
    Ok(())
}

/// 删除一条 Serenity 候选记录（回馈闭环中的删除操作）
#[tauri::command]
pub async fn delete_serenity_pick(state: State<'_, AppState>, id: String) -> Result<(), String> {
    use crate::commands::error::ErrorResponse;
    use axagent_entities::reco_picks;
    use sea_orm::{EntityTrait, ModelTrait};

    let db = state.harness.db();
    let pick = reco_picks::Entity::find_by_id(&id)
        .one(db)
        .await
        .map_err(|e| {
            ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询候选记录失败: {e}"))
        })?
        .ok_or_else(|| "候选记录不存在".to_string())?;
    pick.delete(db).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("删除候选记录失败: {e}"))
    })?;
    // 同步清空 Serenity 全局缓存，避免下次荐股仍包含已删除的候选
    axagent_stock_analysis::recommender::clear_serenity_candidate_cache();
    tracing::info!("[serenity] 已删除候选记录: {id}，Serenity 缓存已同步清空");
    Ok(())
}

// ── [D1 借鉴] 批量反思 (B1+B2 闭环) ──
//
// 借鉴 TradingAgents 反思机制: 持仓期到达时,自动批量 resolve 所有
// `status='pending'` 的 stock_reflections row,无需用户手动逐条触发。
//
// 流程:
//   1. 扫 stock_reflections where status='pending',按 created_at ASC 处理
//   2. 对每条 row:
//      - 读 stock_analyses by original_analysis_id
//      - 计算持仓期: today - as_of_date
//      - 若 today - as_of_date >= decision_expected_holding_days (默认 28):
//        调 run_reflection_workflow(reflection_id=Some(rid)) 走 B3 UPDATE 路径
//      - 否则 skip (持仓期未到)
//   3. [D2 借鉴] Resolved FIFO 清理: 删除 90 天前或超 1000 条的 completed row
//   4. 返回 { total_pending, resolved, failed, skipped_young, cleaned_up }
//
// 调用方:
//   - `CronExecutor` 每天 18:00 调一次(收市后批量反思)
//   - 前端调试按钮: 手动立即跑一轮
