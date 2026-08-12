// SPDX-License-Identifier: AGPL-3.0-only

//! 能力集扫描模块
//!
//! 自动聚合四类能力来源：工具 Tools、技能 Skills、MCP 服务器/工具、工作流模板。
//! 结果归一化为 `CapabilityInventory`，供 Agent 节点作为 context_source 注入。

use serde::{Deserialize, Serialize};

// ── DTO 定义 ──────────────────────────────────────────────────

/// 能力来源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySource {
    Tool,
    Skill,
    McpTool,
    Workflow,
}

impl CapabilitySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Skill => "skill",
            Self::McpTool => "mcp_tool",
            Self::Workflow => "workflow",
        }
    }
}

/// 能力条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source: CapabilitySource,
    pub source_id: String,
    pub capability_type: String,
    pub applicable_scenarios: Vec<String>,
    pub example_deliverables: Vec<String>,
    pub metadata: serde_json::Value,
}

/// 能力清单聚合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInventory {
    pub tools: Vec<CapabilityEntry>,
    pub skills: Vec<CapabilityEntry>,
    pub mcp_tools: Vec<CapabilityEntry>,
    pub workflows: Vec<CapabilityEntry>,
    pub scanned_at: i64,
    pub total_count: usize,
}

impl CapabilityInventory {
    pub fn new() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            tools: Vec::new(),
            skills: Vec::new(),
            mcp_tools: Vec::new(),
            workflows: Vec::new(),
            scanned_at: now,
            total_count: 0,
        }
    }

    pub fn recalc_count(&mut self) {
        self.total_count =
            self.tools.len() + self.skills.len() + self.mcp_tools.len() + self.workflows.len();
    }

    /// 全部条目展平为一个列表（供 Agent 注入 context）
    pub fn all_entries(&self) -> Vec<&CapabilityEntry> {
        let mut v: Vec<&CapabilityEntry> = Vec::new();
        v.extend(self.tools.iter());
        v.extend(self.skills.iter());
        v.extend(self.mcp_tools.iter());
        v.extend(self.workflows.iter());
        v
    }
}

impl Default for CapabilityInventory {
    fn default() -> Self {
        Self::new()
    }
}

// ── 扫描服务 ──────────────────────────────────────────────────

/// 能力扫描服务
pub struct CapabilityService;

impl CapabilityService {
    /// 扫描当前系统全部可用能力
    ///
    /// 从四个来源聚合：
    /// - `axagent-tools::registry` —— 已注册的工具描述
    /// - skill 状态存储 —— 已启用的技能
    /// - MCP 服务器/工具注册表 —— 通过 mcp_manager
    /// - workflow_template 表 —— 已保存的工作流模板
    ///
    /// 当前版本返回空清单（占位），后续在 wiring 层注入实际注册表。
    pub async fn scan_capabilities() -> CapabilityInventory {
        let inv = CapabilityInventory::new();
        // TODO: 接入实际注册表（tools::registry、skill_states、mcp、workflow_template）
        inv
    }
}
