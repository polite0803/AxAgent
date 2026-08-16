// SPDX-License-Identifier: AGPL-3.0-only

/** 待审批请求（HITL 工作流审批） */
export interface ApprovalRequest {
  id: string;
  executionId: string;
  nodeId: string;
  workflowId: string;
  title: string;
  message: string;
  status: "pending" | "approved" | "rejected" | "expired";
  approver?: string;
  timeoutSecs: number;
  expiresAt: number;
  createdAt: number;
  decision?: string;
  comment?: string;
}
