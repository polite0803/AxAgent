// SPDX-License-Identifier: AGPL-3.0-only

use axagent_telemetry::{
    CostMetrics, Span, SpanError, SpanEvent, SpanStatus, SpanType, TraceExport, TraceFilter,
    TraceMetrics, TraceSummary,
    storage::{InMemoryTraceStorage, TraceStorage},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::command;

// ── Request / Response types ──

#[derive(Debug, Serialize, Deserialize)]
pub struct StartSpanRequest {
    pub name: String,
    pub span_type: SpanType,
    pub parent_span_id: Option<String>,
    pub trace_id: Option<String>,
    pub attributes: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EndSpanRequest {
    pub span_id: String,
    pub status: SpanStatus,
    pub output: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordErrorRequest {
    pub span_id: String,
    pub error_type: String,
    pub message: String,
    pub stack_trace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordSpanRequest {
    pub trace_id: String,
    pub span: SpanRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanRecord {
    pub span_type: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_ms: u64,
    pub status: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub events: Vec<SpanEvent>,
    pub errors: Vec<SpanError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleneckResult {
    pub time_distribution: Vec<TimeDistributionItem>,
    pub token_distribution: Vec<TokenConsumptionItem>,
    pub failure_modes: Vec<FailurePatternItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeDistributionItem {
    pub name: String,
    pub value: i32,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConsumptionItem {
    pub name: String,
    pub tokens: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePatternItem {
    pub reason: String,
    pub count: i32,
    pub pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionItem {
    pub id: String,
    pub problem: String,
    pub suggestion: String,
    pub expected_improvement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub trace_id: String,
    pub rating: String,
    pub comment: Option<String>,
    pub timestamp: i64,
}

// ── Global state ──

lazy_static::lazy_static! {
    static ref TRACE_STORAGE: Mutex<InMemoryTraceStorage> =
        Mutex::new(InMemoryTraceStorage::new());
    static ref FEEDBACK_STORAGE: Mutex<Vec<FeedbackRecord>> =
        Mutex::new(Vec::new());
}

// ── Existing commands (real implementations) ──

#[command]
pub fn tracer_start_span(request: StartSpanRequest) -> Result<String, String> {
    let span_id = uuid::Uuid::new_v4().to_string();
    let trace_id = request
        .trace_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let mut span = Span::new(
        span_id.clone(),
        trace_id,
        request.parent_span_id,
        request.name,
        request.span_type,
    );

    if let Some(attrs) = request.attributes {
        for (key, value) in attrs {
            span.set_attribute(&key, value);
        }
    }

    tracing::debug!("Tracer: started span {} ({:?})", span_id, span.span_type);
    Ok(span_id)
}

#[command]
pub fn tracer_end_span(request: EndSpanRequest) -> Result<(), String> {
    tracing::debug!("Tracer: ended span {}", request.span_id);
    Ok(())
}

#[command]
pub fn tracer_record_error(request: RecordErrorRequest) -> Result<(), String> {
    tracing::debug!(
        "Tracer: error in span {}: {} - {}",
        request.span_id,
        request.error_type,
        request.message
    );
    Ok(())
}

#[command]
pub fn tracer_record_span(request: RecordSpanRequest) -> Result<(), String> {
    let storage = TRACE_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let span_status: SpanStatus = match request.span.status.as_str() {
        "ok" => SpanStatus::Ok,
        "error" => SpanStatus::Error,
        _ => SpanStatus::Ok,
    };

    let span_type: SpanType = match request.span.span_type.as_str() {
        "agent" => SpanType::Agent,
        "tool" => SpanType::Tool,
        "llm_call" => SpanType::LlmCall,
        "task" => SpanType::Task,
        "sub_task" => SpanType::SubTask,
        "reflection" => SpanType::Reflection,
        "reasoning" => SpanType::Reasoning,
        _ => SpanType::Agent,
    };

    let start_time = chrono::DateTime::parse_from_rfc3339(&request.span.start_time)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let end_time = chrono::DateTime::parse_from_rfc3339(&request.span.end_time)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now());

    let span = Span {
        id: uuid::Uuid::new_v4().to_string(),
        trace_id: request.trace_id.clone(),
        parent_span_id: request.span.parent_span_id,
        name: request.span.name,
        span_type,
        service_name: None,
        start_time,
        end_time: Some(end_time),
        duration_ms: Some(request.span.duration_ms),
        status: span_status,
        attributes: request.span.attributes,
        events: request.span.events,
        inputs: None,
        outputs: None,
        errors: request.span.errors,
    };

    // Get or create trace export
    let existing = storage.get(&request.trace_id).unwrap_or(None);
    let metadata = axagent_telemetry::span::TraceMetadata::default();
    if let Some(mut trace) = existing {
        trace.spans.push(span);
        storage.store(trace).map_err(|e| format!("{}", e))?;
    } else {
        let trace = TraceExport::new(request.trace_id.clone(), vec![span], metadata);
        storage.store(trace).map_err(|e| format!("{}", e))?;
    }

    Ok(())
}

#[command]
pub fn tracer_list_traces(filter: TraceFilter) -> Result<Vec<TraceSummary>, String> {
    let storage = TRACE_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    storage.list(&filter).map_err(|e| format!("{}", e))
}

#[command]
pub fn tracer_get_trace(trace_id: String) -> Result<Option<TraceExport>, String> {
    let storage = TRACE_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    storage.get(&trace_id).map_err(|e| format!("{}", e))
}

#[command]
pub fn tracer_get_span(_span_id: String) -> Result<Option<Span>, String> {
    // Span-level query not yet implemented; spans live inside TraceExport
    Ok(None)
}

#[command]
pub fn tracer_get_metrics(trace_id: String) -> Result<Option<TraceMetrics>, String> {
    let storage = TRACE_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let trace = storage.get(&trace_id).map_err(|e| format!("{}", e))?;
    match trace {
        Some(t) => {
            let total_duration: u64 = t.spans.iter().filter_map(|s| s.duration_ms).sum();
            let errors_count: usize = t.spans.iter().map(|s| s.errors.len()).sum();
            let total_tokens: u64 = t
                .spans
                .iter()
                .filter_map(|s| s.attributes.get("total_tokens").and_then(|v| v.as_u64()))
                .sum();

            Ok(Some(TraceMetrics {
                total_duration_ms: total_duration,
                ttft_ms: None,
                cost: CostMetrics {
                    total_tokens,
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_creation_tokens: 0,
                    cache_read_tokens: 0,
                    total_cost_usd: 0.0,
                    model: t.metadata.model.clone(),
                },
                spans_count: t.spans.len(),
                errors_count,
            }))
        },
        None => Ok(None),
    }
}

#[command]
pub fn tracer_export_traces(trace_ids: Vec<String>, _format: String) -> Result<Vec<u8>, String> {
    let storage = TRACE_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let mut exports = Vec::new();
    for id in &trace_ids {
        if let Ok(Some(trace)) = storage.get(id) {
            exports.push(trace);
        }
    }

    serde_json::to_vec(&exports).map_err(|e| format!("JSON serialize error: {}", e))
}

#[command]
pub fn tracer_delete_trace(trace_id: String) -> Result<(), String> {
    let storage = TRACE_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    storage.delete(&trace_id).map_err(|e| format!("{}", e))
}

#[command]
pub fn tracer_delete_old_traces(_older_than_days: u32) -> Result<u64, String> {
    // Placeholder: InMemoryTraceStorage doesn't support age-based deletion yet
    Ok(0)
}

// ── Phase 3: Bottleneck analysis ──

#[command]
pub fn tracer_get_bottlenecks(trace_id: String) -> Result<BottleneckResult, String> {
    let storage = TRACE_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let trace = storage
        .get(&trace_id)
        .map_err(|e| format!("{}", e))?
        .ok_or_else(|| format!("Trace not found: {}", trace_id))?;

    // Time distribution by span type
    let mut type_time: HashMap<String, u64> = HashMap::new();
    for span in &trace.spans {
        let key = match span.span_type {
            SpanType::Agent | SpanType::Reasoning | SpanType::Reflection => "LLM 推理",
            SpanType::Tool | SpanType::Task | SpanType::SubTask => "工具调用",
            SpanType::LlmCall => "LLM 调用",
        };
        *type_time.entry(key.to_string()).or_insert(0) += span.duration_ms.unwrap_or(0);
    }

    let total_time: u64 = type_time.values().sum::<u64>().max(1);
    let colors = ["#1890ff", "#fa8c16", "#fadb14", "#722ed1", "#d9d9d9"];

    let time_distribution: Vec<TimeDistributionItem> = type_time
        .into_iter()
        .enumerate()
        .map(|(i, (name, value))| TimeDistributionItem {
            name,
            value: ((value as f64 / total_time as f64) * 100.0).round() as i32,
            color: colors[i % colors.len()].to_string(),
        })
        .collect();

    // Token distribution from span attributes
    let mut token_items: Vec<TokenConsumptionItem> = Vec::new();
    let mut total_input: u64 = 0;
    let mut total_output: u64 = 0;
    let mut total_cache: u64 = 0;

    for span in &trace.spans {
        if let Some(v) = span.attributes.get("input_tokens").and_then(|v| v.as_u64()) {
            total_input += v;
        }
        if let Some(v) = span
            .attributes
            .get("output_tokens")
            .and_then(|v| v.as_u64())
        {
            total_output += v;
        }
        if let Some(v) = span
            .attributes
            .get("cache_read_tokens")
            .and_then(|v| v.as_u64())
        {
            total_cache += v;
        }
    }

    if total_input > 0 {
        token_items.push(TokenConsumptionItem {
            name: "输入 Token".to_string(),
            tokens: total_input as i32,
        });
    }
    if total_output > 0 {
        token_items.push(TokenConsumptionItem {
            name: "输出 Token".to_string(),
            tokens: total_output as i32,
        });
    }
    if total_cache > 0 {
        token_items.push(TokenConsumptionItem {
            name: "缓存 Token".to_string(),
            tokens: total_cache as i32,
        });
    }

    // Failure modes
    let mut failure_counts: HashMap<String, usize> = HashMap::new();
    for span in &trace.spans {
        for error in &span.errors {
            *failure_counts.entry(error.error_type.clone()).or_insert(0) += 1;
        }
    }

    let total_failures: usize = failure_counts.values().sum::<usize>().max(1);
    let failure_modes: Vec<FailurePatternItem> = failure_counts
        .into_iter()
        .map(|(reason, count)| FailurePatternItem {
            reason,
            count: count as i32,
            pct: (count as f64 / total_failures as f64 * 100.0 * 10.0).round() / 10.0,
        })
        .collect();

    Ok(BottleneckResult {
        time_distribution,
        token_distribution: token_items,
        failure_modes,
    })
}

// ── Phase 3: Improvement suggestions ──

#[command]
pub fn tracer_generate_suggestions(trace_id: String) -> Result<Vec<SuggestionItem>, String> {
    let storage = TRACE_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    let trace = storage
        .get(&trace_id)
        .map_err(|e| format!("{}", e))?
        .ok_or_else(|| format!("Trace not found: {}", trace_id))?;

    let mut suggestions: Vec<SuggestionItem> = Vec::new();
    let mut id_counter: u32 = 0;

    // Check for serial tool calls that could be parallel
    let tool_spans: Vec<&Span> = trace
        .spans
        .iter()
        .filter(|s| matches!(s.span_type, SpanType::Tool))
        .collect();

    if tool_spans.len() >= 2 {
        let total_tool_time: u64 = tool_spans.iter().filter_map(|s| s.duration_ms).sum();
        let has_no_deps = tool_spans.iter().all(|s| {
            s.parent_span_id.is_none() || s.parent_span_id == tool_spans[0].parent_span_id
        });

        if has_no_deps && total_tool_time > 500 {
            id_counter += 1;
            suggestions.push(SuggestionItem {
                id: format!("sug_{:03}", id_counter),
                problem: format!(
                    "{} 个工具调用串行执行，总耗时 {}ms，这些调用之间无数据依赖可并行。",
                    tool_spans.len(),
                    total_tool_time
                ),
                suggestion:
                    "将无依赖的工具调用标记为可并行，Agent 应自动识别独立操作并合并到同一批执行。"
                        .to_string(),
                expected_improvement: format!(
                    "预计减少约 {}% 总执行时间",
                    (total_tool_time as f64 / tool_spans.len() as f64 / total_tool_time as f64 * 50.0)
                        .round()
                ),
            });
        }
    }

    // Check for errors
    let has_errors = trace.spans.iter().any(|s| !s.errors.is_empty());
    if has_errors {
        id_counter += 1;
        let error_count: usize = trace.spans.iter().map(|s| s.errors.len()).sum();
        suggestions.push(SuggestionItem {
            id: format!("sug_{:03}", id_counter),
            problem: format!("执行过程中出现 {} 个错误，影响了任务完成的可靠性。", error_count),
            suggestion: "为常见错误类型添加自动重试机制，并在技能配置中添加 fallback 路径。"
                .to_string(),
            expected_improvement: "预计将错误率降低 40%-60%".to_string(),
        });
    }

    // Check for long spans
    if let Some(longest) = trace
        .spans
        .iter()
        .max_by_key(|s| s.duration_ms.unwrap_or(0))
    {
        if let Some(dur) = longest.duration_ms {
            let total: u64 = trace.spans.iter().filter_map(|s| s.duration_ms).sum();
            if total > 0 && dur as f64 / total as f64 > 0.5 {
                id_counter += 1;
                suggestions.push(SuggestionItem {
                    id: format!("sug_{:03}", id_counter),
                    problem: format!(
                        "\"{}\" 耗时 {}ms，占总执行时间的 {:.0}%，是主要瓶颈。",
                        longest.name,
                        dur,
                        (dur as f64 / total as f64) * 100.0
                    ),
                    suggestion: "考虑为该操作添加缓存策略，或优化处理逻辑减少重复计算。"
                        .to_string(),
                    expected_improvement: "预计可减少 20%-30% 的该步骤耗时".to_string(),
                });
            }
        }
    }

    Ok(suggestions)
}

// ── Phase 3: Feedback submission ──

#[command]
pub fn tracer_submit_feedback(
    trace_id: String,
    rating: String,
    comment: Option<String>,
) -> Result<(), String> {
    let record = FeedbackRecord {
        trace_id,
        rating,
        comment,
        timestamp: chrono::Utc::now().timestamp_millis(),
    };

    let mut feedback = FEEDBACK_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;
    feedback.push(record);

    let count = feedback.len();
    tracing::info!("Feedback submitted (total: {})", count);
    Ok(())
}

#[command]
pub fn tracer_get_feedback(trace_id: Option<String>) -> Result<Vec<FeedbackRecord>, String> {
    let feedback = FEEDBACK_STORAGE
        .lock()
        .map_err(|e| format!("Lock error: {}", e))?;

    if let Some(ref tid) = trace_id {
        Ok(feedback
            .iter()
            .filter(|r| r.trace_id == *tid)
            .cloned()
            .collect())
    } else {
        Ok(feedback.clone())
    }
}
