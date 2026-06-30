// SPDX-License-Identifier: AGPL-3.0-only
/* eslint-disable react-hooks/set-state-in-effect */

import { useTracerStore } from "@/stores/devtools/tracerStore";
import { Button, Card, notification, Spin, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

// ── Types ──

export interface ImprovementSuggestion {
  id: string;
  problem: string;
  suggestion: string;
  expectedImprovement: string;
}

/** @deprecated 使用 ImprovementSuggestion */
export type ImprovementSuggestionItem = ImprovementSuggestion;

// ── Component ──

interface ImprovementSuggestionProps {
  traceId: string;
}

export function ImprovementSuggestion({ traceId }: ImprovementSuggestionProps) {
  const { t } = useTranslation();
  const generateSuggestions = useTracerStore((s) => s.generateSuggestions);
  const [suggestions, setSuggestions] = useState<ImprovementSuggestion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    generateSuggestions(traceId)
      .then((result) => {
        if (!cancelled) {
          setSuggestions(result);
          setLoading(false);
        }
      })
      .catch((e: unknown) => {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : String(e));
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [traceId, generateSuggestions]);

  // ── Loading / Error states ──

  if (loading) {
    return (
      <div style={{ textAlign: "center", padding: 32 }}>
        <Spin />
        <Text type="secondary" style={{ display: "block", marginTop: 8 }}>
          {t("trace.improvement.loading", "生成改进建议...")}
        </Text>
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ textAlign: "center", padding: 32 }}>
        <Text type="danger">{t("trace.improvement.error", "生成失败")}: {error}</Text>
      </div>
    );
  }

  const handleApply = (item: ImprovementSuggestion) => {
    notification.info({
      message: t("trace.improvement.applied", "改进已应用"),
      description: item.expectedImprovement,
      placement: "bottomRight",
    });
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
      {suggestions.length === 0
        ? <Text type="secondary">{t("trace.improvement.noSuggestions", "暂无改进建议")}</Text>
        : (
          suggestions.map((item) => (
            <Card
              key={item.id}
              size="small"
              style={{ borderLeft: "3px solid #1890ff" }}
            >
              <div style={{ marginBottom: 8 }}>
                <Text type="danger" strong style={{ fontSize: 12 }}>
                  {t("trace.improvement.problem", "问题")}:
                </Text>
                <Paragraph style={{ margin: "4px 0", fontSize: 13 }}>{item.problem}</Paragraph>
              </div>
              <div style={{ marginBottom: 8 }}>
                <Text type="warning" strong style={{ fontSize: 12 }}>
                  {t("trace.improvement.suggestion", "建议")}:
                </Text>
                <Paragraph style={{ margin: "4px 0", fontSize: 13 }}>{item.suggestion}</Paragraph>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <Text type="success" style={{ fontSize: 12 }}>
                  {item.expectedImprovement}
                </Text>
                <Button type="primary" size="small" onClick={() => handleApply(item)}>
                  {t("trace.improvement.apply", "应用改进")}
                </Button>
              </div>
            </Card>
          ))
        )}
    </div>
  );
}
