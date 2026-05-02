import { WikiOperation } from "@/stores/feature/llmWikiStore";
import {
  ExperimentOutlined,
  LoadingOutlined,
  PlayCircleOutlined,
  SyncOutlined,
  UploadOutlined,
} from "@ant-design/icons";
import { Badge, Card, Empty, Space, Tag, Timeline, Tooltip, Typography } from "antd";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface OperationTimelineProps {
  operations: WikiOperation[];
}

const getOperationIcon = (operation: WikiOperation) => {
  if (operation.status === "running") {
    return <LoadingOutlined spin />;
  }

  switch (operation.operationType) {
    case "ingest":
      return <UploadOutlined />;
    case "compile":
      return <PlayCircleOutlined />;
    case "lint":
      return <ExperimentOutlined />;
    default:
      return <SyncOutlined />;
  }
};

const getStatusColor = (status: string) => {
  switch (status) {
    case "completed":
      return "success";
    case "failed":
      return "error";
    case "running":
      return "processing";
    case "pending":
      return "default";
    default:
      return "default";
  }
};

const getOperationTypeLabel = (type: string, t: (key: string) => string) => {
  switch (type) {
    case "ingest":
      return t("wiki.operation.ingest");
    case "compile":
      return t("wiki.operation.compile");
    case "lint":
      return t("wiki.operation.lint");
    case "sync":
      return t("wiki.operation.sync");
    default:
      return type;
  }
};

const getTargetTypeLabel = (type: string, t: (key: string) => string) => {
  switch (type) {
    case "source":
      return t("wiki.operation.source");
    case "page":
      return t("wiki.operation.page");
    case "schema":
      return t("wiki.operation.schema");
    default:
      return type;
  }
};

const formatDuration = (start: number, end?: number) => {
  if (!end) { return "-"; }
  const diff = end - start;
  if (diff < 1000) { return `${diff}ms`; }
  if (diff < 60000) { return `${(diff / 1000).toFixed(1)}s`; }
  return `${(diff / 60000).toFixed(1)}m`;
};

const formatDate = (timestamp: number) => {
  const date = new Date(timestamp);
  return date.toLocaleString();
};

export function OperationTimeline({ operations }: OperationTimelineProps) {
  const { t } = useTranslation();

  if (operations.length === 0) {
    return <Empty description={t("wiki.operation.noOperations")} image={Empty.PRESENTED_IMAGE_SIMPLE} />;
  }

  return (
    <Timeline
      mode="left"
      items={operations.map((op) => ({
        dot: <Badge status={getStatusColor(op.status) as "success" | "error" | "processing" | "default"} />,
        children: (
          <Card size="small" style={{ marginBottom: 8 }}>
            <Space direction="vertical" size="small" style={{ width: "100%" }}>
              <Space>
                <Tooltip title={formatDate(op.createdAt)}>
                  {getOperationIcon(op)}
                </Tooltip>
                <Tag color="blue">{getOperationTypeLabel(op.operationType, t)}</Tag>
                <Tag>{getTargetTypeLabel(op.targetType, t)}</Tag>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {op.targetId}
                </Text>
              </Space>

              <Space split={<span style={{ color: "#d9d9d9" }}>|</span>}>
                <Text type="secondary" style={{ fontSize: 11 }}>
                  {t("wiki.operation.status")}:
                  <Tag
                    color={op.status === "completed" ? "success" : op.status === "failed" ? "error" : "default"}
                    style={{ marginLeft: 4 }}
                  >
                    {op.status}
                  </Tag>
                </Text>
                <Text type="secondary" style={{ fontSize: 11 }}>
                  {t("wiki.operation.duration")}: {formatDuration(op.createdAt, op.completedAt)}
                </Text>
              </Space>

              {op.errorMessage && (
                <Text type="danger" style={{ fontSize: 12 }}>
                  {op.errorMessage}
                </Text>
              )}

              {op.detailsJson && (
                <div style={{ fontSize: 11, color: "#8c8c8c" }}>
                  {Object.entries(op.detailsJson).slice(0, 3).map(([key, value]) => (
                    <div key={key}>
                      {key}: {String(value)}
                    </div>
                  ))}
                </div>
              )}
            </Space>
          </Card>
        ),
      }))}
    />
  );
}
