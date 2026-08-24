use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::stock_workflow as wf_err;
use axagent_agent_macro::agent_command;
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

#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "列出荐股历史记录")]
#[tauri::command]
pub async fn list_reco_history(
    state: State<'_, AppState>,
    style_filter: Option<String>,
    exclude_styles: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<RecoHistoryItem>, String> {
    use sea_orm::{ConnectionTrait, Statement};
    let db = state.harness.db();
    // 2026-07-31 修复：DB 已切 PostgreSQL（db_config.json db_type=postgres），
    // 原 SQLite 方言 SQL（GROUP_CONCAT + `?` 占位符）在 PG 上必然报错
    // （function group_concat does not exist）→ 前端 catch 静默 → 历史记录永远为空。
    // 按 backend 分支：PG 用 string_agg + $N；SQLite 保持原样。
    // 2026-08-01 修复：GROUP BY 必须包含非聚合列 period——
    // PG 强制 "SELECT 非聚合列必须出现在 GROUP BY"，SQLite 宽松不报（编译测不出，
    // 运行时报错被前端 catch 静默 → 历史列表仍为空）。一次执行只对应一个 period，
    // 加 period 到 GROUP BY 不会拆分分组。
    let is_pg = db.get_database_backend() == sea_orm::DbBackend::Postgres;
    tracing::info!(
        "[list_reco_history] backend={:?} style_filter={:?} exclude_styles={:?} limit={:?} offset={:?}",
        db.get_database_backend(),
        style_filter,
        exclude_styles,
        limit,
        offset
    );

    let mut sql = if is_pg {
        String::from(
            "SELECT generated_at, period, COUNT(*) as stock_count, \
             STRING_AGG(DISTINCT style, ',') as styles, MAX(created_at) as created_at \
             FROM reco_picks WHERE 1=1",
        )
    } else {
        String::from(
            "SELECT generated_at, period, COUNT(*) as stock_count, \
             GROUP_CONCAT(DISTINCT style) as styles, MAX(created_at) as created_at \
             FROM reco_picks WHERE 1=1",
        )
    };
    let mut values: Vec<sea_orm::Value> = Vec::new();

    // style_filter 支持单个或多个（逗号分隔）——趋势智选面板同时认
    // 'serenity'（serenity-screening 工作流产物）和 'bottleneck'
    // （智能荐股内置 SerenityStrategy 产物，业务上都是"趋势智选"）。
    // 注意：占位符编号必须用"已 push 的 values 数量"作基准（base），
    // 不能边循环边用 values.len()——values 在循环后才 push，
    // 旧实现生成 ($1,$1) 导致只匹配第一个风格（2026-08-01 实锤修复）。
    if let Some(ref style) = style_filter {
        let styles: Vec<String> =
            style.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if !styles.is_empty() {
            let base = values.len();
            sql.push_str(" AND style IN (");
            for (i, _) in styles.iter().enumerate() {
                if i > 0 {
                    sql.push(',');
                }
                if is_pg {
                    sql.push_str(&format!("${}", base + i + 1));
                } else {
                    sql.push('?');
                }
            }
            sql.push(')');
            for s in styles {
                values.push(s.into());
            }
        }
    }

    // exclude_styles 同样支持逗号分隔多值（NOT IN）——
    // 智能荐股历史用 exclude_styles="serenity,bottleneck" 排除趋势智选专属产出，
    // 与趋势智选面板的 style_filter="serenity,bottleneck" 互为镜像，两个面板不重复。
    // 占位符基准同样用已 push 的 values.len()（修复 ($1,$1) bug）。
    if let Some(ref style) = exclude_styles {
        let styles: Vec<String> =
            style.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if !styles.is_empty() {
            let base = values.len();
            sql.push_str(" AND style NOT IN (");
            for (i, _) in styles.iter().enumerate() {
                if i > 0 {
                    sql.push(',');
                }
                if is_pg {
                    sql.push_str(&format!("${}", base + i + 1));
                } else {
                    sql.push('?');
                }
            }
            sql.push(')');
            for s in styles {
                values.push(s.into());
            }
        }
    }

    sql.push_str(" GROUP BY generated_at, period ORDER BY generated_at DESC");

    if let Some(l) = limit {
        if is_pg {
            sql.push_str(&format!(" LIMIT ${}", values.len() + 1));
        } else {
            sql.push_str(" LIMIT ?");
        }
        values.push((l as i64).into());
    }
    if let Some(o) = offset {
        if is_pg {
            sql.push_str(&format!(" OFFSET ${}", values.len() + 1));
        } else {
            sql.push_str(" OFFSET ?");
        }
        values.push((o as i64).into());
    }

    let values_count = values.len();
    let backend = if is_pg {
        sea_orm::DbBackend::Postgres
    } else {
        sea_orm::DbBackend::Sqlite
    };
    let stmt = Statement::from_sql_and_values(backend, sql.as_str(), values);

    let rows = db.query_all_raw(stmt).await.map_err(|e| {
        ErrorResponse::new(wf_err::INTERNAL).with_detail(format!("查询荐股历史失败: {e}"))
    })?;

    let items = rows
        .iter()
        .map(|row| RecoHistoryItem {
            generated_at: row.try_get::<String>("", "generated_at").unwrap_or_default(),
            period: row.try_get::<String>("", "period").unwrap_or_default(),
            stock_count: row.try_get::<i64>("", "stock_count").unwrap_or(0),
            styles: row.try_get::<String>("", "styles").unwrap_or_default(),
            created_at: row.try_get::<String>("", "created_at").unwrap_or_default(),
        })
        .collect::<Vec<_>>();

    tracing::info!(
        "[list_reco_history] sql={:?} values_count={} items={}",
        sql,
        values_count,
        items.len()
    );

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

#[agent_command(domain = "finance", safety = Safe, call_mode = StateOnly, description =  "获取荐股详情")]
#[tauri::command]
pub async fn get_reco_detail(
    state: State<'_, AppState>,
    generated_at: String,
    style_filter: Option<String>,
    exclude_styles: Option<String>,
) -> Result<Vec<RecoDetailItem>, String> {
    use axagent_entities::reco_picks;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let db = state.harness.db();
    let mut query =
        reco_picks::Entity::find().filter(reco_picks::Column::GeneratedAt.eq(&generated_at));

    // style_filter 支持单个或多个（逗号分隔）——同上，保持 list/detail 过滤语义一致
    if let Some(ref style) = style_filter {
        let styles: Vec<String> =
            style.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if !styles.is_empty() {
            query = query.filter(reco_picks::Column::Style.is_in(styles));
        }
    }

    // exclude_styles 逗号分隔多值（NOT IN）——与 list 语义一致，智能荐股详情排除趋势智选专属风格
    if let Some(ref style) = exclude_styles {
        let styles: Vec<String> =
            style.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        if !styles.is_empty() {
            // ColumnTrait 自带 is_not_in（与上方 is_in 对应），无需 Expr
            query = query.filter(reco_picks::Column::Style.is_not_in(styles));
        }
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
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "批量删除荐股记录")]
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
#[agent_command(domain = "finance", safety = Caution, call_mode = StateOnly, description =  "删除Serenity候选记录")]
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
    axagent_analysis_engine::recommender::clear_serenity_candidate_cache();
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
