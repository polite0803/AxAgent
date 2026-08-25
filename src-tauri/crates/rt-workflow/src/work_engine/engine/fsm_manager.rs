// SPDX-License-Identifier: AGPL-3.0-only

//! FSM 管理器
//!
//! 管理 FSM 实例的生命周期，包括创建、加载、保存、删除。
//! 支持持久化到存储后端（数据库或内存）。
//!
//! # 架构位置
//! - 实现层：rt-workflow（hybrid 层）
//! - 依赖：harness::business_state_machine（FSM 定义 + 持久化接口）
//! - 被 WorkflowEngine 调用，管理业务状态机实例

use std::collections::HashMap;
use std::sync::Arc;

use axagent_harness::business_state_machine::{
    BusinessStateMachine, FsmDecisionLog, FsmDecisionType, FsmPersistence, FsmPersistenceError,
    FsmRuntimeState,
};
use tokio::sync::RwLock;

use super::fsm_executor::FsmExecutor;

/// FSM 实例状态
#[derive(Debug, Clone, PartialEq)]
pub enum FsmInstanceStatus {
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 已暂停
    Paused,
    /// 错误
    Error(String),
}

/// FSM 实例信息
#[derive(Debug, Clone)]
pub struct FsmInstanceInfo {
    /// 实例 ID
    pub instance_id: String,
    /// 状态机 ID
    pub fsm_id: String,
    /// 当前状态 ID
    pub current_state_id: String,
    /// 实例状态
    pub status: FsmInstanceStatus,
    /// 创建时间戳
    pub created_at_ms: u64,
    /// 最后更新时间戳
    pub updated_at_ms: u64,
}

/// FSM 管理器
///
/// 负责：
/// 1. FSM 实例的创建、加载、保存、删除
/// 2. FSM 执行器的生命周期管理
/// 3. 决策日志的记录和查询
pub struct FsmManager {
    /// 状态机定义存储（fsm_id → 定义）
    definitions: Arc<RwLock<HashMap<String, BusinessStateMachine>>>,
    /// 活跃的 FSM 执行器（instance_id → FsmExecutor）
    executors: Arc<RwLock<HashMap<String, Arc<FsmExecutor>>>>,
    /// 决策日志存储（instance_id → 日志列表）
    decision_logs: Arc<RwLock<HashMap<String, Vec<FsmDecisionLog>>>>,
    /// 持久化后端
    persistence: Arc<dyn FsmPersistence>,
}

impl FsmManager {
    /// 创建新的 FSM 管理器（使用内存持久化）
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(RwLock::new(HashMap::new())),
            executors: Arc::new(RwLock::new(HashMap::new())),
            decision_logs: Arc::new(RwLock::new(HashMap::new())),
            persistence: Arc::new(
                axagent_harness::business_state_machine::MemoryFsmPersistence::new(),
            ),
        }
    }

    /// 使用指定持久化后端创建管理器
    pub fn with_persistence(persistence: Arc<dyn FsmPersistence>) -> Self {
        Self {
            definitions: Arc::new(RwLock::new(HashMap::new())),
            executors: Arc::new(RwLock::new(HashMap::new())),
            decision_logs: Arc::new(RwLock::new(HashMap::new())),
            persistence,
        }
    }

    /// 注册状态机定义
    pub async fn register_definition(&self, fsm: BusinessStateMachine) -> Result<(), String> {
        fsm.validate().map_err(|e| e.to_string())?;
        self.definitions.write().await.insert(fsm.id.clone(), fsm);
        Ok(())
    }

    /// 创建 FSM 实例
    pub async fn create_instance(
        &self,
        fsm_id: &str,
        instance_id: impl Into<String>,
    ) -> Result<String, FsmPersistenceError> {
        let fsm = self
            .definitions
            .read()
            .await
            .get(fsm_id)
            .cloned()
            .ok_or_else(|| FsmPersistenceError::NotFound(fsm_id.to_string()))?;

        let instance_id = instance_id.into();
        let executor = Arc::new(FsmExecutor::new(fsm, &instance_id));

        // 记录创建决策
        let state = executor.current_state().await;
        let decision_log = FsmDecisionLog {
            id: generate_id(),
            instance_id: instance_id.clone(),
            timestamp_ms: state.created_at_ms,
            decision_type: FsmDecisionType::Create,
            from_state: String::new(),
            to_state: Some(state.current_state_id.clone()),
            context: None,
            description: Some("FSM 实例创建".to_string()),
        };

        // 保存决策日志
        self.decision_logs.write().await.insert(instance_id.clone(), vec![decision_log]);

        // 保存运行时状态
        let state = executor.current_state().await;
        let logs = self.decision_logs.read().await.get(&instance_id).cloned().unwrap_or_default();
        self.persistence.save_state(&state, &logs)?;

        // 注册执行器
        self.executors.write().await.insert(instance_id.clone(), executor);

        Ok(instance_id)
    }

    /// 从持久化加载 FSM 实例
    pub async fn load_instance(
        &self,
        instance_id: &str,
    ) -> Result<Arc<FsmExecutor>, FsmPersistenceError> {
        // 检查是否已在内存中
        if let Some(executor) = self.executors.read().await.get(instance_id).cloned() {
            return Ok(executor);
        }

        // 从持久化加载
        let (state, logs) = self
            .persistence
            .load_state(instance_id)?
            .ok_or_else(|| FsmPersistenceError::NotFound(instance_id.to_string()))?;

        // 查找 FSM 定义
        let fsm = self.definitions.read().await.get(&state.fsm_id).cloned().ok_or_else(|| {
            FsmPersistenceError::NotFound(format!("FSM 定义未找到: {}", state.fsm_id))
        })?;

        // 创建执行器
        let executor = Arc::new(FsmExecutor::from_snapshot(fsm, state));

        // 恢复决策日志
        self.decision_logs.write().await.insert(instance_id.to_string(), logs);

        // 注册执行器
        self.executors.write().await.insert(instance_id.to_string(), executor.clone());

        Ok(executor)
    }

    /// 获取执行器（如果不存在则尝试加载）
    pub async fn get_executor(
        &self,
        instance_id: &str,
    ) -> Result<Arc<FsmExecutor>, FsmPersistenceError> {
        // 先检查内存
        if let Some(executor) = self.executors.read().await.get(instance_id).cloned() {
            return Ok(executor);
        }

        // 尝试从持久化加载
        self.load_instance(instance_id).await
    }

    /// 保存实例状态
    pub async fn save_instance(&self, instance_id: &str) -> Result<(), FsmPersistenceError> {
        let executor = self.get_executor(instance_id).await?;
        let state = executor.current_state().await;
        let logs = self.decision_logs.read().await.get(instance_id).cloned().unwrap_or_default();
        self.persistence.save_state(&state, &logs)
    }

    /// 转移状态（带决策日志记录）
    pub async fn transition(
        &self,
        instance_id: &str,
        target_state_id: &str,
        context: Option<axagent_harness::business_state_machine::FsmContext>,
    ) -> Result<(), axagent_harness::business_state_machine::FsmTransitionError> {
        let executor = self.get_executor(instance_id).await.map_err(|e| {
            axagent_harness::business_state_machine::FsmTransitionError::InvalidTransition {
                from: String::new(),
                to: format!("加载实例失败: {e}"),
            }
        })?;
        let from_state = executor.current_state().await.current_state_id.clone();

        // 执行转移
        executor.transition_to(target_state_id, context.clone()).await?;

        // 记录决策日志
        let to_state = target_state_id.to_string();
        let decision_log = FsmDecisionLog {
            id: generate_id(),
            instance_id: instance_id.to_string(),
            timestamp_ms: current_timestamp_ms(),
            decision_type: FsmDecisionType::Transition,
            from_state,
            to_state: Some(to_state),
            context,
            description: Some("状态转移".to_string()),
        };

        self.decision_logs
            .write()
            .await
            .entry(instance_id.to_string())
            .or_default()
            .push(decision_log);

        Ok(())
    }

    /// 记录守卫条件评估决策
    pub async fn record_guard_decision(
        &self,
        instance_id: &str,
        from_state: &str,
        allowed: bool,
        context: Option<axagent_harness::business_state_machine::FsmContext>,
        description: Option<String>,
    ) {
        let desc = description
            .unwrap_or_else(|| format!("守卫条件评估: {}", if allowed { "通过" } else { "拒绝" }));

        let decision_log = FsmDecisionLog {
            id: generate_id(),
            instance_id: instance_id.to_string(),
            timestamp_ms: current_timestamp_ms(),
            decision_type: FsmDecisionType::GuardEvaluation,
            from_state: from_state.to_string(),
            to_state: None,
            context,
            description: Some(desc),
        };

        self.decision_logs
            .write()
            .await
            .entry(instance_id.to_string())
            .or_default()
            .push(decision_log);
    }

    /// 获取决策日志（用于时间旅行）
    pub async fn get_decision_logs(&self, instance_id: &str) -> Vec<FsmDecisionLog> {
        self.decision_logs.read().await.get(instance_id).cloned().unwrap_or_default()
    }

    /// 获取实例信息列表
    pub async fn list_instances(&self) -> Vec<FsmInstanceInfo> {
        let executors = self.executors.read().await;
        let mut instances = Vec::new();

        for (instance_id, executor) in executors.iter() {
            let state = executor.current_state().await;
            let status = if state.is_completed {
                FsmInstanceStatus::Completed
            } else {
                FsmInstanceStatus::Running
            };

            instances.push(FsmInstanceInfo {
                instance_id: instance_id.clone(),
                fsm_id: state.fsm_id,
                current_state_id: state.current_state_id,
                status,
                created_at_ms: state.created_at_ms,
                updated_at_ms: state.updated_at_ms,
            });
        }

        instances
    }

    /// 删除实例
    pub async fn delete_instance(&self, instance_id: &str) -> Result<(), FsmPersistenceError> {
        self.executors.write().await.remove(instance_id);
        self.decision_logs.write().await.remove(instance_id);
        self.persistence.delete_instance(instance_id)?;
        Ok(())
    }

    /// 重置实例
    pub async fn reset_instance(&self, instance_id: &str) -> Result<(), FsmPersistenceError> {
        let executor = self.get_executor(instance_id).await?;
        executor.reset().await;

        // 记录重置决策
        let state = executor.current_state().await;
        let decision_log = FsmDecisionLog {
            id: generate_id(),
            instance_id: instance_id.to_string(),
            timestamp_ms: current_timestamp_ms(),
            decision_type: FsmDecisionType::Reset,
            from_state: state.current_state_id.clone(),
            to_state: Some(state.current_state_id),
            context: None,
            description: Some("FSM 实例重置".to_string()),
        };

        self.decision_logs
            .write()
            .await
            .entry(instance_id.to_string())
            .or_default()
            .push(decision_log);

        // 持久化
        let state = executor.current_state().await;
        let logs = self.decision_logs.read().await.get(instance_id).cloned().unwrap_or_default();
        self.persistence.save_state(&state, &logs)?;

        Ok(())
    }

    /// 获取实例历史状态轨迹（时间旅行）
    pub async fn get_state_timeline(&self, instance_id: &str) -> Vec<FsmRuntimeState> {
        let executor = match self.get_executor(instance_id).await {
            Ok(ex) => ex,
            Err(_) => return Vec::new(),
        };

        let mut timeline = Vec::new();
        let history = executor.transition_history().await;

        // 从初始状态开始，重建每个时刻的状态
        let state = executor.current_state().await;
        let fsm_id = state.fsm_id.clone();
        let instance_id_clone = state.instance_id.clone();
        let created_at = state.created_at_ms;

        for record in &history {
            let timeline_state = FsmRuntimeState {
                current_state_id: record.to.clone(),
                previous_state_id: Some(record.from.clone()),
                transition_history: vec![record.clone()],
                instance_id: instance_id_clone.clone(),
                fsm_id: fsm_id.clone(),
                created_at_ms: created_at,
                updated_at_ms: record.timestamp_ms,
                is_completed: false,
            };
            timeline.push(timeline_state);
        }

        timeline
    }
}

impl Default for FsmManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── 辅助函数 ──

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn generate_id() -> String {
    format!("log_{}", uuid::Uuid::new_v4())
}

// ── 单元测试 ──

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::business_state_machine::BusinessStateMachine;

    #[tokio::test]
    async fn test_fsm_manager_creation() {
        let manager = FsmManager::new();
        let instances = manager.list_instances().await;
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_register_definition() {
        let manager = FsmManager::new();
        let fsm = BusinessStateMachine::approval_flow();
        let result = manager.register_definition(fsm).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_instance() {
        let manager = FsmManager::new();
        let fsm = BusinessStateMachine::approval_flow();
        manager.register_definition(fsm).await.unwrap();

        let instance_id = manager.create_instance("approval_flow", "test-1").await.unwrap();
        assert_eq!(instance_id, "test-1");

        let instances = manager.list_instances().await;
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_id, "test-1");
    }

    #[tokio::test]
    async fn test_transition() {
        let manager = FsmManager::new();
        let fsm = BusinessStateMachine::approval_flow();
        manager.register_definition(fsm).await.unwrap();

        manager.create_instance("approval_flow", "test-1").await.unwrap();
        let result = manager.transition("test-1", "under_review", None).await;
        assert!(result.is_ok());

        let executor = manager.get_executor("test-1").await.unwrap();
        let state = executor.current_state().await;
        assert_eq!(state.current_state_id, "under_review");
    }

    #[tokio::test]
    async fn test_decision_logs() {
        let manager = FsmManager::new();
        let fsm = BusinessStateMachine::approval_flow();
        manager.register_definition(fsm).await.unwrap();

        manager.create_instance("approval_flow", "test-1").await.unwrap();
        manager.transition("test-1", "under_review", None).await.unwrap();
        manager.transition("test-1", "approved", None).await.unwrap();

        let logs = manager.get_decision_logs("test-1").await;
        // 应包含: Create + Transition + Transition = 3 条
        assert_eq!(logs.len(), 3);
        assert!(matches!(logs[0].decision_type, FsmDecisionType::Create));
        assert!(matches!(logs[1].decision_type, FsmDecisionType::Transition));
        assert!(matches!(logs[2].decision_type, FsmDecisionType::Transition));
    }

    #[tokio::test]
    async fn test_load_and_save() {
        let manager = FsmManager::new();
        let fsm = BusinessStateMachine::approval_flow();
        manager.register_definition(fsm).await.unwrap();

        manager.create_instance("approval_flow", "test-1").await.unwrap();
        manager.transition("test-1", "under_review", None).await.unwrap();

        // 保存
        manager.save_instance("test-1").await.unwrap();

        // 从持久化加载
        let executor = manager.load_instance("test-1").await.unwrap();
        let state = executor.current_state().await;
        assert_eq!(state.current_state_id, "under_review");
    }

    #[tokio::test]
    async fn test_delete_instance() {
        let manager = FsmManager::new();
        let fsm = BusinessStateMachine::approval_flow();
        manager.register_definition(fsm).await.unwrap();

        manager.create_instance("approval_flow", "test-1").await.unwrap();
        assert_eq!(manager.list_instances().await.len(), 1);

        manager.delete_instance("test-1").await.unwrap();
        assert_eq!(manager.list_instances().await.len(), 0);
    }

    #[tokio::test]
    async fn test_reset_instance() {
        let manager = FsmManager::new();
        let fsm = BusinessStateMachine::approval_flow();
        manager.register_definition(fsm).await.unwrap();

        manager.create_instance("approval_flow", "test-1").await.unwrap();
        manager.transition("test-1", "under_review", None).await.unwrap();

        let executor = manager.get_executor("test-1").await.unwrap();
        assert_eq!(executor.current_state().await.current_state_id, "under_review");

        manager.reset_instance("test-1").await.unwrap();
        let executor = manager.get_executor("test-1").await.unwrap();
        assert_eq!(executor.current_state().await.current_state_id, "submitted");
    }

    #[tokio::test]
    async fn test_invalid_transition_blocked() {
        let manager = FsmManager::new();
        let fsm = BusinessStateMachine::approval_flow();
        manager.register_definition(fsm).await.unwrap();

        manager.create_instance("approval_flow", "test-1").await.unwrap();

        // 非法转移
        let result = manager.transition("test-1", "approved", None).await;
        assert!(result.is_err());
    }
}
