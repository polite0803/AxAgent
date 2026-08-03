// SPDX-License-Identifier: AGPL-3.0-only

import { resolveDynamicArray } from "@/lib/dynamicUI/utils";
import type { DynamicUIProps } from "@/types";
import { Alert } from "antd";
import { lazy, Suspense } from "react";
import { useTranslation } from "react-i18next";

type ChartType = "line" | "bar" | "pie" | "scatter" | "area";

const LazyRechartsRenderer = lazy(
  () =>
    import("./ChartRendererImpl").then((m) => ({
      default: m.ChartRendererImpl,
    })).catch(() => ({
      default: (() => <RechartsNotInstalled />) as React.FC<{
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

function RechartsNotInstalled() {
  const { t } = useTranslation();
  return (
    <Alert
      title={t("dynamicUI.rechartsRequired")}
      description={t("dynamicUI.rechartsInstallHint")}
      type="warning"
      showIcon
    />
  );
}

function ChartLoading() {
  const { t } = useTranslation();
  return (
    <Alert
      title={t("dynamicUI.loadingChart")}
      type="info"
      showIcon
    />
  );
}

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
      <Suspense fallback={<ChartLoading />}>
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
