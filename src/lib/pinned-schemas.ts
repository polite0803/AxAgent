// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 动态页面钉入导航（Pinned Schemas）分组与排序工具
 *
 * 钉入配置的持久化已迁移到后端数据库（dynamic_ui_pins 表），
 * 由 `useDynamicUIStore` 的 pins / fetchPins / pinSchema / unpinSchema / updatePin 管理。
 *
 * 本文件仅保留前端分组定义与纯函数（分组排序、按组聚合），
 * 不再包含任何 localStorage 读写逻辑。
 */

/** schemaId → PinnedSchemaConfig 的映射（由 store 的 pins 派生） */
export type PinnedSchemaMap = Record<string, PinnedSchemaConfig>;

/** 页面钉入配置 */
export interface PinnedSchemaConfig {
  schemaId: string;
  title: string;
  group: string;
  position: number;
}

/** 预置分组定义——实际显示用 labelKey 走 i18n，label 仅作后备 */
export const PIN_GROUPS = [
  { key: "dashboard", label: "Dashboard", labelKey: "pinnedGroups.dashboard" },
  { key: "report", label: "Report", labelKey: "pinnedGroups.report" },
  { key: "monitor", label: "Monitor", labelKey: "pinnedGroups.monitor" },
  { key: "other", label: "Other", labelKey: "pinnedGroups.other" },
] as const;

/** 分组 key 在 PIN_GROUPS 中的序号，未匹配组返回 Infinity */
function groupOrder(group: string): number {
  const idx = PIN_GROUPS.findIndex((g) => g.key === group);
  return idx === -1 ? Infinity : idx;
}

/**
 * 按分组 + 组内排序位返回层级数组。
 *
 * 排序规则：
 * 1. 分组顺序按 PIN_GROUPS 定义排列
 * 2. 未匹配 PIN_GROUPS 的分组排在最后
 * 3. 同一组内按 position 升序
 */
export function getPinnedSchemasByGroup(
  schemas: PinnedSchemaMap,
): Array<{ group: string; items: PinnedSchemaConfig[] }> {
  // 按组收集
  const groupMap = new Map<string, PinnedSchemaConfig[]>();
  for (const config of Object.values(schemas)) {
    const list = groupMap.get(config.group);
    if (list) {
      list.push(config);
    } else {
      groupMap.set(config.group, [config]);
    }
  }

  // 组内按 position 排序
  for (const items of groupMap.values()) {
    items.sort((a, b) => a.position - b.position);
  }

  // 按 PIN_GROUPS 顺序排列，未匹配的放最后
  const ordered: Array<{ group: string; items: PinnedSchemaConfig[] }> = [];
  const unmatched: Array<{ group: string; items: PinnedSchemaConfig[] }> = [];

  for (const [group, items] of groupMap.entries()) {
    const entry = { group, items };
    if (PIN_GROUPS.some((g) => g.key === group)) {
      ordered.push(entry);
    } else {
      unmatched.push(entry);
    }
  }

  ordered.sort((a, b) => groupOrder(a.group) - groupOrder(b.group));
  unmatched.sort((a, b) => a.group.localeCompare(b.group));

  return [...ordered, ...unmatched];
}
