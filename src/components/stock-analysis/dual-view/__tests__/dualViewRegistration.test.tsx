/**
 * Tests for the 5 dual views registered via the side-effect index:
 * value / debate / risk / screener / analysts
 *
 * 顶层 import 触发 side-effect 注册。
 * beforeEach 也触发一次,补偿 afterEach 清空。
 */
import "@/components/stock-analysis/dual-view";
import { getDualView, isDualViewEnabled, listDualViews } from "@/lib/dualView";
import { beforeEach, describe, expect, it } from "vitest";

describe("dual-view side-effect registration", () => {
  beforeEach(async () => {
    // afterEach 清空了 registry,重新触发 side-effect
    await import("@/components/stock-analysis/dual-view");
  });

  it("至少 5 个 dual view 被注册(value/debate/risk/screener/analysts)", () => {
    const ids = listDualViews().map((v) => v.id);
    expect(ids).toContain("value");
    expect(ids).toContain("debate");
    expect(ids).toContain("risk");
    expect(ids).toContain("screener");
    expect(ids).toContain("analysts");
  });

  it("value 试点:title/icon/defaultTab 完整", () => {
    const v = getDualView("value");
    expect(v?.title).toBeTruthy();
    expect(v?.icon).toBeTruthy();
    expect(v?.defaultTab).toBe("analyze");
    expect(typeof v?.compact).toBe("function");
    expect(typeof v?.panel).toBe("function");
  });

  it("debate 试点 defaultTab=analyze", () => {
    const v = getDualView("debate");
    expect(v?.defaultTab).toBe("analyze");
  });

  it("risk 接入 defaultTab=analyze", () => {
    const v = getDualView("risk");
    expect(v?.defaultTab).toBe("analyze");
  });

  it("screener 接入 defaultTab=market", () => {
    const v = getDualView("screener");
    expect(v?.defaultTab).toBe("market");
  });

  it("analysts 接入 defaultTab=analyze", () => {
    const v = getDualView("analysts");
    expect(v?.defaultTab).toBe("analyze");
  });

  it("5 个 dual view 都启用", () => {
    for (const id of ["value", "debate", "risk", "screener", "analysts"]) {
      expect(isDualViewEnabled(id)).toBe(true);
    }
  });
});
