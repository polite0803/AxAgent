import {
  _resetDualViewRegistry,
  getDualView,
  isDualViewEnabled,
  listDualViews,
  registerDualView,
} from "@/lib/dualView";
import { afterEach, describe, expect, it, vi } from "vitest";

describe("dualView registry", () => {
  afterEach(() => _resetDualViewRegistry());

  it("registerDualView 存储并允许 getDualView 读取", () => {
    registerDualView({
      id: "test-1",
      title: "Test 1",
      icon: "Box",
      defaultTab: "analyze",
      compact: () => "compact",
      panel: () => "panel",
    });
    expect(getDualView("test-1")?.title).toBe("Test 1");
  });

  it("重复注册同 id 给出警告", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    registerDualView({
      id: "dup",
      title: "A",
      icon: "Box",
      defaultTab: "analyze",
      compact: () => "a",
      panel: () => "a",
    });
    registerDualView({
      id: "dup",
      title: "B",
      icon: "Box",
      defaultTab: "analyze",
      compact: () => "b",
      panel: () => "b",
    });
    expect(warn).toHaveBeenCalledWith(expect.stringContaining("duplicate registration"));
    expect(getDualView("dup")?.title).toBe("B"); // 后注册的覆盖
    warn.mockRestore();
  });

  it("isDualViewEnabled 区分 noDualView 标记", () => {
    registerDualView({
      id: "blocked",
      title: "X",
      icon: "Box",
      defaultTab: "analyze",
      compact: () => "",
      panel: () => "",
      noDualView: true,
    });
    expect(isDualViewEnabled("blocked")).toBe(false);
  });

  it("isDualViewEnabled 未注册 id 返回 false", () => {
    expect(isDualViewEnabled("nonexistent")).toBe(false);
  });

  it("listDualViews 返回所有已注册", () => {
    registerDualView({ id: "a", title: "A", icon: "Box", defaultTab: "analyze", compact: () => "", panel: () => "" });
    registerDualView({ id: "b", title: "B", icon: "Box", defaultTab: "analyze", compact: () => "", panel: () => "" });
    expect(listDualViews()).toHaveLength(2);
  });

  it("_resetDualViewRegistry 清空注册表", () => {
    registerDualView({ id: "x", title: "X", icon: "Box", defaultTab: "analyze", compact: () => "", panel: () => "" });
    _resetDualViewRegistry();
    expect(getDualView("x")).toBeUndefined();
  });
});
