// SPDX-License-Identifier: AGPL-3.0-only

/** 技能学习闭环（借鉴 Hermes Agent）前端类型，与后端 DTO 对齐（snake_case） */

// SkillProposal 已有权威定义于 ./index（types/index.ts），此处复用避免重复定义
import type { SkillProposal } from "./index";

export type PendingOperationType =
  | "create_skill"
  | "patch_skill"
  | "edit_skill"
  | "delete_skill"
  | "write_file"
  | "remove_file";

export type ApprovalStatus = "pending" | "approved" | "rejected" | "expired";

export type RiskLevel = "low" | "medium" | "high" | "critical";

export interface PendingSkillOperation {
  id: string;
  operation_type: PendingOperationType;
  skill_id: string | null;
  skill_name: string | null;
  file_path: string | null;
  proposal: SkillProposal | null;
  content: string;
  reason: string;
  risk_level: RiskLevel;
  created_at: string;
  status: ApprovalStatus;
  approved_at: string | null;
  rejected_at: string | null;
  rejection_reason: string | null;
}

export interface SkillLearningConfig {
  min_tool_calls_for_creation: number;
  min_steps_for_creation: number;
  enable_skill_creation: boolean;
  enable_skill_patching: boolean;
  enable_background_review: boolean;
  write_approval_gate: boolean;
  max_review_messages: number;
  review_interval_secs: number;
  dedup_similarity_threshold: number;
  storage_path: string;
  skills_root: string;
}

export interface LearnSkillInput {
  name?: string;
  description?: string;
  source_type: string;
  content: string;
  context?: string;
  auto_approve?: boolean;
}

export interface LearnSkillResult {
  skill_name: string;
  skill_path: string;
  skill_content: string;
  references: string[];
  confidence: number;
  steps_taken: string[];
  requires_approval: boolean;
  operation_id: string | null;
}

/** 记忆写审批门 */
export interface MemoryWriteApprovalConfig {
  enabled: boolean;
  min_importance_for_approval: number;
  auto_approve_namespaces: string[];
}

export interface PendingMemoryWrite {
  id: string;
  content: string;
  namespace: string | null;
  importance: number;
  reason: string;
  status: string;
}
