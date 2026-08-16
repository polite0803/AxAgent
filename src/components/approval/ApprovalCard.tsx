// SPDX-License-Identifier: AGPL-3.0-only

import { useApprovalStore } from "@/stores/feature/approvalStore";
import type { ApprovalRequest } from "@/types";
import { Alert, Button, Card, Space, Tag, theme, Typography } from "antd";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface ApprovalCardProps {
  approval: ApprovalRequest;
  onApproved?: (approvalId: string) => void;
  onRejected?: (approvalId: string) => void;
  /** 审批备注（由父组件传入，批准/拒绝时传递给后端） */
  note?: string;
}

export function ApprovalCard({ approval, onApproved, onRejected, note }: ApprovalCardProps) {
  const { token } = theme.useToken();
  const { t } = useTranslation();
  const { resumeApproval, cancelApproval, loading } = useApprovalStore();

  const handleApprove = async () => {
    const ok = await resumeApproval(approval.executionId, approval.id, "approved", undefined, note);
    if (ok) { onApproved?.(approval.id); }
  };

  const handleReject = async () => {
    const ok = await resumeApproval(approval.executionId, approval.id, "rejected", undefined, note);
    if (ok) { onRejected?.(approval.id); }
  };

  const handleCancel = async () => {
    await cancelApproval(approval.executionId, approval.id);
  };

  const statusColor = approval.status === "pending"
    ? "orange"
    : approval.status === "approved"
    ? "green"
    : approval.status === "rejected"
    ? "red"
    : "default";

  const expiresSoon = approval.status === "pending" && approval.expiresAt * 1000 < Date.now() + 5 * 60 * 1000;

  return (
    <Card
      size="small"
      className="mb-2"
      styles={{ body: { padding: token.paddingSM } }}
    >
      <Space direction="vertical" size={4} className="w-full">
        <div className="flex items-center justify-between">
          <Tag color={statusColor}>{t(`approval.status.${approval.status}`)}</Tag>
          <Text type="secondary" className="text-xs">
            {new Date(approval.createdAt * 1000).toLocaleString()}
          </Text>
        </div>

        <Text strong>{approval.title}</Text>

        <Paragraph type="secondary" className="mb-2">
          {approval.message}
        </Paragraph>

        {expiresSoon && (
          <Alert
            type="warning"
            showIcon
            message={t("approval.expiresSoon")}
            description={t("approval.expiresAt")}
            style={{ marginBottom: 8, fontSize: 12 }}
          />
        )}

        {approval.status === "pending" && (
          <Space>
            <Button
              type="primary"
              size="small"
              loading={loading}
              onClick={handleApprove}
            >
              {t("approval.approve")}
            </Button>
            <Button
              danger
              size="small"
              loading={loading}
              onClick={handleReject}
            >
              {t("approval.reject")}
            </Button>
            <Button
              size="small"
              onClick={handleCancel}
            >
              {t("approval.cancel")}
            </Button>
          </Space>
        )}
      </Space>
    </Card>
  );
}
