// SPDX-License-Identifier: AGPL-3.0-only

import { GraphData, GraphView, type GraphViewHandle } from "@/components/wiki/GraphView";
import { showBackendError } from "@/lib/errorI18n";
import { invoke } from "@/lib/invoke";
import { adaptMemoryToGraphData, computeBacklinkCounts, type MemoryGraphResponse } from "@/lib/memoryGraphAdapter";
import { App, Empty, Spin, theme } from "antd";
import { RefreshCw, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

export interface MemoryGraphViewProps {
  onClose: () => void;
}

export function MemoryGraphView({ onClose }: MemoryGraphViewProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message } = App.useApp();

  const [graphData, setGraphData] = useState<GraphData | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [highlightedNodeIds, setHighlightedNodeIds] = useState<Set<string>>(new Set());
  const [detailOpen, setDetailOpen] = useState(false);

  const graphViewRef = useRef<GraphViewHandle>(null);
  const lastRequestRef = useRef(0);

  const loadGraph = useCallback(async () => {
    const requestId = Date.now();
    lastRequestRef.current = requestId;
    setLoading(true);

    try {
      const result = await invoke<MemoryGraphResponse>("list_knowledge_graph");
      if (lastRequestRef.current !== requestId) { return; }

      const data = adaptMemoryToGraphData(result);
      const backlinkMap = computeBacklinkCounts(data);
      data.nodes = data.nodes.map((node) => ({
        ...node,
        backlinkCount: backlinkMap.get(node.id) ?? 0,
      }));

      setGraphData(data);
    } catch (error) {
      if (lastRequestRef.current === requestId) {
        showBackendError(message, error);
        setGraphData({ nodes: [], edges: [] });
      }
    } finally {
      if (lastRequestRef.current === requestId) {
        setLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    loadGraph();
  }, [loadGraph]);

  const stats = useMemo(() => {
    if (!graphData) { return { nodes: 0, edges: 0 }; }
    return { nodes: graphData.nodes.length, edges: graphData.edges.length };
  }, [graphData]);

  const handleNodeClick = useCallback((nodeId: string) => {
    setSelectedNodeId(nodeId);
    setDetailOpen(true);
    setHighlightedNodeIds(new Set([nodeId]));
  }, []);

  const handleDeselect = useCallback(() => {
    setSelectedNodeId(null);
    setHighlightedNodeIds(new Set());
    setDetailOpen(false);
  }, []);

  const selectedNode = useMemo(() => {
    if (!selectedNodeId || !graphData) { return null; }
    return graphData.nodes.find((n) => n.id === selectedNodeId) ?? null;
  }, [selectedNodeId, graphData]);

  const selectedRelations = useMemo(() => {
    if (!selectedNodeId || !graphData) { return []; }
    const relations: Array<{
      id: string;
      type: string;
      direction: "outgoing" | "incoming";
      otherTitle?: string;
    }> = [];

    for (const edge of graphData.edges) {
      if (edge.source === selectedNodeId) {
        const target = graphData.nodes.find((n) => n.id === edge.target);
        relations.push({
          id: edge.source + "->" + edge.target,
          type: edge.type,
          direction: "outgoing",
          otherTitle: target?.title,
        });
      } else if (edge.target === selectedNodeId) {
        const source = graphData.nodes.find((n) => n.id === edge.source);
        relations.push({
          id: edge.source + "->" + edge.target,
          type: edge.type,
          direction: "incoming",
          otherTitle: source?.title,
        });
      }
    }
    return relations;
  }, [selectedNodeId, graphData]);

  return (
    <div
      style={{
        height: 480,
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: token.borderRadiusLG,
        background: token.colorBgContainer,
        overflow: "hidden",
        display: "flex",
        flexDirection: "column",
        position: "relative",
      }}
    >
      {/* 头部工具栏 */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "8px 12px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
          background: token.colorBgElevated,
          flexShrink: 0,
        }}
      >
        <span style={{ fontSize: 13, fontWeight: 500, color: token.colorText }}>
          {t("memory.graph.title")}
        </span>
        <span style={{ fontSize: 12, color: token.colorTextTertiary }}>
          · {stats.nodes} {t("memory.graph.nodeCount", { count: stats.nodes })} · {stats.edges}{" "}
          {t("memory.graph.edgeCount", { count: stats.edges })}
        </span>
        <div style={{ flex: 1 }} />
        <button
          type="button"
          onClick={loadGraph}
          title={t("memory.graph.refresh")}
          style={{
            padding: "4px",
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: token.borderRadiusSM,
            background: "transparent",
            cursor: "pointer",
            color: token.colorTextSecondary,
            display: "flex",
            alignItems: "center",
          }}
        >
          <RefreshCw size={14} />
        </button>
        <button
          type="button"
          onClick={onClose}
          title={t("common.close")}
          style={{
            padding: "4px",
            border: `1px solid ${token.colorBorderSecondary}`,
            borderRadius: token.borderRadiusSM,
            background: "transparent",
            cursor: "pointer",
            color: token.colorTextSecondary,
            display: "flex",
            alignItems: "center",
          }}
        >
          <X size={14} />
        </button>
      </div>

      {/* 图谱区域 */}
      <div style={{ flex: 1, position: "relative", minHeight: 0 }}>
        {loading
          ? (
            <div style={{ display: "flex", justifyContent: "center", alignItems: "center", height: "100%" }}>
              <Spin size="large" />
            </div>
          )
          : !graphData || graphData.nodes.length === 0
          ? (
            <div style={{ display: "flex", justifyContent: "center", alignItems: "center", height: "100%" }}>
              <Empty
                description={t("memory.graph.emptyDescription")}
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            </div>
          )
          : (
            <GraphView
              ref={graphViewRef}
              data={graphData}
              wikiId="memory"
              onNodeClick={handleNodeClick}
              onDeselect={handleDeselect}
              selectedNodeId={selectedNodeId}
              highlightedNodeIds={highlightedNodeIds}
            />
          )}
      </div>

      {/* 节点详情浮层 */}
      {detailOpen && selectedNode && (
        <div
          style={{
            position: "absolute",
            right: 8,
            top: 50,
            width: 260,
            maxHeight: "calc(100% - 60px)",
            background: token.colorBgElevated,
            border: `1px solid ${token.colorBorder}`,
            borderRadius: token.borderRadius,
            boxShadow: token.boxShadowSecondary,
            overflow: "auto",
            padding: 12,
            fontSize: 12,
          }}
        >
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
            <strong style={{ fontSize: 13 }}>{selectedNode.title}</strong>
            <button
              type="button"
              onClick={() => setDetailOpen(false)}
              style={{ border: "none", background: "transparent", cursor: "pointer", color: token.colorTextTertiary }}
            >
              <X size={12} />
            </button>
          </div>
          <div style={{ color: token.colorTextSecondary, marginBottom: 8 }}>
            {selectedNode.path} · {t("memory.graph.mentionCount", { count: selectedNode.linkCount })}
          </div>
          {selectedNode.tags.length > 0 && (
            <div style={{ marginBottom: 8 }}>
              <div style={{ color: token.colorTextTertiary, marginBottom: 4 }}>{t("memory.graph.tags")}</div>
              <div style={{ display: "flex", flexWrap: "wrap", gap: 4 }}>
                {selectedNode.tags.slice(0, 6).map((tag, i) => (
                  <span
                    key={i}
                    style={{
                      padding: "2px 6px",
                      background: token.colorFillTertiary,
                      borderRadius: 4,
                      color: token.colorTextSecondary,
                      fontSize: 11,
                    }}
                  >
                    {tag}
                  </span>
                ))}
              </div>
            </div>
          )}
          {selectedRelations.length > 0 && (
            <div>
              <div style={{ color: token.colorTextTertiary, marginBottom: 4 }}>
                {t("memory.graph.relationships")} ({selectedRelations.length})
              </div>
              <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                {selectedRelations.slice(0, 8).map((rel) => (
                  <div
                    key={rel.id}
                    style={{
                      padding: "4px 6px",
                      background: token.colorFillQuaternary,
                      borderRadius: 4,
                      color: token.colorText,
                      fontSize: 11,
                    }}
                  >
                    {rel.direction === "outgoing" ? "→" : "←"} {rel.type} {rel.otherTitle ? `· ${rel.otherTitle}` : ""}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
