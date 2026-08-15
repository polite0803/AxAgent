// SPDX-License-Identifier: AGPL-3.0-only

//! 能力集清单 DTO
//!
//! 定义需求发现工作流使用的能力清单聚合结构（`CapabilityInventory` /
//! `CapabilityEntry` / `CapabilitySource`）。能力来源统一复用上游能力发现索引
//! （`capability_indexer` 的能力护照），由命令层在 wiring 层读取组装，不再本地扫描落库。

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
    Agent,
}

impl CapabilitySource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Skill => "skill",
            Self::McpTool => "mcp_tool",
            Self::Workflow => "workflow",
            Self::Agent => "agent",
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
    pub agents: Vec<CapabilityEntry>,
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
            agents: Vec::new(),
            scanned_at: now,
            total_count: 0,
        }
    }

    pub fn recalc_count(&mut self) {
        self.total_count = self.tools.len()
            + self.skills.len()
            + self.mcp_tools.len()
            + self.workflows.len()
            + self.agents.len();
    }

    /// 全部条目展平为一个列表（供 Agent 注入 context）
    pub fn all_entries(&self) -> Vec<&CapabilityEntry> {
        let mut v: Vec<&CapabilityEntry> = Vec::new();
        v.extend(self.tools.iter());
        v.extend(self.skills.iter());
        v.extend(self.mcp_tools.iter());
        v.extend(self.workflows.iter());
        v.extend(self.agents.iter());
        v
    }
}

impl Default for CapabilityInventory {
    fn default() -> Self {
        Self::new()
    }
}
