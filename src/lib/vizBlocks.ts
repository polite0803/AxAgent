// SPDX-License-Identifier: AGPL-3.0-only

/**
 * G15 可视化协议（viz_blocks）— 11 种 chart kind 统一 schema + 注册器
 *
 * 对齐 DojoAgents 的 viz_blocks 协议：LLM / 工作流 / Agent 产出统一格式的可视化块，
 * 前端通过 `VizBlockRenderer` 渲染。
 *
 * ## 11 种 chart kind
 *
 * 1. line_chart      — 折线图（多 series）
 * 2. bar_chart       — 柱状图（垂直 / 水平 / 堆叠）
 * 3. area_chart      — 面积图（堆叠 / 渐变）
 * 4. pie_chart       — 饼图 / 环形图
 * 5. scatter_chart   — 散点图（含气泡图）
 * 6. heatmap         — 热力图（行业涨跌 / 相关性矩阵）
 * 7. candlestick     — K 线图（A 股专用）
 * 8. treemap         — 矩形树图（市值 / 行业占比）
 * 9. sankey          — 桑基图（资金流向 / 产业链传导）
 * 10. gauge          — 仪表盘（评分 / 风险等级）
 * 11. table          — 表格（带条件格式 / 排序）
 *
 * ## 设计原则
 *
 * - **数据与展示分离**：data 字段保存原始数据，options 保存样式
 * - **可序列化**：所有字段可 JSON.stringify，便于 IPC / SSE / 持久化
 * - **可扩展**：通过 `kind` 路由到不同渲染器，新 kind 在 Renderer 中注册即可
 * - **与 DualView 对齐**：每个 block 都有 compact / full 两种渲染模式
 *
 * ## 使用示例
 *
 * ```ts
 * const block: VizBlock = {
 *   id: "mainline-strength-001",
 *   kind: "bar_chart",
 *   title: "市场主线强度",
 *   data: [
 *     { name: "AI 算力", value: 92 },
 *     { name: "光模块", value: 78 },
 *   ],
 *   options: { orientation: "vertical", showLegend: true },
 *   meta: { scene: "market-mainline", generatedAt: Date.now() },
 * };
 * renderVizBlock(block);
 * ```
 */

import type { ReactNode } from "react";

// ── 11 种 chart kind ────────────────────────────────────────────────────────

export type VizBlockKind =
  | "line_chart"
  | "bar_chart"
  | "area_chart"
  | "pie_chart"
  | "scatter_chart"
  | "heatmap"
  | "candlestick"
  | "treemap"
  | "sankey"
  | "gauge"
  | "table";

// ── 通用数据点 ─────────────────────────────────────────────────────────────

/** 通用二维数据点（line / bar / area / scatter 通用） */
export interface VizPoint {
  /** X 轴标签（日期 / 名称 / 类目） */
  name: string;
  /** 主数值（单 series 时使用） */
  value?: number;
  /** 多 series 时使用键值对，键 = series 名称 */
  [seriesKey: string]: string | number | undefined;
}

/** 饼图数据点 */
export interface VizPieSlice {
  name: string;
  value: number;
  color?: string;
}

/** 散点 / 气泡图数据点 */
export interface VizScatterPoint {
  x: number;
  y: number;
  z?: number; // 气泡大小
  label?: string;
  group?: string;
}

/** 热力图数据点 */
export interface VizHeatmapPoint {
  x: string;
  y: string;
  value: number;
}

/** K 线数据点 */
export interface VizCandle {
  date: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume?: number;
}

/** 矩形树图节点 */
export interface VizTreemapNode {
  name: string;
  value: number;
  children?: VizTreemapNode[];
  color?: string;
}

/** 桑基图节点 / 边 */
export interface VizSankey {
  nodes: Array<{ name: string }>;
  links: Array<{ source: string; target: string; value: number }>;
}

/** 仪表盘分段 */
export interface VizGaugeRange {
  from: number;
  to: number;
  color: string;
  label?: string;
}

/** 表格单元格（带条件格式） */
export interface VizTableCell {
  value: string | number;
  /** 文字颜色（条件格式） */
  color?: string;
  /** 背景色 */
  background?: string;
  /** 是否加粗 */
  bold?: boolean;
}

/** 表格列定义 */
export interface VizTableColumn {
  key: string;
  title: string;
  width?: number;
  align?: "left" | "center" | "right";
  sortable?: boolean;
}

// ── 各 kind 的 options 类型 ────────────────────────────────────────────────

export interface LineChartOptions {
  series?: string[]; // 多 series 时指定键名
  smooth?: boolean;
  showLegend?: boolean;
  xAxisLabel?: string;
  yAxisLabel?: string;
  /** 是否堆叠（面积图常用） */
  stack?: boolean;
}

export interface BarChartOptions {
  orientation?: "vertical" | "horizontal";
  stack?: boolean;
  showLegend?: boolean;
  xAxisLabel?: string;
  yAxisLabel?: string;
  /** 每根柱子的颜色映射 */
  colors?: string[];
}

export interface AreaChartOptions extends LineChartOptions {
  gradient?: boolean;
  opacity?: number;
}

export interface PieChartOptions {
  /** 内半径 0-1（>0 时为环形图） */
  innerRadius?: number;
  showLegend?: boolean;
  showLabel?: boolean;
}

export interface ScatterChartOptions {
  showLegend?: boolean;
  xAxisLabel?: string;
  yAxisLabel?: string;
  /** z 值映射到的气泡半径范围 [min, max] px */
  bubbleSizeRange?: [number, number];
}

export interface HeatmapOptions {
  /** 颜色映射：低 → 高 */
  colorRange?: [string, string];
  /** 数值范围（超出会被截断） */
  valueRange?: [number, number];
  showLegend?: boolean;
}

export interface CandlestickOptions {
  showVolume?: boolean;
  upColor?: string;
  downColor?: string;
  maLines?: number[]; // 例如 [5, 20, 60]
}

export interface TreemapOptions {
  showLabel?: boolean;
  /** 节点边框色 */
  borderColor?: string;
  /** 父节点透明度 */
  parentOpacity?: number;
}

export interface SankeyOptions {
  nodeWidth?: number;
  nodePadding?: number;
  showLabel?: boolean;
}

export interface GaugeOptions {
  min?: number;
  max?: number;
  /** 当前值 */
  value: number;
  /** 分段着色 */
  ranges: VizGaugeRange[];
  /** 中心标题 */
  title?: string;
  /** 单位 */
  unit?: string;
}

export interface TableOptions {
  columns: VizTableColumn[];
  /** 是否启用斑马纹 */
  striped?: boolean;
  /** 是否启用行选择 */
  selectable?: boolean;
  /** 默认排序字段 */
  defaultSortKey?: string;
  defaultSortOrder?: "asc" | "desc";
}

// ── VizBlock 主类型 ────────────────────────────────────────────────────────

/**
 * 可视化块 — LLM / 工作流 / Agent 产出的统一可视化数据结构
 *
 * 每种 kind 对应一组 data + options 类型，由 VizBlockRenderer 路由渲染。
 */
export interface VizBlock {
  /** 唯一 ID（用于 React key + 持久化） */
  id: string;
  /** chart kind */
  kind: VizBlockKind;
  /** 标题 */
  title?: string;
  /** 副标题 / 描述 */
  subtitle?: string;
  /** 原始数据（具体结构按 kind 不同） */
  data: unknown;
  /** 渲染选项（具体结构按 kind 不同） */
  options?: Record<string, unknown>;
  /** 元数据：来源场景 / 生成时间 / 关联 ID */
  meta?: VizBlockMeta;
}

export interface VizBlockMeta {
  /** 来源场景：market-mainline / paper-portfolio / screenshot-diagnosis / ... */
  scene?: string;
  /** 生成时间戳（ms） */
  generatedAt?: number;
  /** 关联的 run_id / message_id / news_id */
  runId?: string;
  messageId?: string;
  newsId?: string;
  /** 生成方：llm / workflow / agent / system */
  source?: "llm" | "workflow" | "agent" | "system";
  /** 自定义标签 */
  tags?: string[];
}

// ── VisualizationPolicy：场景 → viz_block 配置矩阵 ─────────────────────────

/**
 * 可视化策略 — 按场景 ID 决定使用哪些 viz_block kind 及其默认配置。
 *
 * 用于工作流模板和 Agent 输出规范：根据当前任务场景，告诉 LLM 应该输出哪些 viz_block。
 */
export interface VisualizationPolicyEntry {
  /** 场景 ID */
  sceneId: string;
  /** 场景名称（i18n key） */
  sceneName: string;
  /** 推荐使用的 chart kind 列表 */
  recommendedKinds: VizBlockKind[];
  /** 每个 kind 的默认 options */
  defaultOptions?: Partial<Record<VizBlockKind, Record<string, unknown>>>;
  /** 输出约束：最多生成 N 个 block */
  maxBlocks?: number;
}

/** 内置场景 ID 矩阵（与 G2/G3/G4/G6 场景对齐） */
export const VIZ_POLICY_SCENE_IDS = {
  MARKET_MAINLINE: "market-mainline",
  PAPER_PORTFOLIO: "paper-portfolio",
  SCREENSHOT_DIAGNOSIS: "screenshot-diagnosis",
  INDUSTRY_CHAIN: "industry-chain",
  CROSS_MARKET_NEWS: "cross-market-news",
  QUANT_BACKTEST: "quant-backtest",
} as const;

export type VizPolicySceneId = (typeof VIZ_POLICY_SCENE_IDS)[keyof typeof VIZ_POLICY_SCENE_IDS];

/** 内置 VisualizationPolicy 默认矩阵 */
export const DEFAULT_VIZ_POLICIES: Record<string, VisualizationPolicyEntry> = {
  [VIZ_POLICY_SCENE_IDS.MARKET_MAINLINE]: {
    sceneId: VIZ_POLICY_SCENE_IDS.MARKET_MAINLINE,
    sceneName: "viz.scene.market-mainline",
    recommendedKinds: ["bar_chart", "line_chart", "table"],
    defaultOptions: {
      bar_chart: { orientation: "horizontal", showLegend: true },
      line_chart: { smooth: true, showLegend: true },
    },
    maxBlocks: 3,
  },
  [VIZ_POLICY_SCENE_IDS.PAPER_PORTFOLIO]: {
    sceneId: VIZ_POLICY_SCENE_IDS.PAPER_PORTFOLIO,
    sceneName: "viz.scene.paper-portfolio",
    recommendedKinds: ["pie_chart", "line_chart", "table"],
    defaultOptions: {
      pie_chart: { innerRadius: 0.4, showLegend: true, showLabel: true },
      line_chart: { smooth: true, showLegend: true },
    },
    maxBlocks: 3,
  },
  [VIZ_POLICY_SCENE_IDS.SCREENSHOT_DIAGNOSIS]: {
    sceneId: VIZ_POLICY_SCENE_IDS.SCREENSHOT_DIAGNOSIS,
    sceneName: "viz.scene.screenshot-diagnosis",
    recommendedKinds: ["gauge", "treemap", "table"],
    defaultOptions: {
      gauge: {},
      treemap: { showLabel: true },
    },
    maxBlocks: 4,
  },
  [VIZ_POLICY_SCENE_IDS.INDUSTRY_CHAIN]: {
    sceneId: VIZ_POLICY_SCENE_IDS.INDUSTRY_CHAIN,
    sceneName: "viz.scene.industry-chain",
    recommendedKinds: ["sankey", "table"],
    defaultOptions: {
      sankey: { nodeWidth: 20, nodePadding: 10, showLabel: true },
    },
    maxBlocks: 2,
  },
  [VIZ_POLICY_SCENE_IDS.CROSS_MARKET_NEWS]: {
    sceneId: VIZ_POLICY_SCENE_IDS.CROSS_MARKET_NEWS,
    sceneName: "viz.scene.cross-market-news",
    recommendedKinds: ["heatmap", "bar_chart", "table"],
    defaultOptions: {
      heatmap: { colorRange: ["#52c41a", "#ff4d4f"], showLegend: true },
      bar_chart: { orientation: "vertical", showLegend: true },
    },
    maxBlocks: 3,
  },
  [VIZ_POLICY_SCENE_IDS.QUANT_BACKTEST]: {
    sceneId: VIZ_POLICY_SCENE_IDS.QUANT_BACKTEST,
    sceneName: "viz.scene.quant-backtest",
    recommendedKinds: ["line_chart", "candlestick", "table"],
    defaultOptions: {
      line_chart: { smooth: false, showLegend: true },
      candlestick: { showVolume: true, maLines: [5, 20, 60] },
    },
    maxBlocks: 4,
  },
};

// ── VizBlockRenderer 注册器 ────────────────────────────────────────────────

/**
 * 单个 kind 的渲染器实现
 *
 * 每种 kind 注册一个 render 函数，由 `VizBlockRenderer` 组件按 kind 路由调用。
 */
export interface VizBlockKindRenderer {
  kind: VizBlockKind;
  /** 完整 panel 渲染 */
  render: (block: VizBlock) => ReactNode;
  /** 紧凑 chat bubble 渲染（可空，回退到 render） */
  renderCompact?: (block: VizBlock) => ReactNode;
}

const vizBlockRegistry = new Map<VizBlockKind, VizBlockKindRenderer>();

/**
 * 注册某种 chart kind 的渲染器
 *
 * 通常在应用初始化时调用（如 `VizBlockRenderer.tsx` 顶部 import 时副作用注册）。
 */
export function registerVizBlockKindRenderer(renderer: VizBlockKindRenderer): void {
  if (vizBlockRegistry.has(renderer.kind)) {
    console.warn(`[vizBlocks] duplicate registration for kind: ${renderer.kind}`);
  }
  vizBlockRegistry.set(renderer.kind, renderer);
}

/** 获取某种 kind 的渲染器 */
export function getVizBlockKindRenderer(kind: VizBlockKind): VizBlockKindRenderer | undefined {
  return vizBlockRegistry.get(kind);
}

/** 列出所有已注册的 kind */
export function listRegisteredVizBlockKinds(): VizBlockKind[] {
  return Array.from(vizBlockRegistry.keys());
}

/** 校验 VizBlock 数据结构基本合法性 */
export function validateVizBlock(block: VizBlock): string[] {
  const errors: string[] = [];
  if (!block.id) { errors.push("id 缺失"); }
  if (!block.kind) { errors.push("kind 缺失"); }
  if (!vizBlockRegistry.has(block.kind)) {
    errors.push(`未注册的 kind: ${block.kind}`);
  }
  if (!block.data) { errors.push("data 缺失"); }
  return errors;
}

/** 测试用：清空注册表 */
export function _resetVizBlockRegistry(): void {
  vizBlockRegistry.clear();
}

// ── Helper：从 VizBlockMeta 推断 sceneId 对应的 policy ─────────────────────

/** 根据场景 ID 获取 VisualizationPolicy */
export function getVisualizationPolicy(sceneId: string): VisualizationPolicyEntry | undefined {
  return DEFAULT_VIZ_POLICIES[sceneId];
}

/** 列出所有内置场景策略 */
export function listVisualizationPolicies(): VisualizationPolicyEntry[] {
  return Object.values(DEFAULT_VIZ_POLICIES);
}
