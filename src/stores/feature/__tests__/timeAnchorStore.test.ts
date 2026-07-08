import { timeAnchorHelpers, useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

const { isValidPastDate, todayIso, DATE_RE } = timeAnchorHelpers;

beforeEach(() => {
  // 重置 store + 清除 localStorage 持久化
  useTimeAnchorStore.setState({
    asOfDate: null,
    mode: "live",
    tourSeen: false,
    pendingLiveConfirm: false,
  });
  if (typeof localStorage !== "undefined") {
    localStorage.removeItem("axagent-time-anchor");
  }
});

afterEach(() => {
  if (typeof localStorage !== "undefined") {
    localStorage.removeItem("axagent-time-anchor");
  }
});

describe("timeAnchorHelpers", () => {
  it("DATE_RE matches YYYY-MM-DD only", () => {
    expect(DATE_RE.test("2026-06-01")).toBe(true);
    expect(DATE_RE.test("2026/06/01")).toBe(false);
    expect(DATE_RE.test("2026-6-1")).toBe(false);
    expect(DATE_RE.test("garbage")).toBe(false);
  });

  it("isValidPastDate accepts past date", () => {
    // 用一个 30 天前的日期
    const past = new Date();
    past.setDate(past.getDate() - 30);
    const s = past.toISOString().slice(0, 10);
    expect(isValidPastDate(s)).toBe(true);
  });

  it("isValidPastDate accepts today", () => {
    const today = todayIso();
    // today <= today 为 true（包括等于），允许用户选择当天
    expect(isValidPastDate(today)).toBe(true);
  });

  it("isValidPastDate rejects future date", () => {
    const future = new Date();
    future.setDate(future.getDate() + 30);
    const s = future.toISOString().slice(0, 10);
    expect(isValidPastDate(s)).toBe(false);
  });

  it("isValidPastDate rejects malformed input", () => {
    expect(isValidPastDate("not-a-date")).toBe(false);
    expect(isValidPastDate("")).toBe(false);
    expect(isValidPastDate("2026-13-01")).toBe(false);
  });
});

describe("useTimeAnchorStore — transitions", () => {
  it("starts in live mode with null asOfDate", () => {
    const s = useTimeAnchorStore.getState();
    expect(s.asOfDate).toBeNull();
    expect(s.mode).toBe("live");
    expect(s.tourSeen).toBe(false);
  });

  it("enterReplay sets asOfDate and mode=replay", () => {
    const past = new Date();
    past.setDate(past.getDate() - 7);
    const d = past.toISOString().slice(0, 10);
    useTimeAnchorStore.getState().enterReplay(d);
    const s = useTimeAnchorStore.getState();
    expect(s.asOfDate).toBe(d);
    expect(s.mode).toBe("replay");
    expect(s.pendingLiveConfirm).toBe(false);
  });

  it("enterReplay rejects future date (no state change)", () => {
    useTimeAnchorStore.getState().enterReplay("2099-01-01");
    expect(useTimeAnchorStore.getState().asOfDate).toBeNull();
    expect(useTimeAnchorStore.getState().mode).toBe("live");
  });

  it("enterLive from replay requires confirmation when requireConfirm=true", () => {
    const past = new Date();
    past.setDate(past.getDate() - 7);
    const d = past.toISOString().slice(0, 10);
    useTimeAnchorStore.getState().enterReplay(d);
    const ok = useTimeAnchorStore.getState().enterLive(true);
    expect(ok).toBe(false);
    expect(useTimeAnchorStore.getState().pendingLiveConfirm).toBe(true);
    expect(useTimeAnchorStore.getState().mode).toBe("replay");
  });

  it("confirmPendingLive transitions back to live", () => {
    const past = new Date();
    past.setDate(past.getDate() - 7);
    const d = past.toISOString().slice(0, 10);
    useTimeAnchorStore.getState().enterReplay(d);
    useTimeAnchorStore.getState().enterLive(true);
    useTimeAnchorStore.getState().confirmPendingLive();
    const s = useTimeAnchorStore.getState();
    expect(s.asOfDate).toBeNull();
    expect(s.mode).toBe("live");
    expect(s.pendingLiveConfirm).toBe(false);
  });

  it("cancelPendingLive keeps replay mode", () => {
    const past = new Date();
    past.setDate(past.getDate() - 7);
    const d = past.toISOString().slice(0, 10);
    useTimeAnchorStore.getState().enterReplay(d);
    useTimeAnchorStore.getState().enterLive(true);
    useTimeAnchorStore.getState().cancelPendingLive();
    const s = useTimeAnchorStore.getState();
    expect(s.mode).toBe("replay");
    expect(s.pendingLiveConfirm).toBe(false);
  });

  it("enterReplayWorkbench forces override (does not inherit from live)", () => {
    // 先 live
    expect(useTimeAnchorStore.getState().mode).toBe("live");
    const past = new Date();
    past.setDate(past.getDate() - 14);
    const d = past.toISOString().slice(0, 10);
    useTimeAnchorStore.getState().enterReplayWorkbench(d);
    const s = useTimeAnchorStore.getState();
    expect(s.asOfDate).toBe(d);
    expect(s.mode).toBe("replay");
  });

  it("enterBacktestSweep sets mode=backtest_sweep", () => {
    const past = new Date();
    past.setDate(past.getDate() - 7);
    const d = past.toISOString().slice(0, 10);
    useTimeAnchorStore.getState().enterBacktestSweep(d);
    const s = useTimeAnchorStore.getState();
    expect(s.asOfDate).toBe(d);
    expect(s.mode).toBe("backtest_sweep");
  });

  it("markTourSeen persists", () => {
    useTimeAnchorStore.getState().markTourSeen();
    expect(useTimeAnchorStore.getState().tourSeen).toBe(true);
  });

  it("setAsOfDate(null) goes back to live", () => {
    const past = new Date();
    past.setDate(past.getDate() - 7);
    const d = past.toISOString().slice(0, 10);
    useTimeAnchorStore.getState().enterReplay(d);
    useTimeAnchorStore.getState().setAsOfDate(null);
    const s = useTimeAnchorStore.getState();
    expect(s.asOfDate).toBeNull();
    expect(s.mode).toBe("live");
  });

  it("setAsOfDate rejects future date (no state change)", () => {
    useTimeAnchorStore.getState().setAsOfDate("2099-01-01");
    expect(useTimeAnchorStore.getState().asOfDate).toBeNull();
    expect(useTimeAnchorStore.getState().mode).toBe("live");
  });

  // P2-7: enterReplay 必须重置本地降级显示,避免上次 replay 残留
  it("enterReplay resets degradation state (count and log to 0)", () => {
    // 假装有残留降级
    useTimeAnchorStore.setState({
      degradationCount: 5,
      degradationLog: [
        { vendor: "old", method: "old_method", reason: "stale", as_of: "2026-01-01" },
      ],
    });
    const past = new Date();
    past.setDate(past.getDate() - 7);
    const d = past.toISOString().slice(0, 10);
    useTimeAnchorStore.getState().enterReplay(d);
    const s = useTimeAnchorStore.getState();
    expect(s.degradationCount).toBe(0);
    expect(s.degradationLog).toEqual([]);
  });
});
