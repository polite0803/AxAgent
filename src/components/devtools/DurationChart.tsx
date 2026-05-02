import type { TraceMetrics } from "@/types/tracer";
import { Card, Col, Row, Statistic } from "antd";
import { useTranslation } from "react-i18next";

interface DurationChartProps {
  metrics: TraceMetrics;
}

function formatDuration(ms: number): string {
  if (ms < 1000) { return `${ms}ms`; }
  if (ms < 60000) { return `${(ms / 1000).toFixed(1)}s`; }
  return `${(ms / 60000).toFixed(1)}m`;
}

export function DurationChart({ metrics }: DurationChartProps) {
  const { t } = useTranslation();
  return (
    <div className="p-4">
      <Card title={t("devtools.durationOverview")}>
        <Row gutter={16}>
          <Col span={8}>
            <Statistic
              title={t("devtools.totalDuration")}
              value={metrics.total_duration_ms}
              formatter={(value) => formatDuration(Number(value))}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title={t("devtools.spansCount")}
              value={metrics.spans_count}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title={t("devtools.avgPerSpan")}
              value={metrics.spans_count > 0 ? metrics.total_duration_ms / metrics.spans_count : 0}
              formatter={(value) => formatDuration(Number(value))}
            />
          </Col>
        </Row>
      </Card>

      {metrics.ttft_ms && (
        <Card title={t("devtools.timeToFirstToken")} className="mt-4">
          <Statistic
            value={metrics.ttft_ms}
            formatter={(value) => formatDuration(Number(value))}
          />
        </Card>
      )}

      <Card title={t("devtools.performanceMetrics")} className="mt-4">
        <Row gutter={16}>
          <Col span={12}>
            <div className="text-gray-500 mb-1">{t("devtools.errorRate")}</div>
            <div className="text-2xl">
              {metrics.spans_count > 0
                ? `${((metrics.errors_count / metrics.spans_count) * 100).toFixed(1)}%`
                : "0%"}
            </div>
          </Col>
          <Col span={12}>
            <div className="text-gray-500 mb-1">{t("devtools.errorCount")}</div>
            <div className="text-2xl text-red-500">{metrics.errors_count}</div>
          </Col>
        </Row>
      </Card>
    </div>
  );
}
