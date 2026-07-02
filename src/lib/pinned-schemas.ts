// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 动态页面钉入导航（Pinned Schemas）配置管理
 *
 * 将所有用户"钉入导航"的动态页面配置持久化到 localStorage，
 * 通过分组+排序位组织展现顺序。
 *
 * 数据格式（localStorage）：
 *   key: "ax-pinned-dynamic-pages"
 *   value: JSON.stringify(PinnedSchemaMap)
 */

/** schemaId → PinnedSchemaConfig 的映射 */
export type PinnedSchemaMap = Record<string, PinnedSchemaConfig>;

/** 存储 key */
const STORAGE_KEY = "ax-pinned-dynamic-pages";

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

// ─── 内部辅助 ───────────────────────────────────────────

/** 从 localStorage 读取原始 JSON */
function readRaw(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

/** 解析为 PinnedSchemaMap，失败返回空对象 */
function parseRaw(raw: string | null): PinnedSchemaMap {
  if (!raw) { return {}; }
  try {
    const parsed = JSON.parse(raw);
    if (typeof parsed === "object" && parsed !== null) {
      return parsed as PinnedSchemaMap;
    }
    return {};
  } catch {
    return {};
  }
}

/** 将 map 序列化写入 localStorage */
function write(map: PinnedSchemaMap): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(map));
  } catch {
    // localStorage 满或不可用时静默失败
  }
}

/** 分组 key 在 PIN_GROUPS 中的序号，未匹配组返回 Infinity */
function groupOrder(group: string): number {
  const idx = PIN_GROUPS.findIndex((g) => g.key === group);
  return idx === -1 ? Infinity : idx;
}

// ─── 公开 API ───────────────────────────────────────────

/** 获取所有钉入配置 */
export function getPinnedSchemas(): PinnedSchemaMap {
  return parseRaw(readRaw());
}

/** 获取指定 schema 的钉入配置 */
export function getPinnedSchema(schemaId: string): PinnedSchemaConfig | undefined {
  return getPinnedSchemas()[schemaId];
}

/** 设置/覆盖一个钉入配置 */
export function setPinnedSchema(config: PinnedSchemaConfig): void {
  const all = getPinnedSchemas();
  all[config.schemaId] = {
    schemaId: config.schemaId,
    title: config.title,
    group: config.group,
    position: config.position,
  };
  write(all);
}

/** 移除指定 schema 的钉入配置 */
export function removePinnedSchema(schemaId: string): void {
  const all = getPinnedSchemas();
  if (schemaId in all) {
    delete all[schemaId];
    write(all);
  }
}

/** 判断 schema 是否已被钉入 */
export function isPinned(schemaId: string): boolean {
  return schemaId in getPinnedSchemas();
}

/** 更新指定 schema 的排序位 */
export function updatePinnedPosition(schemaId: string, position: number): void {
  const all = getPinnedSchemas();
  if (schemaId in all) {
    all[schemaId] = { ...all[schemaId], position };
    write(all);
  }
}

/** 更新指定 schema 的分组 */
export function updatePinnedGroup(schemaId: string, group: string): void {
  const all = getPinnedSchemas();
  if (schemaId in all) {
    all[schemaId] = { ...all[schemaId], group };
    write(all);
  }
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

/**
 * 计算指定分组的下一可用排序位。
 *
 * 查找该组内当前最大 position，返回 max + 1。
 * 该组无任何条目时返回 0。
 */
export function getNextPosition(group: string, all: PinnedSchemaMap): number {
  let max = -1;
  for (const config of Object.values(all)) {
    if (config.group === group && config.position > max) {
      max = config.position;
    }
  }
  return max + 1;
}
