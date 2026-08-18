// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { BudgetState, TaskReport } from "@/types";
import { create } from "zustand";

interface SchedulerStore {
  budget: BudgetState | null;
  loading: boolean;
  error: string | null;
  /** 拉取预算用量（含熔断状态）。 */
  fetchBudget: () => Promise<void>;
  /** 设置成本上限（USD），传 null 表示不限。保存后刷新本地预算态并返回新预算。 */
  setBudget: (maxBudget: number | null) => Promise<BudgetState>;
  /** 取长时任务报告。 */
  getTaskReport: (taskId: string) => Promise<TaskReport>;
  /** 恢复引导：把未完成任务重新入队，返回被恢复的任务 id。 */
  restorePendingTasks: () => Promise<string[]>;
}

export const useSchedulerStore = create<SchedulerStore>((set) => ({
  budget: null,
  loading: false,
  error: null,

  fetchBudget: async () => {
    set({ loading: true, error: null });
    try {
      const budget = await invoke<BudgetState>("get_budget_usage");
      set({ budget, loading: false });
    } catch (e) {
      set({ loading: false, error: String(e) });
    }
  },

  setBudget: async (maxBudget) => {
    set({ loading: true, error: null });
    try {
      const budget = await invoke<BudgetState>("set_budget", {
        max_budget: maxBudget,
      });
      set({ budget, loading: false });
      return budget;
    } catch (e) {
      set({ loading: false, error: String(e) });
      throw e;
    }
  },

  getTaskReport: async (taskId) => {
    return invoke<TaskReport>("get_task_report", { task_id: taskId });
  },

  restorePendingTasks: async () => {
    return invoke<string[]>("restore_pending_tasks");
  },
}));
