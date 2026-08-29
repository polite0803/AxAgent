// SPDX-License-Identifier: AGPL-3.0-only

//! 能力护照执行统计 repository —— 反馈闭环的持久化层。
//!
//! # 职责
//! - 写：接线点（rt-workflow 引擎 / skill 执行 / agent 执行）调 [`record_execution`]，
//!   维护 total_calls / success_count / 近 N 次成功率窗口 / 平均耗时。
//! - 读：`CapabilityIndexerImpl` 在返回护照前调 [`merge_stats_into_passport`]
//!   （或 [`list_all`] 批量合并），把 DB 统计写进 `CapabilityPassportDto.stats`。
//!
//! # 近 N 次成功率
//! `recent_window` 存 JSON 数组（[0/1,...]，窗口大小 [`RECENT_WINDOW_SIZE`]=5），
//! 成功率 = 窗口内 1 的个数 / 窗口长度，与护照 `CapabilityStats.recent_success_rate`
//! （"近 5 次成功率"）语义对齐。

use sea_orm::*;

use axagent_entities::capability_stats;
use axagent_harness::capability::CapabilityStats;
use axagent_harness::core_error::Result;
use axagent_harness::util_fns::now_ts;

/// 近 N 次成功率窗口大小（与护照 CapabilityStats.recent_success_rate 语义对齐）
pub const RECENT_WINDOW_SIZE: usize = 5;

/// 记录一次能力执行结果。
///
/// 幂等安全：capability_id 不存在时创建行，存在时更新聚合字段。
/// `success=true` 时 recent_window 尾部追加 1，否则追加 0；窗口超长裁掉最旧。
#[allow(clippy::too_many_arguments)]
pub async fn record_execution(
    db: &DatabaseConnection,
    capability_id: &str,
    success: bool,
    duration_ms: u64,
) -> Result<()> {
    let now = now_ts();

    let existing = capability_stats::Entity::find_by_id(capability_id).one(db).await?;

    let (total_calls, success_count, window, avg_duration_ms, last_executed_at) =
        if let Some(row) = existing {
            let mut w: Vec<i32> = serde_json::from_str(&row.recent_window).unwrap_or_default();
            w.push(if success { 1 } else { 0 });
            while w.len() > RECENT_WINDOW_SIZE {
                w.remove(0);
            }
            let new_total = row.total_calls + 1;
            let new_success = row.success_count + if success { 1 } else { 0 };
            let new_avg = if row.total_calls == 0 {
                duration_ms as i64
            } else {
                (row.avg_duration_ms * row.total_calls + duration_ms as i64)
                    / row.total_calls.saturating_add(1)
            };
            (new_total, new_success, w, new_avg, Some(now))
        } else {
            (
                1_i64,
                if success { 1_i64 } else { 0_i64 },
                vec![if success { 1 } else { 0 }],
                duration_ms as i64,
                Some(now),
            )
        };

    let am = capability_stats::ActiveModel {
        capability_id: Set(capability_id.to_string()),
        total_calls: Set(total_calls),
        success_count: Set(success_count),
        recent_window: Set(serde_json::to_string(&window).unwrap_or_else(|_| "[]".to_string())),
        avg_duration_ms: Set(avg_duration_ms),
        last_executed_at: Set(last_executed_at),
        updated_at: Set(now),
    };

    // UPSERT：主键冲突时更新聚合字段
    let _ = capability_stats::Entity::insert(am.clone())
        .on_conflict(
            sea_query::OnConflict::column(capability_stats::Column::CapabilityId)
                .update_columns([
                    capability_stats::Column::TotalCalls,
                    capability_stats::Column::SuccessCount,
                    capability_stats::Column::RecentWindow,
                    capability_stats::Column::AvgDurationMs,
                    capability_stats::Column::LastExecutedAt,
                    capability_stats::Column::UpdatedAt,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

/// 读取单条能力统计（能力存在但从未执行时返回 None）。
pub async fn get_stats(
    db: &DatabaseConnection,
    capability_id: &str,
) -> Result<Option<CapabilityStats>> {
    let row = capability_stats::Entity::find_by_id(capability_id).one(db).await?;
    Ok(row.map(stats_from_row))
}

/// 批量读取全部能力统计（启动合并 / 检索前批量合并用）。
pub async fn list_all(db: &DatabaseConnection) -> Result<Vec<(String, CapabilityStats)>> {
    let rows = capability_stats::Entity::find().all(db).await?;
    Ok(rows.into_iter().map(|r| (r.capability_id.clone(), stats_from_row(r))).collect())
}

/// 把 DB 统计合并进护照（就地修改 `passport.stats`）。
///
/// 合并规则：DB 有数据时以 DB 为准（total_calls / success_count / recent_success_rate /
/// avg_duration_seconds）；DB 无数据时保留护照原值（保持零值语义，探索提权生效）。
pub fn merge_stats_into_passport(
    passport: &mut axagent_harness::CapabilityPassportDto,
    stats: Option<&CapabilityStats>,
) {
    if let Some(s) = stats {
        if s.total_calls > 0 {
            passport.stats = s.clone();
            // avg_duration_seconds 与护照顶层字段同步（排序器 γ 维度消费顶层字段）
            if s.avg_duration_seconds > 0.0 {
                passport.avg_duration_seconds = Some(s.avg_duration_seconds);
            }
        }
    }
}

fn stats_from_row(row: capability_stats::Model) -> CapabilityStats {
    let window: Vec<i32> = serde_json::from_str(&row.recent_window).unwrap_or_default();
    let recent_success_rate = if window.is_empty() {
        0.0
    } else {
        window.iter().filter(|&&v| v == 1).count() as f64 / window.len() as f64
    };
    CapabilityStats {
        total_calls: std::cmp::max(row.total_calls, 0) as u64,
        success_count: std::cmp::max(row.success_count, 0) as u64,
        avg_duration_seconds: (std::cmp::max(row.avg_duration_ms, 0) as f64) / 1000.0,
        recent_success_rate,
        circuit_state: "closed".to_string(),
    }
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
    async fn record_execution_upserts_and_accumulates() {
        let db = setup().await;
        record_execution(&db, "workflow:test", true, 100).await.expect("首次记录应成功");
        record_execution(&db, "workflow:test", true, 200).await.expect("二次记录应成功");
        record_execution(&db, "workflow:test", false, 300).await.expect("三次记录应成功");

        let s = get_stats(&db, "workflow:test").await.expect("读取应成功").expect("应有记录");
        assert_eq!(s.total_calls, 3);
        assert_eq!(s.success_count, 2);
        // 窗口 [1,1,0] → 近 3 次成功率 2/3
        assert!((s.recent_success_rate - 2.0 / 3.0).abs() < 1e-6);
        // 平均耗时 (100+200+300)/3 = 200ms
        assert!((s.avg_duration_seconds - 0.2).abs() < 1e-6);
    }

    #[tokio::test]
    async fn recent_window_caps_at_five() {
        let db = setup().await;
        for i in 0..8 {
            record_execution(&db, "skill:test", i % 2 == 0, 50).await.expect("记录应成功");
        }
        let s = get_stats(&db, "skill:test").await.expect("读取应成功").expect("应有记录");
        assert_eq!(s.total_calls, 8);
        // 窗口只保留最近 5 次（i=3..7: 结果 [0,1,0,1,0] → 成功率 2/5）
        assert!((s.recent_success_rate - 2.0 / 5.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn merge_stats_into_passport_overrides_zero() {
        let db = setup().await;
        record_execution(&db, "tool:read_file", true, 120).await.expect("记录应成功");
        let all = list_all(&db).await.expect("列表应成功");
        let mut passport = axagent_harness::CapabilityPassportDto::default();
        passport.capability_id = "tool:read_file".to_string();
        merge_stats_into_passport(
            &mut passport,
            all.iter().find(|(id, _)| id == "tool:read_file").map(|(_, s)| s),
        );
        assert_eq!(passport.stats.total_calls, 1);
        assert!(passport.stats.recent_success_rate > 0.0);
        assert_eq!(passport.avg_duration_seconds, Some(0.12));
    }
}
