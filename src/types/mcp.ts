// SPDX-License-Identifier: AGPL-3.0-only

export type McpTransport = "stdio" | "http" | "sse";
export type McpPermissionPolicy = "ask" | "allow_safe" | "allow_all";
export type ToolExecutionStatus =
  | "pending"
  | "running"
  | "success"
  | "failed"
  | "cancelled";

export type McpServerSource = "builtin" | "custom";

export type McpServer = {
  id: string;
  name: string;
  /** 用户友好别名（如 "网页搜索"、"文件系统"） */
  alias?: string;
  /** 功能描述（如 "通过 DuckDuckGo 搜索网页"） */
  description?: string;
  transport: McpTransport;
  command?: string;
  argsJson?: string;
  endpoint?: string;
  envJson?: string;
  enabled: boolean;
  permissionPolicy: McpPermissionPolicy;
  source: McpServerSource;
  discoverTimeoutSecs?: number;
  executeTimeoutSecs?: number;
  headersJson?: string;
  iconType?: string;
  iconValue?: string;
};

export type McpMode = "auto" | "manual" | "disabled";

export type ToolDescriptor = {
  id: string;
  serverId: string;
  name: string;
  description?: string;
  inputSchemaJson?: string;
};

export type ToolExecution = {
  id: string;
  conversationId: string;
  messageId?: string;
  serverId: string;
  toolName: string;
  status: ToolExecutionStatus;
  inputPreview?: string;
  outputPreview?: string;
  errorMessage?: string;
  durationMs?: number;
  createdAt: string;
  approvalStatus?: string;
};

export type CreateMcpServerInput = {
  name: string;
  transport: McpTransport;
  command?: string;
  args?: string[];
  endpoint?: string;
  env?: Record<string, string>;
  enabled?: boolean;
  permissionPolicy?: McpPermissionPolicy;
  discoverTimeoutSecs?: number;
  executeTimeoutSecs?: number;
  headersJson?: string;
  iconType?: string;
  iconValue?: string;
};

export type UpdateMcpServerInput = Partial<CreateMcpServerInput>;
