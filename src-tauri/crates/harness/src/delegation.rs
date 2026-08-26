// SPDX-License-Identifier: AGPL-3.0-only

//! 子代理委托增强模块 (P1-7)
//!
//! 借鉴 Hermes Agent 的委托工程细节：
//! - DELEGATE_BLOCKED_TOOLS: 阻止子代理使用的工具列表
//! - TLS 审批回调: 线程本地存储的审批回调机制
//! - 生命周期管理: 子代理的创建、运行、完成、取消等生命周期

use parking_lot::Mutex;
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DELEGATE_BLOCKED_TOOLS: 阻止子代理使用的工具列表
// ---------------------------------------------------------------------------

/// 默认阻止子代理使用的工具（风险高或需要用户直接授权的工具）
pub const DELEGATE_BLOCKED_TOOLS: &[&str] = &[
    // 文件系统危险操作
    "delete_file",
    "delete_directory",
    "move_file",
    // 网络请求
    "fetch",
    "curl",
    "web_search",
    // 系统操作
    "run_command",
    "shell",
    "terminal",
    // 敏感操作
    "api_key",
    "credential",
];

/// 工具过滤配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolFilterConfig {
    /// 阻止的工具列表
    #[serde(alias = "blocked_tools")]
    pub blocked_tools: HashSet<String>,
    /// 允许的工具列表（空表示不限制）
    #[serde(alias = "allowed_tools")]
    pub allowed_tools: HashSet<String>,
    /// 是否使用默认阻止列表
    #[serde(alias = "use_default_blocked")]
    pub use_default_blocked: bool,
}

impl Default for ToolFilterConfig {
    fn default() -> Self {
        Self {
            blocked_tools: DELEGATE_BLOCKED_TOOLS.iter().map(|s| s.to_string()).collect(),
            allowed_tools: HashSet::new(),
            use_default_blocked: true,
        }
    }
}

impl ToolFilterConfig {
    /// 检查工具是否被允许
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        // 如果有明确的允许列表
        if !self.allowed_tools.is_empty() {
            return self.allowed_tools.contains(tool_name);
        }

        // 检查是否在阻止列表中
        if self.blocked_tools.contains(tool_name) {
            return false;
        }

        true
    }

    /// 过滤工具列表
    pub fn filter_tools(&self, tools: &[String]) -> Vec<String> {
        tools.iter().filter(|t| self.is_tool_allowed(t)).cloned().collect()
    }

    /// 添加阻止的工具
    pub fn block_tool(&mut self, tool: &str) {
        self.blocked_tools.insert(tool.to_string());
    }

    /// 移除阻止的工具
    pub fn unblock_tool(&mut self, tool: &str) {
        self.blocked_tools.remove(tool);
    }
}

// ---------------------------------------------------------------------------
// TLS 审批回调: 线程本地存储的审批回调机制
// ---------------------------------------------------------------------------

/// 审批回调函数类型
pub type ApprovalCallback = Box<dyn Fn(ApprovalRequest) -> ApprovalResponse + Send + Sync>;

/// 审批请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    /// 请求 ID
    pub id: String,
    /// 子代理 ID
    #[serde(alias = "sub_agent_id")]
    pub sub_agent_id: String,
    /// 需要审批的操作描述
    pub action: String,
    /// 风险等级
    #[serde(alias = "risk_level")]
    pub risk_level: RiskLevel,
    /// 上下文信息
    pub context: serde_json::Value,
}

/// 风险等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            RiskLevel::Low => "低",
            RiskLevel::Medium => "中",
            RiskLevel::High => "高",
            RiskLevel::Critical => "严重",
        }
    }
}

/// 审批响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalResponse {
    pub approved: bool,
    pub reason: Option<String>,
    #[serde(alias = "modified_action")]
    pub modified_action: Option<String>,
}

// TLS 审批回调存储 — 使用 thread_local! 存储每个线程的审批回调
thread_local! {
    static APPROVAL_CALLBACK: RefCell<Option<Arc<Mutex<ApprovalCallback>>>> = RefCell::new(None);
}

/// 设置当前线程的审批回调
pub fn set_approval_callback(callback: ApprovalCallback) {
    let arc = Arc::new(Mutex::new(callback));
    APPROVAL_CALLBACK.with(|cell| {
        *cell.borrow_mut() = Some(arc);
    });
}

/// 获取当前线程的审批回调
pub fn get_approval_callback() -> Option<Arc<Mutex<ApprovalCallback>>> {
    APPROVAL_CALLBACK.with(|cell| cell.borrow().clone())
}

/// 清除当前线程的审批回调
pub fn clear_approval_callback() {
    APPROVAL_CALLBACK.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// 执行审批
pub fn execute_approval(request: ApprovalRequest) -> ApprovalResponse {
    // 检查风险等级
    if request.risk_level == RiskLevel::Critical {
        return ApprovalResponse {
            approved: false,
            reason: Some("严重风险操作需要用户直接授权".to_string()),
            modified_action: None,
        };
    }

    // 如果有回调，使用回调
    if let Some(callback_arc) = get_approval_callback() {
        let callback = callback_arc.lock();
        return callback(request);
    }

    // 默认：低风险自动批准，其他拒绝
    match request.risk_level {
        RiskLevel::Low => ApprovalResponse { approved: true, reason: None, modified_action: None },
        _ => ApprovalResponse {
            approved: false,
            reason: Some("需要审批但未设置审批回调".to_string()),
            modified_action: None,
        },
    }
}

// ---------------------------------------------------------------------------
// 子代理生命周期管理
// ---------------------------------------------------------------------------

/// 子代理生命周期状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentLifecycleState {
    /// 已创建
    Created,
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 暂停
    Paused,
    /// 等待审批
    AwaitingApproval,
    /// 执行中（具体任务）
    Executing,
    /// 总结中
    Summarizing,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 已超时
    TimedOut,
}

/// 子代理生命周期事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEvent {
    pub id: String,
    #[serde(alias = "sub_agent_id")]
    pub sub_agent_id: String,
    #[serde(alias = "from_state")]
    pub from_state: SubAgentLifecycleState,
    #[serde(alias = "to_state")]
    pub to_state: SubAgentLifecycleState,
    pub timestamp: u64,
    pub reason: Option<String>,
}

/// 子代理生命周期管理器
pub struct SubAgentLifecycleManager {
    sequence: AtomicU64,
    events: Arc<Mutex<Vec<LifecycleEvent>>>,
    max_history: usize,
}

impl Default for SubAgentLifecycleManager {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl SubAgentLifecycleManager {
    pub fn new(max_history: usize) -> Self {
        Self { sequence: AtomicU64::new(0), events: Arc::new(Mutex::new(Vec::new())), max_history }
    }

    /// 创建生命周期事件
    pub fn transition(
        &self,
        sub_agent_id: &str,
        from: SubAgentLifecycleState,
        to: SubAgentLifecycleState,
        reason: Option<&str>,
    ) -> LifecycleEvent {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        let event = LifecycleEvent {
            id: format!("evt-{}-{}", sub_agent_id, seq),
            sub_agent_id: sub_agent_id.to_string(),
            from_state: from,
            to_state: to,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            reason: reason.map(|s| s.to_string()),
        };

        let mut events = self.events.lock();
        events.push(event.clone());
        // 限制历史长度
        while events.len() > self.max_history {
            events.remove(0);
        }

        event
    }

    /// 获取指定子代理的事件历史
    pub fn get_history(&self, sub_agent_id: &str) -> Vec<LifecycleEvent> {
        let events = self.events.lock();
        events.iter().filter(|e| e.sub_agent_id == sub_agent_id).cloned().collect()
    }

    /// 获取所有事件
    pub fn get_all_events(&self) -> Vec<LifecycleEvent> {
        let events = self.events.lock();
        events.clone()
    }

    /// 清理历史
    pub fn clear_history(&self) {
        let mut events = self.events.lock();
        events.clear();
    }
}

// ---------------------------------------------------------------------------
// 子代理委托配置
// ---------------------------------------------------------------------------

/// 子代理委托配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegationConfig {
    /// 工具过滤配置
    #[serde(alias = "tool_filter")]
    pub tool_filter: ToolFilterConfig,
    /// 是否启用审批回调
    #[serde(alias = "enable_approval_callback")]
    pub enable_approval_callback: bool,
    /// 最大执行时间（秒）
    #[serde(alias = "max_execution_time_secs")]
    pub max_execution_time_secs: u64,
    /// 是否允许嵌套委派
    #[serde(alias = "allow_nested_delegation")]
    pub allow_nested_delegation: bool,
    /// 最大嵌套深度
    #[serde(alias = "max_nesting_depth")]
    pub max_nesting_depth: u32,
    /// 是否记录生命周期事件
    #[serde(alias = "record_lifecycle")]
    pub record_lifecycle: bool,
    /// 默认审批策略
    #[serde(alias = "default_approval_strategy")]
    pub default_approval_strategy: ApprovalStrategy,
}

impl Default for DelegationConfig {
    fn default() -> Self {
        Self {
            tool_filter: ToolFilterConfig::default(),
            enable_approval_callback: true,
            max_execution_time_secs: 300,
            allow_nested_delegation: false,
            max_nesting_depth: 2,
            record_lifecycle: true,
            default_approval_strategy: ApprovalStrategy::AutoApproveLowRisk,
        }
    }
}

/// 默认审批策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStrategy {
    /// 低风险自动批准，其他需要审批
    AutoApproveLowRisk,
    /// 所有操作都需要审批
    RequireApprovalAll,
    /// 所有操作都自动批准（不推荐）
    AutoApproveAll,
    /// 基于角色的策略
    RoleBased,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_filter_default() {
        let config = ToolFilterConfig::default();
        assert!(!config.is_tool_allowed("delete_file"));
        assert!(!config.is_tool_allowed("run_command"));
        assert!(config.is_tool_allowed("read_file"));
        assert!(config.is_tool_allowed("search_code"));
    }

    #[test]
    fn test_tool_filter_allowed_list() {
        let config = ToolFilterConfig {
            allowed_tools: HashSet::from(["read_file".to_string(), "search_code".to_string()]),
            ..Default::default()
        };

        assert!(config.is_tool_allowed("read_file"));
        assert!(config.is_tool_allowed("search_code"));
        assert!(!config.is_tool_allowed("delete_file"));
        assert!(!config.is_tool_allowed("run_command"));
    }

    #[test]
    fn test_tool_filter_block_unblock() {
        let mut config = ToolFilterConfig::default();
        config.unblock_tool("run_command");
        assert!(config.is_tool_allowed("run_command"));

        config.block_tool("read_file");
        assert!(!config.is_tool_allowed("read_file"));
    }

    #[test]
    fn test_filter_tools() {
        let config = ToolFilterConfig::default();
        let tools = vec![
            "read_file".to_string(),
            "delete_file".to_string(),
            "run_command".to_string(),
            "search_code".to_string(),
        ];

        let filtered = config.filter_tools(&tools);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"read_file".to_string()));
        assert!(filtered.contains(&"search_code".to_string()));
    }

    #[test]
    fn test_lifecycle_manager() {
        let manager = SubAgentLifecycleManager::new(100);

        let evt1 = manager.transition(
            "agent-1",
            SubAgentLifecycleState::Created,
            SubAgentLifecycleState::Initializing,
            None,
        );
        assert_eq!(evt1.from_state, SubAgentLifecycleState::Created);
        assert_eq!(evt1.to_state, SubAgentLifecycleState::Initializing);

        let evt2 = manager.transition(
            "agent-1",
            SubAgentLifecycleState::Initializing,
            SubAgentLifecycleState::Running,
            Some("初始化完成"),
        );
        assert_eq!(evt2.from_state, SubAgentLifecycleState::Initializing);
        assert_eq!(evt2.to_state, SubAgentLifecycleState::Running);

        let history = manager.get_history("agent-1");
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_risk_level() {
        assert_eq!(RiskLevel::Low.as_str(), "低");
        assert_eq!(RiskLevel::Critical.as_str(), "严重");
    }
}
