// SPDX-License-Identifier: AGPL-3.0-only

//! G16 DojoExtension Protocol — 扩展接入契约层
//!
//! 对齐 DojoAgents 的 DojoExtension Protocol：外部扩展（IDE 插件 / 浏览器扩展 /
//! 第三方服务）通过统一 trait 接入 AxInvest，提供 5 项能力：
//!
//! 1. `health` — 健康检查（心跳 / 版本 / 能力探测）
//! 2. `tool_specs` — 扩展暴露的工具规格（OpenAI Function Calling 格式）
//! 3. `execute_command` — 执行扩展命令（DAG 化的命令路由）
//! 4. `dashboard_cards` — 仪表盘卡片贡献（扩展向前端 Dashboard 注入卡片）
//! 5. `prompt_context` — 提示词上下文贡献（扩展向 LLM system prompt 注入片段）
//!
//! ## 设计原则
//!
//! - **零业务依赖**：trait 仅依赖 serde / async-trait，符合 harness 角色
//! - **声明式能力**：扩展通过 trait 方法声明能力，运行时按需调用
//! - **注册表机制**：通过 `DojoExtensionRegistry` 管理多个扩展，按 extension_id 索引
//! - **可选方法**：所有方法都有默认实现，扩展按需 override
//!
//! ## 与现有 trait 的关系
//!
//! - `PluginHook`：聚焦生命周期 hook（on_session_start / on_tool_call 等），事件驱动
//! - `DojoExtension`：聚焦能力声明（health / tools / cards / prompt），声明式
//! - 两者正交，可同时实现：扩展可既挂 hook 又声明能力
//!
//! ## 使用示例
//!
//! ```ignore
//! use axagent_harness::dojo_extension::{DojoExtension, DojoExtensionHealth, DojoCommandSpec};
//!
//! struct MyMarketDataExtension;
//!
//! #[async_trait::async_trait]
//! impl DojoExtension for MyMarketDataExtension {
//!     fn extension_id(&self) -> &str { "my-market-data" }
//!     fn version(&self) -> &str { "1.0.0" }
//!
//!     async fn health(&self) -> DojoExtensionHealth {
//!         DojoExtensionHealth::healthy("ok")
//!     }
//!
//!     async fn execute_command(&self, command: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
//!         match command {
//!             "get_latest_quote" => Ok(serde_json::json!({ "price": 100.0 })),
//!             _ => Err(format!("未知命令: {command}")),
//!         }
//!     }
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ── 健康状态 DTO ───────────────────────────────────────────────────────────

/// 扩展健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum DojoExtensionHealth {
    /// 健康
    Healthy {
        /// 人类可读消息（如 "ok"）
        message: String,
        /// 版本号
        version: String,
    },
    /// 降级（部分能力可用）
    Degraded {
        message: String,
        /// 不可用的能力列表
        unavailable_capabilities: Vec<String>,
    },
    /// 不可用
    Unhealthy {
        message: String,
        /// 错误代码
        error_code: Option<String>,
    },
}

impl DojoExtensionHealth {
    /// 快速构造 Healthy 状态
    pub fn healthy(message: impl Into<String>) -> Self {
        Self::Healthy { message: message.into(), version: "unknown".to_string() }
    }

    /// 快速构造 Unhealthy 状态
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self::Unhealthy { message: message.into(), error_code: None }
    }

    /// 是否健康（含降级）
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Healthy { .. } | Self::Degraded { .. })
    }
}

// ── 工具规格 DTO ───────────────────────────────────────────────────────────

/// 扩展暴露的工具规格（OpenAI Function Calling 兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DojoToolSpec {
    /// 工具名（如 "get_latest_quote"）
    pub name: String,
    /// 工具描述（LLM 可见）
    pub description: String,
    /// JSON Schema 参数定义
    pub parameters: serde_json::Value,
    /// 是否需要用户授权
    pub requires_approval: bool,
    /// 是否为幂等工具（用于 Guardrail）
    pub idempotent: bool,
}

impl DojoToolSpec {
    /// 快速构造一个简单工具规格
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: serde_json::json!({ "type": "object", "properties": {} }),
            requires_approval: false,
            idempotent: true,
        }
    }
}

// ── 命令规格 DTO ───────────────────────────────────────────────────────────

/// 扩展命令规格（用于 `execute_command` 路由）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DojoCommandSpec {
    /// 命令名
    pub name: String,
    /// 命令描述
    pub description: String,
    /// 参数 schema
    pub args_schema: serde_json::Value,
    /// 返回值 schema
    pub returns_schema: Option<serde_json::Value>,
    /// 是否危险操作（需用户确认）
    pub destructive: bool,
}

// ── Dashboard 卡片 DTO ─────────────────────────────────────────────────────

/// 扩展贡献的 Dashboard 卡片
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DojoDashboardCard {
    /// 卡片 ID（前端按此 ID 渲染）
    pub card_id: String,
    /// 卡片标题（i18n key 或纯文本）
    pub title: String,
    /// 卡片描述
    pub description: Option<String>,
    /// 卡片类型：chart / table / stat / list / iframe
    pub card_type: DojoDashboardCardType,
    /// 卡片数据（前端按 card_type 渲染）
    pub data: serde_json::Value,
    /// 排序权重（数字越小越靠前）
    pub sort_order: i32,
    /// 卡片刷新间隔（秒），None 表示不自动刷新
    pub refresh_interval_secs: Option<u32>,
    /// 卡片所属场景（与 VisualizationPolicy sceneId 对齐）
    pub scene_id: Option<String>,
}

/// 卡片类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DojoDashboardCardType {
    /// 图表（对应 VizBlock）
    Chart,
    /// 表格
    Table,
    /// 统计数字
    Stat,
    /// 列表
    List,
    /// 嵌入式网页
    Iframe,
}

// ── Prompt 上下文贡献 DTO ──────────────────────────────────────────────────

/// 扩展贡献的 LLM system prompt 上下文片段
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DojoPromptContext {
    /// 片段 ID（用于去重 / 排序）
    pub context_id: String,
    /// 片段内容
    pub content: String,
    /// 优先级（数字越大越靠前）
    pub priority: i32,
    /// 是否截断（超过 token 上限时是否优先保留）
    pub sticky: bool,
    /// 关联的标签（用于按场景过滤）
    pub tags: Vec<String>,
}

// ── DojoExtension trait ───────────────────────────────────────────────────

/// DojoExtension Protocol — 扩展接入 AxInvest 的统一契约
///
/// 所有方法都有默认实现，扩展按需 override。
#[async_trait]
pub trait DojoExtension: Send + Sync {
    /// 扩展唯一 ID（如 "my-market-data"）
    fn extension_id(&self) -> &str;

    /// 扩展版本（语义化版本）
    fn version(&self) -> &str {
        "0.0.0"
    }

    /// 扩展显示名（人类可读）
    fn display_name(&self) -> &str {
        self.extension_id()
    }

    /// 扩展描述
    fn description(&self) -> &str {
        ""
    }

    /// 健康检查 — 默认返回 Healthy
    async fn health(&self) -> DojoExtensionHealth {
        DojoExtensionHealth::healthy("ok")
    }

    /// 声明的工具规格列表 — 默认空
    async fn tool_specs(&self) -> Vec<DojoToolSpec> {
        Vec::new()
    }

    /// 声明的命令规格列表 — 默认空
    async fn command_specs(&self) -> Vec<DojoCommandSpec> {
        Vec::new()
    }

    /// 执行命令 — 默认返回 "未实现"
    async fn execute_command(
        &self,
        command: &str,
        _args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Err(format!("扩展 {} 未实现命令: {command}", self.extension_id()))
    }

    /// 贡献的 Dashboard 卡片 — 默认空
    async fn dashboard_cards(&self, _scene_id: Option<&str>) -> Vec<DojoDashboardCard> {
        Vec::new()
    }

    /// 贡献的 Prompt 上下文片段 — 默认空
    async fn prompt_context(&self, _scene_id: Option<&str>) -> Vec<DojoPromptContext> {
        Vec::new()
    }

    /// 能力声明（用于 health 检查时列举）
    fn capabilities(&self) -> Vec<String> {
        vec![]
    }
}

// ── DojoExtensionRegistry — 扩展注册表 ────────────────────────────────────

/// 扩展注册表 — 管理所有已注册的 DojoExtension
///
/// 通过 `tokio::sync::RwLock` 保护内部 HashMap，支持并发读写。
/// 注册表本身是 `Send + Sync`，可作为 `Arc<DojoExtensionRegistry>` 共享。
#[derive(Default)]
pub struct DojoExtensionRegistry {
    extensions: RwLock<HashMap<String, Arc<dyn DojoExtension>>>,
}

impl DojoExtensionRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册扩展（按 extension_id 索引，重复注册会覆盖并返回旧值）
    pub async fn register(
        &self,
        extension: Arc<dyn DojoExtension>,
    ) -> Option<Arc<dyn DojoExtension>> {
        let id = extension.extension_id().to_string();
        let mut map = self.extensions.write().await;
        map.insert(id, extension)
    }

    /// 注销扩展
    pub async fn unregister(&self, extension_id: &str) -> Option<Arc<dyn DojoExtension>> {
        let mut map = self.extensions.write().await;
        map.remove(extension_id)
    }

    /// 获取单个扩展
    pub async fn get(&self, extension_id: &str) -> Option<Arc<dyn DojoExtension>> {
        let map = self.extensions.read().await;
        map.get(extension_id).cloned()
    }

    /// 列出所有已注册扩展
    pub async fn list(&self) -> Vec<Arc<dyn DojoExtension>> {
        let map = self.extensions.read().await;
        map.values().cloned().collect()
    }

    /// 列出所有已注册 extension_id
    pub async fn list_ids(&self) -> Vec<String> {
        let map = self.extensions.read().await;
        map.keys().cloned().collect()
    }

    /// 聚合所有扩展的健康状态
    pub async fn health_all(&self) -> HashMap<String, DojoExtensionHealth> {
        let map = self.extensions.read().await;
        let mut out = HashMap::with_capacity(map.len());
        for (id, ext) in map.iter() {
            out.insert(id.clone(), ext.health().await);
        }
        out
    }

    /// 聚合所有扩展的工具规格
    pub async fn tool_specs_all(&self) -> Vec<DojoToolSpec> {
        let map = self.extensions.read().await;
        let mut out = Vec::new();
        for ext in map.values() {
            out.extend(ext.tool_specs().await);
        }
        out
    }

    /// 聚合所有扩展的命令规格
    pub async fn command_specs_all(&self) -> Vec<DojoCommandSpec> {
        let map = self.extensions.read().await;
        let mut out = Vec::new();
        for ext in map.values() {
            out.extend(ext.command_specs().await);
        }
        out
    }

    /// 调用指定扩展的命令
    pub async fn execute(
        &self,
        extension_id: &str,
        command: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let ext =
            self.get(extension_id).await.ok_or_else(|| format!("扩展未注册: {extension_id}"))?;
        ext.execute_command(command, args).await
    }

    /// 聚合所有扩展的 Dashboard 卡片（按 scene_id 过滤）
    pub async fn dashboard_cards_all(&self, scene_id: Option<&str>) -> Vec<DojoDashboardCard> {
        let map = self.extensions.read().await;
        let mut out = Vec::new();
        for ext in map.values() {
            out.extend(ext.dashboard_cards(scene_id).await);
        }
        // 按 sort_order 升序
        out.sort_by_key(|c| c.sort_order);
        out
    }

    /// 聚合所有扩展的 Prompt 上下文（按 scene_id 过滤）
    pub async fn prompt_context_all(&self, scene_id: Option<&str>) -> Vec<DojoPromptContext> {
        let map = self.extensions.read().await;
        let mut out = Vec::new();
        for ext in map.values() {
            out.extend(ext.prompt_context(scene_id).await);
        }
        // 按 priority 降序
        out.sort_by_key(|b| std::cmp::Reverse(b.priority));
        out
    }
}

// ── 全局注册表（单例） ─────────────────────────────────────────────────────

use std::sync::OnceLock;

static GLOBAL_REGISTRY: OnceLock<DojoExtensionRegistry> = OnceLock::new();

/// 获取全局 DojoExtension 注册表
pub fn global_registry() -> &'static DojoExtensionRegistry {
    GLOBAL_REGISTRY.get_or_init(DojoExtensionRegistry::new)
}

// ── 测试辅助 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExt {
        id: String,
    }

    #[async_trait]
    impl DojoExtension for TestExt {
        fn extension_id(&self) -> &str {
            &self.id
        }
        async fn execute_command(
            &self,
            command: &str,
            _args: &serde_json::Value,
        ) -> Result<serde_json::Value, String> {
            if command == "ping" {
                Ok(serde_json::json!({ "pong": true }))
            } else {
                Err("未知命令".to_string())
            }
        }
    }

    #[tokio::test]
    async fn test_registry_basic() {
        let registry = DojoExtensionRegistry::new();
        let ext: Arc<dyn DojoExtension> = Arc::new(TestExt { id: "test-ext".to_string() });
        assert!(registry.register(ext).await.is_none());

        let result = registry.execute("test-ext", "ping", &serde_json::json!({})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["pong"], true);

        let result = registry.execute("test-ext", "unknown", &serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check() {
        let ext = TestExt { id: "health-test".to_string() };
        let health = ext.health().await;
        assert!(health.is_available());
    }

    #[test]
    fn test_health_status_helpers() {
        let h = DojoExtensionHealth::healthy("ok");
        assert!(h.is_available());

        let u = DojoExtensionHealth::unhealthy("down");
        assert!(!u.is_available());
    }
}
