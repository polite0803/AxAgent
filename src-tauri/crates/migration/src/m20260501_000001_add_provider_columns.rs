use sea_orm_migration::prelude::*;

/// 补齐 providers 表中 entity 已定义但迁移中缺失的列：
/// - custom_headers (v1.2 新增, 前端配置自定义 HTTP 头)
/// - icon (v1.2 新增, 提供商图标)
/// - builtin_id (v1.2 新增, 内置提供商标识)
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260501_000001_add_provider_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 custom_headers 列
        if !manager
            .has_column("providers", "custom_headers")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(Providers::Table)
                        .add_column_if_not_exists(
                            ColumnDef::new(Alias::new("custom_headers")).string().null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 添加 icon 列
        if !manager.has_column("providers", "icon").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Providers::Table)
                        .add_column_if_not_exists(
                            ColumnDef::new(Alias::new("icon")).string().null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        // 添加 builtin_id 列
        if !manager.has_column("providers", "builtin_id").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(Providers::Table)
                        .add_column_if_not_exists(
                            ColumnDef::new(Alias::new("builtin_id")).string().null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Providers::Table)
                    .drop_column(Alias::new("custom_headers"))
                    .drop_column(Alias::new("icon"))
                    .drop_column(Alias::new("builtin_id"))
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

/// 使用原始表名引用以避免引用原始迁移中定义的 entity
#[derive(Iden)]
enum Providers {
    Table,
}
