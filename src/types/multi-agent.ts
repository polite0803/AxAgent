// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G5 Multi-Agent 固定角色 pool 前端类型定义
 *
 * 与后端 `commands/multi_agent.rs` 中的 DTO 对齐。
 * 后端使用 #[serde(rename_all = "camelCase")]，前端类型按 camelCase 命名。
 */

/** G5 角色信息（前端展示用，对应后端 MultiAgentRoleInfo） */
export interface MultiAgentRoleInfo {
  /** 角色 ID：analyst / implementer / reviewer */
  id: string;
  /** 角色名称（中文） */
  name: string;
  /** 角色描述 */
  description: string;
  /** 最大并发数 */
  maxConcurrent: number;
  /** 超时秒数 */
  timeoutSeconds: number;
}

/** delegate_task 命令入参（对应后端 DelegateTaskInput） */
export interface DelegateTaskInput {
  /** 目标角色 ID：analyst / implementer / reviewer */
  roleName: string;
  /** 子任务描述（中文） */
  task: string;
  /** 上下文变量（可选，JSON 形式注入到 user message 前） */
  context?: Record<string, unknown> | null;
  /** LLM 供应商 ID */
  providerId: string;
  /** 模型 ID */
  modelId: string;
  /** 温度（可选，默认 0.2） */
  temperature?: number;
  /** 最大输出 tokens（可选，默认 2048） */
  maxTokens?: number;
}

/** delegate_task 命令输出（对应后端 DelegateTaskResult） */
export interface DelegateTaskResult {
  /** 委派 ID（用于追踪） */
  delegationId: string;
  /** 角色 ID */
  roleName: string;
  /** LLM 生成的文本输出 */
  content: string;
  /** 输入 tokens */
  promptTokens: number;
  /** 输出 tokens */
  completionTokens: number;
  /** 调用耗时（毫秒） */
  durationMs: number;
}

/** 委派历史记录（前端本地维护，无后端表） */
export interface DelegationHistoryEntry {
  /** 委派 ID */
  delegationId: string;
  /** 角色 ID */
  roleName: string;
  /** 任务描述 */
  task: string;
  /** LLM 输出 */
  content: string;
  /** 调用时间戳（ms） */
  timestamp: number;
  /** 耗时（ms） */
  durationMs: number;
  /** token 使用 */
  promptTokens: number;
  completionTokens: number;
  /** 是否成功 */
  success: boolean;
  /** 错误信息（失败时） */
  error?: string;
}
