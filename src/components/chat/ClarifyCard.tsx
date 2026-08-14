// SPDX-License-Identifier: AGPL-3.0-only

import { useConversationStore } from "@/stores";
import { Button, Card, Space, Tag, theme, Typography } from "antd";
import { GitBranch, Loader2, Sparkles, Target } from "lucide-react";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/**
 * 认知编排澄清卡片（Clarify 分支）：路由置信度落入模糊区间（0.60 ≤ 置信度 ≤ 0.90）时，
 * 后端返回 Top2 候选能力，前端在此卡片展示候选供用户选择。选中后调用 executeClarify
 * 携带 forcedCapabilityId 二次执行，后端跳过三层路由直接分发。
 */
export function ClarifyCard() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const pendingClarification = useConversationStore(
    (s) => s.pendingClarification,
  );
  const executeClarify = useConversationStore((s) => s.executeClarify);

  const [appeared, setAppeared] = useState(false);
  const [executingId, setExecutingId] = useState<string | null>(null);

  useEffect(() => {
    if (!pendingClarification) {
      return;
    }
    const timer = setTimeout(() => setAppeared(true), 50);
    return () => clearTimeout(timer);
  }, [pendingClarification]);

  if (!pendingClarification) {
    return null;
  }

  const candidates = pendingClarification.candidates;

  const handleSelect = async (capabilityId: string) => {
    if (executingId) {
      return;
    }
    setExecutingId(capabilityId);
    try {
      await executeClarify(capabilityId);
    } finally {
      // executeClarify 完成后 pendingClarification 已清空，组件将卸载
      setExecutingId(null);
    }
  };

  const cardStyle: React.CSSProperties = {
    marginTop: 8,
    borderColor: token.colorPrimary,
    opacity: appeared ? 1 : 0,
    transform: appeared ? "translateY(0)" : "translateY(-10px)",
    transition: "box-shadow 0.3s ease-out, transform 0.3s ease-out",
  };

  return (
    <Card
      size="small"
      style={cardStyle}
      styles={{ body: { padding: "14px 16px" } }}
    >
      <Space orientation="vertical" style={{ width: "100%" }} size={12}>
        <Space size={10} align="start">
          <div
            style={{
              width: 32,
              height: 32,
              borderRadius: 8,
              backgroundColor: `${token.colorPrimary}15`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              flexShrink: 0,
            }}
          >
            <Sparkles size={18} style={{ color: token.colorPrimary }} />
          </div>
          <div style={{ flex: 1 }}>
            <Text
              strong
              style={{ fontSize: 13, display: "block", marginBottom: 4 }}
            >
              {t("cognitive.clarifyTitle")}
            </Text>
            <Text
              type="secondary"
              style={{ fontSize: 13, display: "block" }}
            >
              {t("cognitive.clarifyPrompt")}
            </Text>
          </div>
        </Space>

        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 8,
          }}
        >
          {candidates.map((candidate) => {
            const isLoading = executingId === candidate.capabilityId;
            return (
              <Button
                key={candidate.capabilityId}
                type="default"
                size="middle"
                block
                loading={isLoading}
                disabled={executingId !== null}
                onClick={() => handleSelect(candidate.capabilityId)}
                style={{
                  height: "auto",
                  padding: "10px 12px",
                  textAlign: "left",
                  borderRadius: 8,
                  justifyContent: "flex-start",
                  alignItems: "flex-start",
                }}
              >
                <Space orientation="vertical" style={{ width: "100%" }} size={6}>
                  <Space size={6} wrap>
                    {candidate.kind === "workflow"
                      ? (
                        <Tag
                          icon={<GitBranch size={12} />}
                          color="processing"
                          style={{ marginInlineEnd: 0 }}
                        >
                          {t("cognitive.candidateWorkflow")}
                        </Tag>
                      )
                      : (
                        <Tag
                          icon={<Target size={12} />}
                          color="geekblue"
                          style={{ marginInlineEnd: 0 }}
                        >
                          {t("cognitive.candidateAgent")}
                        </Tag>
                      )}
                    {candidate.domain && (
                      <Tag style={{ marginInlineEnd: 0 }}>
                        {candidate.domain}
                      </Tag>
                    )}
                    <Tag
                      style={{ marginInlineEnd: 0 }}
                      color={candidate.score >= 0.8 ? "green" : "orange"}
                    >
                      {Math.round(candidate.score * 100)}%
                    </Tag>
                  </Space>
                  <Text strong style={{ fontSize: 14, lineHeight: 1.4 }}>
                    {candidate.name}
                  </Text>
                  {candidate.description && (
                    <Text
                      type="secondary"
                      style={{
                        fontSize: 12.5,
                        lineHeight: 1.5,
                        whiteSpace: "pre-wrap",
                      }}
                    >
                      {candidate.description}
                    </Text>
                  )}
                </Space>
              </Button>
            );
          })}
        </div>

        <Space size={8} align="center" style={{ color: token.colorTextTertiary }}>
          {executingId
            ? (
              <>
                <Loader2 size={13} className="clarify-spin" />
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("cognitive.clarifyExecuting")}
                </Text>
              </>
            )
            : (
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("cognitive.clarifyHint")}
              </Text>
            )}
        </Space>
      </Space>

      <style>
        {`
        .clarify-spin {
          animation: clarify-spin 1s linear infinite;
        }
        @keyframes clarify-spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}
      </style>
    </Card>
  );
}
