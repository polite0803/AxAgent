// SPDX-License-Identifier: AGPL-3.0-only

//! 推理策略自动选择路由器（Phase 4 / P2）
//!
//! 根据任务特征自动选择最优推理引擎：
//! - 简单单步任务 → ReactEngine（ReAct 模式：思考 - 行动 - 观察）
//! - 多步有分支任务 → TreeOfThoughts（思维树多路探索）
//! - 需要验证复核的任务 → ReasoningStateMachine（状态驱动的验证循环）

use std::fmt;

// ── 推理引擎枚举 ──

/// 可用的推理引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReasoningEngine {
    /// ReAct 引擎：思考→行动→观察 循环，适合简单单步任务
    ReactEngine,
    /// 思维树引擎：多路分支探索，适合多步复杂任务
    TreeOfThoughts,
    /// 推理状态机：带验证和复核的状态驱动引擎
    ReasoningStateMachine,
}

impl fmt::Display for ReasoningEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReasoningEngine::ReactEngine => write!(f, "ReactEngine"),
            ReasoningEngine::TreeOfThoughts => write!(f, "TreeOfThoughts"),
            ReasoningEngine::ReasoningStateMachine => write!(f, "ReasoningStateMachine"),
        }
    }
}

// ── 任务特征提取 ──

/// 从任务描述中提取的结构化特征
#[derive(Debug, Clone)]
pub struct TaskFeatures {
    /// 任务节点数量（工作流中的步骤数）
    pub node_count: usize,
    /// 预计工具调用轮数
    pub estimated_tool_rounds: usize,
    /// 输入字符串长度
    pub input_length: usize,
    /// 是否需要验证 / 复核
    pub requires_verification: bool,
    /// 是否有多分支路径
    pub has_branches: bool,
    /// 是否有条件判断节点
    pub has_conditions: bool,
    /// 任务描述文本（用于 LLM 辅助判断时传入）
    pub task_description: String,
}

impl Default for TaskFeatures {
    fn default() -> Self {
        Self {
            node_count: 1,
            estimated_tool_rounds: 1,
            input_length: 0,
            requires_verification: false,
            has_branches: false,
            has_conditions: false,
            task_description: String::new(),
        }
    }
}

// ── 路由器 ──

/// 推理路由器可配置的阈值参数。
///
/// 所有字段均可通过配置文件覆盖，默认值保持与原有硬编码行为一致。
/// 配合 `ab_testing` 模块可进行 A/B 测试对比不同阈值的实际效果。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterThresholds {
    /// node_count > 此值 → ReasoningStateMachine
    #[serde(default = "default_state_machine_nodes")]
    pub state_machine_min_nodes: usize,

    /// estimated_tool_rounds > 此值 → TreeOfThoughts
    #[serde(default = "default_tot_tool_rounds")]
    pub tree_of_thoughts_min_tool_rounds: usize,

    /// node_count > 此值 → TreeOfThoughts（需不超过 state_machine_min_nodes）
    #[serde(default = "default_tot_nodes")]
    pub tree_of_thoughts_min_nodes: usize,
}

fn default_state_machine_nodes() -> usize {
    10
}
fn default_tot_tool_rounds() -> usize {
    3
}
fn default_tot_nodes() -> usize {
    5
}

impl Default for RouterThresholds {
    fn default() -> Self {
        Self {
            state_machine_min_nodes: 10,
            tree_of_thoughts_min_tool_rounds: 3,
            tree_of_thoughts_min_nodes: 5,
        }
    }
}

/// 根据启发式规则选择推理引擎。
///
/// 决策规则（按优先级）：
/// 1. requires_verification || node_count > state_machine_min_nodes → ReasoningStateMachine
/// 2. has_branches || has_conditions || estimated_tool_rounds > tot_tool_rounds || node_count > tot_nodes → TreeOfThoughts
/// 3. 默认 → ReactEngine
pub fn route_reasoning_engine(features: &TaskFeatures) -> ReasoningEngine {
    route_reasoning_engine_with_thresholds(features, &RouterThresholds::default())
}

/// 使用自定义阈值选择推理引擎（支持 A/B 测试）。
pub fn route_reasoning_engine_with_thresholds(
    features: &TaskFeatures,
    thresholds: &RouterThresholds,
) -> ReasoningEngine {
    // 规则 1：复杂验证任务 → 推理状态机
    if features.requires_verification || features.node_count > thresholds.state_machine_min_nodes {
        return ReasoningEngine::ReasoningStateMachine;
    }

    // 规则 2：多步分支任务 → 思维树
    if features.has_branches
        || features.has_conditions
        || features.estimated_tool_rounds > thresholds.tree_of_thoughts_min_tool_rounds
        || features.node_count > thresholds.tree_of_thoughts_min_nodes
    {
        return ReasoningEngine::TreeOfThoughts;
    }

    // 规则 3：默认 → ReAct
    ReasoningEngine::ReactEngine
}

/// 根据任务描述文本自动提取特征并选择引擎。
///
/// 这是一个纯启发式函数（不依赖 LLM 调用），基于：
/// - 关键词匹配：verify / 复核 / 审核 / audit → requires_verification
/// - 关键词匹配：分支 / 选择 / 条件 / switch → has_conditions
/// - 字符串长度推断轮数
pub fn auto_select_engine(task_description: &str) -> (ReasoningEngine, TaskFeatures) {
    let lower = task_description.to_lowercase();

    // 验证需求检测
    let requires_verification = lower.contains("验证")
        || lower.contains("复核")
        || lower.contains("审核")
        || lower.contains("audit")
        || lower.contains("检查")
        || lower.contains("校验")
        || lower.contains("verify");

    // 分支条件检测
    let has_conditions = lower.contains("条件")
        || lower.contains("分支")
        || lower.contains("选择")
        || lower.contains("switch")
        || lower.contains("if")
        || lower.contains("判断");

    // 多分支检测
    let has_branches = lower.contains("多路径")
        || lower.contains("多方案")
        || lower.contains("对比")
        || lower.contains("比较")
        || lower.contains("择优")
        || lower.contains("权衡");

    // 工具轮数估算
    let estimated_tool_rounds = estimate_tool_rounds(task_description);

    // 节点数估算
    let node_count = estimate_node_count(task_description);

    let features = TaskFeatures {
        node_count,
        estimated_tool_rounds,
        input_length: task_description.len(),
        requires_verification,
        has_branches,
        has_conditions,
        task_description: task_description.to_string(),
    };

    let engine = route_reasoning_engine(&features);
    (engine, features)
}

/// 估算工具调用轮数（基于关键词匹配）
fn estimate_tool_rounds(description: &str) -> usize {
    let lower = description.to_lowercase();

    // 工具相关关键词计数
    let tool_keywords = [
        "搜索",
        "search",
        "读取",
        "read",
        "写入",
        "write",
        "调用",
        "call",
        "查询",
        "query",
        "下载",
        "download",
        "上传",
        "upload",
        "分析",
        "analyze",
        "解析",
        "parse",
        "生成",
        "generate",
        "计算",
        "calculate",
    ];

    let keyword_count: usize = tool_keywords
        .iter()
        .map(|kw| lower.matches(kw).count())
        .sum();

    // 每个关键词贡献 0.5 轮，最小 1 轮
    std::cmp::max(1, keyword_count / 2)
}

/// 估算工作流节点数（基于关键词匹配）
fn estimate_node_count(description: &str) -> usize {
    let lower = description.to_lowercase();

    let step_keywords = [
        "步骤",
        "第一步",
        "第二步",
        "第三步",
        "第四步",
        "第五步",
        "首先",
        "然后",
        "接着",
        "最后",
        "同时",
        "或者",
        "step",
        "first",
        "then",
        "next",
        "finally",
        "concurrently",
    ];

    let count: usize = step_keywords
        .iter()
        .map(|kw| lower.matches(kw).count())
        .sum();

    std::cmp::max(1, count + 1)
}

// ── 带 LLM 辅助的精进模式 ──

/// 使用 LLM 对特征提取进行辅助判断的提示词模板
pub const FEATURE_ANALYSIS_PROMPT: &str = r#"Analyze the following task and return a JSON with these fields:
- requires_verification (bool): does the task need fact-checking, audit, or verification?
- has_branches (bool): are there multiple alternative paths or solutions?
- has_conditions (bool): does the task involve conditional logic or branching?
- estimated_complexity (string): "simple", "medium", or "complex"

Return ONLY the JSON object, no other text.

Task: "#;

/// 结合启发式判断和 LLM 结果选择引擎。
///
/// `llm_analysis` 应为 LLM 返回的 JSON，包含 `requires_verification`、`has_branches`、`has_conditions`、`estimated_complexity`。
pub fn select_with_llm_hint(features: &TaskFeatures, llm_analysis: &str) -> ReasoningEngine {
    let mut enhanced = features.clone();

    // 解析 LLM 结果
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(llm_analysis) {
        if let Some(b) = v["requires_verification"].as_bool() {
            enhanced.requires_verification |= b;
        }
        if let Some(b) = v["has_branches"].as_bool() {
            enhanced.has_branches |= b;
        }
        if let Some(b) = v["has_conditions"].as_bool() {
            enhanced.has_conditions |= b;
        }
        if let Some(s) = v["estimated_complexity"].as_str() {
            match s {
                "complex" => {
                    enhanced.node_count = enhanced.node_count.max(10);
                    enhanced.estimated_tool_rounds = enhanced.estimated_tool_rounds.max(5);
                },
                "medium" => {
                    enhanced.node_count = enhanced.node_count.max(5);
                    enhanced.estimated_tool_rounds = enhanced.estimated_tool_rounds.max(3);
                },
                _ => {},
            }
        }
    }

    route_reasoning_engine(&enhanced)
}

// ── A/B 测试集成 ──

/// A/B 测试变体：使用不同阈值配置对比推理效果。
///
/// 典型用法：
/// - 变体 A（对照组）：使用默认阈值 `RouterThresholds::default()`
/// - 变体 B（实验组）：使用自定义阈值（如更宽松的 ToT 触发条件）
///
/// 返回选择的引擎和使用的阈值配置标识。
pub fn ab_test_route(features: &TaskFeatures, variant: &str) -> (ReasoningEngine, String) {
    let thresholds = match variant {
        "control" | "default" => RouterThresholds::default(),
        "aggressive_tot" => RouterThresholds {
            tree_of_thoughts_min_nodes: 3,
            tree_of_thoughts_min_tool_rounds: 2,
            ..Default::default()
        },
        "conservative_sm" => RouterThresholds {
            state_machine_min_nodes: 15,
            ..Default::default()
        },
        other => {
            // 尝试从 JSON 解析自定义阈值
            serde_json::from_str(other).unwrap_or_default()
        },
    };
    let engine = route_reasoning_engine_with_thresholds(features, &thresholds);
    (engine, variant.to_string())
}

/// 为 A/B 测试生成试验记录（供 `ab_testing` 模块消费）。
pub fn create_ab_trial(
    features: &TaskFeatures,
    engine: ReasoningEngine,
    variant: &str,
    task_desc: &str,
) -> serde_json::Value {
    serde_json::json!({
        "variant": variant,
        "engine": engine.to_string(),
        "features": {
            "node_count": features.node_count,
            "estimated_tool_rounds": features.estimated_tool_rounds,
            "input_length": features.input_length,
            "requires_verification": features.requires_verification,
            "has_branches": features.has_branches,
            "has_conditions": features.has_conditions,
        },
        "task": task_desc,
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_task_routes_to_react() {
        let features = TaskFeatures {
            node_count: 1,
            estimated_tool_rounds: 1,
            input_length: 50,
            requires_verification: false,
            has_branches: false,
            has_conditions: false,
            task_description: "读取文件并返回内容".into(),
        };
        assert_eq!(route_reasoning_engine(&features), ReasoningEngine::ReactEngine);
    }

    #[test]
    fn test_branching_task_routes_to_tree() {
        let features = TaskFeatures {
            node_count: 6,
            estimated_tool_rounds: 4,
            input_length: 200,
            requires_verification: false,
            has_branches: true,
            has_conditions: false,
            task_description: "比较三个方案的优劣".into(),
        };
        assert_eq!(route_reasoning_engine(&features), ReasoningEngine::TreeOfThoughts);
    }

    #[test]
    fn test_verification_task_routes_to_state_machine() {
        let features = TaskFeatures {
            node_count: 3,
            estimated_tool_rounds: 2,
            input_length: 100,
            requires_verification: true,
            has_branches: false,
            has_conditions: false,
            task_description: "验证计算结果是否正确".into(),
        };
        assert_eq!(route_reasoning_engine(&features), ReasoningEngine::ReasoningStateMachine);
    }

    #[test]
    fn test_auto_select_simple() {
        let (engine, _) = auto_select_engine("读取 D 盘的文件列表");
        assert_eq!(engine, ReasoningEngine::ReactEngine);
    }

    #[test]
    fn test_auto_select_complex() {
        let (engine, _) =
            auto_select_engine("首先解析所有发票，然后验证金额是否匹配，最后生成审计报告");
        assert_eq!(engine, ReasoningEngine::ReasoningStateMachine);
    }

    #[test]
    fn test_auto_select_branching() {
        let (engine, _) =
            auto_select_engine("对比分析三个供应商的报价，选择最优方案，然后下载对应的合同模板");
        assert_eq!(engine, ReasoningEngine::TreeOfThoughts);
    }

    #[test]
    fn test_large_node_count_routes_to_state_machine() {
        let features = TaskFeatures {
            node_count: 15,
            estimated_tool_rounds: 2,
            input_length: 100,
            requires_verification: false,
            has_branches: false,
            has_conditions: false,
            task_description: "处理复杂流程".into(),
        };
        assert_eq!(route_reasoning_engine(&features), ReasoningEngine::ReasoningStateMachine);
    }

    #[test]
    fn test_custom_thresholds_tot_more_sensitive() {
        let thresholds = RouterThresholds {
            tree_of_thoughts_min_nodes: 2,
            tree_of_thoughts_min_tool_rounds: 1,
            ..Default::default()
        };
        // node_count=3, tool_rounds=2 → with custom thresholds this triggers ToT
        let features = TaskFeatures {
            node_count: 3,
            estimated_tool_rounds: 2,
            input_length: 100,
            requires_verification: false,
            has_branches: false,
            has_conditions: false,
            task_description: "分析数据".into(),
        };
        assert_eq!(
            route_reasoning_engine_with_thresholds(&features, &thresholds),
            ReasoningEngine::TreeOfThoughts
        );
    }

    #[test]
    fn test_ab_test_route_variants() {
        let features = TaskFeatures {
            node_count: 6,
            estimated_tool_rounds: 2,
            input_length: 100,
            requires_verification: false,
            has_branches: false,
            has_conditions: false,
            task_description: "六步流程".into(),
        };
        // default: node_count=6 > 5 → TreeOfThoughts
        let (engine_a, variant_a) = ab_test_route(&features, "default");
        assert_eq!(engine_a, ReasoningEngine::TreeOfThoughts);
        assert_eq!(variant_a, "default");

        // conservative_sm: state_machine_min_nodes=15, node_count=6 → ToT
        let (engine_b, variant_b) = ab_test_route(&features, "conservative_sm");
        assert_eq!(engine_b, ReasoningEngine::TreeOfThoughts);
        assert_eq!(variant_b, "conservative_sm");
    }
}
