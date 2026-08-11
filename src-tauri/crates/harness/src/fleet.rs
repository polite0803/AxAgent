// SPDX-License-Identifier: AGPL-3.0-only

//! Fleet 抽象契约 — 多办公室（AI 团队）协作的统一接口。
//!
//! ## 设计动机
//!
//! AxAgent 已有的多 Agent 能力分散在三个实现：
//! - **`runtime/swarm/Team`**：跨进程团队（纯内存，无持久化，无生命周期管理）
//! - **`trajectory/SubAgentRegistry`**：SubAgent 层级树（JSON 文件持久化）
//! - **`agent/AgentSession.team_id`**：单 Agent 会话的团队归属字段（无强约束）
//!
//! 三者各自为政，缺少统一的「舰队」一等公民抽象，导致：
//! 1. 无法跨 crate 查询「某舰队下所有成员状态」
//! 2. Team 仅内存态，重启丢失
//! 3. 缺少舰队级生命周期（暂停/恢复/停止）
//! 4. 缺少对话级智能路由（Dispatcher）
//!
//! 本模块在 harness 层定义：
//! 1. **共享 DTO** — `Fleet` / `FleetMember` / `FleetStatus` / `FleetMemberStatus`
//! 2. **`FleetRepository` trait** — 舰队与成员的持久化与查询接口
//! 3. **`IntentDispatcher` trait + `DispatchEvent`** — 群聊智能路由的统一抽象
//!
//! ## 实现方
//!
//! - `axagent_trajectory::SeaOrmFleetRepository` → 实现 `FleetRepository` trait（SeaORM 持久化，SQLite + PostgreSQL 双兼容）
//! - `axagent_agent::LlmDispatcher` → 实现 `IntentDispatcher` trait（LLM Function Calling 路由）
//! - wiring 层在 `init/state.rs` 注入到 `AppState`
//!
//! ## 接入计划
//!
//! - **P0**：trait + DTO 定义（本阶段）+ SeaORM 实现
//! - **P1**：LlmDispatcher 实现 + Tauri 命令暴露
//! - **P2**：前端 Phaser 像素办公室 + 智能路由对话

use serde::{Deserialize, Serialize};

// ============================================================================
// 共享 DTO
// ============================================================================

/// 舰队（办公室）状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetStatus {
    /// 活跃 — 成员可接收任务
    #[default]
    Active,
    /// 暂停 — 整个舰队停止接收新任务，运行中任务继续
    Paused,
    /// 停止 — 舰队已停止，所有成员离线
    Stopped,
}

/// 舰队成员状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetMemberStatus {
    /// 空闲 — 可接收任务
    #[default]
    Idle,
    /// 忙碌 — 正在执行任务
    Busy,
    /// 暂停 — 用户手动暂停，不接收新任务
    Paused,
    /// 错误 — 上次任务失败
    Error,
    /// 离线 — 成员已离开舰队
    Offline,
}

/// 舰队元数据 — 业务层可扩展信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetMetadata {
    /// 业务描述
    pub description: String,
    /// 最大成员数（0 表示无限制）
    pub max_members: u32,
    /// 协作策略名称（由下游业务系统填充，如 "ecommerce_ops" / "customer_service"）
    pub strategy: Option<String>,
    /// 自定义标签
    pub tags: Vec<String>,
}

/// 舰队（办公室）— 一个正在运行的 AI 团队
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fleet {
    /// 唯一 ID（UUID）
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 场景模板 slug（可选，下游业务系统可填）
    pub scene_template_slug: Option<String>,
    /// 舰队状态
    pub status: FleetStatus,
    /// 创建时间（Unix 毫秒）
    pub created_at: i64,
    /// 更新时间（Unix 毫秒）
    pub updated_at: i64,
    /// 业务元数据
    pub metadata: FleetMetadata,
}

/// 舰队成员 — 办公室里的一个 agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetMember {
    /// 唯一 ID（UUID）
    pub id: String,
    /// 所属舰队 ID
    pub fleet_id: String,
    /// 关联的 AgentSession ID（由 SessionManager 创建）
    pub agent_id: String,
    /// agent slug（业务标识，用于 Dispatcher 路由）
    pub agent_slug: String,
    /// 显示名称
    pub display_name: String,
    /// 角色描述（注入到 Dispatcher prompt；与 agent_profile_id 二选一，均可）
    pub role: String,
    /// 关联的 AgentProfile ID（AgentProfile = 角色 + 专家组合，定义成员智能体身份）
    pub agent_profile_id: Option<String>,
    /// 房间 ID（前端 Phaser 渲染位置，如 "manager" / "meeting"）
    pub room_id: String,
    /// 成员状态
    pub status: FleetMemberStatus,
    /// 加入时间（Unix 毫秒）
    pub joined_at: i64,
    /// 今日 token 用量（实时累计，由 Dispatcher 事件更新）
    pub today_tokens: u64,
    /// 累计 token 用量
    pub total_tokens: u64,
}

// ============================================================================
// FleetRepository trait
// ============================================================================

/// 舰队持久化与查询的统一接口。
///
/// ## 异步设计
///
/// 所有方法都是 `async fn`，实现方使用 `tokio::sync::RwLock` 提供内部可变性。
/// SeaORM 实现直接走数据库连接池，无需外层锁。
///
/// ## 错误处理
///
/// 返回 `Result<T, String>`，实现方把内部错误转换为 `String` 返回，
/// 不传播 panic，符合 harness 错误隔离约定（与 `SharedBlackboard` 一致）。
#[async_trait::async_trait]
pub trait FleetRepository: Send + Sync {
    /// 创建舰队
    async fn create_fleet(&self, fleet: Fleet) -> Result<Fleet, String>;

    /// 列出所有舰队（可选状态过滤）
    async fn list_fleets(&self, status_filter: Option<FleetStatus>) -> Result<Vec<Fleet>, String>;

    /// 获取舰队详情
    async fn get_fleet(&self, fleet_id: &str) -> Result<Option<Fleet>, String>;

    /// 更新舰队状态
    async fn update_fleet_status(&self, fleet_id: &str, status: FleetStatus) -> Result<(), String>;

    /// 删除舰队（同时删除所有成员）
    async fn delete_fleet(&self, fleet_id: &str) -> Result<(), String>;

    /// 列出舰队下所有成员
    async fn list_members(&self, fleet_id: &str) -> Result<Vec<FleetMember>, String>;

    /// 添加成员到舰队
    async fn add_member(&self, member: FleetMember) -> Result<FleetMember, String>;

    /// 获取单个成员
    async fn get_member(&self, member_id: &str) -> Result<Option<FleetMember>, String>;

    /// 更新成员状态
    async fn update_member_status(
        &self,
        member_id: &str,
        status: FleetMemberStatus,
    ) -> Result<(), String>;

    /// 累加成员 token 用量（today_tokens + total_tokens 同时累加）
    async fn add_member_tokens(&self, member_id: &str, tokens: u64) -> Result<(), String>;

    /// 重置成员今日 token（每日定时任务调用）
    async fn reset_daily_tokens(&self, fleet_id: &str) -> Result<(), String>;

    /// 移除成员
    async fn remove_member(&self, member_id: &str) -> Result<(), String>;
}

// ============================================================================
// IntentDispatcher trait + DispatchEvent
// ============================================================================

/// 调度事件 — Dispatcher 在路由与执行过程中产生的事件流
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DispatchEvent {
    /// 路由决策 — 调度员决定路由到某个 agent
    #[serde(rename_all = "camelCase")]
    Routing {
        /// 目标 agent slug
        agent_slug: String,
        /// 目标 agent ID
        agent_id: String,
        /// 目标房间 ID（前端据此移动精灵）
        room_id: String,
        /// 任务摘要（注入到 agent 的 prompt）
        task_summary: String,
    },
    /// Agent 处理中的中间状态
    #[serde(rename_all = "camelCase")]
    Process {
        /// agent slug
        agent_slug: String,
        /// agent ID
        agent_id: String,
        /// 状态描述
        status: String,
    },
    /// Agent 回复消息
    #[serde(rename_all = "camelCase")]
    AgentMessage {
        /// agent slug
        agent_slug: String,
        /// agent ID
        agent_id: String,
        /// 回复内容
        content: String,
    },
    /// Agent 状态变更
    #[serde(rename_all = "camelCase")]
    AgentStatus {
        /// agent slug
        agent_slug: String,
        /// agent ID
        agent_id: String,
        /// 新状态
        status: FleetMemberStatus,
    },
    /// Token 用量上报
    #[serde(rename_all = "camelCase")]
    TokenUsage {
        /// agent slug
        agent_slug: String,
        /// agent ID
        agent_id: String,
        /// 输入 token 数
        input_tokens: u64,
        /// 输出 token 数
        output_tokens: u64,
    },
    /// 流结束
    Complete,
    /// 错误
    Error {
        /// 错误消息
        message: String,
    },
}

/// 聊天消息（Dispatcher 输入）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchChatMessage {
    /// 角色：user / assistant / system
    pub role: String,
    /// 消息内容
    pub content: String,
    /// 关联的 agent slug（assistant 消息才有）
    pub agent_slug: Option<String>,
}

/// 群聊智能路由的统一抽象。
///
/// 实现方接收用户消息 + 历史，通过 LLM Function Calling 决定路由到哪个 agent，
/// 然后调用 `SessionManager::run_turn_with_tools` 执行，并产生事件流。
///
/// ## 设计要点
///
/// - **流式输出**：`dispatch_stream` 返回 `DispatchEvent` 流，前端 SSE 消费
/// - **动态 prompt**：每次调用都从 `FleetRepository` 重新加载成员列表构建 prompt
/// - **错误隔离**：内部错误转为 `DispatchEvent::Error` 事件，不中断流
///
/// ## 接入计划
///
/// - **P0**：trait 定义（本阶段）
/// - **P1**：`axagent_agent::LlmDispatcher` 实现
#[async_trait::async_trait]
pub trait IntentDispatcher: Send + Sync {
    /// 流式调度 — 返回事件流，调用方消费 SSE
    async fn dispatch_stream(
        &self,
        fleet_id: &str,
        user_message: &str,
        history: Vec<DispatchChatMessage>,
    ) -> Result<Vec<DispatchEvent>, String>;

    /// 直接 DM 指定 agent（绕过 Dispatcher 路由）
    async fn direct_message_stream(
        &self,
        fleet_id: &str,
        agent_slug: &str,
        user_message: &str,
        history: Vec<DispatchChatMessage>,
    ) -> Result<Vec<DispatchEvent>, String>;
}

/// Fleet 意图分类 LLM 调用 trait（供 `LlmDispatcher` 注入）。
///
/// ## 设计动机
///
/// `axagent_agent` 是 consumer crate，按铁律只能依赖 `axagent-harness`，
/// 不能直接依赖 `axagent-providers`。因此 LLM 调用能力通过本 trait 注入：
/// wiring 层实现此 trait（包装 `ProviderLlmBridge` 或直接走 `ProviderAdapter`），
/// 在 `init/state.rs` 注入到 `LlmDispatcher`。
///
/// ## 输出约定
///
/// `route()` 返回 LLM 原始文本响应，期望是 JSON：
/// ```json
/// {"agent_slug": "copywriter", "reason": "用户要求写产品文案"}
/// ```
/// 解析失败时由 `LlmDispatcher` 兜底为第一个可用成员。
#[async_trait::async_trait]
pub trait FleetIntentLlm: Send + Sync {
    /// 调用 LLM 做意图分类
    ///
    /// - `system_prompt`: 系统提示词（含成员列表与路由规则）
    /// - `user_prompt`: 用户消息（可选含历史摘要）
    /// - 返回：LLM 原始响应文本
    async fn route(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String>;
}

/// `FleetIntentLlm` 的空实现 — 用于测试 / 离线模式（始终返回空字符串）
pub struct NoopFleetIntentLlm;

#[async_trait::async_trait]
impl FleetIntentLlm for NoopFleetIntentLlm {
    async fn route(&self, _system_prompt: &str, _user_prompt: &str) -> Result<String, String> {
        Ok(String::new())
    }
}

// ============================================================================
// Noop 实现（用于测试 / 离线模式）
// ============================================================================

/// 空实现 — 返回空结果，不执行任何操作
pub struct NoopFleetRepository;

#[async_trait::async_trait]
impl FleetRepository for NoopFleetRepository {
    async fn create_fleet(&self, fleet: Fleet) -> Result<Fleet, String> {
        Ok(fleet)
    }
    async fn list_fleets(&self, _status_filter: Option<FleetStatus>) -> Result<Vec<Fleet>, String> {
        Ok(Vec::new())
    }
    async fn get_fleet(&self, _fleet_id: &str) -> Result<Option<Fleet>, String> {
        Ok(None)
    }
    async fn update_fleet_status(
        &self,
        _fleet_id: &str,
        _status: FleetStatus,
    ) -> Result<(), String> {
        Ok(())
    }
    async fn delete_fleet(&self, _fleet_id: &str) -> Result<(), String> {
        Ok(())
    }
    async fn list_members(&self, _fleet_id: &str) -> Result<Vec<FleetMember>, String> {
        Ok(Vec::new())
    }
    async fn add_member(&self, member: FleetMember) -> Result<FleetMember, String> {
        Ok(member)
    }
    async fn get_member(&self, _member_id: &str) -> Result<Option<FleetMember>, String> {
        Ok(None)
    }
    async fn update_member_status(
        &self,
        _member_id: &str,
        _status: FleetMemberStatus,
    ) -> Result<(), String> {
        Ok(())
    }
    async fn add_member_tokens(&self, _member_id: &str, _tokens: u64) -> Result<(), String> {
        Ok(())
    }
    async fn reset_daily_tokens(&self, _fleet_id: &str) -> Result<(), String> {
        Ok(())
    }
    async fn remove_member(&self, _member_id: &str) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fleet_status_serde() {
        let status = FleetStatus::Active;
        let json = serde_json::to_string(&status).expect("测试：JSON序列化应成功");
        assert_eq!(json, "\"active\"");
        let de: FleetStatus = serde_json::from_str(&json).expect("测试：JSON反序列化应成功");
        assert_eq!(de, FleetStatus::Active);
    }

    #[test]
    fn test_member_status_serde() {
        let status = FleetMemberStatus::Busy;
        let json = serde_json::to_string(&status).expect("测试：JSON序列化应成功");
        assert_eq!(json, "\"busy\"");
    }

    #[test]
    fn test_dispatch_event_tagged_enum() {
        let event = DispatchEvent::Routing {
            agent_slug: "copywriter".to_string(),
            agent_id: "agt_001".to_string(),
            room_id: "showroom".to_string(),
            task_summary: "写产品文案".to_string(),
        };
        let json = serde_json::to_string(&event).expect("测试：JSON序列化应成功");
        assert!(json.contains("\"type\":\"routing\""));
        assert!(json.contains("\"agentSlug\":\"copywriter\""));
    }

    #[test]
    fn test_noop_repository() {
        let noop = NoopFleetRepository;
        let result = futures::executor::block_on(noop.list_fleets(None)).expect("测试应成功");
        assert!(result.is_empty());
    }

    #[test]
    fn test_fleet_metadata_default() {
        let meta = FleetMetadata::default();
        assert_eq!(meta.max_members, 0);
        assert!(meta.strategy.is_none());
        assert!(meta.tags.is_empty());
    }
}
