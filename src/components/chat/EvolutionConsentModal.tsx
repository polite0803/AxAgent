// SPDX-License-Identifier: AGPL-3.0-only

import { logIpcError } from "@/lib/invoke";
import { useEvolutionStore } from "@/stores";
import type { CapabilityGapProposal, CapabilityGapType } from "@/types";
import { Button, Modal, Space, Tag, Tooltip, Typography } from "antd";
import { AlertTriangle, CheckCircle, Lightbulb, Shield, ShieldOff, Sparkles, XCircle } from "lucide-react";
import React, { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const GAP_TYPE_CONFIG: Record<
  CapabilityGapType,
  { color: string; bg: string; icon: React.ReactNode; labelKey: string }
> = {
  guard_rule: {
    color: "red",
    bg: "var(--ant-color-error-bg)",
    icon: <ShieldOff size={14} />,
    labelKey: "evolutionConsentModal.gapType.guardRule",
  },
  exempt_authorize: {
    color: "orange",
    bg: "var(--ant-color-warning-bg)",
    icon: <Shield size={14} />,
    labelKey: "evolutionConsentModal.gapType.exemptAuthorize",
  },
  capability_missing: {
    color: "blue",
    bg: "var(--ant-color-primary-bg)",
    icon: <Lightbulb size={14} />,
    labelKey: "evolutionConsentModal.gapType.capabilityMissing",
  },
  skill_evolution: {
    color: "purple",
    bg: "var(--ant-purple-1, #f3e8ff)",
    icon: <Sparkles size={14} />,
    labelKey: "evolutionConsentModal.gapType.skillEvolution",
  },
};

/**
 * 能力补齐/进化改进提议同意弹窗（T0.13）
 *
 * 后端三触发点（安全拦截 / NO_CANDIDATE / Clarify 兜底无候选）生成提议后，
 * 经 `evolution-consent-request` 事件下发，本组件弹窗展示提议内容。
 * 用户同意则调用 `capability_gap_consent` 命令回传，后端执行补齐；
 * 拒绝则回传 false，后端保持原安全行为。
 */
export const EvolutionConsentModal: React.FC = () => {
  const { t } = useTranslation();
  const pendingConsent = useEvolutionStore((s) => s.pendingConsent);
  const respondConsent = useEvolutionStore((s) => s.respondConsent);
  const [loading, setLoading] = useState<string | null>(null);
  const [showDetails, setShowDetails] = useState(false);

  // 取 pending 列表中的第一个
  const entries = useMemo(() => Object.entries(pendingConsent), [pendingConsent]);
  const currentEntry = entries.length > 0 ? entries[0] : null;
  const [proposalId, proposal] = currentEntry ?? [null, null as CapabilityGapProposal | null];
  const visible = entries.length > 0 && proposal != null;

  const gapCfg = proposal ? GAP_TYPE_CONFIG[proposal.gapType] ?? GAP_TYPE_CONFIG.capability_missing : null;

  const handleDecision = useCallback(
    async (approved: boolean) => {
      if (!proposalId) {
        return;
      }
      setLoading(approved ? "approve" : "reject");
      try {
        await respondConsent(proposalId, approved);
      } catch (e) {
        logIpcError("EvolutionConsentModal.respondConsent")(e);
      } finally {
        setLoading(null);
      }
    },
    [proposalId, respondConsent],
  );

  if (!visible || !proposal) {
    return null;
  }

  return (
    <Modal
      title={
        <Space size={8}>
          <AlertTriangle size={18} style={{ color: "var(--ant-color-warning)" }} />
          <span id="evolution-consent-modal-title">{t("evolutionConsentModal.title")}</span>
          {gapCfg && (
            <Tag
              color={gapCfg.color}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                margin: 0,
              }}
            >
              {gapCfg.icon}
              {t(gapCfg.labelKey)}
            </Tag>
          )}
        </Space>
      }
      open={visible}
      closable={false}
      mask={{ closable: false }}
      width={580}
      aria-labelledby="evolution-consent-modal-title"
      footer={
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            width: "100%",
          }}
        >
          <div style={{ fontSize: 12, color: "var(--ant-color-text-secondary)" }}>
            {entries.length > 1
              ? t("evolutionConsentModal.pendingCount", {
                count: String(entries.length),
                current: "1",
              })
              : t("evolutionConsentModal.pendingOne")}
          </div>
          <Space size={8}>
            <Button
              danger
              icon={<XCircle size={14} />}
              loading={loading === "reject"}
              onClick={() => handleDecision(false)}
            >
              {t("evolutionConsentModal.reject")}
            </Button>
            <Button
              type="primary"
              icon={<CheckCircle size={14} />}
              loading={loading === "approve"}
              onClick={() => handleDecision(true)}
            >
              {t("evolutionConsentModal.approve")}
            </Button>
          </Space>
        </div>
      }
      onCancel={() => handleDecision(false)}
      destroyOnHidden
    >
      <Space orientation="vertical" size={16} style={{ width: "100%" }}>
        {/* 提议标题和原因 */}
        <div
          style={{
            padding: "10px 14px",
            backgroundColor: gapCfg?.bg ?? "var(--ant-color-primary-bg)",
            borderRadius: 8,
            borderLeft: "3px solid var(--ant-color-primary)",
          }}
        >
          <Text strong style={{ fontSize: 14, display: "block" }}>
            {proposal.title}
          </Text>
          <Text type="secondary" style={{ fontSize: 13, display: "block", marginTop: 4 }}>
            {proposal.reason}
          </Text>
        </div>

        {/* 具体改动 */}
        <div>
          <Text strong style={{ fontSize: 13, display: "block", marginBottom: 4 }}>
            {t("evolutionConsentModal.proposal")}
          </Text>
          <Text type="secondary" style={{ fontSize: 13 }}>
            {proposal.proposal}
          </Text>
        </div>

        {/* 影响范围和回滚方式 */}
        <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
          <Tooltip title={proposal.impact}>
            <div
              style={{
                flex: 1,
                minWidth: 200,
                padding: "8px 12px",
                backgroundColor: "var(--ant-color-fill-tertiary)",
                borderRadius: 6,
              }}
            >
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("evolutionConsentModal.impact")}
              </Text>
              <Text style={{ fontSize: 13, display: "block", marginTop: 2 }}>
                {proposal.impact}
              </Text>
            </div>
          </Tooltip>
          <Tooltip title={proposal.rollback}>
            <div
              style={{
                flex: 1,
                minWidth: 200,
                padding: "8px 12px",
                backgroundColor: "var(--ant-color-fill-tertiary)",
                borderRadius: 6,
              }}
            >
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("evolutionConsentModal.rollback")}
              </Text>
              <Text style={{ fontSize: 13, display: "block", marginTop: 2 }}>
                {proposal.rollback}
              </Text>
            </div>
          </Tooltip>
        </div>

        {/* 原始提议 JSON 详情 */}
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
                ? t("evolutionConsentModal.collapseDetails")
                : t("evolutionConsentModal.expandDetails")}
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
              {JSON.stringify(proposal, null, 2)}
            </pre>
          )}
          {!showDetails && (
            <Text
              type="secondary"
              ellipsis
              style={{
                fontSize: 12,
                fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                backgroundColor: "var(--ant-color-fill-tertiary)",
                padding: "6px 10px",
                borderRadius: 6,
                display: "block",
                maxWidth: "100%",
              }}
            >
              {JSON.stringify(proposal).slice(0, 200)}
            </Text>
          )}
        </div>
      </Space>
    </Modal>
  );
};
