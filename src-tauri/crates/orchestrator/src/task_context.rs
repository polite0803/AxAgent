// SPDX-License-Identifier: AGPL-3.0-only

//! 行业级 TaskContext — 执行逻辑隔离
//!
//! 为每个行业工作流提供独立的执行上下文，实现：
//! - 状态隔离：各行业独立维护自己的执行状态
//! - 配置隔离：行业特定的配置只能在该行业上下文中访问
//! - 资源隔离：Token 预算、缓存等资源按行业分区
//! - 安全隔离：防止跨行业数据泄漏

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::industry_adapters::types::IndustryContext;
use crate::token_budget::IndustryTokenBudgetManager;

/// 行业任务上下文状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskContextState {
    /// 空闲
    Idle,
    /// 初始化中
    Initializing,
    /// 执行中
    Running,
    /// 暂停
    Paused,
    /// 已完成
    Completed,
    /// 出错
    Error,
}

/// 行业任务上下文
///
/// 每个行业工作流拥有独立的 TaskContext，提供：
/// - 行业适配器访问
/// - Token 预算管理
/// - 执行状态跟踪
/// - 资源隔离
#[derive(Debug)]
pub struct IndustryTaskContext {
    /// 行业 ID
    industry_id: String,
    /// 行业上下文信息
    context: IndustryContext,
    /// 上下文状态
    state: RwLock<TaskContextState>,
    /// 执行计数器
    execution_count: RwLock<u64>,
    /// 最近执行时间（毫秒）
    last_execution_ms: RwLock<Option<u64>>,
    /// 行业特定扩展数据
    extensions: RwLock<HashMap<String, serde_json::Value>>,
    /// Token 预算管理器引用
    token_budget: Arc<IndustryTokenBudgetManager>,
    /// 是否启用隔离模式
    isolation_enabled: bool,
}

impl IndustryTaskContext {
    /// 创建行业任务上下文
    pub fn new(
        industry_id: &str,
        context: IndustryContext,
        token_budget: Arc<IndustryTokenBudgetManager>,
    ) -> Self {
        Self {
            industry_id: industry_id.to_string(),
            context,
            state: RwLock::new(TaskContextState::Idle),
            execution_count: RwLock::new(0),
            last_execution_ms: RwLock::new(None),
            extensions: RwLock::new(HashMap::new()),
            token_budget,
            isolation_enabled: true,
        }
    }

    /// 获取行业 ID
    pub fn industry_id(&self) -> &str {
        &self.industry_id
    }

    /// 获取行业上下文
    pub fn context(&self) -> &IndustryContext {
        &self.context
    }

    /// 获取当前状态
    pub async fn state(&self) -> TaskContextState {
        *self.state.read().await
    }

    /// 设置状态
    pub async fn set_state(&self, state: TaskContextState) {
        let mut current = self.state.write().await;
        *current = state;
    }

    /// 开始执行
    pub async fn begin_execution(&self) -> Result<(), String> {
        let mut state = self.state.write().await;
        match *state {
            TaskContextState::Idle | TaskContextState::Completed => {
                *state = TaskContextState::Running;
                drop(state);

                let mut count = self.execution_count.write().await;
                *count += 1;

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let mut last = self.last_execution_ms.write().await;
                *last = Some(now);

                Ok(())
            },
            ref current => Err(format!("无法从 {:?} 状态转换到 Running", current)),
        }
    }

    /// 完成执行
    pub async fn complete_execution(&self, success: bool) {
        let mut state = self.state.write().await;
        *state = if success {
            TaskContextState::Completed
        } else {
            TaskContextState::Error
        };
    }

    /// 获取执行次数
    pub async fn execution_count(&self) -> u64 {
        *self.execution_count.read().await
    }

    /// 获取最近执行时间
    pub async fn last_execution_time(&self) -> Option<u64> {
        *self.last_execution_ms.read().await
    }

    /// 设置扩展数据
    pub async fn set_extension(&self, key: &str, value: serde_json::Value) {
        let mut extensions = self.extensions.write().await;
        extensions.insert(key.to_string(), value);
    }

    /// 获取扩展数据
    pub async fn get_extension(&self, key: &str) -> Option<serde_json::Value> {
        let extensions = self.extensions.read().await;
        extensions.get(key).cloned()
    }

    /// 删除扩展数据
    pub async fn remove_extension(&self, key: &str) -> Option<serde_json::Value> {
        let mut extensions = self.extensions.write().await;
        extensions.remove(key)
    }

    /// 检查隔离模式是否启用
    pub fn is_isolation_enabled(&self) -> bool {
        self.isolation_enabled
    }

    /// 设置隔离模式
    pub fn set_isolation(&mut self, enabled: bool) {
        self.isolation_enabled = enabled;
    }

    /// 获取 Token 预算管理器
    pub fn token_budget(&self) -> &Arc<IndustryTokenBudgetManager> {
        &self.token_budget
    }

    /// 校验行业匹配（用于隔离检查）
    pub fn check_industry_match(&self, required_industry: &str) -> bool {
        if !self.isolation_enabled {
            return true; // 隔离关闭时允许跨行业访问
        }
        self.industry_id == required_industry
    }

    /// 生成上下文摘要
    pub async fn summary(&self) -> TaskContextSummary {
        TaskContextSummary {
            industry_id: self.industry_id.clone(),
            state: *self.state.read().await,
            execution_count: *self.execution_count.read().await,
            last_execution_ms: *self.last_execution_ms.read().await,
            extension_keys: self.extensions.read().await.keys().cloned().collect(),
        }
    }
}

/// 任务上下文摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContextSummary {
    pub industry_id: String,
    pub state: TaskContextState,
    pub execution_count: u64,
    pub last_execution_ms: Option<u64>,
    pub extension_keys: Vec<String>,
}

/// 行业上下文管理器
///
/// 管理所有行业的 TaskContext，提供：
/// - 上下文注册和注销
/// - 上下文查找
/// - 批量状态查询
/// - 隔离检查
#[derive(Debug, Default)]
pub struct IndustryContextManager {
    /// 所有行业上下文
    contexts: RwLock<HashMap<String, Arc<IndustryTaskContext>>>,
    /// Token 预算管理器
    token_budget: Arc<IndustryTokenBudgetManager>,
}

impl IndustryContextManager {
    /// 创建新的上下文管理器
    pub fn new(token_budget: Arc<IndustryTokenBudgetManager>) -> Self {
        Self { contexts: RwLock::new(HashMap::new()), token_budget }
    }

    /// 注册行业上下文
    pub async fn register(
        &self,
        industry_id: &str,
        context: IndustryContext,
    ) -> Arc<IndustryTaskContext> {
        let task_context =
            Arc::new(IndustryTaskContext::new(industry_id, context, self.token_budget.clone()));

        let mut contexts = self.contexts.write().await;
        contexts.insert(industry_id.to_string(), task_context.clone());

        task_context
    }

    /// 获取行业上下文
    pub async fn get(&self, industry_id: &str) -> Option<Arc<IndustryTaskContext>> {
        let contexts = self.contexts.read().await;
        contexts.get(industry_id).cloned()
    }

    /// 注销行业上下文
    pub async fn unregister(&self, industry_id: &str) -> bool {
        let mut contexts = self.contexts.write().await;
        contexts.remove(industry_id).is_some()
    }

    /// 列出所有行业 ID
    pub async fn list_industries(&self) -> Vec<String> {
        let contexts = self.contexts.read().await;
        contexts.keys().cloned().collect()
    }

    /// 获取所有上下文摘要
    pub async fn get_all_summaries(&self) -> Vec<TaskContextSummary> {
        let contexts = self.contexts.read().await;
        let mut summaries = Vec::new();

        for ctx in contexts.values() {
            summaries.push(ctx.summary().await);
        }

        summaries
    }

    /// 检查跨行业访问是否允许
    pub async fn check_access(&self, source_industry: &str, target_industry: &str) -> bool {
        if source_industry == target_industry {
            return true; // 同行业直接允许
        }

        // 查找目标上下文
        if let Some(ctx) = self.get(target_industry).await {
            ctx.check_industry_match(source_industry)
        } else {
            false // 目标行业不存在，拒绝访问
        }
    }

    /// 获取 Token 预算管理器
    pub fn token_budget(&self) -> &Arc<IndustryTokenBudgetManager> {
        &self.token_budget
    }

    /// 获取行业数量
    pub async fn count(&self) -> usize {
        let contexts = self.contexts.read().await;
        contexts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context(industry_id: &str) -> IndustryContext {
        IndustryContext {
            session_id: Some(format!("session-{}", industry_id)),
            user_id: Some("test-user".to_string()),
            workspace_id: Some("test-workspace".to_string()),
            inputs: serde_json::json!({"industry": industry_id}),
            history: vec![],
            knowledge_ids: vec![],
            metadata: serde_json::Value::Null,
        }
    }

    #[tokio::test]
    async fn test_create_task_context() {
        let token_budget = Arc::new(IndustryTokenBudgetManager::new());
        let ctx = IndustryTaskContext::new(
            "test-industry",
            create_test_context("test-industry"),
            token_budget,
        );

        assert_eq!(ctx.industry_id(), "test-industry");
        assert_eq!(ctx.state().await, TaskContextState::Idle);
        assert_eq!(ctx.execution_count().await, 0);
    }

    #[tokio::test]
    async fn test_begin_execution() {
        let token_budget = Arc::new(IndustryTokenBudgetManager::new());
        let ctx = IndustryTaskContext::new("test", create_test_context("test"), token_budget);

        assert!(ctx.begin_execution().await.is_ok());
        assert_eq!(ctx.state().await, TaskContextState::Running);
        assert_eq!(ctx.execution_count().await, 1);
    }

    #[tokio::test]
    async fn test_complete_execution() {
        let token_budget = Arc::new(IndustryTokenBudgetManager::new());
        let ctx = IndustryTaskContext::new("test", create_test_context("test"), token_budget);

        ctx.begin_execution().await.unwrap();
        ctx.complete_execution(true).await;

        assert_eq!(ctx.state().await, TaskContextState::Completed);
    }

    #[tokio::test]
    async fn test_isolation_check() {
        let token_budget = Arc::new(IndustryTokenBudgetManager::new());
        let ctx =
            IndustryTaskContext::new("industry-a", create_test_context("industry-a"), token_budget);

        assert!(ctx.check_industry_match("industry-a"));
        assert!(!ctx.check_industry_match("industry-b"));
    }

    #[tokio::test]
    async fn test_extensions() {
        let token_budget = Arc::new(IndustryTokenBudgetManager::new());
        let ctx = IndustryTaskContext::new("test", create_test_context("test"), token_budget);

        ctx.set_extension("key1", serde_json::json!("value1")).await;
        let val = ctx.get_extension("key1").await;
        assert_eq!(val, Some(serde_json::json!("value1")));

        let removed = ctx.remove_extension("key1").await;
        assert_eq!(removed, Some(serde_json::json!("value1")));

        let val2 = ctx.get_extension("key1").await;
        assert_eq!(val2, None);
    }

    #[tokio::test]
    async fn test_context_manager() {
        let token_budget = Arc::new(IndustryTokenBudgetManager::new());
        let manager = IndustryContextManager::new(token_budget);

        manager.register("industry-1", create_test_context("industry-1")).await;
        manager.register("industry-2", create_test_context("industry-2")).await;

        assert_eq!(manager.count().await, 2);

        let industries = manager.list_industries().await;
        assert!(industries.contains(&"industry-1".to_string()));
        assert!(industries.contains(&"industry-2".to_string()));
    }

    #[tokio::test]
    async fn test_cross_industry_access() {
        let token_budget = Arc::new(IndustryTokenBudgetManager::new());
        let manager = IndustryContextManager::new(token_budget);

        manager.register("industry-1", create_test_context("industry-1")).await;
        manager.register("industry-2", create_test_context("industry-2")).await;

        assert!(manager.check_access("industry-1", "industry-1").await);
        assert!(!manager.check_access("industry-1", "industry-2").await);
        assert!(!manager.check_access("industry-2", "industry-1").await);
    }

    #[tokio::test]
    async fn test_summary() {
        let token_budget = Arc::new(IndustryTokenBudgetManager::new());
        let ctx = IndustryTaskContext::new("test", create_test_context("test"), token_budget);

        ctx.begin_execution().await.unwrap();
        ctx.set_extension("test-key", serde_json::json!(42)).await;

        let summary = ctx.summary().await;
        assert_eq!(summary.industry_id, "test");
        assert_eq!(summary.state, TaskContextState::Running);
        assert_eq!(summary.execution_count, 1);
        assert!(summary.last_execution_ms.is_some());
        assert!(summary.extension_keys.contains(&"test-key".to_string()));
    }
}
