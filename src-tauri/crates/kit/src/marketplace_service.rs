//! Re-export shim — 实际实现在 `axagent_dao::marketplace_service`。
//! 业务逻辑（SeaORM Entity/Column/ActiveModel）下沉到 dao 层后，
//! 上层调用方（gateway / core）只需保持 `axagent_kit::marketplace_service::*` 路径不变。

pub use axagent_dao::marketplace_service::*;
