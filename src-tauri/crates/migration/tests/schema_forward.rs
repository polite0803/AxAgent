// SPDX-License-Identifier: AGPL-3.0-only

//! 迁移 crate 前向校验：空库经 bootstrap（`run_initialization`）后，
//! 核心表必须存在且可被仓库层正常读写。同时验证无备份目录时 `list_backups` 安全返回空。
//!
//! 纯内存 SQLite，无外部依赖。`cargo test -p axagent-migration` 即可运行。

use axagent_dao::db::create_test_pool;
use axagent_dao::repo::conversation::list_conversations;
use axagent_dao::repo::credential_repo::list_credentials;
use axagent_dao::repo::memory::list_namespaces;
use axagent_dao::repo::provider::list_keys_for_provider;
use axagent_migration::list_backups;

#[tokio::test]
async fn forward_bootstrap_produces_usable_schema() {
    // create_test_pool 内部已执行 run_initialization（前向迁移/bootstrap）。
    let h = create_test_pool().await.unwrap();
    let db = &h.conn;

    // 若下列任一表不存在，对应查询会直接报错；能返回空 vec 即证明 schema 已就绪。
    let conversations = list_conversations(db).await.unwrap();
    assert!(conversations.is_empty(), "conversations 表应存在且为空");

    let credentials = list_credentials(db).await.unwrap();
    assert!(credentials.is_empty(), "credentials 表应存在且为空");

    let namespaces = list_namespaces(db).await.unwrap();
    assert!(namespaces.is_empty(), "memory_namespaces 表应存在且为空");

    let keys = list_keys_for_provider(db, "any-provider").await.unwrap();
    assert!(keys.is_empty(), "provider_keys 表应存在且为空");
}

#[test]
fn list_backups_safe_when_no_backups() {
    // 无备份目录时应安全返回空列表，而非崩溃。
    let backups = list_backups();
    assert!(backups.is_empty(), "无备份时 list_backups 应返回空");
}
