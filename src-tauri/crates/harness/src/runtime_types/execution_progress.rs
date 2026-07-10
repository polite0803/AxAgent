// SPDX-License-Identifier: AGPL-3.0-only
//
// Agent 执行进度追踪器

use std::sync::Mutex;
use std::time::Instant;

/// 单条工具执行记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolExecutionRecord {
    #[serde(rename = "toolName")]
    pub tool_name: String,
    #[serde(rename = "startedAt")]
    pub started_at: u64,
    #[serde(rename = "completedAt")]
    pub completed_at: Option<u64>,
    #[serde(rename = "isError")]
    pub is_error: bool,
    #[serde(rename = "outputSummary")]
    pub output_summary: Option<String>,
    #[serde(rename = "inputSummary")]
    pub input_summary: Option<String>,
}

/// Agent 执行进度快照
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentExecutionProgressSnapshot {
    #[serde(rename = "running")]
    pub running: bool,
    #[serde(rename = "phase")]
    pub phase: String,
    #[serde(rename = "currentIteration")]
    pub current_iteration: usize,
    #[serde(rename = "maxIterations")]
    pub max_iterations: usize,
    #[serde(rename = "currentTool")]
    pub current_tool: Option<String>,
    #[serde(rename = "currentToolStartedAt")]
    pub current_tool_started_at: Option<u64>,
    #[serde(rename = "executedToolCount")]
    pub executed_tool_count: usize,
    #[serde(rename = "failedToolCount")]
    pub failed_tool_count: usize,
    #[serde(rename = "recentTools")]
    pub recent_tools: Vec<ToolExecutionRecord>,
    #[serde(rename = "lastError")]
    pub last_error: Option<String>,
    #[serde(rename = "statusMessage")]
    pub status_message: String,
}

/// Agent 执行进度追踪器
pub struct AgentExecutionProgress {
    inner: Mutex<ProgressInner>,
}

struct ProgressInner {
    running: bool,
    phase: String,
    current_iteration: usize,
    max_iterations: usize,
    current_tool: Option<String>,
    tool_started_at: Option<Instant>,
    executed_tool_count: usize,
    failed_tool_count: usize,
    recent_tools: Vec<ToolExecutionRecord>,
    last_error: Option<String>,
    status_message: String,
}

impl AgentExecutionProgress {
    pub fn new(max_iterations: usize) -> Self {
        Self {
            inner: Mutex::new(ProgressInner {
                running: false,
                phase: "idle".into(),
                current_iteration: 0,
                max_iterations,
                current_tool: None,
                tool_started_at: None,
                executed_tool_count: 0,
                failed_tool_count: 0,
                recent_tools: Vec::new(),
                last_error: None,
                status_message: String::new(),
            }),
        }
    }

    pub fn start(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.running = true;
        inner.phase = "init".into();
        inner.status_message = "正在初始化...".into();
    }

    pub fn set_phase(&self, phase: &str, msg: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.phase = phase.to_string();
        inner.status_message = msg.to_string();
    }

    pub fn set_iteration(&self, iter: usize) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.current_iteration = iter;
    }

    pub fn begin_tool(&self, tool_name: &str, input: Option<&str>) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.current_tool = Some(tool_name.to_string());
        inner.tool_started_at = Some(Instant::now());
        inner.phase = "tool_exec".into();
        inner.status_message = format!("正在执行工具: {}...", tool_name);
        let now_ms = now_millis();
        inner.recent_tools.push(ToolExecutionRecord {
            tool_name: tool_name.to_string(),
            started_at: now_ms,
            completed_at: None,
            is_error: false,
            output_summary: None,
            input_summary: input.map(|s| {
                let short: String = s.chars().take(80).collect();
                if s.len() > 80 {
                    format!("{}…", short)
                } else {
                    short
                }
            }),
        });
        if inner.recent_tools.len() > 20 {
            inner.recent_tools.remove(0);
        }
    }

    pub fn end_tool(&self, is_error: bool, output: Option<&str>) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.executed_tool_count += 1;
        if is_error {
            inner.failed_tool_count += 1;
        }
        let tool_name = inner.current_tool.clone();
        if let Some(last) = inner.recent_tools.last_mut()
            && last.completed_at.is_none()
        {
            last.completed_at = Some(now_millis());
            last.is_error = is_error;
            last.output_summary = output.map(|s| {
                let short: String = s.chars().take(200).collect();
                if s.len() > 200 {
                    format!("{}…", short)
                } else {
                    short
                }
            });
        }
        inner.current_tool = None;
        inner.tool_started_at = None;
        inner.phase = "llm_call".into();
        inner.status_message = if is_error {
            format!("工具 {} 执行失败", tool_name.unwrap_or_default())
        } else {
            "正在调用模型...".into()
        };
    }

    pub fn record_error(&self, error: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.last_error = Some(error.to_string());
        inner.status_message = format!("错误: {}", error);
    }

    pub fn finish(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.running = false;
        inner.phase = "done".into();
        inner.status_message = "Agent 执行完成".into();
    }

    pub fn fail(&self, error: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.running = false;
        inner.phase = "error".into();
        inner.last_error = Some(error.to_string());
        inner.status_message = format!("执行失败: {}", error);
    }

    pub fn snapshot(&self) -> AgentExecutionProgressSnapshot {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        AgentExecutionProgressSnapshot {
            running: inner.running,
            phase: inner.phase.clone(),
            current_iteration: inner.current_iteration,
            max_iterations: inner.max_iterations,
            current_tool: inner.current_tool.clone(),
            current_tool_started_at: inner
                .tool_started_at
                .map(|i| now_millis() - (i.elapsed().as_millis() as u64)),
            executed_tool_count: inner.executed_tool_count,
            failed_tool_count: inner.failed_tool_count,
            recent_tools: inner.recent_tools.clone(),
            last_error: inner.last_error.clone(),
            status_message: inner.status_message.clone(),
        }
    }
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
