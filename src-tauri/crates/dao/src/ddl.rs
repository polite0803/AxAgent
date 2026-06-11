//! Backward-compat shim — delegates to `migrations::run_migrations`.
//!
//! 历史背景：v001 之前 dao crate 启动时跑这个函数 DROP `seaql_migrations` +
//! 全量 DDL，每次启动都重建。Phase 2 Task 2.1 引入 versioned migration
//! 框架（`migrations` 模块），不再有"全量重建"语义。
//!
//! 为避免破坏所有现有 call sites（`db::create_pool` / 测试 / 第三方
//! 集成），这里保留同名函数但只做转发。内部 DROP `seaql_migrations`
//! 行为已删除（无 seaql 依赖）；所有 schema 演进都走 `migrations`。
//!
//! 注：原签名 `&impl ConnectionTrait` 已变更为 `&DatabaseConnection`，
//! 因为 `migrations::run_migrations` 的内部 `up()` 函数需要
//! `&DatabaseConnection`（不可能把 `&impl ConnectionTrait` upcast 成
//! `&DatabaseConnection`——sea_orm 没有提供 back-reference）。所有 call
//! sites 传的都是 `&DatabaseConnection`，所以这个变化不破坏兼容性。

use sea_orm::DatabaseConnection;
use sea_orm::DbErr;

/// 旧 API：执行所有数据表 DDL（幂等，适用新/旧数据库）。
///
/// 实际行为已变更为：补跑所有未应用的 schema 迁移。多次调用幂等，
/// 重启安全。
pub async fn run_initialization(db: &DatabaseConnection) -> Result<(), DbErr> {
    crate::migrations::run_migrations(db).await
}
