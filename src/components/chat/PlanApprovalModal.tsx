// SPDX-License-Identifier: AGPL-3.0-only

import { logIpcError } from "@/lib/invoke";
import { useAgentStore } from "@/stores";
import { Button, Modal, Space, Tag, theme, Typography } from "antd";
import { CheckCircle, FileText, XCircle } from "lucide-react";
import React, { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface ParsedPlan {
  task_preview?: string;
  selected_engine?: string;
  features?: Record<string, unknown>;
  note?: string;
}

/**
 * 计划确认闸门弹窗（P0-2）
 *
 * 当 agent 判定任务复杂且开启计划确认时弹出 Modal，展示后端生成的计划草稿
 * （任务预览、自动选择的推理引擎、结构化特征）。提供"批准执行"与"拒绝"两个操作。
 */
export const PlanApprovalModal: React.FC = () => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const pendingPlan = useAgentStore((s) => s.pendingPlan);
  const approvePlan = useAgentStore((s) => s.approvePlan);
  const [loading, setLoading] = useState<string | null>(null);
  const [showDetails, setShowDetails] = useState(false);

  const parsed = useMemo<ParsedPlan | null>(() => {
    if (!pendingPlan?.plan) {
      return null;
    }
    try {
      return JSON.parse(pendingPlan.plan) as ParsedPlan;
    } catch {
      return null;
    }
  }, [pendingPlan]);

  const handleDecision = useCallback(
    async (decision: string) => {
      if (!pendingPlan) {
        return;
      }
      setLoading(decision);
      try {
        await approvePlan(pendingPlan.conversationId, decision);
      } catch (e) {
        logIpcError("PlanApprovalModal.approveError")(e);
      } finally {
        setLoading(null);
      }
    },
    [pendingPlan, approvePlan],
  );

  if (!pendingPlan) {
    return null;
  }

  const inputPreview = pendingPlan.plan.slice(0, 800);

  return (
    <Modal
      title={
        <Space size={8}>
          <FileText size={18} style={{ color: "var(--ant-color-primary)" }} />
          <span>{t("planApprovalModal.title")}</span>
        </Space>
      }
      open={!!pendingPlan}
      closable={false}
      maskClosable={false}
      width={560}
      footer={
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "flex-end",
            width: "100%",
            gap: 8,
          }}
        >
          <Button
            danger
            data-testid="plan-approval-reject"
            icon={<XCircle size={14} />}
            loading={loading === "reject"}
            onClick={() => handleDecision("reject")}
          >
            {t("planApprovalModal.reject")}
          </Button>
          <Button
            type="primary"
            data-testid="plan-approval-approve"
            icon={<CheckCircle size={14} />}
            loading={loading === "approve"}
            onClick={() => handleDecision("approve")}
          >
            {t("planApprovalModal.approve")}
          </Button>
        </div>
      }
      destroyOnHidden
    >
      <Space direction="vertical" size={16} style={{ width: "100%" }}>
        {/* 任务预览 */}
        {parsed?.task_preview && (
          <div
            style={{
              padding: "10px 14px",
              backgroundColor: token.colorPrimaryBg,
              borderRadius: 8,
              borderLeft: "3px solid var(--ant-color-primary)",
            }}
          >
            <Text strong style={{ fontSize: 14, display: "block" }}>
              {t("planApprovalModal.taskPreview")}
            </Text>
            <Text type="secondary" style={{ fontSize: 13 }}>
              {parsed.task_preview}
            </Text>
          </div>
        )}

        {/* 推理引擎 */}
        {parsed?.selected_engine && (
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("planApprovalModal.engine")}
            </Text>
            <Tag color="blue" style={{ marginLeft: 6 }}>
              {parsed.selected_engine}
            </Tag>
          </div>
        )}

        {/* 结构化特征 */}
        {parsed?.features && (
          <div>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("planApprovalModal.features")}
            </Text>
            <pre
              style={{
                margin: "6px 0 0",
                padding: 10,
                fontSize: 12,
                fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                backgroundColor: "var(--ant-color-fill-tertiary)",
                borderRadius: 6,
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                maxHeight: 160,
                overflow: "auto",
                lineHeight: 1.5,
              }}
            >
              {JSON.stringify(parsed.features, null, 2)}
            </pre>
          </div>
        )}

        {/* 原始计划 JSON */}
        <div>
          <div
            onClick={() => setShowDetails(!showDetails)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                setShowDetails(!showDetails);
              }
            }}
            style={{
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              gap: 4,
              marginBottom: showDetails ? 8 : 0,
            }}
          >
            <Text type="secondary" style={{ fontSize: 12 }}>
              {showDetails
                ? t("planApprovalModal.collapsePlan")
                : t("planApprovalModal.expandPlan")}
            </Text>
          </div>
          {showDetails && (
            <pre
              style={{
                margin: 0,
                padding: 10,
                fontSize: 12,
                fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                backgroundColor: "var(--ant-color-fill-tertiary)",
                borderRadius: 6,
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
                maxHeight: 200,
                overflow: "auto",
                lineHeight: 1.5,
              }}
            >
              {pendingPlan.plan || t("planApprovalModal.empty")}
            </pre>
          )}
          {!showDetails && inputPreview && (
            <Paragraph
              type="secondary"
              ellipsis={{ rows: 2 }}
              style={{
                fontSize: 12,
                fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                backgroundColor: "var(--ant-color-fill-tertiary)",
                padding: "6px 10px",
                borderRadius: 6,
                margin: 0,
              }}
            >
              {inputPreview}
            </Paragraph>
          )}
        </div>
      </Space>
    </Modal>
  );
};
