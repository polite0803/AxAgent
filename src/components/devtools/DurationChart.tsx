import type { TraceMetrics } from "@/types/tracer";
import { Card, Statistic, Row, Col } from "antd";

interface DurationChartProps {
  metrics: TraceMetrics;
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${(ms / 60000).toFixed(1)}m`;
}

export function DurationChart({ metrics }: DurationChartProps) {
  return (
    <div className="p-4">
      <Card title="耗时概览">
        <Row gutter={16}>
          <Col span={8}>
            <Statistic
              title="总耗时"
              value={metrics.total_duration_ms}
              formatter={(value) => formatDuration(Number(value))}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title="Span 数量"
              value={metrics.spans_count}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title="平均每 Span"
              value={metrics.spans_count > 0 ? metrics.total_duration_ms / metrics.spans_count : 0}
              formatter={(value) => formatDuration(Number(value))}
            />
          </Col>
        </Row>
      </Card>

      {metrics.ttft_ms && (
        <Card title="Time to First Token" className="mt-4">
          <Statistic
            value={metrics.ttft_ms}
            formatter={(value) => formatDuration(Number(value))}
          />
        </Card>
      )}

      <Card title="性能指标" className="mt-4">
        <Row gutter={16}>
          <Col span={12}>
            <div className="text-gray-500 mb-1">错误率</div>
            <div className="text-2xl">
              {metrics.spans_count > 0
                ? `${((metrics.errors_count / metrics.spans_count) * 100).toFixed(1)}%`
                : "0%"}
            </div>
          </Col>
          <Col span={12}>
            <div className="text-gray-500 mb-1">错误数量</div>
            <div className="text-2xl text-red-500">{metrics.errors_count}</div>
          </Col>
        </Row>
      </Card>
    </div>
  );
}
