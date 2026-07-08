import { describe, expect, it } from "vitest";
import { resolveTimelineJump } from "../timelineJump";

describe("resolveTimelineJump - 修复 evidence chip 死链", () => {
  it("abstract panelKey 'decision' → 滚到顶部 DecisionBanner (不再 setActiveTab 'decision')", () => {
    const plan = resolveTimelineJump("analyze", "decision");
    expect(plan.activeTab).toBeUndefined();
    expect(plan.scrollTo).toBe("decision-banner-top");
    expect(plan.navigateTo).toBeUndefined();
  });

  it("abstract panelKey 'trade' → 跳到 /trade 路由", () => {
    const plan = resolveTimelineJump("execute", "trade");
    expect(plan.activeTab).toBeUndefined();
    expect(plan.navigateTo).toBe("/trade");
    expect(plan.scrollTo).toBeUndefined();
  });

  it("tabKey 'market' + sheet panelKey 'concepts' → 切到 market tab + 打开 concepts sheet", () => {
    const plan = resolveTimelineJump("market", "concepts");
    expect(plan.activeTab).toBe("market");
    expect(plan.sheetTab).toBe("concepts");
  });

  it("tabKey 'analyze' + 主区 panelKey 'analysts' → 切到 analysts tab", () => {
    const plan = resolveTimelineJump("analyze", "analysts");
    expect(plan.activeTab).toBe("analysts");
  });

  it("tabKey 'analyze' + 主区 panelKey 'debate' → 切到 debate tab", () => {
    const plan = resolveTimelineJump("analyze", "debate");
    expect(plan.activeTab).toBe("debate");
  });

  it("tabKey 'analyze' + 主区 panelKey 'value' → 切到 value tab", () => {
    const plan = resolveTimelineJump("analyze", "value");
    expect(plan.activeTab).toBe("value");
  });

  it("tabKey 'analyze' + 主区 panelKey 'risk' → 切到 risk tab", () => {
    const plan = resolveTimelineJump("analyze", "risk");
    expect(plan.activeTab).toBe("risk");
  });

  it("tabKey 'analyze' + 主区 panelKey 'reflection' → 切到 reflection tab", () => {
    const plan = resolveTimelineJump("analyze", "reflection");
    expect(plan.activeTab).toBe("reflection");
  });

  it("tabKey 'analyze' + 主区 panelKey 'evolution' → 切到 evolution tab", () => {
    const plan = resolveTimelineJump("analyze", "evolution");
    expect(plan.activeTab).toBe("evolution");
  });

  it("未知的 panelKey + 有效 tabKey → 切到该 tab", () => {
    const plan = resolveTimelineJump("analyze", "未知面板");
    expect(plan.activeTab).toBeUndefined();
    expect(plan.scrollTo).toBe("decision-banner-top"); // 兜底
  });

  it("完全未知的 tabKey + panelKey → 兜底滚到决策 hero", () => {
    const plan = resolveTimelineJump("foo", "bar");
    expect(plan.activeTab).toBeUndefined();
    expect(plan.scrollTo).toBe("decision-banner-top");
  });

  it("空 tabKey + 空 panelKey → 兜底", () => {
    const plan = resolveTimelineJump(undefined, undefined);
    expect(plan).toEqual({ scrollTo: "decision-banner-top" });
  });

  it("tabKey 'execute' + 非 trade panelKey → 不切 tab (execute 没有对应主区 tab)", () => {
    const plan = resolveTimelineJump("execute", "foo");
    expect(plan.activeTab).toBeUndefined();
  });

  it("tabKey 'execute' 不再 setActiveTab 'decision' (回归保护)", () => {
    const plan = resolveTimelineJump("execute");
    expect(plan.activeTab).toBeUndefined();
    expect(plan.scrollTo).toBe("decision-banner-top");
  });
});
