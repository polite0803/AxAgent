export type ExpertCategory =
  | "general"
  | "development"
  | "security"
  | "data"
  | "devops"
  | "design"
  | "writing"
  | "business";

export const EXPERT_CATEGORY_LABELS: Record<ExpertCategory, string> = {
  general: "通用",
  development: "开发",
  security: "安全",
  data: "数据",
  devops: "运维",
  design: "设计",
  writing: "写作",
  business: "商业",
};

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
  /** 推荐权限模式: default=需要审批, accept_edits=自动接受编辑, full_access=全部自动 */
  recommendPermissionMode?: "default" | "accept_edits" | "full_access";

  /** 推荐的工具名称列表（导入时自动解析匹配） */
  recommendedTools?: string[];
  /** 推荐的工作流模板 ID 列表（导入时自动解析并创建） */
  recommendedWorkflows?: string[];
}
