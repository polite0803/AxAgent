// SPDX-License-Identifier: AGPL-3.0-only

/** 待审批请求（HITL 工作流审批） */
export interface ApprovalRequest {
  id: string;
  execution_id: string;
  node_id: string;
  workflow_id: string;
  title: string;
  message: string;
  status: "pending" | "approved" | "rejected" | "expired";
  approver?: string;
  timeout_secs: number;
  expires_at: number;
  created_at: number;
  decision?: string;
  comment?: string;
}
