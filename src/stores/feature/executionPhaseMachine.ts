// SPDX-License-Identifier: AGPL-3.0-only

/** 执行阶段状态机：类型定义 + 转换规则 + 活跃/终端判断 */

export type ExecutionPhase =
  | "idle"
  | "planning"
  | "executing"
  | "waiting_permission"
  | "completed"
  | "failed"
  | "cancelled";

export const PHASE_TRANSITIONS: Record<ExecutionPhase, ExecutionPhase[]> = {
  idle: ["planning", "executing", "completed", "failed", "cancelled"],
  planning: ["executing", "failed", "cancelled"],
  executing: [
    "executing",
    "waiting_permission",
    "completed",
    "failed",
    "cancelled",
  ],
  waiting_permission: ["executing", "cancelled"],
  completed: ["idle", "executing"],
  failed: ["idle", "executing"],
  cancelled: ["idle", "executing"],
};

export const ACTIVE_PHASES: Set<ExecutionPhase> = new Set([
  "planning",
  "executing",
  "waiting_permission",
]);

export const TERMINAL_PHASES: Set<ExecutionPhase> = new Set([
  "completed",
  "failed",
  "cancelled",
]);
