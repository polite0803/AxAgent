use axagent_agent::tool_recommender::patterns::{UsagePattern, UsagePatternDB};
use axagent_agent::tool_recommender::{ContextAnalyzer, ToolRecommendation, ToolRecommender};
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct RecommendationResult {
    pub tools: Vec<ToolScoreInfo>,
    pub reasoning: String,
    pub confidence: f32,
    pub alternatives: Vec<AlternativeSetInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolScoreInfo {
    pub tool_id: String,
    pub tool_name: String,
    pub score: f32,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlternativeSetInfo {
    pub description: String,
    pub tools: Vec<String>,
    pub tradeoffs: Vec<String>,
}

fn task_context_to_result(recommendation: ToolRecommendation) -> RecommendationResult {
    RecommendationResult {
        tools: recommendation
            .tools
            .into_iter()
            .map(|t| ToolScoreInfo {
                tool_id: t.tool_id,
                tool_name: t.tool_name,
                score: t.score,
                reasons: t.reasons,
            })
            .collect(),
        reasoning: recommendation.reasoning,
        confidence: recommendation.confidence,
        alternatives: recommendation
            .alternatives
            .into_iter()
            .map(|a| AlternativeSetInfo {
                description: a.description,
                tools: a.tools,
                tradeoffs: a.tradeoffs,
            })
            .collect(),
    }
}

#[command]
pub fn analyze_task(task_description: String) -> Result<RecommendationResult, String> {
    let analyzer = ContextAnalyzer::new();
    let context = analyzer.analyze(&task_description);

    let recommender = ToolRecommender::new();
    let recommendation = recommender.recommend(&context);

    Ok(task_context_to_result(recommendation))
}

#[command]
pub fn get_tool_recommendations(
    task_description: String,
    _task_type: Option<String>,
) -> Result<RecommendationResult, String> {
    let analyzer = ContextAnalyzer::new();
    let context = analyzer.analyze(&task_description);

    let recommender = ToolRecommender::new();
    let recommendation = recommender.recommend(&context);

    Ok(task_context_to_result(recommendation))
}

#[command]
pub fn get_available_tools() -> Result<Vec<ToolInfo>, String> {
    let recommender = ToolRecommender::new();
    let tools: Vec<ToolInfo> = recommender
        .tool_index
        .tools
        .values()
        .map(|t| ToolInfo {
            id: t.id.0.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            categories: t.categories.clone(),
        })
        .collect();

    Ok(tools)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub categories: Vec<String>,
}

#[command]
pub fn get_tools_by_category(category: String) -> Result<Vec<ToolInfo>, String> {
    let recommender = ToolRecommender::new();
    let tools: Vec<ToolInfo> = recommender
        .tool_index
        .get_by_category(&category)
        .into_iter()
        .map(|t| ToolInfo {
            id: t.id.0.clone(),
            name: t.name.clone(),
            description: t.description.clone(),
            categories: t.categories.clone(),
        })
        .collect();

    Ok(tools)
}

#[command]
pub fn record_tool_usage(
    user_id: String,
    task_signature: String,
    tools_used: Vec<String>,
    success: bool,
    duration_ms: u64,
) -> Result<(), String> {
    let mut pattern_db = UsagePatternDB::new();

    let pattern = UsagePattern {
        pattern_id: uuid::Uuid::new_v4().to_string(),
        task_signature,
        tools_used,
        usage_count: 1,
        success_rate: if success { 1.0 } else { 0.0 },
        avg_duration_ms: duration_ms,
        last_used: chrono::Utc::now(),
    };

    pattern_db.add_pattern(&user_id, pattern);

    Ok(())
}
