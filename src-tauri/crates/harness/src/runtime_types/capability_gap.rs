// SPDX-License-Identifier: AGPL-3.0-only
//
// 能力补齐提议相关类型 — 认知编排器双通道闭环（通道一：能力补齐）的 DTO 基座。
//
// 这些类型位于 harness（foundation）层：纯 DTO / 纯枚举，零 axagent-* 依赖。
// 消费方：cognitive.rs（拒绝/NO_CANDIDATE/Clarify 三触发点生成提议）、
// 前端同意弹窗（EvolutionConsentModal）按 camelCase 反序列化展示。

/// 安全拦截命中模式的攻击类别（与 `PatternPromptGuard` 现有分类对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptAttackCategory {
    /// 角色覆盖（如 ignore all previous instructions、developer mode 诱导）
    RoleOverride,
    /// 分隔符注入（如 ChatML `<|im_start|>`、伪造 `---BEGIN---` 系统提示）
    Delimiter,
    /// 任务劫持（如 forget your training、your new task is）
    TaskHijack,
    /// 提示词泄露（如 print your system prompt、reveal your instructions）
    PromptLeak,
    /// 越狱（jailbreak、override your safety 等通用模式）
    Jailbreak,
}

/// 能力缺口类型 — 决定提议走哪条补齐路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityGapType {
    /// 防护规则缺口：攻击手法不在静态列表 → 提议补防护规则（挂 Disposer 可回滚）
    GuardRule,
    /// 误伤豁免：本地 IDE 合法诉求被静态规则命中 → 提议有界安全豁免授权
    ExemptAuthorize,
    /// 能力缺口：系统当前无能力安全处理 → 提议生成工作流 / 技能 / 工具
    CapabilityMissing,
    /// 技能偏弱：贝叶斯后验判定表现不佳 → 提议进化改进该技能
    SkillEvolution,
}

/// 能力补齐提议（前端弹窗展示 / 用户同意 / 落库）。
///
/// - 生成方：`build_capability_gap_proposal`（cognitive.rs）
/// - 消费方：前端 EvolutionConsentModal → 用户同意/拒绝 → `apply_capability_gap_proposal`
/// - 所有补齐动作必须经用户显式同意（铁律），`rollback` 字段说明可逆副作用（挂 Disposer）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityGapProposal {
    /// 提议唯一 ID（如 `gap:{timestamp}`）
    pub id: String,
    /// 缺口类型（GuardRule / ExemptAuthorize / CapabilityMissing）
    pub gap_type: CapabilityGapType,
    /// 安全拦截场景才有：命中模式的攻击类别
    pub category: Option<PromptAttackCategory>,
    /// 前端弹窗标题
    pub title: String,
    /// 新增防护规则 / 授权范围 / 补齐工作流描述
    pub proposal: String,
    /// 命中原因 + 缺口说明
    pub reason: String,
    /// 影响范围
    pub impact: String,
    /// 可逆副作用说明（挂 Disposer）
    pub rollback: String,
    /// 提议创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
}
