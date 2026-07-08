//! 专家 ID 注册表 + 股票分析约束文本。
//!
//! 各专家提示词定义在 `agency_experts/stock-analysis/*.md`，
//! 格式为 YAML frontmatter + Markdown body。
//! 实际加载由 `stock_analysis_setup.rs` 的 `include_str!` 编译期嵌入完成。

/// 22 个专家 ID（对应 22 个 .md 文件）
pub const EXPERT_IDS: &[&str] = &[
    "market-analyst",
    "sentiment-analyst",
    "news-analyst",
    "fundamentals-analyst",
    "policy-analyst",
    "hot-money-tracker",
    "lockup-watcher",
    "research-analyst",
    "sector-analyst",
    "bull-researcher",
    "bear-researcher",
    "bull-r2",
    "bear-r2",
    "bull-r3",
    "bear-r3",
    "aggressive-debator",
    "conservative-debator",
    "neutral-debator",
    "research-manager",
    "trader",
    "reflection",
    "value-investor",
    "data-quality-inspector",
    "rule-checker",
    "catalyst-analyst",
];

/// 股票分析硬约束文本（HEAD 锚定——利用 LLM primacy 效应）。
/// 置于 system prompt 头部，确保"禁编造/禁预测"被模型优先关注。
pub const STOCK_HARD_CONSTRAINTS: &str = "\
## 关键约束（最高优先级，必须遵守）
1. **反幻觉**：所有数字必须能在\"上游节点输出\"找到来源；缺失必须写 `信息缺失` 或 `\"data_gaps\"`，**禁止编造**。
2. **禁预测大盘点位 / 个股目标价 / 目标涨幅**。仅描述\"在何种条件下偏向哪个方向\"及其所需证据。";

/// 股票分析软约束文本（TAIL 锚定——利用 LLM recency 效应）。
/// 置于 system prompt 尾部，确保按协作/自检标准输出。
pub const STOCK_COLLAB_REMINDER: &str = "\
## 协作与自检（输出前必过）
- 你的输出会被辩论 / 风险 / 决策节点引用：论点要具体、引用要可查、立场要明确。
- 输出前自查 3 项：① 数字有来源？② 论点前后一致？③ 是否回避了关键风险？";

/// 时间锚定提示：告知 LLM 当前为 "as_of_date" 之前的封闭世界，禁止引用未来信息。
/// 拼到 system prompt 头部（在 STOCK_HARD_CONSTRAINTS 之前），最高优先级。
pub fn asof_system_prompt(as_of_date: &str) -> String {
    format!(
        "## 时间锚定（最高优先级，禁止违反）\n\
         - **当前分析世界观：截至 {as_of_date} 收盘**\n\
         - **严格禁止**引用 {as_of_date} 之后发生的任何新闻、行情、公告、研报、宏观事件\n\
         - 如不得不引用某条「近期」信息，请先判断它的时间戳是否在 {as_of_date} 之前；否则视为不存在\n\
         - 涉及未来时使用「假设/条件/概率」等开放语，不得使用「已经/正在/将/预期」等断言性时态\n\
         - 若训练知识跨越该截止日，仍以「截至 {as_of_date}」为准；冲突时倾向保守/无数据"
    )
}

/// 判断角色名是否为 stock-analysis 工作流下注入了 A 股约束的 5 个角色。
/// 与 `agent_executor.rs` 4a-pre 的 matches! 完全一致。
pub fn is_stock_role(role: &str) -> bool {
    matches!(
        role,
        "stock-analyst" | "debater" | "risk-evaluator" | "trader" | "decision-maker"
    )
}

#[cfg(test)]
mod asof_prompt_tests {
    use super::*;

    #[test]
    fn asof_prompt_mentions_cutoff_date() {
        let p = asof_system_prompt("2026-06-01");
        assert!(p.contains("2026-06-01"));
        assert!(p.contains("时间锚定"));
        assert!(p.contains("禁止"));
    }

    /// Snapshot: 防止 as-of 文案被意外删改；变更需 review 升级
    #[test]
    fn snapshot_asof_block_is_stable() {
        let p = asof_system_prompt("2026-06-01");
        let expected_prefix = "## 时间锚定（最高优先级，禁止违反）\n";
        assert!(p.starts_with(expected_prefix));
        // 关键否定词都在
        for kw in [
            "严格禁止",
            "假设/条件/概率",
            "已经/正在/将/预期",
            "截至 2026-06-01",
        ] {
            assert!(p.contains(kw), "asof prompt missing keyword: {kw}");
        }
    }
}
