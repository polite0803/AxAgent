// SPDX-License-Identifier: AGPL-3.0-only

import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import { Badge, Button, Card, Collapse, Empty, Tag, theme, Typography } from "antd";
import { Activity, ChevronDown, Clock, Play, TrendingDown, TrendingUp } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowNode } from "../types/workflow.types";

const { Text } = Typography;

interface EvolutionTabProps {
  currentWorkflowId: string | null;
  nodes: WorkflowNode[];
}

interface EvolutionEntry {
  skillId: string;
  version: number;
  timestamp: number;
  type?: string;
  metrics?: {
    successRate?: number;
    avgLatencyMs?: number;
  };
  previousMetrics?: {
    successRate?: number;
    avgLatencyMs?: number;
  };
  abTestWon?: boolean;
}

interface ABTestResult {
  variantA: string;
  variantB: string;
  winner: string | null;
  confidence: number;
  metricsA: Record<string, number>;
  metricsB: Record<string, number>;
}

export const EvolutionTab: React.FC<EvolutionTabProps> = React.memo(({ currentWorkflowId, nodes }) => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const evolutionStore = useEvolutionStore();

  const [nodeHistories, setNodeHistories] = useState<Record<string, EvolutionEntry[]>>({});
  const [abResults, setAbResults] = useState<ABTestResult[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);

    try {
      // fetch evolution history for each node
      const histories: Record<string, EvolutionEntry[]> = {};
      for (const node of nodes) {
        try {
          const history = evolutionStore.getSkillEvolutionHistory(node.id) as unknown as EvolutionEntry[]; // SAFE: store method returns IPC data; runtime shape matches EvolutionEntry[]
          if (history && history.length > 0) {
            histories[node.id] = history;
          }
        } catch {
          // node has no history — skip
        }
      }
      if (!cancelled) { setNodeHistories(histories); }

      // fetch A/B test results
      try {
        const results = evolutionStore.getABTestResults(currentWorkflowId ?? "") as unknown as ABTestResult[]; // SAFE: store method returns IPC data; runtime shape matches ABTestResult[]
        if (!cancelled && results) { setAbResults(results); }
      } catch {
        // no AB results
      }
    } finally {
      if (!cancelled) { setLoading(false); }
    }

    return () => {
      cancelled = true;
    };
  }, [currentWorkflowId, nodes, evolutionStore]);

  const handleTriggerAll = useCallback(() => {
    for (const node of nodes) {
      try {
        evolutionStore.triggerSkillEvolution(node.id);
      } catch {
        // ignore individual failures
      }
    }
  }, [nodes, evolutionStore]);

  const handleTriggerSingle = useCallback((nodeId: string) => {
    try {
      evolutionStore.triggerSkillEvolution(nodeId);
    } catch {
      // ignore
    }
  }, [evolutionStore]);

  const formatTime = useCallback((ts: number): string => {
    try {
      return new Date(ts * 1000).toLocaleString("zh-CN", {
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return String(ts);
    }
  }, []);

  const hasAnyHistory = Object.keys(nodeHistories).length > 0;

  return (
    <div style={{ height: "100%", overflowY: "auto", padding: "12px" }}>
      {/* 顶部标题 + 全部触发按钮 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 12,
        }}
      >
        <Text strong style={{ fontSize: 13, color: token.colorText }}>
          {t("workflow.evolution.title")}
        </Text>
        {nodes.length > 0 && (
          <Button
            type="primary"
            size="small"
            icon={<Play size={12} />}
            onClick={handleTriggerAll}
            style={{ fontSize: 12 }}
          >
            {t("workflow.evolution.triggerAll")}
          </Button>
        )}
      </div>

      {/* 可优化节点列表 */}
      {nodes.length === 0
        ? <Empty description={t("workflow.evolution.noNodes")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
        : !hasAnyHistory && !loading
        ? <Empty description={t("workflow.evolution.noHistory")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
        : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {nodes.map((node) => {
              const history = nodeHistories[node.id];
              const latest = history?.[0];
              const prev = history?.[1];

              const srDiff = latest?.metrics?.successRate != null && prev?.metrics?.successRate != null
                ? latest.metrics.successRate - prev.metrics.successRate
                : null;
              const latDiff = latest?.metrics?.avgLatencyMs != null && prev?.metrics?.avgLatencyMs != null
                ? latest.metrics.avgLatencyMs - prev.metrics.avgLatencyMs
                : null;

              return (
                <Card
                  key={node.id}
                  size="small"
                  style={{
                    background: token.colorBgContainer,
                    border: `1px solid ${token.colorBorderSecondary}`,
                  }}
                  styles={{ body: { padding: "8px 10px" } }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 4 }}>
                        <Text
                          style={{
                            fontSize: 13,
                            fontWeight: 500,
                            color: token.colorText,
                            maxWidth: 140,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                        >
                          {node.title || node.id}
                        </Text>
                        <Tag
                          color="purple"
                          style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
                        >
                          {node.type}
                        </Tag>
                      </div>

                      {latest
                        ? (
                          <div
                            style={{
                              display: "flex",
                              alignItems: "center",
                              gap: 10,
                              fontSize: 11,
                              color: token.colorTextSecondary,
                            }}
                          >
                            <span style={{ display: "flex", alignItems: "center", gap: 2 }}>
                              <Activity size={10} />
                              v{latest.version}
                            </span>
                            <span style={{ display: "flex", alignItems: "center", gap: 2 }}>
                              <Clock size={10} />
                              {formatTime(latest.timestamp)}
                            </span>
                            {srDiff != null && (
                              <span
                                style={{
                                  display: "flex",
                                  alignItems: "center",
                                  gap: 2,
                                  color: srDiff >= 0 ? token.colorSuccess : token.colorError,
                                }}
                              >
                                {srDiff >= 0 ? <TrendingUp size={10} /> : <TrendingDown size={10} />}
                                {t("workflow.evolution.successRate", {
                                  diff: `${srDiff >= 0 ? "+" : ""}${(srDiff * 100).toFixed(1)}%`,
                                })}
                              </span>
                            )}
                            {latDiff != null && (
                              <span
                                style={{
                                  display: "flex",
                                  alignItems: "center",
                                  gap: 2,
                                  color: latDiff <= 0 ? token.colorSuccess : token.colorError,
                                }}
                              >
                                {latDiff <= 0 ? <TrendingDown size={10} /> : <TrendingUp size={10} />}
                                {t("workflow.evolution.latency", {
                                  diff: `${latDiff >= 0 ? "+" : ""}${latDiff.toFixed(0)}ms`,
                                })}
                              </span>
                            )}
                          </div>
                        )
                        : (
                          <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>
                            {t("workflow.evolution.noEvolutionRecord")}
                          </Text>
                        )}
                    </div>

                    {latest?.abTestWon && (
                      <Badge
                        status="success"
                        text={t("workflow.evolution.abTestWon")}
                        style={{ fontSize: 10, marginRight: 6 }}
                      />
                    )}

                    <Button
                      type="link"
                      size="small"
                      icon={<Play size={12} />}
                      onClick={() => handleTriggerSingle(node.id)}
                      style={{ fontSize: 11, flexShrink: 0 }}
                    >
                      {t("workflow.evolution.triggerEvolution")}
                    </Button>
                  </div>
                </Card>
              );
            })}
          </div>
        )}

      {/* A/B 测试区 */}
      <div style={{ marginTop: 16 }}>
        <Collapse
          ghost
          size="small"
          expandIcon={({ isActive }) => (
            <ChevronDown size={12} style={{ transform: isActive ? "rotate(180deg)" : undefined, transition: "0.2s" }} />
          )}
          items={[
            {
              key: "ab-test",
              label: (
                <Text strong style={{ fontSize: 12, color: token.colorText }}>
                  {t("workflow.evolution.abTestTitle", { count: abResults.length })}
                </Text>
              ),
              children: abResults.length === 0
                ? (
                  <Empty
                    description={t("workflow.evolution.noActiveABTest")}
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    style={{ padding: "8px 0" }}
                  />
                )
                : (
                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {abResults.map((res) => (
                      <Card
                        key={`${res.variantA}-${res.variantB}`}
                        size="small"
                        style={{ background: token.colorFillTertiary, border: "none" }}
                        styles={{ body: { padding: "8px 10px" } }}
                      >
                        <div
                          style={{
                            display: "flex",
                            justifyContent: "space-between",
                            alignItems: "center",
                            marginBottom: 4,
                          }}
                        >
                          <Text style={{ fontSize: 12, color: token.colorText }}>
                            {res.variantA} vs {res.variantB}
                          </Text>
                          {res.winner && (
                            <Tag
                              color="green"
                              style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
                            >
                              {t("workflow.evolution.winner", { winner: res.winner })}
                            </Tag>
                          )}
                        </div>
                        <div style={{ display: "flex", gap: 16, fontSize: 11, color: token.colorTextSecondary }}>
                          <span>
                            {t("workflow.evolution.confidence", { value: (res.confidence * 100).toFixed(1) })}
                          </span>
                          {Object.entries(res.metricsA).map(([k, v]) => (
                            <span key={k}>
                              {k}: A={typeof v === "number" ? v.toFixed(2) : String(v)}{" "}
                              / B={typeof res.metricsB[k] === "number"
                                ? (res.metricsB[k] as number).toFixed(2)
                                : String(res.metricsB[k])}
                            </span>
                          ))}
                        </div>
                      </Card>
                    ))}
                  </div>
                ),
            },
          ]}
        />
      </div>
    </div>
  );
});
