// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_trajectory::ProactiveSuggestionType;
use axagent_trajectory::{
    ContextFeatures, ContextPredictor, PredictionResult as TrajectoryPredictionResult,
    ProactiveAssistant, ProactiveConfig as TrajProactiveConfig,
    ProactiveSuggestion as TrajProactiveSuggestion, SuggestionAction, SuggestionEngine,
    TaskPrefetcher,
};
use axagent_trajectory::{PrefetchResult, PrefetchResults, PrefetchType};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProactiveSuggestion {
    pub id: String,
    pub suggestion_type: String,
    pub title: String,
    pub description: String,
    pub action: serde_json::Value,
    pub priority: String,
    pub created_at: String,
    pub expires_at: String,
    pub accepted: Option<bool>,
}

impl From<&TrajProactiveSuggestion> for ProactiveSuggestion {
    fn from(s: &TrajProactiveSuggestion) -> Self {
        let action = match &s.action {
            SuggestionAction::PrefetchCompletion { language, context } => {
                serde_json::json!({ "type": "PrefetchCompletion", "language": language, "context": context })
            },
            SuggestionAction::ShowRefactorOptions { target } => {
                serde_json::json!({ "type": "ShowRefactorOptions", "target": target })
            },
            SuggestionAction::GenerateDocs { topic } => {
                serde_json::json!({ "type": "GenerateDocs", "topic": topic })
            },
            SuggestionAction::GenerateTests { target } => {
                serde_json::json!({ "type": "GenerateTests", "target": target })
            },
            SuggestionAction::ShowOptimizations { target } => {
                serde_json::json!({ "type": "ShowOptimizations", "target": target })
            },
            SuggestionAction::ShowLearningResources { topic } => {
                serde_json::json!({ "type": "ShowLearningResources", "topic": topic })
            },
            SuggestionAction::CausalInsight { from_entity, to_entity } => {
                serde_json::json!({ "type": "CausalInsight", "fromEntity": from_entity, "toEntity": to_entity })
            },
        };

        let suggestion_type = match s.suggestion_type {
            ProactiveSuggestionType::Completion => "Completion",
            ProactiveSuggestionType::Refactor => "Refactor",
            ProactiveSuggestionType::Documentation => "Documentation",
            ProactiveSuggestionType::Test => "Test",
            ProactiveSuggestionType::Optimization => "Optimization",
            ProactiveSuggestionType::Debug => "Debug",
            ProactiveSuggestionType::Learning => "Learning",
            ProactiveSuggestionType::CausalInsight => "CausalInsight",
        };

        let priority = match s.priority {
            axagent_trajectory::Priority::Low => "low",
            axagent_trajectory::Priority::Medium => "medium",
            axagent_trajectory::Priority::High => "high",
            axagent_trajectory::Priority::Critical => "critical",
        };

        Self {
            id: s.id.clone(),
            suggestion_type: suggestion_type.to_string(),
            title: s.title.clone(),
            description: s.description.clone(),
            action,
            priority: priority.to_string(),
            created_at: s.created_at.to_rfc3339(),
            expires_at: s.expires_at.to_rfc3339(),
            accepted: s.accepted,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextPrediction {
    pub predicted_intent: serde_json::Value,
    pub confidence: f32,
    pub reasoning: String,
    pub suggested_actions: Vec<serde_json::Value>,
    pub context_window: serde_json::Value,
    pub created_at: String,
}

impl From<&axagent_trajectory::ContextPrediction> for ContextPrediction {
    fn from(p: &axagent_trajectory::ContextPrediction) -> Self {
        let predicted_intent = match &p.predicted_intent {
            axagent_trajectory::PredictedIntent::CodeCompletion { language, context } => {
                serde_json::json!({ "type": "CodeCompletion", "language": language, "context": context })
            },
            axagent_trajectory::PredictedIntent::Documentation { topic } => {
                serde_json::json!({ "type": "Documentation", "topic": topic })
            },
            axagent_trajectory::PredictedIntent::Search { query_type } => {
                serde_json::json!({ "type": "Search", "queryType": query_type })
            },
            axagent_trajectory::PredictedIntent::Refactoring { target } => {
                serde_json::json!({ "type": "Refactoring", "target": target })
            },
            axagent_trajectory::PredictedIntent::Debug { error } => {
                serde_json::json!({ "type": "Debug", "error": error })
            },
            axagent_trajectory::PredictedIntent::TestGeneration { target } => {
                serde_json::json!({ "type": "TestGeneration", "target": target })
            },
            axagent_trajectory::PredictedIntent::Unknown => {
                serde_json::json!({ "type": "Unknown" })
            },
        };

        let suggested_actions: Vec<serde_json::Value> = p
            .suggested_actions
            .iter()
            .map(|a| {
                serde_json::json!({
                    "action_type": a.action_type,
                    "title": a.title,
                    "description": a.description,
                    "priority": match a.priority {
                        axagent_trajectory::Priority::Low => "low",
                        axagent_trajectory::Priority::Medium => "medium",
                        axagent_trajectory::Priority::High => "high",
                        axagent_trajectory::Priority::Critical => "critical",
                    }
                })
            })
            .collect();

        let context_window = serde_json::json!({
            "files": p.context_window.files,
            "recentActions": p.context_window.recent_actions,
            "currentLanguage": p.context_window.current_language,
            "projectType": p.context_window.project_type,
        });

        Self {
            predicted_intent,
            confidence: p.confidence,
            reasoning: p.reasoning.clone(),
            suggested_actions,
            context_window,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PredictionResult {
    pub predictions: Vec<ContextPrediction>,
}

impl From<TrajectoryPredictionResult> for PredictionResult {
    fn from(result: TrajectoryPredictionResult) -> Self {
        Self { predictions: result.predictions.iter().map(ContextPrediction::from).collect() }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProactiveConfig {
    pub enabled: bool,
    pub max_suggestions: i32,
    pub suggestion_ttl_minutes: i32,
    pub prediction_confidence_threshold: f32,
    pub prefetch_enabled: bool,
    pub reminder_enabled: bool,
}

impl From<&TrajProactiveConfig> for ProactiveConfig {
    fn from(c: &TrajProactiveConfig) -> Self {
        Self {
            enabled: c.enabled,
            max_suggestions: c.max_suggestions as i32,
            suggestion_ttl_minutes: c.suggestion_ttl_minutes as i32,
            prediction_confidence_threshold: c.prediction_confidence_threshold,
            prefetch_enabled: c.prefetch_enabled,
            reminder_enabled: c.reminder_enabled,
        }
    }
}

impl From<ProactiveConfig> for TrajProactiveConfig {
    fn from(c: ProactiveConfig) -> Self {
        Self {
            enabled: c.enabled,
            max_suggestions: c.max_suggestions as usize,
            suggestion_ttl_minutes: c.suggestion_ttl_minutes as i64,
            prediction_confidence_threshold: c.prediction_confidence_threshold,
            prefetch_enabled: c.prefetch_enabled,
            reminder_enabled: c.reminder_enabled,
        }
    }
}

pub struct ProactiveService {
    assistant: ProactiveAssistant,
    predictor: ContextPredictor,
    suggestion_engine: SuggestionEngine,
    prefetcher: TaskPrefetcher,
    /// 上次刷新的顶层预测意图（实体 ID + 时间戳 epoch ms），用于观测意图转移
    last_intent: Option<(String, i64)>,
}

/// 意图转移延迟超过该值时只记转移、不记延迟（对预取时机已无意义）
const MAX_INTENT_GAP_MS: i64 = 10 * 60 * 1000;

/// 每次刷新最多注入的因果建议数
const CAUSAL_SUGGESTIONS_PER_REFRESH: usize = 2;

/// 建议 priority → 显著度约定映射（真实数据 priority 的归一化，集中一处可审计）。
/// critical=1.0 / high=0.85 / medium=0.6 / low=0.35，未知值取 low。
fn priority_to_salience(priority: &str) -> f64 {
    match priority {
        "critical" => 1.0,
        "high" => 0.85,
        "medium" => 0.6,
        _ => 0.35,
    }
}

/// 建议类型 → 信号源：因果洞察建议来自因果边层，其余常规建议由预测引擎生成。
fn suggestion_source(suggestion_type: &str) -> axagent_trajectory::SignalSource {
    if suggestion_type == "CausalInsight" {
        axagent_trajectory::SignalSource::CausalInsight
    } else {
        axagent_trajectory::SignalSource::ContextPrediction
    }
}

/// 显著性仲裁排序（`saliency_enabled` 开启时调用）：
/// 建议转为信号参与竞争-广播，胜者按广播顺序排前，未胜出者保持原顺序附后（不丢弃）。
fn arbitrate_suggestions(
    mut suggestions: Vec<ProactiveSuggestion>,
    arbiter: &mut axagent_trajectory::SaliencyArbiter,
) -> Vec<ProactiveSuggestion> {
    let signals = suggestions
        .iter()
        .map(|s| {
            axagent_trajectory::SaliencySignal::new(
                suggestion_source(&s.suggestion_type),
                priority_to_salience(&s.priority),
                s.id.clone(),
            )
        })
        .collect();
    let packet = arbiter.arbitrate(signals);
    let winner_order: Vec<&str> =
        packet.winners.iter().map(|w| w.signal.origin_id.as_str()).collect();
    let mut ordered: Vec<ProactiveSuggestion> = Vec::with_capacity(suggestions.len());
    for wid in &winner_order {
        if let Some(pos) = suggestions.iter().position(|s| s.id == *wid) {
            ordered.push(suggestions.remove(pos));
        }
    }
    ordered.extend(suggestions);
    ordered
}

/// 觉知观测（R2：全部输入取真实运行时数据，取不到的传 None，不喂伪造值）。
/// 随建议刷新周期驱动；快照写入 memory_items 的 `__sys_awareness__` namespace。
async fn observe_awareness(state: &AppState) {
    let now = Utc::now();
    let query = axagent_trajectory::TrajectoryQuery {
        time_range: Some((now - chrono::Duration::minutes(10), now)),
        limit: Some(200),
        ..Default::default()
    };
    let trajectories =
        state.trajectory_storage.query_trajectories(&query).await.unwrap_or_default();
    let recent_event_count = trajectories.len();

    // 工具结果序列（trajectory steps 的 is_error 取反），取最近 20 个
    let mut tool_results: Vec<bool> = Vec::new();
    for t in &trajectories {
        for step in &t.steps {
            if let Some(results) = &step.tool_results {
                for r in results {
                    tool_results.push(!r.is_error);
                }
            }
        }
    }
    let window_start = tool_results.len().saturating_sub(20);
    let tool_window = &tool_results[window_start..];

    let active_sessions = state.agent_session_manager.session_count().await;

    // 上一次广播作为主导关注点输入（仲裁未启用时为 None，不虚构）
    let last_broadcast = state.memory.saliency_arbiter.lock().await.last_broadcast().cloned();

    let snapshot_json = {
        let mut monitor = state.memory.awareness_monitor.lock().await;
        let input = axagent_trajectory::AwarenessInput {
            recent_event_count,
            active_sessions,
            avg_context_ratio: None, // 暂无低成本的压缩比例聚合查询，宁缺毋假
            recent_tool_results: tool_window,
        };
        let frame = monitor.observe(input, last_broadcast.as_ref());
        let snapshot = monitor.should_snapshot(&frame).then(|| monitor.snapshot_content(&frame));
        if snapshot.is_some() {
            monitor.mark_snapshotted();
        }
        snapshot
    };

    if let Some(content) = snapshot_json {
        // 校准采样：因果边的 confidence（把握度）vs strength（实际命中率）。
        // 数据链 = knowledge_relations.properties，随边观测自动更新，不可编造。
        let calibrator_pairs = axagent_trajectory::list_causal_edge_stats(
            state.trajectory_storage.db(),
            axagent_dao::repo::knowledge_graph::TRAJECTORY_KB_ID,
        )
        .await
        .unwrap_or_default();
        let mut calibrator = axagent_trajectory::ConfidenceCalibrator::default();
        for stats in calibrator_pairs {
            calibrator.record(stats.confidence, stats.strength());
        }

        // 快照内容 = 帧数据 + 校准摘要（纯结构化 JSON，R3）
        let content = match calibrator.bias_summary() {
            Some(summary) => {
                let mut value: serde_json::Value =
                    serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}));
                value["calibration"] =
                    serde_json::to_value(&summary).unwrap_or(serde_json::Value::Null);
                value.to_string()
            },
            None => content,
        };

        let req = axagent_trajectory::AddMemoryRequest {
            target: "awareness".to_string(),
            content,
            tier: axagent_trajectory::MemoryTier::Working,
            importance: 0.3,
            nature: Default::default(),
            provenance: None,
            tags: vec!["awareness".to_string()],
            expires_at: None,
            namespace_id: Some(axagent_trajectory::AWARENESS_NAMESPACE.to_string()),
        };
        let service = state.memory_service.clone();
        tauri::async_runtime::spawn(async move {
            service.read().await.add_memory_advanced(req).await;
        });
    }
}

impl ProactiveService {
    pub fn new() -> Self {
        Self {
            assistant: ProactiveAssistant::new(),
            predictor: ContextPredictor::new(),
            suggestion_engine: SuggestionEngine::new(),
            prefetcher: TaskPrefetcher::new(),
            last_intent: None,
        }
    }

    pub fn get_suggestions(&self) -> Vec<ProactiveSuggestion> {
        self.assistant
            .get_active_suggestions()
            .iter()
            .map(|s| ProactiveSuggestion::from(*s))
            .collect()
    }

    pub async fn refresh_suggestions(
        &mut self,
        features: ContextFeatures,
        storage: Option<&axagent_trajectory::TrajectoryStorage>,
    ) -> Vec<ProactiveSuggestion> {
        if !self.assistant.is_enabled() {
            return vec![];
        }

        self.assistant.cleanup_expired();

        let prediction_result = self.predictor.predict(&features);

        // 因果层：意图转移观测 + 基于因果边的可解释建议。
        // storage 未提供或因果开关关闭时，两者均为空操作，行为与此前一致。
        if let Some(storage) = storage {
            self.observe_intent_transition(storage, &prediction_result).await;
            for prediction in &prediction_result.predictions {
                for suggestion in
                    storage.causal_suggestions(prediction, CAUSAL_SUGGESTIONS_PER_REFRESH).await
                {
                    self.assistant.add_suggestion(suggestion);
                }
            }
        }

        for prediction in &prediction_result.predictions {
            // 记录预测以支持模式分析和偏差修正
            self.assistant.record_prediction(prediction.clone());

            let new_suggestions =
                self.suggestion_engine.generate_suggestions(&features, prediction, None);
            for suggestion in new_suggestions {
                self.assistant.add_suggestion(suggestion);
            }
        }

        self.get_suggestions()
    }

    /// 观测相邻两次刷新间的顶层意图转移 `intent:A → intent:B`。
    ///
    /// 同一意图连续出现不成边；间隔超过 [`MAX_INTENT_GAP_MS`] 时只记转移不记延迟。
    async fn observe_intent_transition(
        &mut self,
        storage: &axagent_trajectory::TrajectoryStorage,
        result: &TrajectoryPredictionResult,
    ) {
        let curr = result
            .predictions
            .iter()
            .find(|p| !matches!(p.predicted_intent, axagent_trajectory::PredictedIntent::Unknown))
            .map(|p| axagent_trajectory::prediction_intent_entity(&p.predicted_intent));
        let Some(curr) = curr.filter(|e| !e.is_empty()) else {
            return;
        };

        let now_ms = Utc::now().timestamp_millis();
        if let Some((prev, prev_at)) = &self.last_intent {
            let delay_ms = now_ms - *prev_at;
            let delay = (delay_ms > 0 && delay_ms <= MAX_INTENT_GAP_MS).then_some(delay_ms);
            storage.observe_intent_transition(prev, &curr, delay).await;
        }
        self.last_intent = Some((curr, now_ms));
    }

    pub fn dismiss_suggestion(&mut self, id: &str) -> bool {
        self.assistant.dismiss_suggestion(id).is_some()
    }

    pub fn accept_suggestion(&mut self, id: &str) -> bool {
        self.assistant.accept_suggestion(id).is_some()
    }

    pub fn snooze_suggestion(&mut self, id: &str, duration_minutes: i64) -> bool {
        self.assistant.snooze_suggestion(id, duration_minutes).is_some()
    }

    pub fn predict(&self, context: ContextFeatures) -> PredictionResult {
        self.predictor.predict(&context).into()
    }

    pub fn get_config(&self) -> ProactiveConfig {
        ProactiveConfig::from(self.assistant.get_config())
    }

    pub fn update_config(&mut self, config: TrajProactiveConfig) {
        self.assistant.update_config(config);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.assistant.set_enabled(enabled);
    }

    pub fn is_enabled(&self) -> bool {
        self.assistant.is_enabled()
    }

    pub fn prefetch_for_predictions(
        &mut self,
        predictions: &[serde_json::Value],
    ) -> PrefetchResults {
        let mut results = PrefetchResults::new();

        for pred in predictions {
            let intent_type = pred
                .get("predicted_intent")
                .and_then(|v| v.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let id = pred
                .get("resource_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            let (prefetch_type, estimated_ms) = match intent_type {
                "CodeCompletion" => (PrefetchType::CodeCompletion, 200),
                "Search" => (PrefetchType::SearchResults, 200),
                "Documentation" => (PrefetchType::Documentation, 500),
                "Refactoring" => (PrefetchType::ContextAnalysis, 800),
                "TestGeneration" => (PrefetchType::ContextAnalysis, 600),
                "Debug" => (PrefetchType::ContextAnalysis, 300),
                _ => continue,
            };

            if let Some(cached) = self.prefetcher.get_cached(&id) {
                results.add(cached.clone());
                results.critical_path.push(id.clone());
                continue;
            }

            let result = PrefetchResult {
                prefetch_type,
                resource_id: id.clone(),
                data: None,
                ready: false,
                estimated_prepare_time_ms: estimated_ms,
                created_at: Utc::now(),
            };

            self.prefetcher.cache_result(result.clone());
            results.add(result);
            results.critical_path.push(id);
        }

        results
    }
}

impl Default for ProactiveService {
    fn default() -> Self {
        Self::new()
    }
}

#[agent_command(domain = proactive, safety = Safe, call_mode = StateOnly, description = "列出主动建议")]
#[tauri::command]
pub async fn proactive_list_suggestions(
    state: State<'_, AppState>,
) -> Result<Vec<ProactiveSuggestion>, String> {
    let service = state.proactive_service.read().await;
    Ok(service.get_suggestions())
}

/// 觉知摘要（只读，零副作用）：帧缓冲 + 因果边校准摘要 + 上次广播。
/// 帧缓冲为空 = 觉知观测尚未运行过，前端据此显示"暂无数据"。
#[agent_command(domain = proactive, safety = Safe, call_mode = StateOnly, description = "读取觉知摘要")]
#[tauri::command]
pub async fn proactive_awareness_summary(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let monitor = state.memory.awareness_monitor.lock().await;
    let frames: Vec<axagent_trajectory::AwarenessFrame> =
        monitor.frames().iter().rev().take(20).rev().cloned().collect();

    let calibration = if frames.is_empty() {
        None
    } else {
        let pairs = axagent_trajectory::list_causal_edge_stats(
            state.trajectory_storage.db(),
            axagent_dao::repo::knowledge_graph::TRAJECTORY_KB_ID,
        )
        .await
        .unwrap_or_default();
        let mut calibrator = axagent_trajectory::ConfidenceCalibrator::default();
        for stats in pairs {
            calibrator.record(stats.confidence, stats.strength());
        }
        calibrator.bias_summary()
    };

    let last_broadcast = state.memory.saliency_arbiter.lock().await.last_broadcast().cloned();

    serde_json::to_value(AwarenessSummaryPayload { frames, calibration, last_broadcast })
        .map_err(|e| format!("Failed to serialize awareness summary: {}", e))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AwarenessSummaryPayload {
    frames: Vec<axagent_trajectory::AwarenessFrame>,
    calibration: Option<axagent_trajectory::BiasSummary>,
    last_broadcast: Option<axagent_trajectory::BroadcastPacket>,
}

#[agent_command(domain = proactive, safety = Caution, call_mode = StateInput, description = "刷新主动建议")]
#[tauri::command]
pub async fn proactive_refresh_suggestions(
    state: State<'_, AppState>,
    context: serde_json::Value,
) -> Result<Vec<ProactiveSuggestion>, String> {
    let features: ContextFeatures = serde_json::from_value(context)
        .map_err(|e| format!("Failed to parse context features: {}", e))?;

    observe_awareness(&state).await;

    let mut service = state.proactive_service.write().await;
    let mut suggestions =
        service.refresh_suggestions(features, Some(state.trajectory_storage.as_ref())).await;

    // 显著性仲裁排序（默认关闭；关闭时行为与此前完全一致）
    if state.memory.saliency_enabled.load(std::sync::atomic::Ordering::Relaxed) {
        let mut arbiter = state.memory.saliency_arbiter.lock().await;
        suggestions = arbitrate_suggestions(suggestions, &mut arbiter);
    }

    Ok(suggestions)
}

#[agent_command(domain = proactive, safety = Safe, call_mode = StateInput, description = "预测上下文意图")]
#[tauri::command]
pub async fn proactive_predict(
    state: State<'_, AppState>,
    context: serde_json::Value,
) -> Result<PredictionResult, String> {
    let features: ContextFeatures = serde_json::from_value(context)
        .map_err(|e| format!("Failed to parse context features: {}", e))?;

    let service = state.proactive_service.read().await;
    Ok(service.predict(features))
}

#[agent_command(domain = proactive, safety = Dangerous, call_mode = StateInput, description = "丢弃主动建议")]
#[tauri::command]
pub async fn proactive_dismiss_suggestion(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.dismiss_suggestion(&id))
}

#[agent_command(domain = proactive, safety = Caution, call_mode = StateInput, description = "接受主动建议")]
#[tauri::command]
pub async fn proactive_accept_suggestion(
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.accept_suggestion(&id))
}

#[agent_command(domain = proactive, safety = Caution, call_mode = StateInput, description = "延后主动建议")]
#[tauri::command]
pub async fn proactive_snooze_suggestion(
    state: State<'_, AppState>,
    id: String,
    duration: i64,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.snooze_suggestion(&id, duration))
}

#[agent_command(domain = proactive, safety = Caution, call_mode = StateInput, description = "启用或停用主动服务")]
#[tauri::command]
pub async fn proactive_set_enabled(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    service.set_enabled(enabled);
    Ok(true)
}

#[agent_command(domain = proactive, safety = Caution, call_mode = StateInput, description = "更新主动服务配置")]
#[tauri::command]
pub async fn proactive_update_config(
    state: State<'_, AppState>,
    config: ProactiveConfig,
) -> Result<bool, String> {
    let mut service = state.proactive_service.write().await;
    service.update_config(config.into());
    Ok(true)
}

#[agent_command(domain = proactive, safety = Safe, call_mode = StateInput, description = "预取预测结果")]
#[tauri::command]
pub async fn proactive_prefetch(
    state: State<'_, AppState>,
    predictions: Vec<serde_json::Value>,
) -> Result<PrefetchResults, String> {
    let mut service = state.proactive_service.write().await;
    Ok(service.prefetch_for_predictions(&predictions))
}

#[agent_command(domain = proactive, safety = Safe, call_mode = StateInput, description = "列出学习洞察")]
#[tauri::command]
pub async fn list_insights(
    state: State<'_, AppState>,
    category: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<axagent_trajectory::LearningInsight>, String> {
    let is = state.insight_system.read().await;
    let insights = match category.as_deref() {
        Some("pattern") => {
            is.get_insights_by_category(axagent_trajectory::InsightCategory::Pattern)
        },
        Some("preference") => {
            is.get_insights_by_category(axagent_trajectory::InsightCategory::Preference)
        },
        Some("improvement") => {
            is.get_insights_by_category(axagent_trajectory::InsightCategory::Improvement)
        },
        Some("warning") => {
            is.get_insights_by_category(axagent_trajectory::InsightCategory::Warning)
        },
        _ => is.get_insights().iter().collect(),
    };
    let limit = limit.unwrap_or(50);
    let result: Vec<_> = insights.into_iter().take(limit).cloned().collect();
    Ok(result)
}
