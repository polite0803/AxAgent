// SPDX-License-Identifier: AGPL-3.0-only

import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import { Badge, Button, Card, Collapse, Empty, Tag, theme, Typography } from "antd";
import { Activity, ChevronDown, Clock, Play, TrendingDown, TrendingUp } from "lucide-react";
import React, { useCallback, useEffect, useState } from "react";
import type { WorkflowNode } from "../types/workflow.types";

const { Text } = Typography;

interface EvolutionTabProps {
  currentWorkflowId: string | null;
  nodes: WorkflowNode[];
}

interface EvolutionEntry {
  skill_id: string;
  version: number;
  timestamp: number;
  type?: string;
  metrics?: {
    success_rate?: number;
    avg_latency_ms?: number;
  };
  previous_metrics?: {
    success_rate?: number;
    avg_latency_ms?: number;
  };
  ab_test_won?: boolean;
}

interface ABTestResult {
  variant_a: string;
  variant_b: string;
  winner: string | null;
  confidence: number;
  metrics_a: Record<string, number>;
  metrics_b: Record<string, number>;
}

export const EvolutionTab: React.FC<EvolutionTabProps> = React.memo(({ currentWorkflowId, nodes }) => {
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
          const history = evolutionStore.getSkillEvolutionHistory(node.id) as unknown as EvolutionEntry[];
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
        const results = evolutionStore.getABTestResults(currentWorkflowId ?? "") as unknown as ABTestResult[];
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
          进化状态
        </Text>
        {nodes.length > 0 && (
          <Button
            type="primary"
            size="small"
            icon={<Play size={12} />}
            onClick={handleTriggerAll}
            style={{ fontSize: 12 }}
          >
            全部触发进化
          </Button>
        )}
      </div>

      {/* 可优化节点列表 */}
      {nodes.length === 0
        ? <Empty description="当前画布无节点" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        : !hasAnyHistory && !loading
        ? <Empty description="暂无进化记录" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        : (
          <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
            {nodes.map((node) => {
              const history = nodeHistories[node.id];
              const latest = history?.[0];
              const prev = history?.[1];

              const srDiff = latest?.metrics?.success_rate != null && prev?.metrics?.success_rate != null
                ? latest.metrics.success_rate - prev.metrics.success_rate
                : null;
              const latDiff = latest?.metrics?.avg_latency_ms != null && prev?.metrics?.avg_latency_ms != null
                ? latest.metrics.avg_latency_ms - prev.metrics.avg_latency_ms
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
                                成功率 {srDiff >= 0 ? "+" : ""}
                                {(srDiff * 100).toFixed(1)}%
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
                                延迟 {latDiff >= 0 ? "+" : ""}
                                {latDiff.toFixed(0)}ms
                              </span>
                            )}
                          </div>
                        )
                        : (
                          <Text style={{ fontSize: 11, color: token.colorTextTertiary }}>
                            无进化记录 — 可触发首次进化
                          </Text>
                        )}
                    </div>

                    {latest?.ab_test_won && (
                      <Badge status="success" text="A/B 胜出" style={{ fontSize: 10, marginRight: 6 }} />
                    )}

                    <Button
                      type="link"
                      size="small"
                      icon={<Play size={12} />}
                      onClick={() => handleTriggerSingle(node.id)}
                      style={{ fontSize: 11, flexShrink: 0 }}
                    >
                      触发进化
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
                  A/B 测试 ({abResults.length})
                </Text>
              ),
              children: abResults.length === 0
                ? (
                  <Empty
                    description="暂无活跃的 A/B 测试"
                    image={Empty.PRESENTED_IMAGE_SIMPLE}
                    style={{ padding: "8px 0" }}
                  />
                )
                : (
                  <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
                    {abResults.map((res, idx) => (
                      <Card
                        key={idx}
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
                            {res.variant_a} vs {res.variant_b}
                          </Text>
                          {res.winner && (
                            <Tag
                              color="green"
                              style={{ fontSize: 10, margin: 0, padding: "0 4px", lineHeight: "16px" }}
                            >
                              {res.winner} 胜出
                            </Tag>
                          )}
                        </div>
                        <div style={{ display: "flex", gap: 16, fontSize: 11, color: token.colorTextSecondary }}>
                          <span>置信度: {(res.confidence * 100).toFixed(1)}%</span>
                          {Object.entries(res.metrics_a).map(([k, v]) => (
                            <span key={k}>
                              {k}: A={typeof v === "number" ? v.toFixed(2) : String(v)}{" "}
                              / B={typeof res.metrics_b[k] === "number"
                                ? (res.metrics_b[k] as number).toFixed(2)
                                : String(res.metrics_b[k])}
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
