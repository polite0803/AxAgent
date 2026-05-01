import { listen, type UnlistenFn } from "@/lib/invoke";
import { create } from "zustand";

// ---------------------------------------------------------------------------
// 类型
// ---------------------------------------------------------------------------

export interface DreamConsolidationResult {
  executed: boolean;
  memoriesExtracted: number;
  patternsDiscovered: number;
  suggestionsGenerated: number;
  startedAt: number;  // timestamp_ms
  durationSecs: number;
  error: string | null;
}

export type DreamStatus = "idle" | "running" | "completed" | "error";

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

interface DreamStore {
  // 状态
  status: DreamStatus;
  lastResult: DreamConsolidationResult | null;
  isRunning: boolean;
  totalConsolidations: number;
  totalMemoriesExtracted: number;
  totalPatternsDiscovered: number;

  // 事件处理
  handleStarted: () => void;
  handleCompleted: (result: DreamConsolidationResult) => void;
  handleError: (error: string) => void;

  // 重置
  reset: () => void;
}

export const useDreamStore = create<DreamStore>((set, get) => ({
  status: "idle",
  lastResult: null,
  isRunning: false,
  totalConsolidations: 0,
  totalMemoriesExtracted: 0,
  totalPatternsDiscovered: 0,

  handleStarted: () => {
    set({ status: "running", isRunning: true });
  },

  handleCompleted: (result) => {
    const prev = get();
    set({
      status: result.error ? "error" : "completed",
      isRunning: false,
      lastResult: result,
      totalConsolidations: prev.totalConsolidations + 1,
      totalMemoriesExtracted:
        prev.totalMemoriesExtracted + result.memoriesExtracted,
      totalPatternsDiscovered:
        prev.totalPatternsDiscovered + result.patternsDiscovered,
    });

    // 3 秒后恢复 idle（让 UI 有时间展示完成状态）
    setTimeout(() => {
      const currentStatus = get().status;
      if (currentStatus === "completed" || currentStatus === "error") {
        set({ status: "idle" });
      }
    }, 3000);
  },

  handleError: (error) => {
    set({
      status: "error",
      isRunning: false,
      lastResult: {
        executed: false,
        memoriesExtracted: 0,
        patternsDiscovered: 0,
        suggestionsGenerated: 0,
        startedAt: Date.now(),
        durationSecs: 0,
        error,
      },
    });

    setTimeout(() => set({ status: "idle" }), 5000);
  },

  reset: () => {
    set({
      status: "idle",
      lastResult: null,
      isRunning: false,
      totalConsolidations: 0,
      totalMemoriesExtracted: 0,
      totalPatternsDiscovered: 0,
    });
  },
}));

// ---------------------------------------------------------------------------
// 事件监听注册
// ---------------------------------------------------------------------------

let _dreamListenersSetup = false;

export function setupDreamEventListeners(): () => void {
  if (_dreamListenersSetup) {
    return () => {};
  }
  _dreamListenersSetup = true;

  const unlisteners: Promise<UnlistenFn>[] = [];
  const store = useDreamStore.getState();

  unlisteners.push(
    listen<{ timestamp: number; maxDurationSecs: number }>(
      "dream-consolidation-started",
      () => {
        store.handleStarted();
      },
    ),
  );

  unlisteners.push(
    listen<DreamConsolidationResult>(
      "dream-consolidation-completed",
      (event) => {
        store.handleCompleted(event.payload);
      },
    ),
  );

  return () => {
    _dreamListenersSetup = false;
    for (const p of unlisteners) {
      p.then((u) => u());
    }
  };
}
