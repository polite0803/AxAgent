// SPDX-License-Identifier: AGPL-3.0-only

import { useExecutionBridgeStore } from "@/stores";
import type { ExecutionRiskLevel, PendingExecution, TradeDirection } from "@/types";
import { ExclamationCircleOutlined, ReloadOutlined } from "@ant-design/icons";
import { Button, Card, Empty, message, Modal, Space, Table, Tag } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface PendingSignalsListProps {
  /** 自动刷新间隔（毫秒），0 表示不自动刷新 */
  autoRefreshInterval?: number;
}

// ── 辅助函数 ──

function directionTagColor(direction: TradeDirection): string {
  switch (direction) {
    case "buy":
      return "red";
    case "sell":
      return "green";
    default:
      return "default";
  }
}

// 中文风险等级 → 英文键（用于 i18n 和颜色映射）
function riskLevelToKey(level: ExecutionRiskLevel): "low" | "medium" | "high" {
  switch (level) {
    case "高":
      return "high";
    case "中":
      return "medium";
    default:
      return "low";
  }
}

function riskTagColor(level: ExecutionRiskLevel): string {
  const key = riskLevelToKey(level);
  switch (key) {
    case "high":
      return "red";
    case "medium":
      return "orange";
    default:
      return "green";
  }
}

function formatTime(timestamp: number, t: (key: string, params?: Record<string, unknown>) => string): string {
  const date = new Date(timestamp);
  const now = new Date();
  const diff = now.getTime() - timestamp;

  // 1 分钟内显示"刚刚"
  if (diff < 60_000) {
    return t("executionBridge.pending.refresh");
  }
  // 1 小时内显示分钟
  if (diff < 3_600_000) {
    return t("executionBridge.time.minutesAgo", { minutes: Math.floor(diff / 60_000) });
  }
  // 当天显示时间
  if (date.toDateString() === now.toDateString()) {
    return date.toLocaleTimeString();
  }
  return date.toLocaleString();
}

// ── 主组件 ──

export function PendingSignalsList({
  autoRefreshInterval = 30_000,
}: PendingSignalsListProps) {
  const { t } = useTranslation();
  const pendings = useExecutionBridgeStore((s) => s.pendings);
  const loading = useExecutionBridgeStore((s) => s.loading);
  const fetchPendings = useExecutionBridgeStore((s) => s.fetchPendings);
  const confirmPending = useExecutionBridgeStore((s) => s.confirmPending);
  const rejectPending = useExecutionBridgeStore((s) => s.rejectPending);
  const error = useExecutionBridgeStore((s) => s.error);

  const [confirmModalOpen, setConfirmModalOpen] = useState(false);
  const [rejectModalOpen, setRejectModalOpen] = useState(false);
  const [currentPending, setCurrentPending] = useState<PendingExecution | null>(null);
  const [confirmQuantity, setConfirmQuantity] = useState<string>("");
  const [rejectReason, setRejectReason] = useState<string>("");
  const [actionLoading, setActionLoading] = useState(false);

  // 自动刷新
  useEffect(() => {
    if (autoRefreshInterval <= 0) { return; }

    const timer = setInterval(() => {
      fetchPendings().catch(() => {});
    }, autoRefreshInterval);

    return () => clearInterval(timer);
  }, [autoRefreshInterval, fetchPendings]);

  // 显示错误消息
  useEffect(() => {
    if (error) {
      message.error(error);
    }
  }, [error]);

  // ── 事件处理 ──

  const handleConfirm = (record: PendingExecution) => {
    setCurrentPending(record);
    setConfirmQuantity(String(record.quantity || 100));
    setConfirmModalOpen(true);
  };

  const handleReject = (record: PendingExecution) => {
    setCurrentPending(record);
    setRejectReason("");
    setRejectModalOpen(true);
  };

  const submitConfirm = async () => {
    if (!currentPending) { return; }
    const quantity = Number(confirmQuantity);
    if (!quantity || quantity <= 0) {
      message.warning(t("executionBridge.confirm.quantityPlaceholder"));
      return;
    }

    setActionLoading(true);
    try {
      await confirmPending(currentPending.id, quantity);
      message.success(t("executionBridge.toast.confirmSuccess"));
      setConfirmModalOpen(false);
    } catch (e) {
      message.error(String(e));
    } finally {
      setActionLoading(false);
    }
  };

  const submitReject = async () => {
    if (!currentPending) { return; }
    if (!rejectReason.trim()) {
      message.warning(t("executionBridge.reject.reasonPlaceholder"));
      return;
    }

    setActionLoading(true);
    try {
      await rejectPending(currentPending.id, rejectReason.trim());
      message.success(t("executionBridge.toast.rejectSuccess"));
      setRejectModalOpen(false);
    } catch (e) {
      message.error(String(e));
    } finally {
      setActionLoading(false);
    }
  };

  // ── 表格列定义 ──

  const columns: ColumnsType<PendingExecution> = [
    {
      title: t("executionBridge.pending.stockCode"),
      dataIndex: "stockCode",
      key: "stockCode",
      width: 100,
    },
    {
      title: t("executionBridge.pending.stockName"),
      dataIndex: "stockName",
      key: "stockName",
      width: 120,
    },
    {
      title: t("executionBridge.pending.direction"),
      dataIndex: "direction",
      key: "direction",
      width: 80,
      render: (dir: TradeDirection) => (
        <Tag color={directionTagColor(dir)}>
          {t(`executionBridge.pending.${dir === "buy" ? "buy" : dir === "sell" ? "sell" : "hold"}`)}
        </Tag>
      ),
    },
    {
      title: t("executionBridge.pending.price"),
      dataIndex: "price",
      key: "price",
      width: 90,
      align: "right",
      render: (price: number) => price.toFixed(2),
    },
    {
      title: t("executionBridge.pending.reason"),
      dataIndex: "reason",
      key: "reason",
      ellipsis: true,
      render: (text: string) => <span title={text}>{text}</span>,
    },
    {
      title: t("executionBridge.pending.riskLevel"),
      dataIndex: "riskLevel",
      key: "riskLevel",
      width: 90,
      render: (level: ExecutionRiskLevel, record) => (
        <Space>
          <Tag color={riskTagColor(level)}>
            {t(`executionBridge.risk.${riskLevelToKey(level)}`)}
          </Tag>
          {record.riskWarning && (
            <span
              title={record.riskWarning}
              className="text-yellow-600"
            >
              <ExclamationCircleOutlined />
            </span>
          )}
        </Space>
      ),
    },
    {
      title: t("executionBridge.pending.createdAt"),
      dataIndex: "createdAt",
      key: "createdAt",
      width: 120,
      render: (timestamp: number) => formatTime(timestamp, t),
    },
    {
      title: t("executionBridge.pending.actions"),
      key: "actions",
      width: 140,
      fixed: "right",
      render: (_: unknown, record) => (
        <Space size={4}>
          <Button
            size="small"
            type="primary"
            onClick={() => handleConfirm(record)}
          >
            {t("executionBridge.confirm.submit")}
          </Button>
          <Button
            size="small"
            danger
            onClick={() => handleReject(record)}
          >
            {t("executionBridge.reject.submit")}
          </Button>
        </Space>
      ),
    },
  ];

  // ── 渲染 ──

  return (
    <>
      <Card
        title={t("executionBridge.pending.title")}
        extra={
          <Button
            size="small"
            icon={<ReloadOutlined />}
            onClick={() => fetchPendings()}
            loading={loading}
          >
            {t("executionBridge.pending.refresh")}
          </Button>
        }
        size="small"
      >
        {pendings.length === 0
          ? (
            <Empty
              description={t("executionBridge.pending.empty")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )
          : (
            <Table<PendingExecution>
              rowKey="id"
              columns={columns}
              dataSource={pendings}
              loading={loading}
              pagination={false}
              size="small"
              scroll={{ x: 900 }}
            />
          )}
      </Card>

      {/* 确认交易弹窗 */}
      <Modal
        title={t("executionBridge.confirm.title")}
        open={confirmModalOpen}
        onOk={submitConfirm}
        onCancel={() => setConfirmModalOpen(false)}
        confirmLoading={actionLoading}
        destroyOnClose
      >
        {currentPending && (
          <>
            <p className="mb-4">
              {t("executionBridge.confirm.message", {
                direction: t(
                  `executionBridge.pending.${
                    currentPending.direction === "buy" ? "buy" : currentPending.direction === "sell" ? "sell" : "hold"
                  }`,
                ),
                stockName: currentPending.stockName,
                quantity: currentPending.quantity || 100,
                price: currentPending.price.toFixed(2),
              })}
            </p>
            <div className="mb-2">
              <label className="block text-sm text-gray-600 mb-1">
                {t("executionBridge.confirm.quantityLabel")}
              </label>
              <input
                type="number"
                value={confirmQuantity}
                onChange={(e) => setConfirmQuantity(e.target.value)}
                placeholder={t("executionBridge.confirm.quantityPlaceholder")}
                className="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-blue-500"
                min={1}
              />
            </div>
          </>
        )}
      </Modal>

      {/* 驳回交易弹窗 */}
      <Modal
        title={t("executionBridge.reject.title")}
        open={rejectModalOpen}
        onOk={submitReject}
        onCancel={() => setRejectModalOpen(false)}
        confirmLoading={actionLoading}
        okButtonProps={{ danger: true }}
        destroyOnClose
      >
        {currentPending && (
          <>
            <p className="mb-4">
              {t("executionBridge.reject.message", {
                stockName: currentPending.stockName,
              })}
            </p>
            <div className="mb-2">
              <label className="block text-sm text-gray-600 mb-1">
                {t("executionBridge.reject.reasonLabel")}
              </label>
              <textarea
                value={rejectReason}
                onChange={(e) => setRejectReason(e.target.value)}
                placeholder={t("executionBridge.reject.reasonPlaceholder")}
                className="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none focus:ring-2 focus:ring-red-500 resize-none"
                rows={3}
              />
            </div>
          </>
        )}
      </Modal>
    </>
  );
}
