//! 工具编排器 — 将工具调用列表按并发安全性分区并执行
//! 连续的 concurrency_safe 工具组成一个 batch 并行执行，非安全工具串行执行

use std::collections::HashMap;

/// 一个工具执行批次
pub struct ToolBatch {
    /// 批次中的工具调用列表
    pub calls: Vec<ToolBatchItem>,
    /// 是否支持并发执行
    pub is_concurrent: bool,
}

/// 单个工具调用项
pub struct ToolBatchItem {
    /// 工具名称
    pub tool_name: String,
    /// 工具输入参数
    pub tool_input: serde_json::Value,
    /// 工具调用 ID（对应 LLM 返回的 tool_use_id）
    pub tool_use_id: String,
}

/// 工具编排器 — 根据并发安全性将工具调用列表分区为串行/并行批次
pub struct ToolOrchestrator {
    /// 最大并发数（预留，实际限制由 thread::scope 控制）
    #[allow(dead_code)]
    max_concurrency: usize,
}

impl ToolOrchestrator {
    /// 创建新的编排器实例
    pub fn new() -> Self {
        Self {
            max_concurrency: std::env::var("AXAGENT_MAX_TOOL_CONCURRENCY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        }
    }

    /// 将工具调用列表分区为 batch 列表
    /// 连续的 concurrency_safe 工具归入同一个并行批次，非安全工具各自独占批次
    pub fn partition(
        &self,
        tool_calls: &[ToolBatchItem],
        concurrency_map: &HashMap<String, bool>,
    ) -> Vec<ToolBatch> {
        let mut batches = Vec::new();
        let mut current_batch = Vec::new();
        let mut current_is_concurrent = None;

        for call in tool_calls {
            let is_safe = concurrency_map
                .get(&call.tool_name)
                .copied()
                .unwrap_or(false);

            match current_is_concurrent {
                Some(prev) if prev == is_safe => {
                    // 当前工具与上一个并发属性相同，加入同一批次
                    current_batch.push(call.clone());
                }
                _ => {
                    // 并发属性变化，先保存当前批次，再开始新批次
                    if !current_batch.is_empty() {
                        batches.push(ToolBatch {
                            calls: std::mem::take(&mut current_batch),
                            is_concurrent: current_is_concurrent.unwrap_or(false),
                        });
                    }
                    current_batch.push(call.clone());
                    current_is_concurrent = Some(is_safe);
                }
            }
        }

        // 处理最后一个批次
        if !current_batch.is_empty() {
            batches.push(ToolBatch {
                calls: current_batch,
                is_concurrent: current_is_concurrent.unwrap_or(false),
            });
        }

        batches
    }

    /// 检查工具并发执行是否通过 feature flag 启用
    pub fn is_enabled() -> bool {
        axagent_runtime::feature_flags::global_feature_flags().tool_concurrency()
    }
}

impl ToolBatchItem {
    /// 创建新的工具调用项
    pub fn new(tool_name: String, tool_input: serde_json::Value, tool_use_id: String) -> Self {
        Self {
            tool_name,
            tool_input,
            tool_use_id,
        }
    }
}

impl Clone for ToolBatchItem {
    fn clone(&self) -> Self {
        Self {
            tool_name: self.tool_name.clone(),
            tool_input: self.tool_input.clone(),
            tool_use_id: self.tool_use_id.clone(),
        }
    }
}
