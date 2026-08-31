// SPDX-License-Identifier: AGPL-3.0-only

//! 运行时动态工具集 —— 能力按需加载（CapabilityLoad）的执行闭环载体。
//!
//! # 它解决什么
//!
//! 认知编排执行阶段只放行 5 个披露工具，其余能力即使被 `CapabilityView` 展开
//! 定义也不在 `chat_tools` 里 —— LLM 看得见、调不动，L1 → 执行断裂。
//!
//! 而构建期就把全部工具塞进 `chat_tools` 又会撑爆上下文，这正是当初收窄的动机。
//! 折中解是**循环内按需追加**：Agent 调 `CapabilityLoad` 时把该能力的工具定义
//! 追加进本集合，下一次 LLM 调用即可发起 function call。
//!
//! # 作用域
//!
//! 每个会话一份，由 wiring 层创建后共享给三方：
//! - `UnifiedToolRegistry` → 透传进 `ToolContext`，供 `CapabilityLoad` 写入
//! - `ApiClient` → 每次发请求前合并进工具列表
//! - 会话结束随 `Arc` 释放，无需显式清理
//!
//! 因此不存在跨会话串扰：不同会话持有不同 `Arc`，天然隔离。

use crate::types::ChatTool;
use std::sync::{Arc, RwLock};

/// 一次会话内、由 Agent 在循环里按需激活的工具定义集合。
#[derive(Debug, Clone, Default)]
pub struct DynamicToolSet {
    tools: Arc<RwLock<Vec<ChatTool>>>,
}

impl DynamicToolSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个工具定义。
    ///
    /// 返回 `true` 表示新增；同名工具已存在时**不覆盖**，返回 `false`
    /// —— 重复加载同一能力应当是幂等的无操作，覆盖会让先加载的定义被
    /// 后一次（可能来自不同 Agent）的调用悄悄改写。
    pub fn add(&self, tool: ChatTool) -> bool {
        let mut tools = match self.tools.write() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if tools.iter().any(|t| t.function.name == tool.function.name) {
            return false;
        }
        tools.push(tool);
        true
    }

    /// 追加一批工具定义，返回实际新增的条数。
    pub fn extend(&self, new_tools: Vec<ChatTool>) -> usize {
        let mut added = 0usize;
        for t in new_tools {
            if self.add(t) {
                added += 1;
            }
        }
        added
    }

    /// 当前快照（供每次 LLM 请求前合并进工具列表）。
    pub fn snapshot(&self) -> Vec<ChatTool> {
        match self.tools.read() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }

    /// 已激活的工具数量。
    pub fn len(&self) -> usize {
        match self.tools.read() {
            Ok(g) => g.len(),
            Err(p) => p.into_inner().len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChatToolFunction;

    fn tool(name: &str) -> ChatTool {
        ChatTool {
            r#type: "function".to_string(),
            function: ChatToolFunction {
                name: name.to_string(),
                description: Some(format!("{name} 的描述")),
                parameters: None,
            },
        }
    }

    #[test]
    fn add_is_idempotent() {
        let set = DynamicToolSet::new();
        assert!(set.add(tool("Read")));
        assert!(!set.add(tool("Read")), "重复添加同名工具应返回 false");
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn snapshot_reflects_additions() {
        let set = DynamicToolSet::new();
        assert!(set.is_empty());
        set.add(tool("Read"));
        set.add(tool("Grep"));
        let snap = set.snapshot();
        assert_eq!(snap.len(), 2);
        assert!(snap.iter().any(|t| t.function.name == "Grep"));
    }

    #[test]
    fn extend_counts_only_new() {
        let set = DynamicToolSet::new();
        set.add(tool("Read"));
        let added = set.extend(vec![tool("Read"), tool("Grep"), tool("Glob")]);
        assert_eq!(added, 2, "Read 已存在，只应计入 Grep/Glob");
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn clones_share_state() {
        let a = DynamicToolSet::new();
        let b = a.clone();
        a.add(tool("Read"));
        assert_eq!(b.len(), 1, "clone 必须共享同一份 Arc");
    }
}
