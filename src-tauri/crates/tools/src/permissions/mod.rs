//! 细粒度权限系统
//!
//! 规则匹配引擎 + AI 安全分类器 + 拒绝追踪。

pub mod classifier;
pub mod rules;
pub mod tracker;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 权限行为
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

/// 规则来源
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleSource {
    /// CLI 参数传入
    CliArg,
    /// 命令级别
    Command,
    /// 用户设置
    User,
    /// 项目设置 (CLAUDE.md)
    Project,
    /// 会话内动态规则
    Session,
}

/// 规则模式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePattern {
    /// 工具名称匹配模式，如 "Bash(git *)", "FileEdit"
    pub pattern: String,
}

impl RulePattern {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
        }
    }
}

/// 权限规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub behavior: PermissionBehavior,
    pub source: RuleSource,
    pub pattern: RulePattern,
    pub description: String,
}

impl PermissionRule {
    pub fn allow(pattern: &str, description: &str) -> Self {
        Self {
            behavior: PermissionBehavior::Allow,
            source: RuleSource::User,
            pattern: RulePattern::new(pattern),
            description: description.to_string(),
        }
    }

    pub fn deny(pattern: &str, description: &str) -> Self {
        Self {
            behavior: PermissionBehavior::Deny,
            source: RuleSource::User,
            pattern: RulePattern::new(pattern),
            description: description.to_string(),
        }
    }

    pub fn ask(pattern: &str, description: &str) -> Self {
        Self {
            behavior: PermissionBehavior::Ask,
            source: RuleSource::User,
            pattern: RulePattern::new(pattern),
            description: description.to_string(),
        }
    }
}

/// 权限决策（来自规则匹配）
#[derive(Debug, Clone)]
pub struct PermissionDecision {
    pub behavior: PermissionBehavior,
    /// 是哪个规则产生的决策
    pub matched_rule: Option<String>,
    pub reason: String,
}

impl PermissionDecision {
    pub fn new(behavior: PermissionBehavior, reason: impl Into<String>) -> Self {
        Self {
            behavior,
            matched_rule: None,
            reason: reason.into(),
        }
    }

    pub fn from_rule(rule: &PermissionRule) -> Self {
        Self {
            behavior: rule.behavior.clone(),
            matched_rule: Some(rule.pattern.pattern.clone()),
            reason: rule.description.clone(),
        }
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self.behavior, PermissionBehavior::Allow)
    }

    pub fn is_denied(&self) -> bool {
        matches!(self.behavior, PermissionBehavior::Deny)
    }

    pub fn is_ask(&self) -> bool {
        matches!(self.behavior, PermissionBehavior::Ask)
    }
}

/// 细粒度权限策略
///
/// 整合规则匹配、分类器、拒绝追踪，提供统一的权限决策接口。
pub struct PermissionPolicy {
    /// 激活的权限模式
    pub active_mode: PermissionMode,
    /// 允许规则集
    allow_rules: Vec<PermissionRule>,
    /// 拒绝规则集（优先级最高）
    deny_rules: Vec<PermissionRule>,
    /// 询问规则集
    ask_rules: Vec<PermissionRule>,
    /// 工具级权限要求
    tool_requirements: HashMap<String, PermissionMode>,
    /// 拒绝追踪器
    pub denial_tracker: tracker::DenialTracker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PermissionMode {
    /// 仅读取
    ReadOnly = 0,
    /// 工作区写入
    WorkspaceWrite = 1,
    /// 完全访问（危险）
    DangerFullAccess = 2,
    /// 始终询问
    Prompt = 3,
    /// 全部允许
    Allow = 4,
}

impl PermissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionMode::ReadOnly => "read_only",
            PermissionMode::WorkspaceWrite => "workspace_write",
            PermissionMode::DangerFullAccess => "danger_full_access",
            PermissionMode::Prompt => "prompt",
            PermissionMode::Allow => "allow",
        }
    }
}

impl PermissionPolicy {
    pub fn new(mode: PermissionMode) -> Self {
        Self {
            active_mode: mode,
            allow_rules: Vec::new(),
            deny_rules: Vec::new(),
            ask_rules: Vec::new(),
            tool_requirements: HashMap::new(),
            denial_tracker: tracker::DenialTracker::new(),
        }
    }

    /// 添加允许规则
    pub fn allow_rule(mut self, pattern: &str, description: &str) -> Self {
        self.allow_rules
            .push(PermissionRule::allow(pattern, description));
        self
    }

    /// 添加拒绝规则
    pub fn deny_rule(mut self, pattern: &str, description: &str) -> Self {
        self.deny_rules
            .push(PermissionRule::deny(pattern, description));
        self
    }

    /// 添加询问规则
    pub fn ask_rule(mut self, pattern: &str, description: &str) -> Self {
        self.ask_rules
            .push(PermissionRule::ask(pattern, description));
        self
    }

    /// 设置工具的最小权限要求
    pub fn with_tool_requirement(mut self, tool_name: &str, mode: PermissionMode) -> Self {
        self.tool_requirements.insert(tool_name.to_string(), mode);
        self
    }

    /// 授权检查
    ///
    /// 优先级：拒绝规则 > 允许规则 > 询问规则 > 模式检查
    pub fn authorize(&mut self, tool_name: &str, input: &str) -> PermissionDecision {
        // 1. 检查拒绝规则（最高优先级）
        if let Some(rule) = rules::match_rules(tool_name, input, &self.deny_rules) {
            self.denial_tracker.record_denial(tool_name);
            return PermissionDecision::from_rule(rule);
        }

        // 2. 检查允许规则
        if let Some(rule) = rules::match_rules(tool_name, input, &self.allow_rules) {
            return PermissionDecision::from_rule(rule);
        }

        // 3. 检查询问规则
        if let Some(rule) = rules::match_rules(tool_name, input, &self.ask_rules) {
            return PermissionDecision::from_rule(rule);
        }

        // 4. 检查工具的最小模式要求
        if let Some(required_mode) = self.tool_requirements.get(tool_name) {
            if self.active_mode < *required_mode {
                return PermissionDecision::new(
                    PermissionBehavior::Ask,
                    format!(
                        "工具 '{}' 需要 {} 模式，当前为 {} 模式",
                        tool_name,
                        required_mode.as_str(),
                        self.active_mode.as_str()
                    ),
                );
            }
        }

        // 5. 默认根据模式决定
        match self.active_mode {
            PermissionMode::Allow => {
                PermissionDecision::new(PermissionBehavior::Allow, "全局允许模式")
            },
            PermissionMode::Prompt => {
                PermissionDecision::new(PermissionBehavior::Ask, "全局询问模式")
            },
            PermissionMode::ReadOnly => {
                if tool_name.to_lowercase().contains("write")
                    || tool_name.to_lowercase().contains("delete")
                    || tool_name.to_lowercase().contains("edit")
                {
                    PermissionDecision::new(PermissionBehavior::Ask, "只读模式下不允许写入操作")
                } else {
                    PermissionDecision::new(PermissionBehavior::Allow, "只读模式")
                }
            },
            _ => PermissionDecision::new(PermissionBehavior::Ask, "需要用户确认"),
        }
    }

    /// 检查拒绝追踪是否触发降级
    pub fn should_degrade(&self, tool_name: &str) -> bool {
        self.denial_tracker.should_degrade(tool_name)
    }
}
