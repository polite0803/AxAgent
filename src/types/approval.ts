// SPDX-License-Identifier: AGPL-3.0-only

/** 待审批请求（HITL 工作流审批） */
export interface ApprovalRequest {
  id: string;
  execution_id: string;
  workflow_id: string;
  message: string;
  status: "pending" | "approved" | "rejected";
  expires_at: number;
  decision?: string;
}
