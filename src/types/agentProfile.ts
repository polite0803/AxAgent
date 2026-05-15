// AgentProfile — 融合 ExpertRole + AgentRole 的智能体能力集
// 替代原有的分离式 ExpertRole 和 AgentRole 概念

import type { AgentBehaviorMode } from "./agent";
import type { ExpertCategory } from "./expert";

export interface AgentProfile {
  id: string;
  name: string;
  /** i18n 键（用于内置预设），如 "expertPreset.generalAssistant.name" */
  nameKey?: string;
  description: string | null;
  /** i18n 键（用于内置预设），如 "expertPreset.generalAssistant.description" */
  descKey?: string;
  category: ExpertCategory;
  icon: string;
  systemPrompt: string;
  /** AgentRole 类型字符串, null 表示自动推断 */
  agentRole: string | null;
  source: "builtin" | "agency" | "custom";
  tags: string[];
  suggestedProviderId?: string;
  suggestedModelId?: string;
  suggestedTemperature?: number;
  suggestedMaxTokens?: number;
  searchEnabled?: boolean;
  recommendPermissionMode?: AgentBehaviorMode;
  recommendedTools?: string[];
  disallowedTools?: string[];
  recommendedWorkflows?: string[];
  sortOrder: number;
  isEnabled: boolean;
  expertId?: string | null;
  createdAt: number;
  updatedAt: number;
}

// ExpertCategory 已从 ./expert 导入，此处不重复导出
export type { ExpertCategory };

export interface CreateAgentProfileInput {
  name: string;
  description?: string;
  category?: ExpertCategory;
  icon?: string;
  systemPrompt?: string;
  agentRole?: string;
  source?: "builtin" | "agency" | "custom";
  tags?: string[];
  suggestedProviderId?: string;
  suggestedModelId?: string;
  suggestedTemperature?: number;
  suggestedMaxTokens?: number;
  searchEnabled?: boolean;
  recommendPermissionMode?: AgentBehaviorMode;
  recommendedTools?: string[];
  disallowedTools?: string[];
  recommendedWorkflows?: string[];
  expertId?: string;
}

export interface UpdateAgentProfileInput {
  name?: string;
  description?: string | null;
  category?: ExpertCategory;
  icon?: string;
  systemPrompt?: string;
  agentRole?: string | null;
  tags?: string[];
  isEnabled?: boolean;
}

/** 将 AgentProfile 转换为旧版 ExpertRole 格式，用于向后兼容 */
export function agentProfileToExpertRole(
  profile: AgentProfile,
): import("./expert").ExpertRole {
  return {
    id: profile.id,
    displayName: profile.name,
    description: profile.description ?? "",
    category: profile.category,
    icon: profile.icon,
    systemPrompt: profile.systemPrompt,
    source: profile.source,
    tags: profile.tags,
    suggestedProviderId: profile.suggestedProviderId,
    suggestedModelId: profile.suggestedModelId,
    suggestedTemperature: profile.suggestedTemperature,
    suggestedMaxTokens: profile.suggestedMaxTokens,
    searchEnabled: profile.searchEnabled,
    recommendPermissionMode: profile.recommendPermissionMode,
    recommendedTools: profile.recommendedTools,
    recommendedWorkflows: profile.recommendedWorkflows,
    agentProfileId: profile.id,
  };
}
