// SPDX-License-Identifier: AGPL-3.0-only

/** 技能学习闭环（借鉴 Hermes Agent）前端类型 */

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
  operationType: PendingOperationType;
  skillId: string | null;
  skillName: string | null;
  filePath: string | null;
  proposal: SkillProposal | null;
  content: string;
  reason: string;
  riskLevel: RiskLevel;
  createdAt: string;
  status: ApprovalStatus;
  approvedAt: string | null;
  rejectedAt: string | null;
  rejectionReason: string | null;
}

export interface SkillLearningConfig {
  minToolCallsForCreation: number;
  minStepsForCreation: number;
  enableSkillCreation: boolean;
  enableSkillPatching: boolean;
  enableBackgroundReview: boolean;
  writeApprovalGate: boolean;
  maxReviewMessages: number;
  reviewIntervalSecs: number;
  dedupSimilarityThreshold: number;
  storagePath: string;
  skillsRoot: string;
}

export interface LearnSkillInput {
  name?: string;
  description?: string;
  sourceType: string;
  content: string;
  context?: string;
  autoApprove?: boolean;
}

export interface LearnSkillResult {
  skillName: string;
  skillPath: string;
  skillContent: string;
  references: string[];
  confidence: number;
  stepsTaken: string[];
  requiresApproval: boolean;
  operationId: string | null;
}

/** 记忆写审批门 */
export interface MemoryWriteApprovalConfig {
  enabled: boolean;
  minImportanceForApproval: number;
  autoApproveNamespaces: string[];
}

export interface PendingMemoryWrite {
  id: string;
  content: string;
  namespace: string | null;
  importance: number;
  reason: string;
  status: string;
}
