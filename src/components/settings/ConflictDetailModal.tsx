// SPDX-License-Identifier: AGPL-3.0-only

import type { ConflictInfo, ConflictResolutionStrategy } from "@/types";
import { CloudOutlined, DesktopOutlined, MobileOutlined, WarningOutlined } from "@ant-design/icons";
import { Button, Col, Descriptions, Divider, Modal, Row, Space, Tag, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface ConflictDetailModalProps {
  open: boolean;
  conflict: ConflictInfo | null;
  onClose: () => void;
  onResolve: (strategy: ConflictResolutionStrategy) => Promise<void>;
}

/** 冲突解决策略选项 */
const STRATEGY_OPTIONS: {
  value: ConflictResolutionStrategy;
  icon: React.ReactNode;
  color: string;
}[] = [
  {
    value: "keep_local",
    icon: <DesktopOutlined />,
    color: "blue",
  },
  {
    value: "keep_remote",
    icon: <MobileOutlined />,
    color: "orange",
  },
  {
    value: "last_write_wins",
    icon: <CloudOutlined />,
    color: "purple",
  },
  {
    value: "keep_both",
    icon: <WarningOutlined />,
    color: "gold",
  },
];

/** 冲突详情弹窗 */
export function ConflictDetailModal({
  open,
  conflict,
  onClose,
  onResolve,
}: ConflictDetailModalProps) {
  const { t } = useTranslation();
  const [resolving, setResolving] = useState(false);

  useEffect(() => {
    if (!open) {
      setResolving(false);
    }
  }, [open]);

  if (!conflict) { return null; }

  const handleResolve = async (strategy: ConflictResolutionStrategy) => {
    setResolving(true);
    try {
      await onResolve(strategy);
      onClose();
    } finally {
      setResolving(false);
    }
  };

  return (
    <Modal
      open={open}
      title={
        <Space>
          <WarningOutlined style={{ color: "#faad14" }} />
          <span>{t("deviceSync.conflictDetail")}</span>
        </Space>
      }
      onCancel={onClose}
      width={720}
      footer={[
        <Button key="close" onClick={onClose}>
          {t("common.cancel")}
        </Button>,
      ]}
    >
      <Descriptions column={2} bordered size="small">
        <Descriptions.Item label={t("deviceSync.entityType")}>
          <Tag color="blue">{conflict.entity_type}</Tag>
        </Descriptions.Item>
        <Descriptions.Item label={t("deviceSync.entityId")}>
          <Text code>{conflict.entity_id}</Text>
        </Descriptions.Item>
        <Descriptions.Item label={t("deviceSync.localVersion")}>
          {conflict.local_vector.map((v) => `${v.device_id}:${v.counter}`).join(", ")}
        </Descriptions.Item>
        <Descriptions.Item label={t("deviceSync.remoteVersion")}>
          {conflict.remote_vector.map((v) => `${v.device_id}:${v.counter}`).join(", ")}
        </Descriptions.Item>
        <Descriptions.Item label={t("deviceSync.detectedAt")} span={2}>
          {new Date(conflict.detected_at).toLocaleString()}
        </Descriptions.Item>
      </Descriptions>

      <Divider>
        <Text strong>{t("deviceSync.conflictSummary")}</Text>
      </Divider>

      <Space direction="vertical" style={{ width: "100%" }}>
        <Descriptions column={2} size="small" bordered>
          <Descriptions.Item label={t("deviceSync.localData")}>
            {conflict.local_data ? JSON.stringify(JSON.parse(conflict.local_data)).substring(0, 50) + "..." : "-"}
          </Descriptions.Item>
          <Descriptions.Item label={t("deviceSync.remoteData")}>
            {conflict.remote_data ? JSON.stringify(JSON.parse(conflict.remote_data)).substring(0, 50) + "..." : "-"}
          </Descriptions.Item>
        </Descriptions>
      </Space>

      <Divider>
        <Text strong>{t("deviceSync.resolveStrategy")}</Text>
      </Divider>

      <Paragraph type="secondary" style={{ marginBottom: 16 }}>
        {t("deviceSync.selectResolveStrategy")}
      </Paragraph>

      <Row gutter={[12, 12]}>
        {STRATEGY_OPTIONS.map((opt) => (
          <Col span={12} key={opt.value}>
            <Button
              block
              icon={opt.icon}
              loading={resolving}
              onClick={() => handleResolve(opt.value)}
            >
              <Space direction="vertical" size={4}>
                <Tag color={opt.color}>
                  {t(`deviceSync.strategy_${opt.value}`)}
                </Tag>
              </Space>
            </Button>
          </Col>
        ))}
      </Row>
    </Modal>
  );
}
