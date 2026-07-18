// SPDX-License-Identifier: AGPL-3.0-only

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
  /** 系统提示词（builtin/agency 来源有效，custom 来源为空） */
  systemPrompt?: string;
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
  /** 工具域列表（仅 agency/custom 来源有效，展示用；builtin 无此字段） */
  activeDomains?: string[];
  recommendedWorkflows?: string[];
  sortOrder: number;
  isEnabled: boolean;
  expertId?: string | null;
  /**
   * 业务岗位 ID（business_roles.id）。
   * 对应后端 agent_profiles.business_role_id 字段。
   * null 表示未绑定业务岗位。
   */
  businessRoleId?: string | null;
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
  /** 业务岗位 ID（创建时可绑定） */
  businessRoleId?: string;
}

export interface UpdateAgentProfileInput {
  name?: string;
  description?: string | null;
  category?: ExpertCategory;
  icon?: string;
  agentRole?: string | null;
  tags?: string[];
  isEnabled?: boolean;
  /** 业务岗位 ID（传 null 解绑） */
  businessRoleId?: string | null;
}
