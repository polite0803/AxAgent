// drawdown.test.ts — 回撤计算单测（P-2：前端 Vitest）
import { describe, expect, it } from "vitest";

import type { EquityPoint } from "@/types";
import { computeDrawdownPercent } from "./drawdown";

function eq(date: string, equity: number): EquityPoint {
  return { date, equity, cash: equity, positionValue: 0 };
}

describe("computeDrawdownPercent", () => {
  it("空曲线返回空数组", () => {
    expect(computeDrawdownPercent([])).toEqual([]);
  });

  it("单调上涨：回撤恒为 0", () => {
    const curve = [eq("d1", 100), eq("d2", 110), eq("d3", 120)];
    expect(computeDrawdownPercent(curve)).toEqual([0, 0, 0]);
  });

  it("峰后回撤计算正确（120→90 回撤 25%；120→100 回撤 16.67%）", () => {
    const curve = [eq("d1", 100), eq("d2", 120), eq("d3", 90), eq("d4", 100)];
    expect(computeDrawdownPercent(curve)).toEqual([0, 0, -25, -16.67]);
  });

  it("峰值为 0 或负：不产生除零/异常负值", () => {
    const curve = [eq("d1", 0), eq("d2", -10), eq("d3", 5)];
    expect(computeDrawdownPercent(curve)).toEqual([0, 0, 0]);
  });
});
