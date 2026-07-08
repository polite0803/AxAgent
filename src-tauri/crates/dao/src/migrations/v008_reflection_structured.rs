//! v008 — 反思流程结构化升级（借鉴 TradingAgents 反思机制）
//!
//! ## 背景
//!
//! 当前 `stock_reflections` 反思表只存 `actual_outcome`（自然语言字符串）
//! 和 `what_went_wrong` / `fix_for_future` 等 free-form 字段，无法支持：
//!
//! 1. **回测化反思**：TradingAgents 反思 prompt 必传 `raw_return` / `alpha_return`
//!    / `holding_days` / `benchmark_name` 四个结构化变量，让 LLM 在反思时直接
//!    引用"持仓 30 天跌 8%，相对沪深 300 超额 -2.1%"这样的硬数字。本项目反思
//!    只能传"30天跌8% → 失败"这样的自然语言，LLM 容易脑补。
//! 2. **短文本注入**：借鉴 TradingAgents 强制 2-4 句反思输出，本项目反思 PM
//!    节点无字符约束，输出可能 500+ 字。准备在 `verdict` / `lesson_summary`
//!    上加短约束。
//! 3. **检索性能**：`fetch_stock_lessons` 按 (ticker, created_at DESC) 拉近 3
//!    条反思做 `past_context` 注入，缺复合索引 → 全表扫。
//!
//! ## 改动
//!
//! 1. ALTER stock_reflections: 加 7 列（结构化 outcome + 短摘要）
//! 2. CREATE INDEX: (stock_code, created_at DESC) 复合索引
//! 3. 保留 `actual_outcome` 字段作为 fallback，新字段缺失时回退到自然语言
//!
//! ## Phase 2 预告
//!
//! F1 借鉴点要建 `reflection_lessons` 表存可执行规则，本 migration 不做
//! （F1 是单独阶段）。status='pending' / 'resolved' / 'expired' 枚举也
//! 由 Phase 2 B1 引入。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // 1. stock_reflections 结构化 outcome + 短摘要字段
    //    全部用 nullable，旧反思行不需 backfill。
    let alters = &[
        "ALTER TABLE stock_reflections ADD COLUMN raw_return REAL",
        "ALTER TABLE stock_reflections ADD COLUMN alpha_return REAL",
        "ALTER TABLE stock_reflections ADD COLUMN holding_days INTEGER",
        "ALTER TABLE stock_reflections ADD COLUMN benchmark_name TEXT",
        "ALTER TABLE stock_reflections ADD COLUMN verdict TEXT",
        "ALTER TABLE stock_reflections ADD COLUMN alpha_cited TEXT",
        "ALTER TABLE stock_reflections ADD COLUMN lesson_summary TEXT",
    ];
    for sql in alters {
        db.execute_unprepared(sql).await?;
    }

    // 2. (stock_code, created_at DESC) 复合索引
    //    支撑 A1 注入闭环: fetch_stock_lessons / fetch_sector_lessons 按
    //    ticker 拉近 90 天反思的最 N 条。SQLite 默认升序,显式 DESC 提升
    //    ORDER BY 性能。
    db.execute_unprepared(
        "CREATE INDEX IF NOT EXISTS idx_stock_reflections_ticker_created \
         ON stock_reflections(stock_code, created_at DESC)",
    )
    .await?;

    Ok(())
}
