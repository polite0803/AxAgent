// SPDX-License-Identifier: AGPL-3.0-only

import { showBackendError } from "@/lib/errorI18n";
import { invoke, logIpcError } from "@/lib/invoke";
import type { CapacityInfo, SyncQueueItem } from "@/types";
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
  message,
  Popover,
  Progress,
  Row,
  Space,
  Spin,
  Statistic,
  Tag,
  Tooltip,
  Typography,
} from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface SyncStatusProps {
  wikiId: string;
  autoRefresh?: boolean;
  refreshInterval?: number;
  /** 紧凑模式：工具栏里只显示 Badge 按钮，点击 Popover 展示完整面板 */
  compact?: boolean;
  /** 是否在挂载时自动加载（默认 false，避免阻塞页面渲染） */
  autoLoad?: boolean;
}

export function SyncStatus({
  wikiId,
  autoRefresh = false,
  refreshInterval = 30000,
  compact = false,
  autoLoad = false,
}: SyncStatusProps) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(autoLoad);
  const [queueItems, setQueueItems] = useState<SyncQueueItem[]>([]);
  const [capacityInfo, setCapacityInfo] = useState<CapacityInfo | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [processing, setProcessing] = useState(false);
  const [popoverOpen, setPopoverOpen] = useState(false);

  const loadSyncStatus = useCallback(async () => {
    setRefreshing(true);
    try {
      const [queue, capacity] = await Promise.all([
        invoke<SyncQueueItem[]>("wiki_sync_get_queue", { wikiId }),
        invoke<CapacityInfo>("wiki_get_capacity_info", { wikiId }),
      ]);
      setQueueItems(queue || []);
      setCapacityInfo(capacity);
    } catch (e) {
      logIpcError("Failed to load sync status")(e);
    }
    setLoading(false);
    setRefreshing(false);
  }, [wikiId]);

  const loadSyncStatusRef = useRef(loadSyncStatus);

  useEffect(() => {
    loadSyncStatusRef.current = loadSyncStatus;
  }, [loadSyncStatus]);

  // 默认不在挂载时自动加载，避免阻塞页面渲染
  useEffect(() => {
    if (autoLoad) {
      void loadSyncStatusRef.current();
    } else {
      setLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wikiId, autoLoad]);

  useEffect(() => {
    if (!autoRefresh) {
      return;
    }
    const interval = setInterval(() => loadSyncStatusRef.current(), refreshInterval);
    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval]);

  const handleProcessQueue = async () => {
    setProcessing(true);
    try {
      await invoke("wiki_sync_process_pending", { wikiId });
      message.success(t("wiki.sync.processStarted"));
      await loadSyncStatus();
    } catch (e) {
      showBackendError(message, e);
    }
    setProcessing(false);
  };

  const getStatusColor = (
    status: string,
  ): "success" | "error" | "processing" | "default" | "warning" => {
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
        return t("wiki.sync.noteCreated");
      case "note_updated":
        return t("wiki.sync.noteUpdated");
      case "note_deleted":
        return t("wiki.sync.noteDeleted");
      case "link_created":
        return t("wiki.sync.linkCreated");
      case "link_deleted":
        return t("wiki.sync.linkDeleted");
      default:
        return eventType;
    }
  };

  const pendingCount = queueItems.filter((i) => i.status === "pending").length;
  const processingCount = queueItems.filter(
    (i) => i.status === "processing",
  ).length;
  const failedCount = queueItems.filter((i) => i.status === "failed").length;
  const activeCount = pendingCount + processingCount + failedCount;

  // ========== 紧凑模式：工具栏 Badge 按钮 + 极致紧凑 Popover ==========
  if (compact) {
    const badgeColor = failedCount > 0
      ? "error"
      : processingCount > 0
      ? "processing"
      : pendingCount > 0
      ? "warning"
      : "success";
    const badgeCount = activeCount > 0 ? activeCount : 0;

    return (
      <Popover
        open={popoverOpen}
        onOpenChange={(open) => {
          setPopoverOpen(open);
          // 首次打开 Popover 时才加载数据（懒加载）
          if (open && !loading && queueItems.length === 0) {
            void loadSyncStatusRef.current();
          }
        }}
        trigger="click"
        placement="bottomRight"
        arrow={false}
        styles={{ container: { padding: "8px 10px" } }}
        overlayStyle={{ width: 280, maxWidth: "90vw" }}
        content={
          <div style={{ width: "100%" }}>
            {/* 顶部：标题 + 刷新 */}
            <div className="flex items-center justify-between mb-2">
              <Space size={4}>
                <SyncOutlined spin={refreshing || processingCount > 0} style={{ fontSize: 12 }} />
                <Text strong style={{ fontSize: 12 }}>{t("wiki.sync.title")}</Text>
              </Space>
              <Tooltip title={t("wiki.sync.refresh")}>
                <Button
                  type="text"
                  size="small"
                  icon={<ReloadOutlined spin={refreshing} style={{ fontSize: 11 }} />}
                  style={{ width: 20, height: 20, minWidth: 20, padding: 0 }}
                  onClick={() => {
                    void loadSyncStatus();
                  }}
                />
              </Tooltip>
            </div>

            {/* 三统计 — 一行紧凑 */}
            <div className="flex items-center justify-around mb-2" style={{ gap: 4 }}>
              <div className="text-center flex-1">
                <Text type="secondary" style={{ fontSize: 10 }}>{t("wiki.sync.pending")}</Text>
                <div style={{ fontSize: 14, fontWeight: 600 }}>
                  <PauseCircleOutlined style={{ fontSize: 11, marginRight: 2 }} />
                  {pendingCount}
                </div>
              </div>
              <div className="text-center flex-1">
                <Text type="secondary" style={{ fontSize: 10 }}>{t("wiki.sync.processing")}</Text>
                <div style={{ fontSize: 14, fontWeight: 600, color: "#1677ff" }}>
                  <SyncOutlined spin style={{ fontSize: 11, marginRight: 2 }} />
                  {processingCount}
                </div>
              </div>
              <div className="text-center flex-1">
                <Text type="secondary" style={{ fontSize: 10 }}>{t("wiki.sync.failed")}</Text>
                <div style={{ fontSize: 14, fontWeight: 600, color: failedCount > 0 ? "#ff4d4f" : undefined }}>
                  {failedCount > 0
                    ? <CloseCircleOutlined style={{ fontSize: 11, marginRight: 2 }} />
                    : <CheckCircleOutlined style={{ fontSize: 11, marginRight: 2 }} />}
                  {failedCount}
                </div>
              </div>
            </div>

            {/* 立即处理按钮 */}
            {pendingCount > 0 && (
              <Button
                type="primary"
                size="small"
                icon={<PlayCircleOutlined />}
                loading={processing}
                onClick={handleProcessQueue}
                block
                style={{ marginBottom: 8, fontSize: 11, height: 24 }}
              >
                {t("wiki.sync.processNow", { count: pendingCount })}
              </Button>
            )}

            {/* 容量 */}
            {capacityInfo && (
              <div style={{ marginBottom: 8 }}>
                <div className="flex justify-between">
                  <Text type="secondary" style={{ fontSize: 10 }}>{t("wiki.sync.capacity")}</Text>
                  <Text style={{ fontSize: 10 }}>
                    {capacityInfo.totalChunks}/{capacityInfo.maxChunks}
                  </Text>
                </div>
                <Progress
                  percent={capacityInfo.usagePercent}
                  strokeColor={capacityInfo.usagePercent > 90
                    ? "#ff4d4f"
                    : capacityInfo.usagePercent > 70
                    ? "#faad14"
                    : "#52c41a"}
                  size={["100%", 4]}
                  showInfo={false}
                />
              </div>
            )}

            {/* 队列 */}
            <div>
              <Text type="secondary" style={{ fontSize: 10 }}>{t("wiki.sync.queue")}</Text>
              {queueItems.length === 0
                ? (
                  <Empty
                    description={false}
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    style={{ padding: "4px 0", margin: 0 }}
                    imageStyle={{ height: 20 }}
                  >
                    <Text type="secondary" style={{ fontSize: 10 }}>{t("wiki.sync.emptyQueue")}</Text>
                  </Empty>
                )
                : (
                  <div style={{ maxHeight: 120, overflowY: "auto" }}>
                    {queueItems.slice(0, 10).map((item) => (
                      <div
                        key={item.id}
                        className="flex items-center justify-between"
                        style={{ padding: "2px 0" }}
                      >
                        <Space size={3}>
                          <Badge status={getStatusColor(item.status)} />
                          <Tag style={{ margin: 0, fontSize: 9, lineHeight: "14px", padding: "0 3px" }}>
                            {getEventTypeLabel(item.eventType)}
                          </Tag>
                        </Space>
                        <Space size={3}>
                          {item.retryCount > 0 && (
                            <Tag
                              color="warning"
                              style={{ fontSize: 9, margin: 0, lineHeight: "14px", padding: "0 3px" }}
                            >
                              {item.retryCount}
                            </Tag>
                          )}
                          <Text type="secondary" style={{ fontSize: 9 }}>
                            {new Date(item.createdAt * 1000).toLocaleTimeString()}
                          </Text>
                        </Space>
                      </div>
                    ))}
                    {queueItems.length > 10 && (
                      <Text type="secondary" style={{ fontSize: 9 }}>
                        +{queueItems.length - 10}
                      </Text>
                    )}
                  </div>
                )}
            </div>
          </div>
        }
      >
        <Tooltip title={t("wiki.sync.title")}>
          <Badge
            count={badgeCount}
            size="small"
            color={badgeColor === "error"
              ? "#ff4d4f"
              : badgeColor === "processing"
              ? "#1677ff"
              : badgeColor === "warning"
              ? "#faad14"
              : "#52c41a"}
          >
            <Button
              size="small"
              type="text"
              icon={
                <SyncOutlined
                  spin={refreshing || processingCount > 0}
                  style={{
                    color: failedCount > 0 ? "#ff4d4f" : undefined,
                    fontSize: 13,
                  }}
                />
              }
              style={{ width: 26, height: 26 }}
            />
          </Badge>
        </Tooltip>
      </Popover>
    );
  }

  // ========== 完整面板模式 ==========
  if (loading) {
    return (
      <Card size="small">
        <div className="flex items-center justify-center py-8">
          <Spin size="large" />
        </div>
      </Card>
    );
  }

  return (
    <SyncStatusPanel
      loading={loading}
      refreshing={refreshing}
      queueItems={queueItems}
      capacityInfo={capacityInfo}
      pendingCount={pendingCount}
      processingCount={processingCount}
      failedCount={failedCount}
      processing={processing}
      onRefresh={() => {
        void loadSyncStatus();
      }}
      onProcess={handleProcessQueue}
      getStatusColor={getStatusColor}
      getEventTypeLabel={getEventTypeLabel}
      t={t}
      standalone
    />
  );
}

// ========== 内部：完整面板内容（Popover 和 standalone 共用） ==========
interface SyncStatusPanelProps {
  loading: boolean;
  refreshing: boolean;
  queueItems: SyncQueueItem[];
  capacityInfo: CapacityInfo | null;
  pendingCount: number;
  processingCount: number;
  failedCount: number;
  processing: boolean;
  onRefresh: () => void;
  onProcess: () => void;
  getStatusColor: (s: string) => "success" | "error" | "processing" | "default" | "warning";
  getEventTypeLabel: (e: string) => string;
  t: (key: string, params?: Record<string, unknown>) => string;
  standalone?: boolean;
}

function SyncStatusPanel({
  refreshing,
  queueItems,
  capacityInfo,
  pendingCount,
  processingCount,
  failedCount,
  processing,
  onRefresh,
  onProcess,
  getStatusColor,
  getEventTypeLabel,
  t,
  standalone,
}: SyncStatusPanelProps) {
  const content = (
    <Space direction="vertical" size="middle" style={{ width: "100%" }}>
      <div>
        <Row gutter={12}>
          <Col span={8}>
            <Statistic
              title={t("wiki.sync.pending")}
              value={pendingCount}
              prefix={<PauseCircleOutlined />}
              styles={{ content: { fontSize: 18 } }}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title={t("wiki.sync.processing")}
              value={processingCount}
              prefix={<SyncOutlined spin />}
              styles={{ content: { fontSize: 18 } }}
            />
          </Col>
          <Col span={8}>
            <Statistic
              title={t("wiki.sync.failed")}
              value={failedCount}
              styles={{ content: { fontSize: 18, color: failedCount > 0 ? "#ff4d4f" : undefined } }}
              prefix={failedCount > 0 ? <CloseCircleOutlined /> : <CheckCircleOutlined />}
            />
          </Col>
        </Row>
        {pendingCount > 0 && (
          <Button
            type="primary"
            size="small"
            icon={<PlayCircleOutlined />}
            loading={processing}
            onClick={onProcess}
            block
            style={{ marginTop: 8 }}
          >
            {t("wiki.sync.processNow", { count: pendingCount })}
          </Button>
        )}
      </div>

      {capacityInfo && (
        <div>
          <div className="flex justify-between mb-1">
            <Text type="secondary" style={{ fontSize: 11 }}>{t("wiki.sync.capacity")}</Text>
            <Text style={{ fontSize: 11 }}>
              {capacityInfo.totalChunks} / {capacityInfo.maxChunks}
            </Text>
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
      )}

      <div>
        <Text type="secondary" style={{ fontSize: 11 }}>{t("wiki.sync.queue")}</Text>
        {queueItems.length === 0
          ? (
            <Empty
              description={t("wiki.sync.emptyQueue")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              style={{ padding: "12px 0", margin: 0 }}
              imageStyle={{ height: 32 }}
            />
          )
          : (
            <div style={{ maxHeight: 200, overflowY: "auto" }}>
              {queueItems.slice(0, 20).map((item) => (
                <div
                  key={item.id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "4px 0",
                    borderBottom: "1px solid rgba(0,0,0,0.04)",
                  }}
                >
                  <Space size={4}>
                    <Badge status={getStatusColor(item.status)} />
                    <Tag style={{ margin: 0, fontSize: 10 }}>{getEventTypeLabel(item.eventType)}</Tag>
                  </Space>
                  <Space size={4}>
                    {item.retryCount > 0 && (
                      <Tooltip title={t("wiki.sync.retryCount", { count: item.retryCount })}>
                        <Tag color="warning" style={{ fontSize: 10 }}>{item.retryCount}</Tag>
                      </Tooltip>
                    )}
                    <Text type="secondary" style={{ fontSize: 10 }}>
                      {new Date(item.createdAt * 1000).toLocaleTimeString()}
                    </Text>
                  </Space>
                </div>
              ))}
              {queueItems.length > 20 && (
                <Text type="secondary" style={{ fontSize: 10 }}>
                  +{queueItems.length - 20} {t("wiki.sync.more")}
                </Text>
              )}
            </div>
          )}
      </div>
    </Space>
  );

  if (standalone) {
    return (
      <Card
        size="small"
        title={
          <Space>
            <SyncOutlined spin={refreshing} />
            <span>{t("wiki.sync.title")}</span>
          </Space>
        }
        extra={
          <Tooltip title={t("wiki.sync.refresh")}>
            <Button
              type="text"
              size="small"
              icon={<ReloadOutlined spin={refreshing} />}
              onClick={onRefresh}
            />
          </Tooltip>
        }
      >
        {content}
      </Card>
    );
  }
  return content;
}
