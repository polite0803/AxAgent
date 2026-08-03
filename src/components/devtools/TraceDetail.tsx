// SPDX-License-Identifier: AGPL-3.0-only

import { useFormatCny } from "@/stores";
import { useTracerStore } from "@/stores/devtools/tracerStore";
import { Button, Card, Col, Descriptions, Row, Space, Tabs, Tag, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { CostChart } from "./CostChart";
import { DurationChart } from "./DurationChart";
import { SpanDetail } from "./SpanDetail";
import { SpanTree } from "./SpanTree";
import { Timeline } from "./Timeline";

const { Text } = Typography;

function formatDuration(ms?: number): string {
  if (!ms) {
    return "-";
  }
  if (ms < 1000) {
    return `${ms}ms`;
  }
  if (ms < 60000) {
    return `${(ms / 1000).toFixed(1)}s`;
  }
  return `${(ms / 60000).toFixed(1)}m`;
}

function formatTokens(tokens: number): string {
  if (tokens < 1000) {
    return `${tokens}`;
  }
  if (tokens < 1000000) {
    return `${(tokens / 1000).toFixed(1)}K`;
  }
  return `${(tokens / 1000000).toFixed(1)}M`;
}

export function TraceDetail() {
  const { t } = useTranslation();
  // 成本以人民币展示
  const formatCny = useFormatCny();
  const { selectedTrace, selectedSpan, tree, metrics, exportTrace } = useTracerStore();

  const [startedAtFormatted, setStartedAtFormatted] = useState("");
  useEffect(() => {
    if (selectedTrace) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setStartedAtFormatted(
        new Date(selectedTrace.summary.started_at).toLocaleString(),
      );
    }
  }, [selectedTrace?.summary.started_at, selectedTrace]);

  if (!selectedTrace) {
    return null;
  }

  const { trace, summary } = selectedTrace;

  const handleExport = (format: "json" | "csv") => {
    exportTrace(trace.trace_id, format);
  };

  return (
    <div className="h-full flex flex-col">
      <div className="p-4 border-b border-zinc-200">
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
            <Button onClick={() => handleExport("json")}>
              {t("devtools.exportJson")}
            </Button>
            <Button onClick={() => handleExport("csv")}>
              {t("devtools.exportCsv")}
            </Button>
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
                  {formatCny(trace.metadata.total_cost_usd, 4)}
                </Descriptions.Item>
              </Descriptions>
            </Card>
          </Col>
          <Col span={6}>
            <Card size="small">
              <Descriptions column={1} size="small">
                <Descriptions.Item label={t("traceDetail.userId")}>
                  {trace.metadata.user_id}
                </Descriptions.Item>
                <Descriptions.Item label={t("devtools.agentVersion")}>
                  {trace.metadata.agent_version}
                </Descriptions.Item>
                <Descriptions.Item label={t("devtools.startedAt")}>
                  {startedAtFormatted}
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

      <Tabs
        defaultActiveKey="tree"
        className="flex-1 overflow-hidden px-4"
        items={[
          {
            key: "tree",
            label: t("devtools.callTree"),
            children: (
              <div className="flex h-full">
                <div className="flex-1 overflow-auto pr-4">
                  <SpanTree spans={tree} />
                </div>
                {selectedSpan && (
                  <div className="w-96 border-l border-zinc-200 pl-4 overflow-auto">
                    <SpanDetail span={selectedSpan} />
                  </div>
                )}
              </div>
            ),
          },
          {
            key: "timeline",
            label: t("devtools.timeline"),
            children: <Timeline spans={trace.spans} />,
          },
          {
            key: "cost",
            label: t("devtools.costAnalysis"),
            children: metrics ? <CostChart metrics={metrics} /> : null,
          },
          {
            key: "duration",
            label: t("devtools.durationAnalysis"),
            children: metrics ? <DurationChart metrics={metrics} /> : null,
          },
        ]}
      />
    </div>
  );
}
