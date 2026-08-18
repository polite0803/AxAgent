// SPDX-License-Identifier: AGPL-3.0-only

/** 预算用量状态（对应后端 `scheduler::gate::BudgetState`，scheduler_budget 内存态）。 */
export interface BudgetState {
  /** 成本上限（USD）。null = 不限。 */
  maxBudget: number | null;
  /** 已累计成本（USD）。 */
  spent: number;
  /** 是否已熔断（累计成本超限）。 */
  tripped: boolean;
}

/** 审批汇总条目（对后端 `report::ApprovalSummary`）。 */
export interface ApprovalSummary {
  approver: string | null;
  status: string | null;
  decision: string | null;
}

/** 长时任务报告（对后端 `report::TaskReport`）。 */
export interface TaskReport {
  taskId: string;
  title: string;
  status: string;
  durationMs: number;
  /** 成本折算（USD） */
  costUsd: number;
  /** 审批记录汇总 */
  approvals: ApprovalSummary[];
  /** 报告生成时间（epoch ms） */
  generatedAt: number;
}
