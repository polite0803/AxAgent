/** 工具类别枚举，与后端 ToolCategory 严格对应 */
export const ToolCategory = {
  FileRead: "file_read",
  FileWrite: "file_write",
  Shell: "shell",
  Network: "network",
  System: "system",
  Agent: "agent",
  Vcs: "vcs",
  Automation: "automation",
  Communication: "communication",
  AiMedia: "ai_media",
  Integration: "integration",
  Storage: "storage",
  Knowledge: "knowledge",
  Browser: "browser",
  Desktop: "desktop",
} as const;

export type ToolCategory = (typeof ToolCategory)[keyof typeof ToolCategory];

/** 工具类别中文标签映射 */
export const ToolCategoryLabels: Record<ToolCategory, string> = {
  [ToolCategory.FileRead]: "文件读取",
  [ToolCategory.FileWrite]: "文件写入",
  [ToolCategory.Shell]: "Shell 命令",
  [ToolCategory.Network]: "网络请求",
  [ToolCategory.System]: "系统工具",
  [ToolCategory.Agent]: "Agent 工具",
  [ToolCategory.Vcs]: "版本控制",
  [ToolCategory.Automation]: "自动化",
  [ToolCategory.Communication]: "通信",
  [ToolCategory.AiMedia]: "AI 媒体",
  [ToolCategory.Integration]: "外部集成",
  [ToolCategory.Storage]: "存储管理",
  [ToolCategory.Knowledge]: "知识库",
  [ToolCategory.Browser]: "浏览器",
  [ToolCategory.Desktop]: "桌面控制",
};

/** 工具完整信息（与后端 DTO 对齐） */
export type LocalToolInfo = {
  /** 工具主名称 */
  name: string;
  /** 工具描述（给 LLM 看） */
  description: string;
  /** 工具类别字符串（如 "file_read"） */
  category: string;
  /** 是否破坏性操作（不可逆） */
  isDestructive: boolean;
  /** 是否只读操作 */
  isReadOnly: boolean;
  /** 是否可以并发执行 */
  isConcurrencySafe: boolean;
  /** 此工具是否被单独启用（受分类启用状态影响） */
  enabled: boolean;
};

/** 工具组信息（用于 UI 展示） */
export type LocalToolGroupInfo = {
  groupId: string;
  groupName: string;
  /** 分类描述 */
  description: string;
  enabled: boolean;
  tools: LocalToolInfo[];
};

/** 工具权限模式 */
export const PermissionMode = {
  /** 只读：仅允许只读工具 */
  ReadOnly: "read_only",
  /** 允许：允许所有已验证的工具 */
  Allow: "allow",
  /** 工作区写入：允许工作区内文件写入 */
  WorkspaceWrite: "workspace_write",
  /** 完全访问：允许所有操作含危险操作 */
  DangerFullAccess: "danger_full_access",
  /** 提示模式：每次操作需用户确认 */
  Prompt: "prompt",
} as const;

export type PermissionMode = (typeof PermissionMode)[keyof typeof PermissionMode];

/** 权限模式中文标签 */
export const PermissionModeLabels: Record<PermissionMode, string> = {
  [PermissionMode.ReadOnly]: "只读",
  [PermissionMode.Allow]: "允许",
  [PermissionMode.WorkspaceWrite]: "工作区写入",
  [PermissionMode.DangerFullAccess]: "完全访问",
  [PermissionMode.Prompt]: "每次确认",
};
