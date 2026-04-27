import { invoke } from "@tauri-apps/api/core";

export interface ChartGenRequest {
  description: string;
  data?: Record<string, unknown>[];
  chartType?: ChartType;
  title?: string;
}

export type ChartType =
  | "line"
  | "bar"
  | "pie"
  | "scatter"
  | "heatmap"
  | "radar"
  | "treemap"
  | "sankey"
  | "funnel"
  | "gauge";

export interface ChartGenResult {
  option: Record<string, unknown>;
  chartType: ChartType;
  title: string;
}

export async function generateChart(request: ChartGenRequest): Promise<ChartGenResult> {
  return invoke<ChartGenResult>("generate_chart_config", {
    description: request.description,
    data: request.data,
    chartType: request.chartType,
    title: request.title,
  });
}

export function detectChartIntent(message: string): ChartGenRequest | null {
  const patterns = [
    /(?:画|生成|绘制|创建|做一个|show)\s*(?:一个|一张)?\s*(.+?)\s*(?:图表|图|chart|graph)/i,
    /(?:可视化|visualize)\s+(.+)/i,
    /(.+?)\s*(?:的趋势|对比|分布|占比)(?:图)?/i,
  ];

  for (const pattern of patterns) {
    const match = message.match(pattern);
    if (match) {
      return {
        description: match[1] || message,
        chartType: inferChartType(message),
      };
    }
  }

  return null;
}

function inferChartType(message: string): ChartType | undefined {
  if (/趋势|变化|增长|时间|折线|line/i.test(message)) { return "line"; }
  if (/对比|比较|排名|柱状|bar/i.test(message)) { return "bar"; }
  if (/占比|比例|分布|饼图|pie/i.test(message)) { return "pie"; }
  if (/关系|关联|散点|scatter/i.test(message)) { return "scatter"; }
  if (/热力|密度|heatmap/i.test(message)) { return "heatmap"; }
  return undefined;
}
