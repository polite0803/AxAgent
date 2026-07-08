import type { ReactNode } from "react";

/**
 * DualView — 一个 panel 内容的两种视图:完整 panel 视图 + 紧凑 chat bubble 视图。
 * 注册到 dualViewRegistry,id 与 panelKey 对齐。
 *
 * 典型使用:
 *   registerDualView({
 *     id: "value",
 *     title: "估值评估",
 *     icon: "Banknote",
 *     defaultTab: "analyze",
 *     compact: (data) => <CompactValueAssessment data={data} />,
 *     panel: (data) => <ValueAssessmentPanel data={data} />,
 *   });
 */
export interface DualView<T = unknown> {
  id: string;
  title: string;
  icon: string; // lucide-react name,渲染时由 DualViewRenderer 动态 import
  defaultTab: "market" | "analyze" | "execute";
  compact: (data: T) => ReactNode;
  panel: (data: T) => ReactNode;
  /** 不参与 dual view 的黑名单,例如高频 ticker / 配置面板,可选 */
  noDualView?: boolean;
}

const registry = new Map<string, DualView<unknown>>();

export function registerDualView<T>(view: DualView<T>): void {
  if (registry.has(view.id)) {
    console.warn(`[dualView] duplicate registration: ${view.id}`);
  }
  registry.set(view.id, view as DualView<unknown>);
}

export function getDualView(id: string): DualView | undefined {
  return registry.get(id);
}

export function listDualViews(): DualView[] {
  return Array.from(registry.values());
}

export function isDualViewEnabled(id: string): boolean {
  const v = registry.get(id);
  return !!v && !v.noDualView;
}

/** 测试用:清空注册表。仅在 __test__ 环境调用。 */
export function _resetDualViewRegistry(): void {
  registry.clear();
}
