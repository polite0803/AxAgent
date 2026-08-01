// SPDX-License-Identifier: AGPL-3.0-only

import { listen } from "@/lib/invoke";
import { useAgentPanelStore } from "@/stores/shared/agentPanelStore";
import { ExclamationCircleOutlined } from "@ant-design/icons";
import { Alert, Button, Modal, Space, Tag, Typography } from "antd";
import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

/**
 * PermissionGate — Agent 写操作前端确认机制
 *
 * 当 Agent 计划执行写操作（文件修改、设置变更、数据删除等）时，
 * 后端通过 `agent-permission-required` 事件通知前端，此组件弹出确认对话框，
 * 用户批准后通过 `agent-permission-response` 事件回传结果。
 *
 * 设计原则：
 * - 非阻塞：不阻断 Agent 主流程，而是暂停执行等待响应
 * - 可超时：超时未响应视为拒绝
 * - 可绕过：用户可选择"本次不再询问"
 */
export function PermissionGate() {
  const { t } = useTranslation();
  const pendingConfirmations = useAgentPanelStore((s) => s.pendingConfirmations);
  const addPendingConfirmation = useAgentPanelStore((s) => s.addPendingConfirmation);
  const resolveConfirmation = useAgentPanelStore((s) => s.resolveConfirmation);
  const firstPending = pendingConfirmations[0];

  // 监听后端的权限请求事件
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let mounted = true;

    const initListener = async () => {
      try {
        unlisten = await listen<{
          requestId: string;
          toolName: string;
          description: string;
        }>("agent-permission-required", (event) => {
          const payload = event.payload;
          if (payload?.requestId && payload.toolName) {
            addPendingConfirmation({
              id: payload.requestId,
              toolName: payload.toolName,
              description: payload.description,
              expiresAt: Date.now() + 30_000, // 30秒超时
            });
          }
        });

        if (!mounted) {
          unlisten?.();
          unlisten = null;
        }
      } catch (err) {
        console.error("[PermissionGate] Failed to initialize event listener:", err);
      }
    };

    initListener();

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [addPendingConfirmation]);

  // 自动超时处理
  useEffect(() => {
    if (!firstPending?.expiresAt) { return; }
    const timeout = setTimeout(() => {
      resolveConfirmation(firstPending.id, false);
    }, Math.max(0, firstPending.expiresAt - Date.now()));
    return () => clearTimeout(timeout);
  }, [firstPending?.id, firstPending?.expiresAt, resolveConfirmation]);

  const handleApprove = () => {
    if (!firstPending) { return; }
    resolveConfirmation(firstPending.id, true);
  };

  const handleReject = () => {
    if (!firstPending) { return; }
    resolveConfirmation(firstPending.id, false);
  };

  const handleApproveAndBypass = () => {
    if (!firstPending) { return; }
    resolveConfirmation(firstPending.id, true, true);
  };

  const confirmationTips = useMemo(
    () => t("agent.permissionGate.tips"),
    [t],
  );

  if (!firstPending) { return null; }

  return (
    <Modal
      open={!!firstPending}
      title={
        <Space>
          <ExclamationCircleOutlined style={{ color: "#faad14" }} />
          <span>{t("agent.permissionGate.title")}</span>
          <Tag color="orange">{firstPending.toolName}</Tag>
        </Space>
      }
      onOk={handleApprove}
      onCancel={handleReject}
      okText={t("agent.permissionGate.approve")}
      cancelText={t("agent.permissionGate.reject")}
      footer={[
        <Button key="reject" danger onClick={handleReject}>
          {t("agent.permissionGate.reject")}
        </Button>,
        firstPending.allowBypass
          ? (
            <Button key="bypass" onClick={handleApproveAndBypass}>
              {t("agent.permissionGate.approveAndBypass")}
            </Button>
          )
          : null,
        <Button key="approve" type="primary" onClick={handleApprove}>
          {t("agent.permissionGate.approve")}
        </Button>,
      ].filter(Boolean)}
    >
      <Alert
        type="warning"
        showIcon
        message={confirmationTips}
        style={{ marginBottom: 16 }}
      />
      <Paragraph>
        <Text strong>{t("agent.permissionGate.description")}：</Text>
      </Paragraph>
      <Paragraph>{firstPending.description}</Paragraph>
      {firstPending.paramsSummary && (
        <>
          <Paragraph>
            <Text strong>{t("agent.permissionGate.params")}：</Text>
          </Paragraph>
          <pre
            style={{
              background: "var(--ant-color-fill-secondary)",
              borderRadius: 8,
              padding: 12,
              maxHeight: 200,
              overflow: "auto",
              fontSize: 12,
            }}
          >
            {firstPending.paramsSummary}
          </pre>
        </>
      )}
    </Modal>
  );
}
