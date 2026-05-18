import type { Span } from "@/types";
import { Card, Descriptions, Table, Tag, Typography } from "antd";
import dayjs from "dayjs";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

function formatDuration(ms?: number): string {
  if (!ms) {
    return "-";
  }
  if (ms < 1000) {
    return `${ms}ms`;
  }
  return `${(ms / 1000).toFixed(2)}s`;
}

function formatTimestamp(ts: string): string {
  return dayjs(ts).format("HH:mm:ss.SSS");
}

interface SpanDetailProps {
  span: Span;
}

export function SpanDetail({ span }: SpanDetailProps) {
  const { t } = useTranslation();
  const errorColumns = [
    { title: "Type", dataIndex: "error_type", key: "error_type" },
    { title: "Message", dataIndex: "message", key: "message" },
  ];

  const eventColumns = [
    {
      title: "Time",
      dataIndex: "timestamp",
      key: "timestamp",
      render: formatTimestamp,
    },
    { title: "Name", dataIndex: "name", key: "name" },
  ];

  return (
    <div className="py-2">
      <div className="text-sm font-medium text-zinc-600 mb-3">
        {t("devtools.spanDetail")}
      </div>

      <Card size="small" className="mb-3">
        <Descriptions column={1} size="small">
          <Descriptions.Item label={t("devtools.spanId")}>
            <Text code copyable className="text-xs">
              {span.id}
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label={t("devtools.traceId")}>
            <Text code copyable className="text-xs">
              {span.trace_id}
            </Text>
          </Descriptions.Item>
          {span.parent_span_id && (
            <Descriptions.Item label="Parent Span">
              <Text code copyable className="text-xs">
                {span.parent_span_id}
              </Text>
            </Descriptions.Item>
          )}
          <Descriptions.Item label={t("devtools.name")}>
            {span.name}
          </Descriptions.Item>
          <Descriptions.Item label={t("devtools.type")}>
            <Tag>{span.span_type.replace("_", " ")}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label={t("devtools.statusLabel")}>
            <Tag color={span.status === "ok" ? "green" : "red"}>
              {span.status}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label={t("devtools.duration")}>
            {formatDuration(span.duration_ms)}
          </Descriptions.Item>
          <Descriptions.Item label={t("devtools.startedAt")}>
            {formatTimestamp(span.start_time)}
          </Descriptions.Item>
          {span.end_time && (
            <Descriptions.Item label={t("devtools.endTime")}>
              {formatTimestamp(span.end_time)}
            </Descriptions.Item>
          )}
        </Descriptions>
      </Card>

      {span.service_name && (
        <Card size="small" className="mb-3">
          <Descriptions column={1} size="small">
            <Descriptions.Item label={t("devtools.service")}>
              {span.service_name}
            </Descriptions.Item>
          </Descriptions>
        </Card>
      )}

      {span.inputs !== undefined && (
        <Card size="small" title={t("devtools.inputs")} className="mb-3">
          <pre className="max-h-48 overflow-auto text-xs bg-zinc-50 p-2 rounded">
            {JSON.stringify(span.inputs, null, 2)}
          </pre>
        </Card>
      )}

      {span.outputs !== undefined && (
        <Card size="small" title={t("devtools.outputs")} className="mb-3">
          <pre className="max-h-48 overflow-auto text-xs bg-zinc-50 p-2 rounded">
            {JSON.stringify(span.outputs, null, 2)}
          </pre>
        </Card>
      )}

      {Object.keys(span.attributes).length > 0 && (
        <Card size="small" title={t("devtools.attributes")} className="mb-3">
          <Descriptions column={1} size="small">
            {Object.entries(span.attributes).map(([key, value]) => (
              <Descriptions.Item key={key} label={key}>
                <Text code>{JSON.stringify(value)}</Text>
              </Descriptions.Item>
            ))}
          </Descriptions>
        </Card>
      )}

      {span.events.length > 0 && (
        <Card size="small" title={t("devtools.events")} className="mb-3">
          <Table
            dataSource={span.events}
            columns={eventColumns}
            size="small"
            pagination={false}
            rowKey="timestamp"
          />
        </Card>
      )}

      {span.errors.length > 0 && (
        <Card size="small" title={t("devtools.errors")} className="mb-3">
          <Table
            dataSource={span.errors}
            columns={errorColumns}
            size="small"
            pagination={false}
            rowKey="timestamp"
          />
        </Card>
      )}
    </div>
  );
}
