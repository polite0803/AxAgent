// SPDX-License-Identifier: AGPL-3.0-only
/**
 * G5 Multi-Agent 固定角色 pool Zustand store
 *
 * 负责：
 * - 列出 G5 固定角色（analyst/implementer/reviewer）
 * - 调用 delegate_task 委派子任务给指定角色
 * - 维护委派历史记录（前端本地，最多 50 条）
 * - loading / error 状态管理
 *
 * 命令清单（与后端 commands/multi_agent.rs 对齐）：
 * - list_multi_agent_roles
 * - delegate_task
 */

import { invoke } from "@/lib/invoke";
import type {
  DelegateTaskInput,
  DelegateTaskResult,
  DelegationHistoryEntry,
  MultiAgentRoleInfo,
} from "@/types/multi-agent";
import { create } from "zustand";

const MAX_HISTORY = 50;

interface MultiAgentState {
  // ── 数据 ──
  /** G5 固定角色列表 */
  roles: MultiAgentRoleInfo[];
  /** 委派历史记录（最新在前） */
  history: DelegationHistoryEntry[];

  // ── 状态 ──
  loadingRoles: boolean;
  delegating: boolean;
  error: string | null;

  // ── Actions ──
  /** 拉取 G5 固定角色列表 */
  fetchRoles: () => Promise<void>;
  /** 委派任务给指定角色 */
  delegateTask: (input: DelegateTaskInput) => Promise<DelegateTaskResult>;
  /** 清空错误 */
  clearError: () => void;
  /** 清空历史记录 */
  clearHistory: () => void;
}

export const useMultiAgentStore = create<MultiAgentState>((set, get) => ({
  roles: [],
  history: [],

  loadingRoles: false,
  delegating: false,
  error: null,

  fetchRoles: async () => {
    set({ loadingRoles: true, error: null });
    try {
      const roles = await invoke<MultiAgentRoleInfo[]>("list_multi_agent_roles");
      set({ roles, loadingRoles: false });
    } catch (e) {
      set({ loadingRoles: false, error: String(e) });
    }
  },

  delegateTask: async (input: DelegateTaskInput) => {
    set({ delegating: true, error: null });
    try {
      const result = await invoke<DelegateTaskResult>("delegate_task", { input });
      const entry: DelegationHistoryEntry = {
        delegationId: result.delegationId,
        roleName: result.roleName,
        task: input.task,
        content: result.content,
        timestamp: Date.now(),
        durationMs: result.durationMs,
        promptTokens: result.promptTokens,
        completionTokens: result.completionTokens,
        success: true,
      };
      const history = [entry, ...get().history].slice(0, MAX_HISTORY);
      set({ history, delegating: false });
      return result;
    } catch (e) {
      const entry: DelegationHistoryEntry = {
        delegationId: `err-${Date.now()}`,
        roleName: input.roleName,
        task: input.task,
        content: "",
        timestamp: Date.now(),
        durationMs: 0,
        promptTokens: 0,
        completionTokens: 0,
        success: false,
        error: String(e),
      };
      const history = [entry, ...get().history].slice(0, MAX_HISTORY);
      set({ history, delegating: false, error: String(e) });
      throw e;
    }
  },

  clearError: () => set({ error: null }),
  clearHistory: () => set({ history: [] }),
}));
