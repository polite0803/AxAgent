// SPDX-License-Identifier: AGPL-3.0-only
//! 分层 Prompt 模板引擎 — 三层路由树的 Prompt 注入层
//!
//! # 架构
//! ```text
//! System Prompt 分层注入:
//! ┌─────────────────────────────────────────┐
//! │ Layer 1: Domain Prompt (业务域层)        │
//! │  "你是数据分析专家..."                    │
//! ├─────────────────────────────────────────┤
//! │ Layer 2: Cluster Prompt (功能集群层)     │
//! │  "在报表生成方面..."                      │
//! ├─────────────────────────────────────────┤
//! │ Layer 3: Capability Prompt (具体能力层)  │
//! │  "执行以下工作流: ..."                    │
//! ├─────────────────────────────────────────┤
//! │ Layer 4: Context Prompt (上下文层)       │
//! │  "用户当前订单状态为..."                   │
//! └─────────────────────────────────────────┘
//! ```
//!
//! # Token 预算管理
//! - 每层有独立的 token 上限
//! - 超出预算时自动截断或精简
//! - 优先级: Capability > Cluster > Domain > Context

use crate::capability::{CapabilityDomain, CapabilityPassportDto};
use crate::capability_clusters::CapabilityCluster;
use crate::cluster_router::ClusterRoutingResult;
use crate::domain_router::DomainRoutingResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ── Prompt 层定义 ──────────────────────────────────

/// Prompt 分层（从低到高优先级）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptLayer {
    /// 业务域层（最底层，最通用）
    Domain,
    /// 功能集群层
    Cluster,
    /// 具体能力层（最顶层，最具体）
    Capability,
    /// 上下文层（用户上下文注入）
    Context,
}

impl PromptLayer {
    /// 默认 token 上限
    pub fn default_token_limit(&self) -> u32 {
        match self {
            PromptLayer::Domain => 500,
            PromptLayer::Cluster => 300,
            PromptLayer::Capability => 800,
            PromptLayer::Context => 200,
        }
    }

    /// 优先级（数字越大优先级越高）
    pub fn priority(&self) -> u8 {
        match self {
            PromptLayer::Domain => 1,
            PromptLayer::Cluster => 2,
            PromptLayer::Capability => 3,
            PromptLayer::Context => 4,
        }
    }
}

// ── Prompt 片段 ──────────────────────────────────

/// Prompt 片段 — 各层注入的最小单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSegment {
    /// 所属层
    pub layer: PromptLayer,
    /// 片段 ID（可选，用于去重和更新）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<String>,
    /// 片段内容
    pub content: String,
    /// 预估 token 数
    pub estimated_tokens: u32,
    /// 是否可截断（超出预算时可精简）
    #[serde(default = "default_true")]
    pub truncatable: bool,
    /// 元数据
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

impl PromptSegment {
    pub fn new(layer: PromptLayer, content: impl Into<String>) -> Self {
        let content = content.into();
        let estimated = estimate_tokens(&content);
        Self {
            layer,
            segment_id: None,
            content,
            estimated_tokens: estimated,
            truncatable: true,
            metadata: None,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.segment_id = Some(id.into());
        self
    }

    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.estimated_tokens = tokens;
        self
    }

    pub fn not_truncatable(mut self) -> Self {
        self.truncatable = false;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

// ── 分层 Prompt 组装结果 ──────────────────────────

/// 分层 Prompt 组装结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayeredPromptResult {
    /// 各层片段
    pub segments: Vec<PromptSegment>,
    /// 最终拼接的完整 System Prompt
    pub system_prompt: String,
    /// 各层 token 使用情况
    pub token_usage: Vec<LayerTokenUsage>,
    /// 总 token 数
    pub total_tokens: u32,
    /// 是否超出预算
    pub over_budget: bool,
    /// 裁剪日志（被截断的片段）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trimmed_segments: Vec<String>,
}

/// 各层 token 使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerTokenUsage {
    pub layer: PromptLayer,
    pub used_tokens: u32,
    pub limit_tokens: u32,
}

// ── Prompt 模板引擎接口 ──────────────────────────

/// 分层 Prompt 模板引擎
///
/// # 职责
/// 1. 根据路由结果加载各层模板
/// 2. 分层注入 Prompt 片段
/// 3. Token 预算管理（自动截断超预算片段）
/// 4. 生成最终 System Prompt
#[async_trait]
pub trait LayeredPromptEngine: Send + Sync {
    /// 组装完整的分层 Prompt
    ///
    /// # 参数
    /// - `domain_result`: L1 域路由结果
    /// - `cluster_result`: L2 簇路由结果
    /// - `capability`: 命中的能力护照（可选）
    /// - `context_segments`: 上下文层片段
    async fn assemble(
        &self,
        domain_result: &DomainRoutingResult,
        cluster_result: &ClusterRoutingResult,
        capability: Option<&CapabilityPassportDto>,
        context_segments: Vec<PromptSegment>,
    ) -> LayeredPromptResult;

    /// 仅组装 Domain + Cluster 层（用于路由前的预 Prompt）
    async fn assemble_pre_routing(
        &self,
        domain: CapabilityDomain,
        cluster: Option<&CapabilityCluster>,
    ) -> LayeredPromptResult;

    /// 获取指定层的模板
    async fn get_template(&self, layer: PromptLayer, key: &str) -> Option<PromptTemplate>;

    /// 设置指定层的模板
    async fn set_template(&self, template: PromptTemplate) -> Result<(), String>;

    /// 删除模板
    async fn remove_template(&self, layer: PromptLayer, key: &str) -> Result<(), String>;

    /// 预览模板（变量替换后）
    async fn render_template(
        &self,
        template_id: &str,
        variables: &serde_json::Value,
    ) -> Result<String, String>;

    /// 估算文本 token 数
    fn estimate_tokens(text: &str) -> u32 {
        estimate_tokens(text)
    }
}

// ── Prompt 模板定义 ──────────────────────────

/// Prompt 模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// 模板 ID（唯一）
    pub template_id: String,
    /// 所属层
    pub layer: PromptLayer,
    /// 模板 key（用于查找，如 "domain:data_analysis"）
    pub key: String,
    /// 模板内容（支持变量: {variable_name}）
    pub content: String,
    /// token 上限
    #[serde(default)]
    pub token_limit: u32,
    /// 变量默认值
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_variables: Option<serde_json::Value>,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 版本号
    #[serde(default = "default_version")]
    pub version: i32,
    /// 是否启用
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_version() -> i32 {
    1
}

fn default_enabled() -> bool {
    true
}

impl PromptTemplate {
    pub fn new(
        template_id: impl Into<String>,
        layer: PromptLayer,
        key: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        let content = content.into();
        Self {
            template_id: template_id.into(),
            layer,
            key: key.into(),
            token_limit: layer.default_token_limit(),
            content,
            default_variables: None,
            description: None,
            version: 1,
            enabled: true,
        }
    }

    pub fn with_token_limit(mut self, limit: u32) -> Self {
        self.token_limit = limit;
        self
    }

    pub fn with_default_variables(mut self, vars: serde_json::Value) -> Self {
        self.default_variables = Some(vars);
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// 用变量替换模板中的 {key} 占位符
    pub fn render(&self, variables: &serde_json::Value) -> String {
        let mut result = self.content.clone();
        if let Some(defaults) = &self.default_variables {
            // 先替换默认值
            if let Some(obj) = defaults.as_object() {
                for (key, value) in obj {
                    let placeholder = format!("{{{}}}", key);
                    if let Some(s) = value.as_str() {
                        result = result.replace(&placeholder, s);
                    } else {
                        result = result.replace(&placeholder, &value.to_string());
                    }
                }
            }
        }
        // 再替换传入变量（优先级更高）
        if let Some(obj) = variables.as_object() {
            for (key, value) in obj {
                let placeholder = format!("{{{}}}", key);
                if let Some(s) = value.as_str() {
                    result = result.replace(&placeholder, s);
                } else {
                    result = result.replace(&placeholder, &value.to_string());
                }
            }
        }
        result
    }
}

// ── Token 估算工具 ──────────────────────────

/// 简易 token 估算（中文 1.5 字/token，英文 4 字符/token）
///
/// 实际 tokenizer 在 implementor 层实现，harness 层仅提供粗略估算。
pub fn estimate_tokens(text: &str) -> u32 {
    let chars = text.chars().count() as u32;
    // 粗略估算: 混合文本取 2 字符/token
    (chars / 2).max(1)
}

// ── 内置默认模板集 ──────────────────────────

/// 获取内置 Prompt 模板（首次启动时注入）
pub fn default_prompt_templates() -> Vec<PromptTemplate> {
    vec![
        // ── Domain 层 ──
        PromptTemplate::new(
            "tpl_domain_data_analysis",
            PromptLayer::Domain,
            "domain:data_analysis",
            "你是一名专业的数据分析助手。擅长处理数据、生成图表、统计分析和报表生成。\n\n请帮助用户完成数据分析任务。",
        )
        .with_token_limit(300)
        .with_description("数据分析域默认模板"),

        PromptTemplate::new(
            "tpl_domain_content_creation",
            PromptLayer::Domain,
            "domain:content_creation",
            "你是一名才华横溢的内容创作助手。擅长撰写文章、生成文案、润色文本和内容编辑。\n\n请帮助用户创作高质量内容。",
        )
        .with_token_limit(300)
        .with_description("内容创作域默认模板"),

        PromptTemplate::new(
            "tpl_domain_communication",
            PromptLayer::Domain,
            "domain:communication",
            "你是一名高效的沟通助手。帮助用户撰写邮件、安排会议、管理日程和团队协作。\n\n请帮助用户完成沟通任务。",
        )
        .with_token_limit(250)
        .with_description("通信域默认模板"),

        PromptTemplate::new(
            "tpl_domain_devops",
            PromptLayer::Domain,
            "domain:devops",
            "你是一名经验丰富的 DevOps 工程师。擅长部署运维、CI/CD 流水线、容器编排和系统监控。\n\n请帮助用户解决运维问题。",
        )
        .with_token_limit(300)
        .with_description("DevOps 域默认模板"),

        PromptTemplate::new(
            "tpl_domain_ai_media",
            PromptLayer::Domain,
            "domain:ai_media",
            "你是一名创意十足的 AI 媒体助手。擅长图像生成、视频制作、音频处理和多媒体内容创作。\n\n请帮助用户创作媒体内容。",
        )
        .with_token_limit(250)
        .with_description("AI媒体域默认模板"),

        PromptTemplate::new(
            "tpl_domain_invest",
            PromptLayer::Domain,
            "domain:invest",
            "你是一名专业的投资顾问。擅长股票分析、基金管理、交易策略和市场洞察。\n\n请帮助用户做出明智的投资决策。注意：投资建议仅供参考，不构成实际投资建议。",
        )
        .with_token_limit(350)
        .with_description("投资域默认模板"),

        PromptTemplate::new(
            "tpl_domain_opc",
            PromptLayer::Domain,
            "domain:opc",
            "你是一名高效的业务运营助手。擅长客户管理、订单处理、库存管理和供应链优化。\n\n请帮助用户完成业务运营任务。",
        )
        .with_token_limit(300)
        .with_description("OPC域默认模板"),

        PromptTemplate::new(
            "tpl_domain_general",
            PromptLayer::Domain,
            "domain:general",
            "你是一名乐于助人的 AI 助手。可以回答各类问题，提供信息查询和知识解释服务。\n\n请帮助用户解答疑问。",
        )
        .with_token_limit(200)
        .with_description("通用域默认模板"),

        // ── Context 层 ──
        PromptTemplate::new(
            "tpl_context_user_profile",
            PromptLayer::Context,
            "context:user_profile",
            "用户信息: {user_summary}\n历史偏好: {user_preferences}",
        )
        .with_token_limit(200)
        .with_description("用户上下文模板"),
    ]
}
