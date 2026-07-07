// SPDX-License-Identifier: AGPL-3.0-only

// 人工审批（HITL）面板 — 显示待审批工作流列表，支持 approve / reject 操作

import { useApprovalStore } from "@/stores";
import type { ApprovalRequest } from "@/types";
import { Badge, Button, Empty, Input, Modal, Space, Spin, Table, Tag, theme, Typography } from "antd";
import { CheckCircle, Clock, Eye, XCircle } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

export function ApprovalPanel() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const pendingApprovals = useApprovalStore((s) => s.pendingApprovals);
  const loading = useApprovalStore((s) => s.loading);
  const panelOpen = useApprovalStore((s) => s.panelOpen);
  const setPanelOpen = useApprovalStore((s) => s.setPanelOpen);
  const fetchPendingApprovals = useApprovalStore((s) => s.fetchPendingApprovals);
  const resumeApproval = useApprovalStore((s) => s.resumeApproval);
  const cancelApproval = useApprovalStore((s) => s.cancelApproval);

  const [actionLoading, setActionLoading] = useState<string | null>(null);
  const [note, setNote] = useState("");

  // 打开时刷新
  useEffect(() => {
    if (panelOpen) {
      fetchPendingApprovals();
    }
  }, [panelOpen, fetchPendingApprovals]);

  const handleApprove = useCallback(
    async (item: ApprovalRequest) => {
      setActionLoading(`approve-${item.id}`);
      await resumeApproval(item.execution_id, item.id, "approved", undefined, note || undefined);
      setActionLoading(null);
      setNote("");
    },
    [resumeApproval, note],
  );

  const handleReject = useCallback(
    async (item: ApprovalRequest) => {
      setActionLoading(`reject-${item.id}`);
      await resumeApproval(item.execution_id, item.id, "rejected", undefined, note || undefined);
      setActionLoading(null);
      setNote("");
    },
    [resumeApproval, note],
  );

  const handleCancel = useCallback(
    async (item: ApprovalRequest) => {
      setActionLoading(`cancel-${item.id}`);
      await cancelApproval(item.execution_id, item.id);
      setActionLoading(null);
    },
    [cancelApproval],
  );

  const columns = useMemo(
    () => [
      {
        title: t("approval.message"),
        dataIndex: "message" as const,
        key: "message",
        width: 240,
        render: (_: unknown, record: ApprovalRequest) => (
          <div>
            <Text strong style={{ fontSize: 13 }}>{record.message}</Text>
            <div>
              <Text type="secondary" style={{ fontSize: 11 }}>
                {t("approval.workflowId")}: {record.workflow_id?.slice(0, 8)}...
              </Text>
            </div>
          </div>
        ),
      },
      {
        title: t("approval.status"),
        dataIndex: "status" as const,
        key: "status",
        width: 100,
        render: (_: unknown, record: ApprovalRequest) => {
          const color = record.status === "pending"
            ? "processing"
            : record.status === "approved"
            ? "success"
            : record.status === "rejected"
            ? "error"
            : "default";
          return <Tag color={color}>{record.status}</Tag>;
        },
      },
      {
        title: t("approval.expiresAt"),
        dataIndex: "expires_at" as const,
        key: "expires_at",
        width: 140,
        render: (_: unknown, record: ApprovalRequest) => {
          const expired = record.expires_at * 1000 < Date.now();
          return (
            <Space>
              <Clock size={14} color={expired ? token.colorError : token.colorTextSecondary} />
              <Text type={expired ? "danger" : "secondary"} style={{ fontSize: 12 }}>
                {new Date(record.expires_at * 1000).toLocaleString()}
              </Text>
            </Space>
          );
        },
      },
      {
        title: t("approval.actions"),
        key: "actions",
        width: 200,
        render: (_: unknown, record: ApprovalRequest) => {
          if (record.status !== "pending") {
            return (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {record.decision ? `${t("approval.decision")}: ${record.decision}` : record.status}
              </Text>
            );
          }
          return (
            <Space size="small">
              <Button
                type="primary"
                size="small"
                icon={<CheckCircle size={14} />}
                loading={actionLoading === `approve-${record.id}`}
                onClick={() => handleApprove(record)}
              >
                {t("approval.approve")}
              </Button>
              <Button
                danger
                size="small"
                icon={<XCircle size={14} />}
                loading={actionLoading === `reject-${record.id}`}
                onClick={() => handleReject(record)}
              >
                {t("approval.reject")}
              </Button>
              <Button
                size="small"
                loading={actionLoading === `cancel-${record.id}`}
                onClick={() => handleCancel(record)}
              >
                {t("approval.cancel")}
              </Button>
            </Space>
          );
        },
      },
    ],
    [t, token, actionLoading, handleApprove, handleReject, handleCancel],
  );

  return (
    <Modal
      title={
        <Space>
          <Eye size={18} />
          <span>{t("approval.panelTitle")}</span>
          {pendingApprovals.length > 0 && <Badge count={pendingApprovals.length} size="small" />}
        </Space>
      }
      open={panelOpen}
      onCancel={() => setPanelOpen(false)}
      width={800}
      footer={null}
      destroyOnClose
    >
      {/* 备注输入 */}
      <Input.TextArea
        placeholder={t("approval.notePlaceholder")}
        value={note}
        onChange={(e) => setNote(e.target.value)}
        rows={2}
        style={{ marginBottom: 12, fontSize: 12 }}
      />

      <Spin spinning={loading}>
        {pendingApprovals.length === 0 && !loading
          ? <Empty description={t("approval.noPending")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
          : (
            <Table
              dataSource={pendingApprovals}
              columns={columns}
              rowKey="id"
              pagination={false}
              size="small"
              scroll={{ y: 400 }}
            />
          )}
      </Spin>
    </Modal>
  );
}
