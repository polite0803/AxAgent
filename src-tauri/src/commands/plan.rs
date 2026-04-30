//! Plan commands — implements the "plan first, then execute" agent work strategy.
//!
//! ## Flow
//! 1. Frontend sends user message → `plan_generate` generates a structured plan via LLM
//! 2. Plan is emitted as `plan-generated` event → frontend renders PlanCard
//! 3. User approves → `plan_execute` runs each step using the agent infrastructure
//! 4. Step updates are emitted as `plan-step-update` events
//! 5. Final result emitted as `plan-execution-complete` event

use crate::app_state::AppState;
use axagent_core::types::{Conversation, Message};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use uuid::Uuid;

// ── Request / Response types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PlanGenerateRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanExecuteRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "planId")]
    pub plan_id: String,
    #[serde(rename = "stepIds")]
    pub step_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct PlanCancelRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "planId")]
    pub plan_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PlanGetRequest {
    #[serde(rename = "planId")]
    pub plan_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PlanListRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "includeCompleted")]
    pub include_completed: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PlanModifyStepRequest {
    #[serde(rename = "planId")]
    pub plan_id: String,
    #[serde(rename = "stepId")]
    pub step_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub approved: Option<bool>,
}

// ── Plan data types (mirrors frontend Plan/PlanStep) ──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: PlanStepStatus,
    #[serde(rename = "estimatedTools", skip_serializing_if = "Option::is_none")]
    pub estimated_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlanStepStatus {
    Pending,
    Approved,
    Rejected,
    Running,
    Completed,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "userMessageId")]
    pub user_message_id: String,
    pub title: String,
    pub steps: Vec<PlanStep>,
    pub status: PlanStatus,
    #[serde(rename = "isActive")]
    pub is_active: bool,
    #[serde(rename = "createdUnderStrategy", skip_serializing_if = "Option::is_none")]
    pub created_under_strategy: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "updatedAt")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PlanStatus {
    Draft,
    Reviewing,
    Approved,
    Executing,
    Completed,
    Cancelled,
}

// ── Event payloads ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct PlanGeneratedEvent {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub plan: Plan,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanStepUpdateEvent {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "planId")]
    pub plan_id: String,
    #[serde(rename = "stepId")]
    pub step_id: String,
    pub status: PlanStepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanExecutionCompleteEvent {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "planId")]
    pub plan_id: String,
    pub status: String, // "completed" | "cancelled"
}

// ── Helper: Create a simple plan from user input ──────────────────────

/// Generate a structured plan from user input.
/// TODO: Replace with LLM-based plan generation using the agent's model.
async fn generate_plan_from_input(
    conversation_id: &str,
    content: &str,
    user_message_id: &str,
) -> Plan {
    let now = chrono::Utc::now().timestamp_millis();

    // Simple heuristic plan generation — real implementation calls LLM
    let steps = if content.contains("implement") || content.contains("实现") {
        vec![
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Analyze requirements".to_string(),
                description: "Understand the task scope and identify key components".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec![]),
                result: None,
            },
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Design solution".to_string(),
                description: "Plan the architecture and component structure".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec![]),
                result: None,
            },
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Implement changes".to_string(),
                description: "Write the actual code changes".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec!["Read", "Write", "Edit", "Bash"].into_iter().map(String::from).collect()),
                result: None,
            },
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Verify & test".to_string(),
                description: "Run tests and verify the implementation works correctly".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec!["Bash"].into_iter().map(String::from).collect()),
                result: None,
            },
        ]
    } else if content.contains("fix") || content.contains("修复") || content.contains("bug") {
        vec![
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Reproduce the issue".to_string(),
                description: "Understand and reproduce the bug".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec![]),
                result: None,
            },
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Identify root cause".to_string(),
                description: "Trace the bug to its source".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec!["Read", "Grep"].into_iter().map(String::from).collect()),
                result: None,
            },
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Apply fix".to_string(),
                description: "Implement the fix and verify".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec!["Edit", "Bash"].into_iter().map(String::from).collect()),
                result: None,
            },
        ]
    } else {
        vec![
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Research".to_string(),
                description: "Gather relevant information and context".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec!["WebSearch", "Read"].into_iter().map(String::from).collect()),
                result: None,
            },
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Execute".to_string(),
                description: "Perform the requested task".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec![]),
                result: None,
            },
            PlanStep {
                id: Uuid::new_v4().to_string(),
                title: "Deliver".to_string(),
                description: "Present results and summary".to_string(),
                status: PlanStepStatus::Pending,
                estimated_tools: Some(vec![]),
                result: None,
            },
        ]
    };

    let title = if content.len() > 60 {
        format!("{}...", &content[..60])
    } else {
        content.to_string()
    };

    Plan {
        id: Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        user_message_id: user_message_id.to_string(),
        title,
        steps,
        status: PlanStatus::Reviewing,
        is_active: true,
        created_under_strategy: Some("plan".to_string()),
        created_at: now,
        updated_at: now,
    }
}

// ── Tauri Commands ────────────────────────────────────────────────────

/// Generate a structured execution plan from the user's message.
/// The plan is emitted as a `plan-generated` event for the frontend to display.
#[tauri::command]
pub async fn plan_generate(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    request: PlanGenerateRequest,
) -> Result<Plan, String> {
    let db = &state.db;

    // Get the conversation to verify it exists
    let _conversation = axagent_core::repo::conversation::get_by_id(db, &request.conversation_id)
        .await
        .map_err(|e| format!("Conversation not found: {}", e))?;

    // Find the user's message
    let messages = axagent_core::repo::message::list_by_conversation_id(
        db,
        &request.conversation_id,
        None,
        Some(1),
    )
    .await
    .map_err(|e| format!("Failed to get messages: {}", e))?;

    let user_message_id = messages
        .first()
        .map(|m| m.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Generate the plan
    let plan = generate_plan_from_input(&request.conversation_id, &request.content, &user_message_id).await;

    // TODO: Persist plan to database (plans table migration needed)

    // Emit plan-generated event
    let _ = app.emit("plan-generated", PlanGeneratedEvent {
        conversation_id: request.conversation_id.clone(),
        plan: plan.clone(),
    });

    Ok(plan)
}

/// Execute an approved plan — runs each step sequentially using the agent infrastructure.
#[tauri::command]
pub async fn plan_execute(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    request: PlanExecuteRequest,
) -> Result<(), String> {
    // TODO: Load plan from database
    // TODO: For each approved step:
    //   1. Emit plan-step-update (status=running)
    //   2. Execute the step using agent tools
    //   3. Emit plan-step-update (status=completed/error)
    //   4. Update plan in database
    // TODO: Emit plan-execution-complete

    let _ = &state;

    // Emit completion event
    let _ = app.emit("plan-execution-complete", PlanExecutionCompleteEvent {
        conversation_id: request.conversation_id,
        plan_id: request.plan_id,
        status: "completed".to_string(),
    });

    Ok(())
}

/// Cancel a plan (reviewing or executing).
#[tauri::command]
pub async fn plan_cancel(
    app: tauri::AppHandle,
    request: PlanCancelRequest,
) -> Result<(), String> {
    let _ = app.emit("plan-execution-complete", PlanExecutionCompleteEvent {
        conversation_id: request.conversation_id,
        plan_id: request.plan_id,
        status: "cancelled".to_string(),
    });

    Ok(())
}

/// Get a plan by ID.
#[tauri::command]
pub async fn plan_get(
    request: PlanGetRequest,
) -> Result<Option<Plan>, String> {
    // TODO: Load from database
    // For now, return None since we don't persist plans yet
    let _ = request;
    Ok(None)
}

/// List plans for a conversation.
#[tauri::command]
pub async fn plan_list(
    request: PlanListRequest,
) -> Result<Vec<Plan>, String> {
    // TODO: Load from database
    let _ = request;
    Ok(vec![])
}

/// Modify a step in a plan (approve, reject, edit).
#[tauri::command]
pub async fn plan_modify_step(
    request: PlanModifyStepRequest,
) -> Result<Option<Plan>, String> {
    // TODO: Load plan from DB, modify step, save back
    let _ = request;
    Ok(None)
}
