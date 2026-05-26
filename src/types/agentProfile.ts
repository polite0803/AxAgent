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
  agentRole?: string | null;
  tags?: string[];
  isEnabled?: boolean;
}

/** 全局角色元数据：从 agent_roles 表同步，提供图标和 i18n key */
export const AGENT_ROLE_META: Record<
  string,
  { icon: string; labelKey: string }
> = {
  coordinator: { icon: "🎯", labelKey: "agentRole.coordinator" },
  researcher: { icon: "🔍", labelKey: "agentRole.researcher" },
  planner: { icon: "📋", labelKey: "agentRole.planner" },
  developer: { icon: "💻", labelKey: "agentRole.developer" },
  reviewer: { icon: "👀", labelKey: "agentRole.reviewer" },
  browser: { icon: "🌐", labelKey: "agentRole.browser" },
  synthesizer: { icon: "🔬", labelKey: "agentRole.synthesizer" },
  executor: { icon: "⚙️", labelKey: "agentRole.executor" },
};

/** 根据角色名获取图标，未知角色返回默认图标 */
export function getAgentRoleIcon(role: string | null | undefined): string {
  if (!role) {
    return "🤖";
  }
  return AGENT_ROLE_META[role]?.icon ?? "🤖";
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
    systemPrompt: "", // AgentProfile 不再预缓存 prompt，运行时从 Role+Expert 拼接
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
