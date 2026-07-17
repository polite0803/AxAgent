// SPDX-License-Identifier: AGPL-3.0-only

import type { ExecutionPhase } from "./executionPhaseMachine";
import { ACTIVE_PHASES } from "./executionPhaseMachine";

/** 工具调用追踪 */
export interface CurrentToolCall {
  toolName: string;
  toolUseId: string;
  conversationId: string;
  startedAt: number;
}

/**
 * 判断是否应清除 currentToolCall。
 * 满足以下任一条件时清除：
 * 1. currentToolCall 属于当前完成事件的会话（直接匹配）
 * 2. currentToolCall 所属会话的状态已是终端态（跨对话残留检测）
 */
export function shouldClearToolCall(
  currentToolCall: CurrentToolCall | null,
  phases: Record<string, ExecutionPhase>,
  doneConversationId: string,
): boolean {
  if (!currentToolCall) {
    return false;
  }
  if (currentToolCall.conversationId === doneConversationId) {
    return true;
  }
  const ownerPhase = phases[currentToolCall.conversationId];
  if (!ownerPhase) {
    // 无 phase 记录（默认 idle）：视为未结束的会话，保留其工具调用，不激进清空
    return false;
  }
  return !ACTIVE_PHASES.has(ownerPhase);
}
