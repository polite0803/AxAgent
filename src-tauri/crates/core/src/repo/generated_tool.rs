use sea_orm::*;

use crate::entity::generated_tools;
use crate::error::Result;

pub async fn list_generated_tools(db: &DatabaseConnection) -> Result<Vec<generated_tools::Model>> {
    let tools = generated_tools::Entity::find()
        .order_by(generated_tools::Column::CreatedAt, Order::Desc)
        .all(db)
        .await?;
    Ok(tools)
}

pub async fn get_generated_tool_by_name(
    db: &DatabaseConnection,
    tool_name: &str,
) -> Result<Option<generated_tools::Model>> {
    let tool = generated_tools::Entity::find()
        .filter(generated_tools::Column::ToolName.eq(tool_name))
        .one(db)
        .await?;
    Ok(tool)
}

pub async fn insert_generated_tool(
    db: &DatabaseConnection,
    id: &str,
    tool_name: &str,
    original_name: &str,
    original_description: &str,
    input_schema: &str,
    output_schema: &str,
    implementation: &str,
    source_info: &str,
    created_at: i64,
) -> Result<()> {
    let model = generated_tools::ActiveModel {
        id: Set(id.to_string()),
        tool_name: Set(tool_name.to_string()),
        original_name: Set(original_name.to_string()),
        original_description: Set(original_description.to_string()),
        input_schema: Set(input_schema.to_string()),
        output_schema: Set(output_schema.to_string()),
        implementation: Set(implementation.to_string()),
        source_info: Set(source_info.to_string()),
        created_at: Set(created_at),
    };
    model.insert(db).await?;
    Ok(())
}

pub async fn delete_generated_tool(db: &DatabaseConnection, id: &str) -> Result<bool> {
    let tool = generated_tools::Entity::find_by_id(id).one(db).await?;
    if let Some(t) = tool {
        t.delete(db).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
