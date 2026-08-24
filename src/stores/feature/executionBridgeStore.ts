// SPDX-License-Identifier: AGPL-3.0-only
// i18n-exempt: 交易执行桥接 store 状态管理（含状态字段/方法名等技术标识符），非 UI 文案。

import {
  confirmPending,
  getExecutionMode,
  listPending,
  onExecutionConfirmed,
  onExecutionFilled,
  onExecutionPending,
  onExecutionRejected,
  onExecutionRiskRejected,
  rejectPending,
  setExecutionMode as setExecutionModeApi,
  submitSignal,
} from "@/lib/execution";
import type { UnlistenFn } from "@/lib/invoke";
import type {
  ExecutionConfirmedEvent,
  ExecutionFilledEvent,
  ExecutionMode,
  ExecutionPendingEvent,
  ExecutionRejectedEvent,
  ExecutionRiskRejectedEvent,
  PendingExecution,
} from "@/types";
import { create } from "zustand";
import { devtools } from "zustand/middleware";

// ── Store 接口 ──

interface ExecutionBridgeStore {
  // === 状态 ===
  mode: ExecutionMode;
  pendings: PendingExecution[];
  loading: boolean;
  error: string | null;

  // === 异步 Actions ===
  fetchMode: () => Promise<void>;
  setMode: (mode: ExecutionMode) => Promise<void>;
  fetchPendings: () => Promise<void>;
  submitSignal: (params: {
    signalCode: string;
    signalAction: string;
    signalReason: string;
    stockName: string;
    currentPrice: number;
  }) => Promise<string>;
  confirmPending: (pendingId: string, quantity: number) => Promise<string>;
  rejectPending: (pendingId: string, reason: string) => Promise<void>;

  // === 事件处理 Actions ===
  handlePending: (event: ExecutionPendingEvent) => void;
  handleFilled: (event: ExecutionFilledEvent) => void;
  handleConfirmed: (event: ExecutionConfirmedEvent) => void;
  handleRejected: (event: ExecutionRejectedEvent) => void;
  handleRiskRejected: (event: ExecutionRiskRejectedEvent) => void;

  // === 清理 ===
  clearError: () => void;
  reset: () => void;
}

// ── 初始状态 ──

const initialState = {
  mode: "manual" as ExecutionMode,
  pendings: [] as PendingExecution[],
  loading: false,
  error: null as string | null,
};

export const useExecutionBridgeStore = create<ExecutionBridgeStore>()(
  devtools(
    (set, get) => ({
      ...initialState,

      // ── 异步 Actions ──

      fetchMode: async () => {
        set({ loading: true });
        try {
          const mode = await getExecutionMode();
          set({ mode, error: null });
        } catch (e) {
          set({ error: String(e) });
        } finally {
          set({ loading: false });
        }
      },

      setMode: async (mode) => {
        set({ loading: true });
        try {
          await setExecutionModeApi({ mode });
          set({ mode, error: null });
        } catch (e) {
          set({ error: String(e) });
        } finally {
          set({ loading: false });
        }
      },

      fetchPendings: async () => {
        set({ loading: true });
        try {
          const pendings = await listPending();
          set({ pendings, error: null });
        } catch (e) {
          set({ error: String(e) });
        } finally {
          set({ loading: false });
        }
      },

      submitSignal: async (params) => {
        set({ loading: true, error: null });
        try {
          const result = await submitSignal(params);
          return result;
        } catch (e) {
          const errMsg = String(e);
          set({ error: errMsg });
          throw e;
        } finally {
          set({ loading: false });
        }
      },

      confirmPending: async (pendingId, quantity) => {
        set({ loading: true, error: null });
        try {
          const tradeId = await confirmPending({ pendingId, quantity });
          // 从待执行列表移除
          set((s) => ({
            pendings: s.pendings.filter((p) => p.id !== pendingId),
          }));
          return tradeId;
        } catch (e) {
          set({ error: String(e) });
          throw e;
        } finally {
          set({ loading: false });
        }
      },

      rejectPending: async (pendingId, reason) => {
        set({ loading: true, error: null });
        try {
          await rejectPending({ pendingId, reason });
          // 从待执行列表移除
          set((s) => ({
            pendings: s.pendings.filter((p) => p.id !== pendingId),
          }));
        } catch (e) {
          set({ error: String(e) });
          throw e;
        } finally {
          set({ loading: false });
        }
      },

      // ── 事件处理 Actions ──

      handlePending: (event) => {
        const pending: PendingExecution = {
          id: event.pendingId,
          stockCode: event.stockCode,
          stockName: event.stockName,
          direction: event.direction,
          price: event.price,
          quantity: 0,
          reason: event.reason,
          riskLevel: event.riskLevel,
          riskWarning: event.riskWarning,
          createdAt: Date.now(),
          status: "pending",
        };
        set((s) => ({
          pendings: [pending, ...s.pendings],
        }));
      },

      handleFilled: (_event) => {
        // 交易已执行，可选择刷新列表
        get().fetchPendings().catch(() => {});
      },

      handleConfirmed: (event) => {
        set((s) => ({
          pendings: s.pendings.filter((p) => p.id !== event.pendingId),
        }));
      },

      handleRejected: (event) => {
        set((s) => ({
          pendings: s.pendings.filter((p) => p.id !== event.pendingId),
        }));
      },

      handleRiskRejected: (event) => {
        set({
          error: `风控检查未通过: ${event.reason}`,
        });
      },

      // ── 清理 ──

      clearError: () => {
        set({ error: null });
      },

      reset: () => {
        set(initialState);
      },
    }),
    { name: "executionBridgeStore" },
  ),
);

// ── 事件监听器注册 ──

let _listenerRefCount = 0;
let _initialized = false;

export function setupExecutionBridgeEventListeners(): () => void {
  _listenerRefCount++;
  if (_listenerRefCount > 1 && _initialized) {
    return () => {
      _listenerRefCount--;
    };
  }

  const unlisteners: Promise<UnlistenFn>[] = [];
  const store = useExecutionBridgeStore.getState();

  unlisteners.push(
    onExecutionPending((event) => store.handlePending(event)),
  );
  unlisteners.push(
    onExecutionFilled((event) => store.handleFilled(event)),
  );
  unlisteners.push(
    onExecutionConfirmed((event) => store.handleConfirmed(event)),
  );
  unlisteners.push(
    onExecutionRejected((event) => store.handleRejected(event)),
  );
  unlisteners.push(
    onExecutionRiskRejected((event) => store.handleRiskRejected(event)),
  );

  _initialized = true;

  // 初始化获取当前模式和待执行列表
  store.fetchMode().catch(() => {});
  store.fetchPendings().catch(() => {});

  return () => {
    _listenerRefCount--;
    if (_listenerRefCount <= 0) {
      _listenerRefCount = 0;
      _initialized = false;
      unlisteners.forEach((u) => u.then((f) => f()));
    }
  };
}
