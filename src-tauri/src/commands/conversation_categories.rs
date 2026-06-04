use crate::AppState;
use axagent_harness::types::*;
use tauri::State;

#[tauri::command]
pub async fn list_conversation_categories(
    state: State<'_, AppState>,
) -> Result<Vec<ConversationCategory>, String> {
    axagent_core::repo::conversation_category::list_conversation_categories(state.harness.db())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_conversation_category(
    state: State<'_, AppState>,
    input: CreateConversationCategoryInput,
) -> Result<ConversationCategory, String> {
    axagent_core::repo::conversation_category::create_conversation_category(
        state.harness.db(),
        input,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_conversation_category(
    state: State<'_, AppState>,
    id: String,
    input: UpdateConversationCategoryInput,
) -> Result<ConversationCategory, String> {
    axagent_core::repo::conversation_category::update_conversation_category(
        state.harness.db(),
        &id,
        input,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_conversation_category(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    axagent_core::repo::conversation_category::delete_conversation_category(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reorder_conversation_categories(
    state: State<'_, AppState>,
    category_ids: Vec<String>,
) -> Result<(), String> {
    axagent_core::repo::conversation_category::reorder_conversation_categories(
        state.harness.db(),
        &category_ids,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_conversation_category_collapsed(
    state: State<'_, AppState>,
    id: String,
    collapsed: bool,
) -> Result<(), String> {
    axagent_core::repo::conversation_category::set_conversation_category_collapsed(
        state.harness.db(),
        &id,
        collapsed,
    )
    .await
    .map_err(|e| e.to_string())
}
