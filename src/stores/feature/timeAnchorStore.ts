import { invoke } from "@/lib/invoke";
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

/**
 * TimeAnchor — 全局时间旅行锚点 store
 *
 * 单一职责：保存"截至过去的某一天"作为分析世界观。
 * 持久化到 localStorage，跨页面共享，跨刷新存活。
 *
 * 模式：
 *   - `live`：默认；分析/荐股用实时数据
 *   - `replay`：用户从 AppHeader 选了 as_of_date，进入"截至过去某日"世界观
 *   - `backtest_sweep`：自动批量回测；不持久化（每次 sweep 独立）
 *
 * 影响：所有受控 API（run_stock_workflow、recommend_stocks 等）入参自动
 * 注入 `as_of_date`，后端 AS_OF.scope 切到对应 context。
 */

export type TimeAnchorMode = "live" | "replay" | "backtest_sweep";

export interface TimeAnchorState {
  /** YYYY-MM-DD；live 模式为 null */
  asOfDate: string | null;
  /** live / replay / backtest_sweep */
  mode: TimeAnchorMode;
  /** 用户是否已 dismiss 首次 Tour 引导气泡 */
  tourSeen: boolean;
  /** 用户最近一次"切回 Live 模式"的二次确认状态（true=已确认可切） */
  pendingLiveConfirm: boolean;
  /** 缺陷 E 修复: 实时降级计数(后端 poll) */
  degradationCount: number;
  /** 缺陷 E 修复: 最近 N 条降级详情(供降级面板展示) */
  degradationLog: Array<{ vendor: string; method: string; reason: string; as_of: string }>;

  setAsOfDate: (date: string | null) => void;
  enterReplay: (date: string) => void;
  /** 切回 Live 模式；requireConfirm=true 时返回 false 等 Modal 确认 */
  enterLive: (requireConfirm?: boolean) => boolean;
  enterBacktestSweep: (date: string) => void;
  /** 从 ReplayWorkbench 进入时强制重选 */
  enterReplayWorkbench: (date: string) => void;
  markTourSeen: () => void;
  confirmPendingLive: () => void;
  cancelPendingLive: () => void;
  /** 缺陷 E 修复: 拉取一次后端降级状态(count + log) */
  refreshDegradation: () => Promise<void>;
  /** 缺陷 E 修复: 启动/停止轮询 */
  startDegradationPolling: () => void;
  stopDegradationPolling: () => void;
}

const DATE_RE = /^\d{4}-\d{2}-\d{2}$/;

function todayIso(): string {
  return new Date().toISOString().slice(0, 10);
}

function isValidPastDate(s: string): boolean {
  if (!DATE_RE.test(s)) { return false; }
  const t = new Date(s + "T00:00:00");
  if (isNaN(t.getTime())) { return false; }
  // 拒绝未来日期（与后端 AsOfContext::new 一致）
  return s <= todayIso();
}

export const useTimeAnchorStore = create<TimeAnchorState>()(
  persist(
    (set, get) => ({
      asOfDate: null,
      mode: "live",
      tourSeen: false,
      pendingLiveConfirm: false,
      degradationCount: 0,
      degradationLog: [],

      setAsOfDate: (date) => {
        if (date === null) {
          set({ asOfDate: null, mode: "live" });
          return;
        }
        if (!isValidPastDate(date)) {
          console.warn("[TimeAnchor] invalid or future date rejected:", date);
          return;
        }
        set({ asOfDate: date, mode: "replay" });
      },

      enterReplay: (date) => {
        if (!isValidPastDate(date)) {
          console.warn("[TimeAnchor] enterReplay rejected:", date);
          return;
        }
        set({ asOfDate: date, mode: "replay", pendingLiveConfirm: false });
        // P2-7 修复: 切到 replay 时清空本地显示和后端全局缓冲(可能有上次
        // replay 残留的降级条目),避免用户进入新 replay 时看到旧数字突变。
        set({ degradationCount: 0, degradationLog: [] });
        invoke<void>("clear_asof_degradation_log").catch(() => {});
        get().startDegradationPolling();
      },

      enterLive: (requireConfirm = false) => {
        const { mode } = get();
        // live→live 直接通过
        if (mode === "live") {
          set({ pendingLiveConfirm: false });
          return true;
        }
        if (requireConfirm) {
          // 提示 UI 弹 Modal 二次确认
          set({ pendingLiveConfirm: true });
          return false;
        }
        set({ asOfDate: null, mode: "live", pendingLiveConfirm: false });
        // 缺陷 E 修复: 切回 live 时清空后端全局降级缓冲 + 停止 polling
        get().stopDegradationPolling();
        invoke<void>("clear_asof_degradation_log").catch(() => {});
        set({ degradationCount: 0, degradationLog: [] });
        return true;
      },

      enterBacktestSweep: (date) => {
        if (!isValidPastDate(date)) {
          console.warn("[TimeAnchor] enterBacktestSweep rejected:", date);
          return;
        }
        set({ asOfDate: date, mode: "backtest_sweep", pendingLiveConfirm: false });
      },

      enterReplayWorkbench: (date) => {
        // ReplayWorkbench 强制重选：忽略当前 asOfDate，直接覆盖
        if (!isValidPastDate(date)) {
          console.warn("[TimeAnchor] enterReplayWorkbench rejected:", date);
          return;
        }
        set({ asOfDate: date, mode: "replay", pendingLiveConfirm: false });
      },

      markTourSeen: () => set({ tourSeen: true }),
      confirmPendingLive: () => {
        set({ asOfDate: null, mode: "live", pendingLiveConfirm: false });
        get().stopDegradationPolling();
        invoke<void>("clear_asof_degradation_log").catch(() => {});
        set({ degradationCount: 0, degradationLog: [] });
      },
      cancelPendingLive: () => set({ pendingLiveConfirm: false }),

      refreshDegradation: async () => {
        try {
          const [count, log] = await Promise.all([
            invoke<number>("get_asof_degradation_count"),
            invoke<Array<{ vendor: string; method: string; reason: string; as_of: string }>>(
              "get_asof_degradation_log",
            ),
          ]);
          set({ degradationCount: count, degradationLog: log });
        } catch (e) {
          // 静默: invoke 失败不打断用户
          console.warn("[TimeAnchor] refreshDegradation failed:", e);
        }
      },
      startDegradationPolling: () => {
        if (typeof window === "undefined") { return; }
        // 用 window 全局 id 存,避免 React 18 strict mode 多次 mount 重复启动
        const w = window as Window & { __ax_degrad_poll_id?: number };
        if (w.__ax_degrad_poll_id) { return; }
        // 立即拉一次,然后每 3s 一次
        get().refreshDegradation();
        w.__ax_degrad_poll_id = window.setInterval(() => {
          get().refreshDegradation();
        }, 3000);
      },
      stopDegradationPolling: () => {
        if (typeof window === "undefined") { return; }
        const w = window as Window & { __ax_degrad_poll_id?: number };
        if (w.__ax_degrad_poll_id) {
          window.clearInterval(w.__ax_degrad_poll_id);
          w.__ax_degrad_poll_id = undefined;
        }
      },
    }),
    {
      name: "axagent-time-anchor",
      storage: createJSONStorage(() => localStorage),
      // 不持久化 transient 字段
      partialize: (s) => ({
        asOfDate: s.asOfDate,
        mode: s.mode,
        tourSeen: s.tourSeen,
      }),
    },
  ),
);

/** 纯函数 helper（不订阅 store） */
export const timeAnchorHelpers = {
  isValidPastDate,
  todayIso,
  DATE_RE,
};
