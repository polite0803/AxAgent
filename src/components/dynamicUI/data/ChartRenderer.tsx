// SPDX-License-Identifier: AGPL-3.0-only

import { resolveDynamicArray } from "@/lib/dynamicUI/utils";
import type { DynamicUIProps } from "@/types";
import { ChartRendererImpl } from "./ChartRendererImpl";

type ChartType = "line" | "bar" | "pie" | "scatter" | "area";

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

  const chartData = resolveDynamicArray(data, dataContext, schema.id);

  return (
    <div style={schema.style as React.CSSProperties}>
      <ChartRendererImpl
        chartType={chartType}
        data={chartData}
        xKey={xKey || "name"}
        yKey={yKey || "value"}
        seriesKey={seriesKey}
      />
    </div>
  );
};
