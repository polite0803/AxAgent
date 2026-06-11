//! axagent-dao — 数据访问层
//!
//! 包含数据库连接管理 (db)、SeaORM 仓库 (repo/) 和 DDL 操作 (ddl)。
//! 也包含跨多个 repo 的服务（如 marketplace_service）— 这些服务全部是
//! SeaORM 数据访问逻辑，留在 dao 层。

pub mod db;
pub mod ddl;
pub mod marketplace_service;
pub mod platform_adapter_impl;
pub mod repo;
pub mod search_sources_impl;
