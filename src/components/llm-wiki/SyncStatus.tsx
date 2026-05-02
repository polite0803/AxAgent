import { invoke } from "@/lib/invoke";
import type { CapacityInfo, SyncQueueItem } from "@/types/llmWiki";
import {
  CheckCircleOutlined,
  CloseCircleOutlined,
  PauseCircleOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  SyncOutlined,
} from "@ant-design/icons";
import {
  Badge,
  Button,
  Card,
  Col,
  Empty,
  List,
  message,
  Progress,
  Row,
  Space,
  Spin,
  Statistic,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface SyncStatusProps {
  wikiId: string;
  autoRefresh?: boolean;
  refreshInterval?: number;
}

export function SyncStatus({
  wikiId,
  autoRefresh = false,
  refreshInterval = 30000,
}: SyncStatusProps) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [queueItems, setQueueItems] = useState<SyncQueueItem[]>([]);
  const [capacityInfo, setCapacityInfo] = useState<CapacityInfo | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [processing, setProcessing] = useState(false);

  useEffect(() => {
    loadSyncStatus();
  }, [wikiId]);

  useEffect(() => {
    if (!autoRefresh) { return; }
    const interval = setInterval(loadSyncStatus, refreshInterval);
    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval, wikiId]);

  const loadSyncStatus = async () => {
    setRefreshing(true);
    try {
      const [queue, capacity] = await Promise.all([
        invoke<SyncQueueItem[]>("wiki_sync_get_queue", { wikiId }),
        invoke<CapacityInfo>("wiki_get_capacity_info", { wikiId }),
      ]);
      setQueueItems(queue || []);
      setCapacityInfo(capacity);
    } catch (e) {
      console.error("Failed to load sync status:", e);
    }
    setLoading(false);
    setRefreshing(false);
  };

  const handleProcessQueue = async () => {
    setProcessing(true);
    try {
      await invoke("wiki_sync_process_pending", { wikiId });
      message.success(t("wiki.sync.processStarted", "Sync process started"));
      await loadSyncStatus();
    } catch (e) {
      message.error(String(e));
    }
    setProcessing(false);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "completed":
        return "success";
      case "failed":
        return "error";
      case "processing":
        return "processing";
      case "pending":
        return "default";
      default:
        return "default";
    }
  };

  const getEventTypeLabel = (eventType: string) => {
    switch (eventType) {
      case "note_created":
        return t("wiki.sync.noteCreated", "Note Created");
      case "note_updated":
        return t("wiki.sync.noteUpdated", "Note Updated");
      case "note_deleted":
        return t("wiki.sync.noteDeleted", "Note Deleted");
      case "link_created":
        return t("wiki.sync.linkCreated", "Link Created");
      case "link_deleted":
        return t("wiki.sync.linkDeleted", "Link Deleted");
      default:
        return eventType;
    }
  };

  if (loading) {
    return (
      <Card size="small">
        <div className="flex items-center justify-center py-8">
          <Spin size="large" />
        </div>
      </Card>
    );
  }

  const pendingCount = queueItems.filter(i => i.status === "pending").length;
  const processingCount = queueItems.filter(i => i.status === "processing").length;
  const failedCount = queueItems.filter(i => i.status === "failed").length;

  return (
    <Space direction="vertical" size="large" style={{ width: "100%" }}>
      <Card
        size="small"
        title={
          <Space>
            <SyncOutlined spin={refreshing} />
            <span>{t("wiki.sync.title", "Sync Status")}</span>
          </Space>
        }
        extra={
          <Tooltip title={t("wiki.sync.refresh", "Refresh")}>
            <Button
              type="text"
              size="small"
              icon={<ReloadOutlined spin={refreshing} />}
              onClick={loadSyncStatus}
            />
          </Tooltip>
        }
      >
        <Row gutter={16}>
          <Col span={8}>
            <Statistic
              title={t("wiki.sync.pending", "Pending")}
              value={pendingCount}
              prefix={<PauseCircleOutlined />}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title={t("wiki.sync.processing", "Processing")}
              value={processingCount}
              prefix={<SyncOutlined spin />}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title={t("wiki.sync.failed", "Failed")}
              value={failedCount}
              valueStyle={{ color: failedCount > 0 ? "#ff4d4f" : undefined }}
              prefix={failedCount > 0 ? <CloseCircleOutlined /> : <CheckCircleOutlined />}
            />
          </Col>
        </Row>

        {pendingCount > 0 && (
          <div className="mt-4">
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              loading={processing}
              onClick={handleProcessQueue}
              block
            >
              {t("wiki.sync.processNow", "Process Queue ({{count}} items)", { count: pendingCount })}
            </Button>
          </div>
        )}
      </Card>

      {capacityInfo && (
        <Card size="small" title={t("wiki.sync.capacity", "Vector Store Capacity")}>
          <div className="mb-3">
            <div className="flex justify-between mb-1">
              <Text>{t("wiki.sync.usage", "Usage")}</Text>
              <Text>{capacityInfo.totalChunks} / {capacityInfo.maxChunks}</Text>
            </div>
            <Progress
              percent={capacityInfo.usagePercent}
              strokeColor={capacityInfo.usagePercent > 90
                ? "#ff4d4f"
                : capacityInfo.usagePercent > 70
                ? "#faad14"
                : "#52c41a"}
              size="small"
            />
          </div>

          {Object.keys(capacityInfo.wikiChunkCounts).length > 0 && (
            <div>
              <Text type="secondary" className="text-xs uppercase">
                {t("wiki.sync.byWiki", "By Wiki")}
              </Text>
              <div className="mt-2">
                {Object.entries(capacityInfo.wikiChunkCounts).map(([id, count]) => (
                  <div key={id} className="flex justify-between items-center py-1">
                    <Text className="text-sm truncate" style={{ maxWidth: 150 }}>{id}</Text>
                    <Tag>{count} chunks</Tag>
                  </div>
                ))}
              </div>
            </div>
          )}
        </Card>
      )}

      <Card size="small" title={t("wiki.sync.queue", "Sync Queue")}>
        {queueItems.length === 0
          ? (
            <Empty
              description={t("wiki.sync.emptyQueue", "Queue is empty")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )
          : (
            <List
              size="small"
              dataSource={queueItems.slice(0, 20)}
              renderItem={(item) => (
                <List.Item className="px-0">
                  <div className="flex items-center justify-between w-full">
                    <Space>
                      <Badge status={getStatusColor(item.status) as any} />
                      <Tag>{getEventTypeLabel(item.eventType)}</Tag>
                    </Space>
                    <Space>
                      {item.retryCount > 0 && (
                        <Tooltip title={t("wiki.sync.retryCount", "{{count}} retries", { count: item.retryCount })}>
                          <Tag color="warning">{item.retryCount}</Tag>
                        </Tooltip>
                      )}
                      <Text type="secondary" className="text-xs">
                        {new Date(item.createdAt * 1000).toLocaleTimeString()}
                      </Text>
                    </Space>
                  </div>
                </List.Item>
              )}
            />
          )}
        {queueItems.length > 20 && (
          <Text type="secondary" className="text-xs">
            +{queueItems.length - 20} {t("wiki.sync.more", "more items")}
          </Text>
        )}
      </Card>
    </Space>
  );
}
