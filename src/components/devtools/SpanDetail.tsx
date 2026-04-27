import type { Span } from "@/types/tracer";
import { Typography, Card, Tag, Descriptions, Table } from "antd";
import dayjs from "dayjs";

const { Text } = Typography;

function formatDuration(ms?: number): string {
  if (!ms) return "-";
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function formatTimestamp(ts: string): string {
  return dayjs(ts).format("HH:mm:ss.SSS");
}

interface SpanDetailProps {
  span: Span;
}

export function SpanDetail({ span }: SpanDetailProps) {
  const errorColumns = [
    { title: "Type", dataIndex: "error_type", key: "error_type" },
    { title: "Message", dataIndex: "message", key: "message" },
  ];

  const eventColumns = [
    { title: "Time", dataIndex: "timestamp", key: "timestamp", render: formatTimestamp },
    { title: "Name", dataIndex: "name", key: "name" },
  ];

  return (
    <div className="py-2">
      <div className="text-sm font-medium text-gray-600 mb-3">
        Span 详情
      </div>

      <Card size="small" className="mb-3">
        <Descriptions column={1} size="small">
          <Descriptions.Item label="ID">
            <Text code copyable className="text-xs">
              {span.id}
            </Text>
          </Descriptions.Item>
          <Descriptions.Item label="Trace ID">
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
          <Descriptions.Item label="Name">{span.name}</Descriptions.Item>
          <Descriptions.Item label="Type">
            <Tag>{span.span_type.replace("_", " ")}</Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Status">
            <Tag color={span.status === "ok" ? "green" : "red"}>
              {span.status}
            </Tag>
          </Descriptions.Item>
          <Descriptions.Item label="Duration">
            {formatDuration(span.duration_ms)}
          </Descriptions.Item>
          <Descriptions.Item label="Start Time">
            {formatTimestamp(span.start_time)}
          </Descriptions.Item>
          {span.end_time && (
            <Descriptions.Item label="End Time">
              {formatTimestamp(span.end_time)}
            </Descriptions.Item>
          )}
        </Descriptions>
      </Card>

      {span.service_name && (
        <Card size="small" className="mb-3">
          <Descriptions column={1} size="small">
            <Descriptions.Item label="Service">
              {span.service_name}
            </Descriptions.Item>
          </Descriptions>
        </Card>
      )}

      {span.inputs !== undefined && (
        <Card size="small" title="Inputs" className="mb-3">
          <pre className="max-h-48 overflow-auto text-xs bg-gray-50 p-2 rounded">
            {JSON.stringify(span.inputs, null, 2)}
          </pre>
        </Card>
      )}

      {span.outputs !== undefined && (
        <Card size="small" title="Outputs" className="mb-3">
          <pre className="max-h-48 overflow-auto text-xs bg-gray-50 p-2 rounded">
            {JSON.stringify(span.outputs, null, 2)}
          </pre>
        </Card>
      )}

      {Object.keys(span.attributes).length > 0 && (
        <Card size="small" title="Attributes" className="mb-3">
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
        <Card size="small" title="Events" className="mb-3">
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
        <Card size="small" title="Errors" className="mb-3">
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
