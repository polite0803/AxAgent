// SPDX-License-Identifier: AGPL-3.0-only

export type ExpertCategory =
  | "general"
  | "development"
  | "security"
  | "data"
  | "finance"
  | "devops"
  | "design"
  | "writing"
  | "business";

export const EXPERT_CATEGORY_KEYS = [
  "general",
  "development",
  "security",
  "data",
  "finance",
  "devops",
  "design",
  "writing",
  "business",
] as const;

export type ExpertCategoryKey = (typeof EXPERT_CATEGORY_KEYS)[number];

/** 分类对应完整 i18n key 的查找表，供 i18n Ally 直接识别。 */
export const EXPERT_CATEGORY_LABELS: Record<ExpertCategoryKey, string> = {
  general: "expertCategory.general",
  development: "expertCategory.development",
  security: "expertCategory.security",
  data: "expertCategory.data",
  finance: "expertCategory.finance",
  devops: "expertCategory.devops",
  design: "expertCategory.design",
  writing: "expertCategory.writing",
  business: "expertCategory.business",
};

/** 使用 AgentProfile (types/agentProfile.ts) 替代。字段映射：displayName→name */
export interface ExpertRole {
  /** 唯一标识，如 "code-reviewer" */
  id: string;
  /** 显示名称，如 "代码审查专家" */
  displayName: string;
  /** 一句话描述 */
  description: string;
  /** 分类 */
  category: ExpertCategory;
  /** 图标 emoji */
  icon: string;
  /** 系统提示词（空字符串表示使用默认提示词） */
  systemPrompt: string;
  /** 来源: builtin=内置预设, agency=agency-agents-zh导入, custom=用户自定义 */
  source: "builtin" | "agency" | "custom";
  /** 搜索标签 */
  tags: string[];

  // 环境预设（选中时可选应用）
  /** 推荐模型供应商 */
  suggestedProviderId?: string;
  /** 推荐模型 */
  suggestedModelId?: string;
  /** 推荐温度 */
  suggestedTemperature?: number;
  /** 推荐最大 token */
  suggestedMaxTokens?: number;
  /** 是否建议开启搜索 */
  searchEnabled?: boolean;

  // Agent 权限控制
  /** 推荐权限模式 */
  recommendPermissionMode?: import("./agent").AgentBehaviorMode;

  /** 推荐的工具名称列表（导入时自动解析匹配） */
  recommendedTools?: string[];
  /** 活跃工具域列表（例如 ["core", "invest"]） */
  activeDomains?: string[];
  /** 推荐的工作流模板 ID 列表（导入时自动解析并创建） */
  recommendedWorkflows?: string[];
  /** 关联的 AgentProfile ID */
  agentProfileId?: string;

  // ── 资历与归属（对齐后端 AgencyExpertDto 扩展字段） ──
  /** 资历等级：junior / mid / senior / expert */
  seniority?: string;
  /** 擅长细分领域列表 */
  specialties?: string[];
  /** 历史成功率（0.0 ~ 1.0） */
  successRate?: number;
  /** 平均执行延迟（毫秒） */
  avgLatencyMs?: number;
  /** 平均 token 成本 */
  avgTokenCost?: number;
}
