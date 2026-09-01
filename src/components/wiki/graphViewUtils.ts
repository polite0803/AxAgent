// SPDX-License-Identifier: AGPL-3.0-only

// GraphView 纯函数工具集（F8 拆分第一步：从 GraphView.tsx 搬出与组件状态无关的
// 布局持久化 / 配色 / 节点尺寸 / 颜色工具，降低主文件体积）。
// 仅 import 类型（type-only），运行时与 GraphView.tsx 无相互依赖。

import { theme } from "antd";
import type { PhysicsNode } from "./graphPhysics";
import type { GraphEdgeType, GraphNode, GraphNodeType } from "./GraphView";

export type TokenType = ReturnType<typeof theme.useToken>["token"];

export const communityPalette = [
  "#5B8FF9",
  "#61DDAA",
  "#65789B",
  "#F6BD16",
  "#7262FD",
  "#78D3F8",
  "#9661BC",
  "#F6903D",
  "#008685",
  "#F08BB4",
  "#1E90FF",
  "#32CD32",
];

export const getNodeColorMap = (token: TokenType): Record<GraphNodeType, string> => ({
  note: token.colorPrimary,
  concept: token.colorSuccess,
  entity: "#FA8C16",
  source: "#EB2F96",
});

// ── 布局持久化：localStorage 存储节点坐标 ──

const LAYOUT_STORAGE_PREFIX = "wiki_graph_layout_";
// LRU 上限：最多保留 10 个 wiki 的布局，超出按 savedAt 时间淘汰最旧的
const LAYOUT_MAX_ENTRIES = 10;
// 单 wiki 布局超过此节点数则不持久化（避免万级节点序列化 500KB+ 逼近配额）
const LAYOUT_MAX_NODES = 2000;

interface SavedLayout {
  positions: Record<string, { x: number; y: number }>;
  savedAt: number;
  // D7: 相机视角持久化，刷新后回到上次导航区域（仅在非默认视角时保存）
  camera?: { x: number; y: number; zoom: number };
}

function pruneLayoutStorage(currentWikiId: string): void {
  // 收集所有布局条目，按 savedAt 升序，超出上限时删除最旧
  const entries: Array<{ wikiId: string; savedAt: number }> = [];
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (!key || !key.startsWith(LAYOUT_STORAGE_PREFIX)) { continue; }
    const wid = key.slice(LAYOUT_STORAGE_PREFIX.length);
    if (wid === currentWikiId) { continue; }
    try {
      const raw = localStorage.getItem(key);
      if (!raw) { continue; }
      const layout = JSON.parse(raw) as SavedLayout;
      entries.push({ wikiId: wid, savedAt: layout.savedAt || 0 });
    } catch {
      // 损坏的条目直接删除
      localStorage.removeItem(key);
    }
  }
  entries.sort((a, b) => a.savedAt - b.savedAt);
  // 已有条目数（不含当前）+ 当前 1 个 > 上限 → 删除最旧的
  const excess = entries.length + 1 - LAYOUT_MAX_ENTRIES;
  for (let i = 0; i < excess; i++) {
    localStorage.removeItem(LAYOUT_STORAGE_PREFIX + entries[i].wikiId);
  }
}

export function saveLayout(
  wikiId: string,
  nodes: PhysicsNode[],
  camera?: { x: number; y: number; zoom: number },
): void {
  // 节点数超阈值时不持久化（避免逼近 localStorage 配额）
  if (nodes.length > LAYOUT_MAX_NODES) { return; }
  try {
    const positions: Record<string, { x: number; y: number }> = {};
    for (const node of nodes) {
      positions[node.id] = { x: node.x, y: node.y };
    }
    const layout: SavedLayout = {
      positions,
      savedAt: Date.now(),
    };
    // D7: 仅在缩放偏离默认视角时持久化相机，避免默认视角的冗余存储
    if (camera && Math.abs(camera.zoom - 1) > 0.01) {
      layout.camera = { x: camera.x, y: camera.y, zoom: camera.zoom };
    }
    // 写入前做 LRU 清理，确保不超过 LAYOUT_MAX_ENTRIES
    pruneLayoutStorage(wikiId);
    localStorage.setItem(LAYOUT_STORAGE_PREFIX + wikiId, JSON.stringify(layout));
  } catch {
    // localStorage 可能已满，静默忽略
  }
}

export function loadLayout(wikiId: string): SavedLayout | null {
  try {
    const raw = localStorage.getItem(LAYOUT_STORAGE_PREFIX + wikiId);
    if (!raw) { return null; }
    return JSON.parse(raw) as SavedLayout;
  } catch {
    return null;
  }
}

/** 清除指定 wiki 的已保存布局（重新布局前调用） */
export function clearLayout(wikiId: string): void {
  try {
    localStorage.removeItem(LAYOUT_STORAGE_PREFIX + wikiId);
  } catch {
    // 静默忽略
  }
}

export function applySavedLayout(nodes: PhysicsNode[], saved: SavedLayout): boolean {
  let matched = 0;
  for (const node of nodes) {
    const savedPos = saved.positions[node.id];
    if (savedPos) {
      node.x = savedPos.x;
      node.y = savedPos.y;
      matched++;
    }
  }
  // 匹配率低于 30% 时整体放弃：清空位置，返回 false 让 initializePositions 重新圆形布局
  if (matched < nodes.length * 0.3) {
    return false;
  }
  // 匹配率 ≥ 30% 但部分未匹配：给未匹配节点做圆形分布，避免堆叠在原点
  const unmatched = nodes.filter((n) => !saved.positions[n.id]);
  if (unmatched.length > 0) {
    const radius = Math.max(200, Math.sqrt(unmatched.length) * 30);
    unmatched.forEach((n, i) => {
      const angle = (i / unmatched.length) * Math.PI * 2;
      n.x = Math.cos(angle) * radius;
      n.y = Math.sin(angle) * radius;
    });
  }
  return true;
}

export const getEdgeTypeStylesMap = (
  token: TokenType,
): Record<GraphEdgeType, { color: string; width: number; animated: boolean }> => ({
  link: { color: token.colorBorderSecondary, width: 0.4, animated: true },
  backlink: { color: token.colorBorder, width: 0.5, animated: true },
  reference: { color: token.colorSuccess, width: 0.5, animated: true },
  derived_from: { color: token.colorWarning, width: 0.5, animated: false },
  contradicts: { color: token.colorError, width: 0.6, animated: false },
  mapping: { color: token.colorInfo, width: 0.4, animated: true },
});

export const edgeTypeLabels: Record<GraphEdgeType, string> = {
  link: "wiki.graph.edgeType.link",
  backlink: "wiki.graph.edgeType.backlink",
  reference: "wiki.graph.edgeType.reference",
  derived_from: "wiki.graph.edgeType.derived",
  contradicts: "wiki.graph.edgeType.contradicts",
  mapping: "wiki.graph.edgeType.mapping",
};

// 节点颜色缓存：nodeId → color
export function buildNodeColorCache(
  nodes: GraphNode[],
  communities?: Map<string, number>,
  token?: TokenType,
): Map<string, string> {
  const cache = new Map<string, string>();
  const typeMap = token ? getNodeColorMap(token) : {
    note: "#1890ff",
    concept: "#52c41a",
    entity: "#fa8c16",
    source: "#eb2f96",
  };
  for (const node of nodes) {
    if (communities && communities.has(node.id)) {
      const cid = communities.get(node.id)!;
      cache.set(node.id, communityPalette[cid % communityPalette.length]);
    } else {
      cache.set(node.id, typeMap[node.type] || typeMap.note);
    }
  }
  return cache;
}

export function getNodeSize(node: GraphNode): number {
  const degree = node.linkCount + node.backlinkCount;
  if (node.type === "entity") { return Math.max(6, Math.min(22, 6 + degree * 0.8)); }
  if (node.type === "concept") { return Math.max(5, Math.min(18, 5 + degree * 0.6)); }
  return Math.max(4, Math.min(15, 4 + degree * 0.4));
}

// ── XML 转义（SVG 导出防注入） ──
export function escapeXml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

// ── 颜色工具：支持 #RRGGBB / #RRGGBBAA / #RGB / rgb()/rgba() ──
export interface RGBA {
  r: number;
  g: number;
  b: number;
  a: number;
}

export function parseColor(color: string): RGBA | null {
  if (!color) { return null; }
  const hexMatch = color.match(/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/);
  if (hexMatch) {
    const hex = hexMatch[1];
    if (hex.length === 3) {
      return {
        r: parseInt(hex[0] + hex[0], 16),
        g: parseInt(hex[1] + hex[1], 16),
        b: parseInt(hex[2] + hex[2], 16),
        a: 255,
      };
    }
    return {
      r: parseInt(hex.slice(0, 2), 16),
      g: parseInt(hex.slice(2, 4), 16),
      b: parseInt(hex.slice(4, 6), 16),
      a: hex.length === 8 ? parseInt(hex.slice(6, 8), 16) : 255,
    };
  }
  const rgbMatch = color.match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)(?:\s*,\s*([\d.]+))?\s*\)$/);
  if (rgbMatch) {
    return {
      r: parseInt(rgbMatch[1], 10),
      g: parseInt(rgbMatch[2], 10),
      b: parseInt(rgbMatch[3], 10),
      a: rgbMatch[4] !== undefined ? Math.round(parseFloat(rgbMatch[4]) * 255) : 255,
    };
  }
  return null;
}

export function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}
