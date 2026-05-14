import { useTracerStore } from "@/stores/devtools/tracerStore";
import { Button, Card, Col, Descriptions, Row, Space, Tabs, Tag, Typography } from "antd";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
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
            <Button onClick={() => handleExport("json")}>{t("devtools.exportJson")}</Button>
            <Button onClick={() => handleExport("csv")}>{t("devtools.exportCsv")}</Button>
          </Space>
        </div>

        <Row gutter={16}>
          <Col span={6}>
            <Card size="small">
              <Descriptions column={1} size="small">
                <Descriptions.Item label={t("devtools.duration")}>
                  {formatDuration(summary.duration_ms)}
                </Descriptions.Item>
                <Descriptions.Item label={t("devtools.spansCount")}>
                  {summary.span_count}
                </Descriptions.Item>
                <Descriptions.Item label={t("devtools.errorCount")}>
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
                <Descriptions.Item label={t("devtools.model")}>
                  {trace.metadata.model}
                </Descriptions.Item>
                <Descriptions.Item label={t("devtools.totalTokens")}>
                  {formatTokens(trace.metadata.total_tokens)}
                </Descriptions.Item>
                <Descriptions.Item label={t("devtools.cost")}>
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
                <Descriptions.Item label={t("devtools.agentVersion")}>
                  {trace.metadata.agent_version}
                </Descriptions.Item>
                <Descriptions.Item label={t("devtools.startedAt")}>
                  {new Date(summary.started_at).toLocaleString()}
                </Descriptions.Item>
              </Descriptions>
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              {metrics && (
                <Descriptions column={1} size="small">
                  <Descriptions.Item label={t("devtools.inputTokens")}>
                    {formatTokens(metrics.cost.input_tokens)}
                  </Descriptions.Item>
                  <Descriptions.Item label={t("devtools.outputTokens")}>
                    {formatTokens(metrics.cost.output_tokens)}
                  </Descriptions.Item>
                  <Descriptions.Item label={t("devtools.cacheRead")}>
                    {formatTokens(metrics.cost.cache_read_tokens)}
                  </Descriptions.Item>
                </Descriptions>
              )}
            </Card>
          </Col>
        </Row>
      </div>

      <Tabs defaultActiveKey="tree" className="flex-1 overflow-hidden px-4">
        <Tabs.TabPane tab={t("devtools.callTree")} key="tree">
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
        <Tabs.TabPane tab={t("devtools.timeline")} key="timeline">
          <Timeline spans={trace.spans} />
        </Tabs.TabPane>
        <Tabs.TabPane tab={t("devtools.costAnalysis")} key="cost">
          {metrics && <CostChart metrics={metrics} />}
        </Tabs.TabPane>
        <Tabs.TabPane tab={t("devtools.durationAnalysis")} key="duration">
          {metrics && <DurationChart metrics={metrics} />}
        </Tabs.TabPane>
      </Tabs>
    </div>
  );
}
