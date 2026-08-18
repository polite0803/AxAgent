// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 任务形态分类器类型 — 原则三核心标尺。
 *
 * 与后端 `axagent_harness::task_shape` 一一对应。
 * 由 `TaskShapeClassifier` 在能力路由前产出，注入到消息、路由决策、Agent 启动参数。
 *
 * ## 序列化对齐说明
 *
 * - `ContextRetentionCost` / `SecurityIsolationNeed`：后端 `#[serde(rename_all = "snake_case")]`，
 *   枚举值序列化为 snake_case 字符串（如 `"long_running_project"`、`"read_only_thought"`）。
 * - `ExecutionStrategy`：后端 externally tagged enum + `#[serde(rename_all = "camelCase")]`，
 *   unit variant 序列化为字符串（如 `"handleLocally"`），
 *   带字段 variant 序列化为对象（如 `{"delegateSingleExpert": {"expertId": "..."}}`）。
 * - `TaskShapeDecision`：后端 `#[serde(rename_all = "camelCase")]`，字段名为 camelCase。
 */

/** 上下文保留成本（高连贯性 → 合并优先）。
 *
 * 值为 snake_case，与后端 `#[serde(rename_all = "snake_case")]` 对齐。 */
export type ContextRetentionCost =
  | "ephemeral"
  | "resumable"
  | "long_running_project"
  | "persistent_journey";

/** 安全隔离需求（高风险 → 拆分 + 物理权限墙优先）。
 *
 * 值为 snake_case，与后端 `#[serde(rename_all = "snake_case")]` 对齐。 */
export type SecurityIsolationNeed =
  | "read_only_thought"
  | "workspace_write"
  | "sandbox_required"
  | "approval_and_air_gap";

/** 执行策略（分类器输出，下游编排唯一入口）。
 *
 * 后端为 externally tagged enum，unit variant 序列化为字符串，
 * 带字段 variant 序列化为 `{variantName: {fields}}` 格式。
 * 使用 `Record<string, unknown>` 兼容两种形态，通过 `strategyKind()` 辅助函数提取策略名。 */
export type ExecutionStrategy =
  | "handleLocally"
  | "orchestratorDag"
  | "approvalGate"
  | { delegateSingleExpert: { expertId: string } }
  | { parallelExperts: { expertIds: string[] } };

/** 从 ExecutionStrategy 提取策略名（用于 i18n 翻译键查找）。 */
export function strategyKind(strategy: ExecutionStrategy): string {
  if (typeof strategy === "string") {
    return strategy;
  }
  // 带字段 variant：取对象第一个 key 作为策略名
  return Object.keys(strategy)[0] ?? "handleLocally";
}

/** 任务形态决策结果 */
export interface TaskShapeDecision {
  /** 上下文保留成本标尺 */
  contextCost: ContextRetentionCost;
  /** 安全隔离需求标尺 */
  isolationNeed: SecurityIsolationNeed;
  /** 推荐执行策略 */
  recommendedStrategy: ExecutionStrategy;
  /** 合并倾向：0~1，越高越倾向合并（减少转述） */
  mergeScore: number;
  /** 拆分倾向：0~1，越高越倾向拆分（保证精度/隔离） */
  splitScore: number;
  /** 关键证据（审计 + 前端展示） */
  evidence: string[];
}
