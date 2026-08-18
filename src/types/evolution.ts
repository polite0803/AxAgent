// SPDX-License-Identifier: AGPL-3.0-only

// 自我进化双通道闭环的前端类型定义（与后端对齐）：
// - 通道一：能力补齐提议（CapabilityGapProposal）——认知编排器拒绝/NO_CANDIDATE/Clarify
//   三触发点生成，前端 EvolutionConsentModal 弹窗征求用户同意。
// - 字段命名与后端 `CapabilityGapProposal`（serde camelCase）逐一对应。

/** 攻击类别（与后端 `PromptAttackCategory` snake_case 对齐）。 */
export type PromptAttackCategory =
  | "role_override"
  | "delimiter"
  | "task_hijack"
  | "prompt_leak"
  | "jailbreak";

/** 能力缺口类型（与后端 `CapabilityGapType` snake_case 对齐）。 */
export type CapabilityGapType =
  | "guard_rule"
  | "exempt_authorize"
  | "capability_missing"
  | "skill_evolution";

/** 运行时动态注册的工具（与后端 `list_runtime_tools` 返回对齐）。 */
export interface RuntimeToolInfo {
  /** 工具名 */
  name: string;
  /** 注册来源：`runtime_evolution`（Agent 进化部署）/ `system_evolution`（自指工具） */
  source: string;
}

/** 能力补齐提议（与后端 `CapabilityGapProposal` camelCase 对齐）。 */
export interface CapabilityGapProposal {
  /** 提议唯一 ID（如 `gap:{timestamp}`） */
  id: string;
  /** 缺口类型（GuardRule / ExemptAuthorize / CapabilityMissing） */
  gapType: CapabilityGapType;
  /** 安全拦截场景才有：命中模式的攻击类别 */
  category?: PromptAttackCategory | null;
  /** 前端弹窗标题 */
  title: string;
  /** 新增防护规则 / 授权范围 / 补齐工作流描述 */
  proposal: string;
  /** 命中原因 + 缺口说明 */
  reason: string;
  /** 影响范围 */
  impact: string;
  /** 可逆副作用说明（挂 Disposer） */
  rollback: string;
  /** 提议创建时间（ISO8601） */
  createdAt: string;
}

// ── 进化证据视图（cognitive_evolution_decision，T5A.4） ──────────
//
// 与后端 `EvolutionEvidenceView` / `ExecutionFeedbackView` /
// `ToolExecutionDetail`（serde camelCase）逐字段对齐。前端展示
// 「决策标签流 → 贝叶斯后验」的进化决策，以及进化产物真实执行反馈。

/** 进化产物真实执行反馈对照明细（按 tool_id，T5A.4）。 */
export interface ToolExecutionDetail {
  /** 进化产物标识（GeneratedTool.id） */
  toolId: string;
  /** 真实执行次数 */
  usageCount: number;
  /** 真实成功次数 */
  successes: number;
  /** 真实失败次数 */
  failures: number;
}

/** 进化产物真实执行反馈汇总（与决策标签推断对照展示）。 */
export interface ExecutionFeedbackView {
  /** 有真实执行反馈的进化产物数 */
  toolCount: number;
  /** 真实执行总次数 */
  totalRuns: number;
  /** 真实成功总次数 */
  totalSuccesses: number;
  /** 真实失败总次数 */
  totalFailures: number;
  /** 真实成功率（0~1，无执行时 0） */
  successRate: number;
  /** 按产物的明细（按 toolId 排序） */
  details: ToolExecutionDetail[];
}

/** 进化决策枚举（与后端 describe() 输出对齐）。 */
export type EvolutionDecision = "evolve" | "stable" | "observe";

/** 会话决策标签流的贝叶斯进化评估结果（cognitive_evolution_decision）。 */
export interface EvolutionEvidenceView {
  /** 进化决策（evolve / stable / observe） */
  decision: EvolutionDecision;
  /** 决策原因（中文） */
  reason: string;
  /** 贝叶斯后验 P(success)（决策标签流 + 真实执行反馈融合后） */
  pSuccess: number;
  /** 已积累的（置信度加权）证据量（含真实执行反馈） */
  evidenceVolume: number;
  /** 消费的证据条数（有效决策标签，排除 clarify/ask 中立） */
  consumedLabels: number;
  /** 决策标签总数 */
  totalLabels: number;
  /** 证据来源的路由路径（去重） */
  routePaths: string[];
  /** 进化产物真实执行反馈汇总（真实成败证据） */
  executionFeedback: ExecutionFeedbackView;
}
