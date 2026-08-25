// SPDX-License-Identifier: AGPL-3.0-only

import { logIpcError } from "@/lib/invoke";
import { useEvolutionStore } from "@/stores";
import type { CapabilityGapProposal, CapabilityGapType } from "@/types";
import { Badge, Button, Popover, Space, Tag, Typography } from "antd";
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
 * 能力缺口通知徽章（非阻塞式）
 *
 * 认知编排器触发能力补齐时不再即时弹窗，而是：
 * 1. 后端静默存储缺口，emit 事件通知前端
 * 2. 本组件显示一个带计数的徽章（头部工具栏）
 * 3. 用户点击后展开 Popover 查看详情并手动处理
 *
 * 与旧 EvolutionConsentModal 的区别：
 * - 旧：auto-popup Modal 阻塞请求流程
 * - 新：badge + Popover，不阻塞，用户主动处理
 */
export const EvolutionConsentModal: React.FC = () => {
  const { t } = useTranslation();
  const pendingConsent = useEvolutionStore((s) => s.pendingConsent);
  const respondConsent = useEvolutionStore((s) => s.respondConsent);
  const [loading, setLoading] = useState<string | null>(null);
  const [showDetails, setShowDetails] = useState(false);

  const entries = useMemo(() => Object.entries(pendingConsent), [pendingConsent]);
  const currentEntry = entries.length > 0 ? entries[0] : null;
  const [proposalId, proposal] = currentEntry ?? [null, null as CapabilityGapProposal | null];

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

  const content = entries.length === 0
    ? (
      <div style={{ padding: "12px 16px", color: "var(--ant-color-text-secondary)" }}>
        {t("evolutionConsentModal.noPending")}
      </div>
    )
    : (
      <Space orientation="vertical" size={12} style={{ width: 380 }}>
        {entries.map(([id, p]) => {
          const cfg = GAP_TYPE_CONFIG[p.gapType] ?? GAP_TYPE_CONFIG.capability_missing;
          return (
            <div
              key={id}
              style={{
                padding: "10px 14px",
                backgroundColor: cfg.bg,
                borderRadius: 8,
                borderLeft: "3px solid var(--ant-color-primary)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                <Tag color={cfg.color} style={{ margin: 0 }}>
                  {cfg.icon}
                  {t(cfg.labelKey)}
                </Tag>
                <Text strong style={{ fontSize: 13 }}>
                  {p.title}
                </Text>
              </div>
              <Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 6 }}>
                {p.reason}
              </Text>
              {id === proposalId && (
                <>
                  <div style={{ display: "flex", gap: 8 }}>
                    <Button
                      size="small"
                      danger
                      icon={<XCircle size={12} />}
                      loading={loading === "reject"}
                      onClick={() => handleDecision(false)}
                    >
                      {t("evolutionConsentModal.reject")}
                    </Button>
                    <Button
                      size="small"
                      type="primary"
                      icon={<CheckCircle size={12} />}
                      loading={loading === "approve"}
                      onClick={() => handleDecision(true)}
                    >
                      {t("evolutionConsentModal.approve")}
                    </Button>
                  </div>
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
                      marginTop: 8,
                    }}
                  >
                    <Text type="secondary" style={{ fontSize: 11 }}>
                      {showDetails
                        ? t("evolutionConsentModal.collapseDetails")
                        : t("evolutionConsentModal.expandDetails")}
                    </Text>
                  </div>
                  {showDetails && (
                    <pre
                      style={{
                        margin: "4px 0 0",
                        padding: 8,
                        fontSize: 11,
                        fontFamily: "var(--font-mono, 'JetBrains Mono', ui-monospace, monospace)",
                        backgroundColor: "var(--ant-color-fill-tertiary)",
                        borderRadius: 6,
                        whiteSpace: "pre-wrap",
                        wordBreak: "break-all",
                        maxHeight: 150,
                        overflow: "auto",
                      }}
                    >
                      {JSON.stringify(proposal, null, 2)}
                    </pre>
                  )}
                </>
              )}
            </div>
          );
        })}
        {entries.length > 1 && (
          <Text type="secondary" style={{ fontSize: 11, textAlign: "center", display: "block" }}>
            {t("evolutionConsentModal.pendingCount", {
              count: String(entries.length),
              current: "1",
            })}
          </Text>
        )}
      </Space>
    );

  return (
    <Popover
      content={content}
      trigger="click"
      placement="bottomRight"
      arrow={false}
    >
      <Badge
        count={entries.length}
        offset={[-4, 4]}
        style={{ cursor: entries.length > 0 ? "pointer" : "default" }}
      >
        <AlertTriangle
          size={18}
          style={{
            color: entries.length > 0
              ? "var(--ant-color-warning)"
              : "var(--ant-color-text-quaternary)",
          }}
        />
      </Badge>
    </Popover>
  );
};
