// SPDX-License-Identifier: AGPL-3.0-only

/**
 * G15 VizBlockRenderer — viz_blocks 协议的渲染入口
 *
 * 通过 recharts 渲染简单图表（line / bar / area / pie / scatter / table），
 * 通过 ECharts（ChartPreview iframe）渲染复杂图表（candlestick / sankey / heatmap / treemap / gauge）。
 *
 * 使用：
 *   import "src/components/viz/VizBlockRenderer"; // 触发副作用注册
 *   <VizBlockRenderer block={block} mode="panel" />
 *
 * 渲染器在模块加载时自动注册到 vizBlockRegistry，无需手动调用。
 */

import { ChartPreview } from "@/components/chat/ArtifactPreview/ChartPreview";
import { Empty, Spin, Table, Tag } from "antd";
import type { ColumnsType } from "antd/es/table";
import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  Area,
  AreaChart,
  Bar,
  BarChart,
  CartesianGrid,
  Cell,
  Legend,
  Line,
  LineChart,
  Pie,
  PieChart,
  ResponsiveContainer,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
  ZAxis,
} from "recharts";

import {
  getVizBlockKindRenderer,
  registerVizBlockKindRenderer,
  validateVizBlock,
  type VizBlock,
  type VizBlockKind,
  type VizBlockKindRenderer,
  type VizCandle,
  type VizGaugeRange,
  type VizHeatmapPoint,
  type VizPieSlice,
  type VizPoint,
  type VizSankey,
  type VizScatterPoint,
  type VizTableColumn,
  type VizTreemapNode,
} from "@/lib/vizBlocks";

const COLORS = [
  "#1677ff",
  "#52c41a",
  "#faad14",
  "#ff4d4f",
  "#722ed1",
  "#13c2c2",
  "#eb2f96",
  "#fa8c16",
];

// ── VizBlockRenderer 主组件 ────────────────────────────────────────────────

interface VizBlockRendererProps {
  block: VizBlock;
  /** 渲染模式：panel=完整 / compact=紧凑 chat bubble */
  mode?: "panel" | "compact";
  /** 高度（默认 320） */
  height?: number;
}

/**
 * 渲染单个 VizBlock
 *
 * 内部按 `block.kind` 路由到对应注册的 `VizBlockKindRenderer`。
 * compact 模式优先使用 renderer.renderCompact，无则降级到 render。
 */
export function VizBlockRenderer({
  block,
  mode = "panel",
  height = 320,
}: VizBlockRendererProps): React.ReactElement {
  const { t } = useTranslation();
  const errors = useMemo(() => validateVizBlock(block), [block]);

  if (errors.length > 0) {
    return (
      <div style={{ padding: 16, color: "#ff4d4f" }}>
        <div>{t("viz.invalidBlock", { defaultValue: "VizBlock 数据非法" })}</div>
        <ul>
          {errors.map((e) => <li key={e}>{e}</li>)}
        </ul>
      </div>
    );
  }

  const renderer = getVizBlockKindRenderer(block.kind);
  if (!renderer) {
    return (
      <Empty
        description={t("viz.unsupportedKind", {
          defaultValue: "未支持的可视化类型: {{kind}}",
          kind: block.kind,
        })}
      />
    );
  }

  const content = mode === "compact" && renderer.renderCompact
    ? renderer.renderCompact(block)
    : renderer.render(block);

  return (
    <div
      className="viz-block-container"
      style={{ width: "100%", minHeight: height, padding: 8 }}
      data-viz-kind={block.kind}
      data-viz-id={block.id}
    >
      {(block.title || block.subtitle) && (
        <div style={{ marginBottom: 8 }}>
          {block.title && (
            <div style={{ fontSize: 14, fontWeight: 600, color: "#1f1f1f" }}>
              {block.title}
            </div>
          )}
          {block.subtitle && (
            <div style={{ fontSize: 12, color: "#8c8c8c", marginTop: 2 }}>
              {block.subtitle}
            </div>
          )}
        </div>
      )}
      {content as React.ReactNode}
    </div>
  );
}

// ── 通用辅助函数 ───────────────────────────────────────────────────────────

function asArray<T>(data: unknown): T[] {
  if (Array.isArray(data)) { return data as T[]; }
  return [];
}

function asNumber(v: unknown, fallback = 0): number {
  if (typeof v === "number") { return v; }
  if (typeof v === "string") {
    const n = Number(v);
    return Number.isFinite(n) ? n : fallback;
  }
  return fallback;
}

// ── 1. line_chart 渲染器 ──────────────────────────────────────────────────

const lineChartRenderer: VizBlockKindRenderer = {
  kind: "line_chart",
  render: (block) => <LineChartFull block={block} />,
  renderCompact: (block) => <LineChartFull block={block} compact />,
};

const LineChartFull: React.FC<{ block: VizBlock; compact?: boolean }> = ({ block, compact }) => {
  const data = asArray<VizPoint>(block.data);
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const series = Array.isArray(opts.series) ? (opts.series as string[]) : undefined;
  const smooth = opts.smooth !== false;
  const showLegend = opts.showLegend === true;
  const height = compact ? 200 : 300;

  if (data.length === 0) {
    return <Empty description="无数据" />;
  }

  return (
    <ResponsiveContainerWrapper height={height}>
      <LineChart data={data}>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis dataKey="name" />
        <YAxis />
        <Tooltip />
        {showLegend && <Legend />}
        {series
          ? series.map((s, i) => (
            <Line
              key={s}
              type={smooth ? "monotone" : "linear"}
              dataKey={s}
              stroke={COLORS[i % COLORS.length]}
              strokeWidth={2}
              dot={false}
            />
          ))
          : (
            <Line
              type={smooth ? "monotone" : "linear"}
              dataKey="value"
              stroke={COLORS[0]}
              strokeWidth={2}
              dot={false}
            />
          )}
      </LineChart>
    </ResponsiveContainerWrapper>
  );
};

// ── 2. bar_chart 渲染器 ───────────────────────────────────────────────────

const barChartRenderer: VizBlockKindRenderer = {
  kind: "bar_chart",
  render: (block) => <BarChartFull block={block} />,
  renderCompact: (block) => <BarChartFull block={block} compact />,
};

const BarChartFull: React.FC<{ block: VizBlock; compact?: boolean }> = ({ block, compact }) => {
  const data = asArray<VizPoint>(block.data);
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const horizontal = opts.orientation === "horizontal";
  const stack = opts.stack === true;
  const showLegend = opts.showLegend === true;
  const colors = Array.isArray(opts.colors) ? (opts.colors as string[]) : COLORS;
  const height = compact ? 200 : 300;

  if (data.length === 0) {
    return <Empty description="无数据" />;
  }

  const seriesKeys = useMemo(() => {
    if (data.length === 0) { return []; }
    const first = data[0];
    return Object.keys(first).filter((k) => k !== "name" && typeof (first as Record<string, unknown>)[k] === "number");
  }, [data]);

  return (
    <ResponsiveContainerWrapper height={height}>
      <BarChart data={data} layout={horizontal ? "vertical" : "horizontal"}>
        <CartesianGrid strokeDasharray="3 3" />
        {horizontal
          ? (
            <>
              <XAxis type="number" />
              <YAxis type="category" dataKey="name" width={80} />
            </>
          )
          : (
            <>
              <XAxis dataKey="name" />
              <YAxis />
            </>
          )}
        <Tooltip />
        {showLegend && <Legend />}
        {seriesKeys.length === 0
          ? <Bar dataKey="value" fill={colors[0]} />
          : seriesKeys.map((s, i) => (
            <Bar
              key={s}
              dataKey={s}
              stackId={stack ? "a" : undefined}
              fill={colors[i % colors.length]}
            >
              {data.map((_, idx) => <Cell key={`cell-${idx}`} fill={colors[(i + idx) % colors.length]} />)}
            </Bar>
          ))}
      </BarChart>
    </ResponsiveContainerWrapper>
  );
};

// ── 3. area_chart 渲染器 ──────────────────────────────────────────────────

const areaChartRenderer: VizBlockKindRenderer = {
  kind: "area_chart",
  render: (block) => <AreaChartFull block={block} />,
  renderCompact: (block) => <AreaChartFull block={block} compact />,
};

const AreaChartFull: React.FC<{ block: VizBlock; compact?: boolean }> = ({ block, compact }) => {
  const data = asArray<VizPoint>(block.data);
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const series = Array.isArray(opts.series) ? (opts.series as string[]) : undefined;
  const stack = opts.stack === true;
  const showLegend = opts.showLegend === true;
  const height = compact ? 200 : 300;

  if (data.length === 0) {
    return <Empty description="无数据" />;
  }

  return (
    <ResponsiveContainerWrapper height={height}>
      <AreaChart data={data}>
        <defs>
          {COLORS.map((c, i) => (
            <linearGradient key={i} id={`area-grad-${i}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor={c} stopOpacity={0.8} />
              <stop offset="95%" stopColor={c} stopOpacity={0.1} />
            </linearGradient>
          ))}
        </defs>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis dataKey="name" />
        <YAxis />
        <Tooltip />
        {showLegend && <Legend />}
        {series
          ? series.map((s, i) => (
            <Area
              key={s}
              type="monotone"
              dataKey={s}
              stackId={stack ? "a" : undefined}
              stroke={COLORS[i % COLORS.length]}
              fill={`url(#area-grad-${i})`}
              strokeWidth={2}
            />
          ))
          : <Area type="monotone" dataKey="value" stroke={COLORS[0]} fill={`url(#area-grad-0)`} strokeWidth={2} />}
      </AreaChart>
    </ResponsiveContainerWrapper>
  );
};

// ── 4. pie_chart 渲染器 ───────────────────────────────────────────────────

const pieChartRenderer: VizBlockKindRenderer = {
  kind: "pie_chart",
  render: (block) => <PieChartFull block={block} />,
  renderCompact: (block) => <PieChartFull block={block} compact />,
};

const PieChartFull: React.FC<{ block: VizBlock; compact?: boolean }> = ({ block, compact }) => {
  const data = asArray<VizPieSlice>(block.data);
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const innerRadius = typeof opts.innerRadius === "number" ? opts.innerRadius : 0;
  const showLegend = opts.showLegend !== false;
  const showLabel = opts.showLabel === true;
  const height = compact ? 200 : 300;

  if (data.length === 0) {
    return <Empty description="无数据" />;
  }

  return (
    <ResponsiveContainerWrapper height={height}>
      <PieChart>
        <Pie
          data={data}
          dataKey="value"
          nameKey="name"
          cx="50%"
          cy="50%"
          innerRadius={innerRadius * 100}
          outerRadius={100}
          label={showLabel}
        >
          {data.map((slice, i) => <Cell key={`cell-${i}`} fill={slice.color || COLORS[i % COLORS.length]} />)}
        </Pie>
        <Tooltip />
        {showLegend && <Legend />}
      </PieChart>
    </ResponsiveContainerWrapper>
  );
};

// ── 5. scatter_chart 渲染器 ───────────────────────────────────────────────

const scatterChartRenderer: VizBlockKindRenderer = {
  kind: "scatter_chart",
  render: (block) => <ScatterChartFull block={block} />,
  renderCompact: (block) => <ScatterChartFull block={block} compact />,
};

const ScatterChartFull: React.FC<{ block: VizBlock; compact?: boolean }> = ({ block, compact }) => {
  const data = asArray<VizScatterPoint>(block.data);
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const showLegend = opts.showLegend === true;
  const height = compact ? 200 : 300;

  if (data.length === 0) {
    return <Empty description="无数据" />;
  }

  // 按 group 分组
  const groups = useMemo(() => {
    const m = new Map<string, VizScatterPoint[]>();
    for (const p of data) {
      const g = p.group ?? "default";
      if (!m.has(g)) { m.set(g, []); }
      m.get(g)!.push(p);
    }
    return Array.from(m.entries());
  }, [data]);

  return (
    <ResponsiveContainerWrapper height={height}>
      <ScatterChart>
        <CartesianGrid strokeDasharray="3 3" />
        <XAxis dataKey="x" name="x" />
        <YAxis dataKey="y" name="y" />
        <ZAxis dataKey="z" range={[60, 400]} name="z" />
        <Tooltip cursor={{ strokeDasharray: "3 3" }} />
        {showLegend && <Legend />}
        {groups.map(([g, points], i) => <Scatter key={g} name={g} data={points} fill={COLORS[i % COLORS.length]} />)}
      </ScatterChart>
    </ResponsiveContainerWrapper>
  );
};

// ── 6. heatmap 渲染器（ECharts） ──────────────────────────────────────────

const heatmapRenderer: VizBlockKindRenderer = {
  kind: "heatmap",
  render: (block) => <EChartsBlock block={block} buildOption={buildHeatmapOption} />,
  renderCompact: (block) => <EChartsBlock block={block} buildOption={buildHeatmapOption} compact />,
};

function buildHeatmapOption(block: VizBlock): Record<string, unknown> {
  const data = asArray<VizHeatmapPoint>(block.data);
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const colorRange = Array.isArray(opts.colorRange) && opts.colorRange.length === 2
    ? (opts.colorRange as [string, string])
    : ["#52c41a", "#ff4d4f"];
  const valueRange = Array.isArray(opts.valueRange) && opts.valueRange.length === 2
    ? (opts.valueRange as [number, number])
    : (() => {
      const vals = data.map((d) => d.value);
      return [Math.min(...vals), Math.max(...vals)];
    })();

  const xs = Array.from(new Set(data.map((d) => d.x)));
  const ys = Array.from(new Set(data.map((d) => d.y)));

  return {
    tooltip: { position: "top" },
    grid: { top: 30, right: 50, bottom: 50, left: 60 },
    xAxis: { type: "category", data: xs, splitArea: { show: true } },
    yAxis: { type: "category", data: ys, splitArea: { show: true } },
    visualMap: {
      min: valueRange[0],
      max: valueRange[1],
      calculable: true,
      orient: "horizontal",
      left: "center",
      bottom: 0,
      inRange: { color: colorRange },
    },
    series: [
      {
        type: "heatmap",
        data: data.map((d) => [xs.indexOf(d.x), ys.indexOf(d.y), d.value]),
        label: { show: true },
        emphasis: { itemStyle: { shadowBlur: 10, shadowColor: "rgba(0, 0, 0, 0.5)" } },
      },
    ],
  };
}

// ── 7. candlestick 渲染器（ECharts） ──────────────────────────────────────

const candlestickRenderer: VizBlockKindRenderer = {
  kind: "candlestick",
  render: (block) => <EChartsBlock block={block} buildOption={buildCandlestickOption} />,
  renderCompact: (block) => <EChartsBlock block={block} buildOption={buildCandlestickOption} compact />,
};

function buildCandlestickOption(block: VizBlock): Record<string, unknown> {
  const data = asArray<VizCandle>(block.data);
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const showVolume = opts.showVolume !== false;
  const upColor = typeof opts.upColor === "string" ? opts.upColor : "#ef232a";
  const downColor = typeof opts.downColor === "string" ? opts.downColor : "#14b143";

  const dates = data.map((d) => d.date);
  const ohlc = data.map((d) => [d.open, d.close, d.low, d.high]);

  return {
    animation: false,
    tooltip: { trigger: "axis", axisPointer: { type: "cross" } },
    axisPointer: { link: [{ xAxisIndex: "all" }] },
    grid: [
      { left: "10%", right: "8%", height: "60%" },
      { left: "10%", right: "8%", top: "75%", height: "16%" },
    ],
    xAxis: [
      {
        type: "category",
        data: dates,
        scale: true,
        boundaryGap: false,
        axisLine: { onZero: false },
        splitLine: { show: false },
      },
      { type: "category", gridIndex: 1, data: dates, scale: true, boundaryGap: false, axisLabel: { show: false } },
    ],
    yAxis: [
      { scale: true, splitArea: { show: true } },
      { gridIndex: 1, splitNumber: 2, axisLabel: { show: false } },
    ],
    series: [
      {
        type: "candlestick",
        data: ohlc,
        itemStyle: { color: upColor, color0: downColor, borderColor: upColor, borderColor0: downColor },
      },
      ...(showVolume
        ? [{
          type: "bar",
          xAxisIndex: 1,
          yAxisIndex: 1,
          data: data.map((d) => ({
            value: d.volume ?? 0,
            itemStyle: { color: d.close >= d.open ? upColor : downColor },
          })),
        }]
        : []),
    ],
  };
}

// ── 8. treemap 渲染器（ECharts） ──────────────────────────────────────────

const treemapRenderer: VizBlockKindRenderer = {
  kind: "treemap",
  render: (block) => <EChartsBlock block={block} buildOption={buildTreemapOption} />,
  renderCompact: (block) => <EChartsBlock block={block} buildOption={buildTreemapOption} compact />,
};

function buildTreemapOption(block: VizBlock): Record<string, unknown> {
  const data = asArray<VizTreemapNode>(block.data);
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const showLabel = opts.showLabel !== false;

  return {
    tooltip: { formatter: (info: { name: string; value: number }) => `${info.name}: ${info.value}` },
    series: [
      {
        type: "treemap",
        data: data.map((d) => ({
          name: d.name,
          value: d.value,
          children: d.children,
          itemStyle: { color: d.color },
        })),
        label: { show: showLabel },
        levels: [
          { itemStyle: { borderColor: "#fff", borderWidth: 0, gapWidth: 1 } },
          { itemStyle: { borderColor: "#ddd", borderWidth: 1, gapWidth: 1 } },
        ],
      },
    ],
  };
}

// ── 9. sankey 渲染器（ECharts） ───────────────────────────────────────────

const sankeyRenderer: VizBlockKindRenderer = {
  kind: "sankey",
  render: (block) => <EChartsBlock block={block} buildOption={buildSankeyOption} />,
  renderCompact: (block) => <EChartsBlock block={block} buildOption={buildSankeyOption} compact />,
};

function buildSankeyOption(block: VizBlock): Record<string, unknown> {
  const data = block.data as VizSankey | undefined;
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const nodeWidth = typeof opts.nodeWidth === "number" ? opts.nodeWidth : 20;
  const nodePadding = typeof opts.nodePadding === "number" ? opts.nodePadding : 10;
  const showLabel = opts.showLabel !== false;

  if (!data || !Array.isArray(data.nodes) || !Array.isArray(data.links)) {
    return {};
  }

  return {
    tooltip: { trigger: "item" },
    series: [
      {
        type: "sankey",
        data: data.nodes,
        links: data.links,
        nodeWidth,
        nodePadding,
        emphasis: { focus: "adjacency" },
        label: { show: showLabel },
        lineStyle: { color: "gradient", curveness: 0.5 },
      },
    ],
  };
}

// ── 10. gauge 渲染器（ECharts） ───────────────────────────────────────────

const gaugeRenderer: VizBlockKindRenderer = {
  kind: "gauge",
  render: (block) => <EChartsBlock block={block} buildOption={buildGaugeOption} />,
  renderCompact: (block) => <EChartsBlock block={block} buildOption={buildGaugeOption} compact />,
};

function buildGaugeOption(block: VizBlock): Record<string, unknown> {
  const data = block.data as { value: number; ranges?: VizGaugeRange[]; title?: string; unit?: string } | undefined;
  const opts = (block.options ?? {}) as Record<string, unknown>;
  const min = typeof opts.min === "number" ? opts.min : 0;
  const max = typeof opts.max === "number" ? opts.max : 100;
  const ranges = (data?.ranges ?? (opts.ranges as VizGaugeRange[] | undefined)) ?? [
    { from: min, to: (min + max) / 2, color: "#52c41a" },
    { from: (min + max) / 2, to: max, color: "#ff4d4f" },
  ];
  const value = typeof data?.value === "number" ? data.value : asNumber(opts.value);

  return {
    series: [
      {
        type: "gauge",
        min,
        max,
        axisLine: {
          lineStyle: {
            width: 18,
            color: ranges.map((r) => [r.to / max, r.color]),
          },
        },
        pointer: { itemStyle: { color: "auto" } },
        axisTick: { distance: -20, length: 6, lineStyle: { color: "#fff", width: 1 } },
        splitLine: { distance: -20, length: 18, lineStyle: { color: "#fff", width: 2 } },
        axisLabel: { color: "inherit", distance: 30 },
        detail: {
          valueAnimation: true,
          formatter: `{value}${data?.unit ?? opts.unit ?? ""}`,
          color: "inherit",
          fontSize: 20,
        },
        title: data?.title ? { show: true, offsetCenter: [0, "30%"], fontSize: 14 } : undefined,
        data: [{ value, name: data?.title ?? "" }],
      },
    ],
  };
}

// ── 11. table 渲染器（Antd Table） ────────────────────────────────────────

const tableRenderer: VizBlockKindRenderer = {
  kind: "table",
  render: (block) => <TableFull block={block} />,
  renderCompact: (block) => <TableFull block={block} compact />,
};

const TableFull: React.FC<{ block: VizBlock; compact?: boolean }> = ({ block, compact }) => {
  const data = asArray<Record<string, unknown>>(block.data);
  const opts = (block.options ?? {}) as {
    columns?: VizTableColumn[];
    striped?: boolean;
    defaultSortKey?: string;
    defaultSortOrder?: "asc" | "desc";
  };
  const columns: ColumnsType<Record<string, unknown>> = useMemo(() => {
    if (opts.columns && opts.columns.length > 0) {
      return opts.columns.map((c) => ({
        title: c.title,
        dataIndex: c.key,
        key: c.key,
        width: c.width,
        align: c.align,
        sorter: c.sortable
          ? (a: Record<string, unknown>, b: Record<string, unknown>) => {
            const av = a[c.key];
            const bv = b[c.key];
            if (typeof av === "number" && typeof bv === "number") { return av - bv; }
            return String(av).localeCompare(String(bv));
          }
          : undefined,
        render: (val: unknown) => {
          if (typeof val === "number") {
            const color = val > 0 ? "#52c41a" : val < 0 ? "#ff4d4f" : undefined;
            return color ? <span style={{ color, fontWeight: 600 }}>{val}</span> : val;
          }
          if (typeof val === "string" && val.startsWith("tag:")) {
            const text = val.slice(4);
            const colorMap: Record<string, string> = {
              buy: "green",
              sell: "red",
              hold: "blue",
              wait: "orange",
            };
            return <Tag color={colorMap[text.toLowerCase()] ?? "default"}>{text}</Tag>;
          }
          return String(val);
        },
      }));
    }
    // 自动从首行推断列
    if (data.length === 0) { return []; }
    return Object.keys(data[0]).map((k) => ({ title: k, dataIndex: k, key: k }));
  }, [opts.columns, data]);

  if (data.length === 0) {
    return <Empty description="无数据" />;
  }

  return (
    <Table
      size={compact ? "small" : "middle"}
      columns={columns}
      dataSource={data.map((row, i) => ({ ...row, _key: i }))}
      rowKey="_key"
      pagination={compact ? false : { pageSize: 10, size: "small" }}
      scroll={compact ? undefined : { x: "max-content" }}
      rowClassName={opts.striped ? (_, i) => (i % 2 === 0 ? "" : "ant-table-row-striped") : undefined}
    />
  );
};

// ── ECharts 包装组件 ──────────────────────────────────────────────────────

interface EChartsBlockProps {
  block: VizBlock;
  buildOption: (block: VizBlock) => Record<string, unknown>;
  compact?: boolean;
}

const EChartsBlock: React.FC<EChartsBlockProps> = ({ block, buildOption, compact }) => {
  const option = useMemo(() => buildOption(block), [block, buildOption]);
  const height = compact ? 220 : 360;
  return (
    <div style={{ width: "100%", height }}>
      <ChartPreview option={option} height={height} />
    </div>
  );
};

// ── ResponsiveContainer 包装 ──────────────────────────────────────────────

const ResponsiveContainerWrapper: React.FC<{ height: number; children: React.ReactElement }> = (
  { height, children },
) => {
  return (
    <div style={{ width: "100%", height }}>
      <ResponsiveContainer width="100%" height="100%">
        {children}
      </ResponsiveContainer>
    </div>
  );
};

// ── 模块加载时自动注册 11 种 kind ──────────────────────────────────────────

const ALL_RENDERERS: VizBlockKindRenderer[] = [
  lineChartRenderer,
  barChartRenderer,
  areaChartRenderer,
  pieChartRenderer,
  scatterChartRenderer,
  heatmapRenderer,
  candlestickRenderer,
  treemapRenderer,
  sankeyRenderer,
  gaugeRenderer,
  tableRenderer,
];

let registered = false;
function ensureRegistered() {
  if (registered) { return; }
  for (const r of ALL_RENDERERS) {
    registerVizBlockKindRenderer(r);
  }
  registered = true;
}

// 副作用执行注册
ensureRegistered();

// ── 多 block 渲染器 ───────────────────────────────────────────────────────

interface VizBlockListRendererProps {
  blocks: VizBlock[];
  mode?: "panel" | "compact";
  height?: number;
  /** 自定义排序：按 meta.scene 优先级排序 */
  sortByScene?: boolean;
}

/**
 * 渲染多个 VizBlock
 *
 * 当 LLM / 工作流一次产出多个 block 时使用，按 block.id 去重。
 */
export function VizBlockListRenderer({
  blocks,
  mode = "panel",
  height,
  sortByScene = false,
}: VizBlockListRendererProps): React.ReactElement {
  const { t } = useTranslation();
  const sorted = useMemo(() => {
    if (!sortByScene) { return blocks; }
    return [...blocks].sort((a, b) => {
      const sa = a.meta?.scene ?? "";
      const sb = b.meta?.scene ?? "";
      return sa.localeCompare(sb);
    });
  }, [blocks, sortByScene]);

  if (sorted.length === 0) {
    return <Empty description={t("viz.noBlocks", { defaultValue: "暂无可视化块" })} />;
  }

  return (
    <div className="viz-block-list" style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {sorted.map((b) => <VizBlockRenderer key={b.id} block={b} mode={mode} height={height} />)}
    </div>
  );
}

// ── 导出 + 类型保护 ───────────────────────────────────────────────────────

export function isVizBlockKind(value: string): value is VizBlockKind {
  return [
    "line_chart",
    "bar_chart",
    "area_chart",
    "pie_chart",
    "scatter_chart",
    "heatmap",
    "candlestick",
    "treemap",
    "sankey",
    "gauge",
    "table",
  ].includes(value);
}

// 加载占位符
export function VizBlockLoading(): React.ReactElement {
  return (
    <div style={{ display: "flex", justifyContent: "center", alignItems: "center", height: 200 }}>
      <Spin />
    </div>
  );
}
