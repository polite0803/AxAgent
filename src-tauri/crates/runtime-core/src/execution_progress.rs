use std::sync::Mutex;
use std::time::Instant;

/// 单条工具执行记录（用于 IPC 返回给前端面板）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolExecutionRecord {
    #[serde(rename = "toolName")]
    pub tool_name: String,
    /// 开始时间 (ms since epoch)
    #[serde(rename = "startedAt")]
    pub started_at: u64,
    /// 完成时间 (ms since epoch)，None 表示还在执行中
    #[serde(rename = "completedAt")]
    pub completed_at: Option<u64>,
    #[serde(rename = "isError")]
    pub is_error: bool,
    /// 输出摘要（前 200 字符）
    #[serde(rename = "outputSummary")]
    pub output_summary: Option<String>,
    /// 工具输入摘要（前 80 字符）
    #[serde(rename = "inputSummary")]
    pub input_summary: Option<String>,
}

/// Agent 执行进度快照（用于 IPC 只读返回）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentExecutionProgressSnapshot {
    #[serde(rename = "running")]
    pub running: bool,
    /// 当前阶段: init / llm_call / tool_exec / compacting / done / error
    #[serde(rename = "phase")]
    pub phase: String,
    /// 当前迭代数 (1-based, 0 表示还未开始首轮)
    #[serde(rename = "currentIteration")]
    pub current_iteration: usize,
    /// 最大迭代数
    #[serde(rename = "maxIterations")]
    pub max_iterations: usize,
    /// 当前正在执行的工具名，None 表示无工具运行
    #[serde(rename = "currentTool")]
    pub current_tool: Option<String>,
    /// 当前工具开始时间 (ms since epoch)
    #[serde(rename = "currentToolStartedAt")]
    pub current_tool_started_at: Option<u64>,
    /// 已执行工具总数
    #[serde(rename = "executedToolCount")]
    pub executed_tool_count: usize,
    /// 失败工具数
    #[serde(rename = "failedToolCount")]
    pub failed_tool_count: usize,
    /// 最近 N 条工具记录（最多 20 条）
    #[serde(rename = "recentTools")]
    pub recent_tools: Vec<ToolExecutionRecord>,
    /// 最后一次错误
    #[serde(rename = "lastError")]
    pub last_error: Option<String>,
    /// 当前阶段状态文本（如 "正在调用模型...", "正在执行工具..."）
    #[serde(rename = "statusMessage")]
    pub status_message: String,
}

/// Agent 执行进度追踪器（内部可变，读写分离）
///
/// 设计原则：
/// - `run_turn()` 同步循环中通过 `&self` 方法更新（内部 Mutex）
/// - `agent_runtime_stats` IPC 异步读取通过 `snapshot()` 返回只读快照
/// - 使用 `std::sync::Mutex` 而非 `tokio::sync::Mutex`
///   因为 lock 持有时间极短（仅读写几个字段），且不在 async 上下文中持有
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

    // ── 写方法（run_turn() 同步循环中调用） ──

    /// 标记开始执行
    pub fn start(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.running = true;
        inner.phase = "init".into();
        inner.status_message = "正在初始化...".into();
    }

    /// 设置当前阶段
    pub fn set_phase(&self, phase: &str, msg: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.phase = phase.to_string();
        inner.status_message = msg.to_string();
    }

    /// 设置当前迭代
    pub fn set_iteration(&self, iter: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.current_iteration = iter;
    }

    /// 标记工具执行开始
    pub fn begin_tool(&self, tool_name: &str, input: Option<&str>) {
        let mut inner = self.inner.lock().unwrap();
        inner.current_tool = Some(tool_name.to_string());
        inner.tool_started_at = Some(Instant::now());
        inner.phase = "tool_exec".into();
        inner.status_message = format!("正在执行工具: {}...", tool_name);
        // 同时往 recent_tools 中插入一条 started 记录
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
        // 只保留最近 20 条
        if inner.recent_tools.len() > 20 {
            inner.recent_tools.remove(0);
        }
    }

    /// 标记工具执行完成
    pub fn end_tool(&self, is_error: bool, output: Option<&str>) {
        let mut inner = self.inner.lock().unwrap();
        inner.executed_tool_count += 1;
        if is_error {
            inner.failed_tool_count += 1;
        }
        // 更新最近一条匹配的 tool 记录
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

    /// 记录错误
    pub fn record_error(&self, error: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.last_error = Some(error.to_string());
        inner.status_message = format!("错误: {}", error);
    }

    /// 标记完成
    pub fn finish(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.running = false;
        inner.phase = "done".into();
        inner.status_message = "Agent 执行完成".into();
    }

    /// 标记失败
    pub fn fail(&self, error: &str) {
        let mut inner = self.inner.lock().unwrap();
        inner.running = false;
        inner.phase = "error".into();
        inner.last_error = Some(error.to_string());
        inner.status_message = format!("执行失败: {}", error);
    }

    // ── 读方法（agent_runtime_stats IPC 调用） ──

    /// 返回只读快照（用于序列化返回给前端）
    pub fn snapshot(&self) -> AgentExecutionProgressSnapshot {
        let inner = self.inner.lock().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_lifecycle() {
        let p = AgentExecutionProgress::new(50);
        let snap = p.snapshot();
        assert!(!snap.running);
        assert_eq!(snap.phase, "idle");
        assert_eq!(snap.current_iteration, 0);
        assert_eq!(snap.executed_tool_count, 0);

        p.start();
        let snap = p.snapshot();
        assert!(snap.running);
        assert_eq!(snap.status_message, "正在初始化...");

        p.set_iteration(1);
        p.begin_tool("Bash", Some("ls -la"));
        let snap = p.snapshot();
        assert_eq!(snap.current_iteration, 1);
        assert_eq!(snap.current_tool, Some("Bash".into()));
        assert_eq!(snap.recent_tools.len(), 1);

        p.end_tool(false, Some("total 42\ndrwxr-xr-x ..."));
        let snap = p.snapshot();
        assert_eq!(snap.current_tool, None);
        assert_eq!(snap.executed_tool_count, 1);
        assert_eq!(snap.failed_tool_count, 0);
        assert_eq!(snap.recent_tools.len(), 1);
        assert!(snap.recent_tools[0].completed_at.is_some());
        assert!(!snap.recent_tools[0].is_error);

        p.finish();
        let snap = p.snapshot();
        assert!(!snap.running);
        assert_eq!(snap.phase, "done");
    }

    #[test]
    fn test_tool_error() {
        let p = AgentExecutionProgress::new(50);
        p.start();
        p.begin_tool("Write", Some("file.txt"));
        p.end_tool(true, Some("Permission denied"));
        let snap = p.snapshot();
        assert_eq!(snap.failed_tool_count, 1);
        assert!(snap.recent_tools[0].is_error);
    }

    #[test]
    fn test_recent_tools_cap() {
        let p = AgentExecutionProgress::new(50);
        p.start();
        for i in 0..25 {
            p.begin_tool(&format!("Tool{}", i), None);
            p.end_tool(false, None);
        }
        let snap = p.snapshot();
        assert_eq!(snap.recent_tools.len(), 20); // capped at 20
        assert_eq!(snap.executed_tool_count, 25);
    }
}
