import { useTracerStore } from "@/stores/devtools/tracerStore";
import { Button, Card, Col, Descriptions, Row, Space, Tabs, Tag, Typography } from "antd";
import { CostChart } from "./CostChart";
import { DurationChart } from "./DurationChart";
import { SpanDetail } from "./SpanDetail";
import { SpanTree } from "./SpanTree";
import { Timeline } from "./Timeline";

const { Text } = Typography;

function formatDuration(ms?: number): string {
  if (!ms) { return "-"; }
  if (ms < 1000) { return `${ms}ms`; }
  if (ms < 60000) { return `${(ms / 1000).toFixed(1)}s`; }
  return `${(ms / 60000).toFixed(1)}m`;
}

function formatCost(cost: number): string {
  return `$${cost.toFixed(4)}`;
}

function formatTokens(tokens: number): string {
  if (tokens < 1000) { return `${tokens}`; }
  if (tokens < 1000000) { return `${(tokens / 1000).toFixed(1)}K`; }
  return `${(tokens / 1000000).toFixed(1)}M`;
}

export function TraceDetail() {
  const { selectedTrace, selectedSpan, tree, metrics, exportTrace } = useTracerStore();

  if (!selectedTrace) { return null; }

  const { trace, summary } = selectedTrace;

  const handleExport = (format: "json" | "csv") => {
    exportTrace(trace.trace_id, format);
  };

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b border-gray-200">
        <div className="flex justify-between items-center mb-4">
          <div>
            <Text strong className="text-lg">
              Trace: {trace.trace_id.slice(0, 8)}...
            </Text>
            <Text type="secondary" className="ml-2">
              Session: {summary.session_id}
            </Text>
          </div>
          <Space>
            <Button onClick={() => handleExport("json")}>导出 JSON</Button>
            <Button onClick={() => handleExport("csv")}>导出 CSV</Button>
          </Space>
        </div>

        <Row gutter={16}>
          <Col span={6}>
            <Card size="small">
              <Descriptions column={1} size="small">
                <Descriptions.Item label="持续时间">
                  {formatDuration(summary.duration_ms)}
                </Descriptions.Item>
                <Descriptions.Item label="Span 数量">
                  {summary.span_count}
                </Descriptions.Item>
                <Descriptions.Item label="错误数量">
                  <Tag color={summary.error_count > 0 ? "red" : "green"}>
                    {summary.error_count}
                  </Tag>
                </Descriptions.Item>
              </Descriptions>
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Descriptions column={1} size="small">
                <Descriptions.Item label="Model">
                  {trace.metadata.model}
                </Descriptions.Item>
                <Descriptions.Item label="Total Tokens">
                  {formatTokens(trace.metadata.total_tokens)}
                </Descriptions.Item>
                <Descriptions.Item label="Cost">
                  {formatCost(trace.metadata.total_cost_usd)}
                </Descriptions.Item>
              </Descriptions>
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Descriptions column={1} size="small">
                <Descriptions.Item label="User ID">
                  {trace.metadata.user_id}
                </Descriptions.Item>
                <Descriptions.Item label="Agent Version">
                  {trace.metadata.agent_version}
                </Descriptions.Item>
                <Descriptions.Item label="Started At">
                  {new Date(summary.started_at).toLocaleString()}
                </Descriptions.Item>
              </Descriptions>
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              {metrics && (
                <Descriptions column={1} size="small">
                  <Descriptions.Item label="Input Tokens">
                    {formatTokens(metrics.cost.input_tokens)}
                  </Descriptions.Item>
                  <Descriptions.Item label="Output Tokens">
                    {formatTokens(metrics.cost.output_tokens)}
                  </Descriptions.Item>
                  <Descriptions.Item label="Cache Read">
                    {formatTokens(metrics.cost.cache_read_tokens)}
                  </Descriptions.Item>
                </Descriptions>
              )}
            </Card>
          </Col>
        </Row>
      </div>

      <Tabs defaultActiveKey="tree" className="flex-1 overflow-hidden px-4">
        <Tabs.TabPane tab="调用树" key="tree">
          <div className="flex h-full">
            <div className="flex-1 overflow-auto pr-4">
              <SpanTree spans={tree} />
            </div>
            {selectedSpan && (
              <div className="w-96 border-l border-gray-200 pl-4 overflow-auto">
                <SpanDetail span={selectedSpan} />
              </div>
            )}
          </div>
        </Tabs.TabPane>
        <Tabs.TabPane tab="时间线" key="timeline">
          <Timeline spans={trace.spans} />
        </Tabs.TabPane>
        <Tabs.TabPane tab="成本分析" key="cost">
          {metrics && <CostChart metrics={metrics} />}
        </Tabs.TabPane>
        <Tabs.TabPane tab="耗时分析" key="duration">
          {metrics && <DurationChart metrics={metrics} />}
        </Tabs.TabPane>
      </Tabs>
    </div>
  );
}
