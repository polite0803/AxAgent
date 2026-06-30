// SPDX-License-Identifier: AGPL-3.0-only

import type { DynamicUIProps } from "@/types";
import { Alert } from "antd";
import { lazy, Suspense } from "react";

type ChartType = "line" | "bar" | "pie" | "scatter" | "area";

/**
 * 图表渲染器。
 * 尝试延迟加载 Recharts，如果未安装则渲染提示。
 */
export const ChartRenderer: React.FC<DynamicUIProps> = ({
  schema,
  dataContext,
}) => {
  const {
    chartType = "bar",
    data,
    xKey,
    yKey,
    seriesKey,
  } = schema.props as {
    chartType?: ChartType;
    data?: Record<string, unknown>[];
    xKey?: string;
    yKey?: string;
    seriesKey?: string;
  };

  const chartData =
    data
    || (dataContext &&
        Array.isArray(
          (dataContext as Record<string, unknown>)[schema.id],
        ))
      ? (
        (dataContext as Record<string, unknown>)[schema.id] as Record<
          string,
          unknown
        >[]
      )
      : [];

  return (
    <div style={schema.style as React.CSSProperties}>
      <Suspense
        fallback={
          <Alert
            message="加载图表组件中..."
            type="info"
            showIcon
          />
        }
      >
        <LazyRechartsRenderer
          chartType={chartType}
          data={chartData}
          xKey={xKey || "name"}
          yKey={yKey || "value"}
          seriesKey={seriesKey}
        />
      </Suspense>
    </div>
  );
};

/**
 * 延迟加载 Recharts 图表渲染子组件。
 * 如果导入失败，显示安装提示。
 */
const LazyRechartsRenderer = lazy(
  () =>
    import("./ChartRendererImpl").catch(() => ({
      default: (() => (
        <Alert
          message="图表功能需要安装 Recharts"
          description="请执行 npm install recharts 或 pnpm add recharts"
          type="warning"
          showIcon
        />
      )) as React.FC<{
        chartType: string;
        data: Record<string, unknown>[];
        xKey: string;
        yKey: string;
        seriesKey?: string;
      }>,
    })) as Promise<{
      default: React.ComponentType<{
        chartType: string;
        data: Record<string, unknown>[];
        xKey: string;
        yKey: string;
        seriesKey?: string;
      }>;
    }>,
);

export default ChartRenderer;
