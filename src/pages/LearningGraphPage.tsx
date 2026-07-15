// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, isTauri } from "@/lib/invoke";
import type { GraphNode, LearningGraph } from "@/types";
import {
  ApartmentOutlined,
  BookOutlined,
  BulbOutlined,
  DatabaseOutlined,
  ReloadOutlined,
  SearchOutlined,
} from "@ant-design/icons";
import {
  Background,
  Controls,
  type Edge,
  MarkerType,
  MiniMap,
  type Node,
  type NodeProps,
  type NodeTypes,
  ReactFlow,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import { Alert, Badge, Button, Card, Empty, Input, Select, Spin, Tag, theme, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import "@xyflow/react/dist/style.css";

const { Text, Title } = Typography;

// ── Node type components ──────────────────────────────────────────────

function SkillNode({ data }: NodeProps) {
  const d = data as Record<string, unknown>;
  const color = (d.color as string) || "#4a9eff";
  return (
    <div
      style={{
        padding: "8px 14px",
        borderRadius: 8,
        border: `1px solid ${color}`,
        background: `${color}22`,
        fontSize: 13,
        fontWeight: 500,
        maxWidth: 180,
        cursor: "pointer",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <BookOutlined style={{ color }} />
        <Text ellipsis>{d.label as string}</Text>
      </div>
      {!!d.detail && (
        <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: 2 }} ellipsis>
          {String(d.detail)}
        </Text>
      )}
    </div>
  );
}

function MemoryNode({ data }: NodeProps) {
  const d = data as Record<string, unknown>;
  const color = (d.color as string) || "#52c41a";
  return (
    <div
      style={{
        padding: "8px 14px",
        borderRadius: 8,
        border: `1px solid ${color}`,
        background: `${color}22`,
        fontSize: 13,
        maxWidth: 200,
        cursor: "pointer",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <DatabaseOutlined style={{ color }} />
        <Text ellipsis>{d.label as string}</Text>
      </div>
      {!!d.category && (
        <Tag color="green" style={{ fontSize: 10, marginTop: 4 }}>
          {String(d.category)}
        </Tag>
      )}
    </div>
  );
}

function InsightNode({ data }: NodeProps) {
  const d = data as Record<string, unknown>;
  const color = (d.color as string) || "#faad14";
  return (
    <div
      style={{
        padding: "8px 14px",
        borderRadius: 8,
        border: `1px solid ${color}`,
        background: `${color}22`,
        fontSize: 13,
        maxWidth: 200,
        cursor: "pointer",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <BulbOutlined style={{ color }} />
        <Text ellipsis>{d.label as string}</Text>
      </div>
      {!!d.detail && (
        <Text type="secondary" style={{ fontSize: 11, display: "block", marginTop: 2 }} ellipsis>
          {String(d.detail)}
        </Text>
      )}
    </div>
  );
}

function EntityNode({ data }: NodeProps) {
  const d = data as Record<string, unknown>;
  const color = (d.color as string) || "#722ed1";
  return (
    <div
      style={{
        padding: "8px 14px",
        borderRadius: 8,
        border: `1px solid ${color}`,
        background: `${color}22`,
        fontSize: 13,
        maxWidth: 200,
        cursor: "pointer",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <ApartmentOutlined style={{ color }} />
        <Text ellipsis>{d.label as string}</Text>
      </div>
      {!!d.category && (
        <Tag color="purple" style={{ fontSize: 10, marginTop: 4 }}>
          {String(d.category)}
        </Tag>
      )}
    </div>
  );
}

const nodeTypes: NodeTypes = {
  skill: SkillNode,
  memory: MemoryNode,
  insight: InsightNode,
  entity: EntityNode,
};

// ── Color map ─────────────────────────────────────────────────────────

const KIND_COLORS: Record<string, string> = {
  skill: "#4a9eff",
  memory: "#52c41a",
  insight: "#faad14",
  entity: "#722ed1",
};

// ── Debounce hook ─────────────────────────────────────────────────────

function useDebounce<T>(value: T, delay: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delay);
    return () => clearTimeout(id);
  }, [value, delay]);
  return debounced;
}

// ── Main page ─────────────────────────────────────────────────────────

export function LearningGraphPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const [loading, setLoading] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [graph, setGraph] = useState<LearningGraph | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [selectedNode, setSelectedNode] = useState<GraphNode | null>(null);
  const [filterKind, setFilterKind] = useState("all");
  const [searchText, setSearchText] = useState("");

  // Debounce search input (300ms) to avoid rapid re-renders while typing
  const debouncedSearch = useDebounce(searchText, 300);

  // ReactFlow state
  const [nodes, setNodes, onNodesChange] = useNodesState([] as Node[]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([] as Edge[]);

  // Fetch graph data (initial load)
  useEffect(() => {
    const load = async () => {
      if (!isTauri()) {
        setLoading(false);
        return;
      }
      setErrorMsg(null);
      try {
        const data = await invoke<LearningGraph>("get_learning_graph");
        setGraph(data);
      } catch (err) {
        console.error("Failed to fetch learning graph:", err);
        setErrorMsg(typeof err === "string" ? err : "Failed to fetch learning graph");
      } finally {
        setLoading(false);
      }
    };
    load();
  }, []);

  // Manual refresh — uses its own refreshing flag so the loading overlay only
  // covers the initial load, while a small spinner shows on the refresh button.
  const handleRefresh = useCallback(async () => {
    if (!isTauri()) { return; }
    setIsRefreshing(true);
    setErrorMsg(null);
    try {
      const data = await invoke<LearningGraph>("get_learning_graph");
      setGraph(data);
    } catch (err) {
      console.error("Failed to fetch learning graph:", err);
      setErrorMsg(typeof err === "string" ? err : "Failed to fetch learning graph");
    } finally {
      setIsRefreshing(false);
    }
  }, []);

  // Convert backend data to ReactFlow format (debounced search applied)
  const filteredNodes = useMemo(() => {
    if (!graph) { return [] as GraphNode[]; }
    return graph.nodes.filter((n) => {
      if (filterKind !== "all" && n.kind !== filterKind) { return false; }
      if (debouncedSearch && !n.label.toLowerCase().includes(debouncedSearch.toLowerCase())) {
        return false;
      }
      return true;
    });
  }, [graph, filterKind, debouncedSearch]);

  useEffect(() => {
    if (!graph || filteredNodes.length === 0) {
      if (filteredNodes.length === 0 && graph) {
        setNodes([]);
        setEdges([]);
      }
      return;
    }
    const nodeMap = new Map(filteredNodes.map((n) => [n.id, n]));

    const rfNodes = filteredNodes.map((n, i) => ({
      id: n.id,
      type: n.kind,
      position: { x: 200 + (i % 5) * 250, y: 80 + Math.floor(i / 5) * 140 },
      data: {
        label: n.label,
        detail: n.detail,
        category: n.category,
        color: KIND_COLORS[n.kind] || "#888",
      },
    }));

    const rfEdges = graph.edges
      .filter((e) => nodeMap.has(e.source) && nodeMap.has(e.target))
      .map((e) => ({
        id: `${e.source}→${e.target}`,
        source: e.source,
        target: e.target,
        animated: true,
        style: { stroke: "#888", strokeWidth: 1 + e.weight * 2 },
        markerEnd: { type: MarkerType.ArrowClosed, color: "#888" } as const,
        label: e.relation === "lexical_overlap"
          ? t("learningGraph.lexicalOverlap")
          : e.relation === "category_match"
          ? t("learningGraph.insights")
          : e.relation,
      }));

    setNodes(rfNodes);
    setEdges(rfEdges);
  }, [filteredNodes, graph, setNodes, setEdges, t]);

  // Handle node click — show detail panel
  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: { id: string }) => {
      if (!graph) { return; }
      const found = graph.nodes.find((n) => n.id === node.id);
      setSelectedNode(found || null);
    },
    [graph],
  );

  // Stats
  const stats = graph?.stats;

  const showInitialLoading = loading && !graph && !errorMsg;
  const showEmpty = !loading && !errorMsg && graph && nodes.length === 0;
  const showError = !!errorMsg;
  const showGraph = !showInitialLoading && !showEmpty && !showError;

  return (
    <div
      style={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        background: token.colorBgElevated,
      }}
    >
      {/* ── Header ── */}
      <div
        style={{
          padding: "12px 20px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          gap: 12,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <Title level={4} style={{ margin: 0 }}>
            {t("learningGraph.title")}
          </Title>
          {stats && (
            <div style={{ display: "flex", gap: 8, fontSize: 12 }}>
              <Badge count={stats.totalSkills} color="#4a9eff" showZero />
              <Text type="secondary">{t("learningGraph.skills")}</Text>
              <Badge count={stats.totalMemories} color="#52c41a" showZero />
              <Text type="secondary">{t("learningGraph.memories")}</Text>
              <Badge count={stats.totalInsights} color="#faad14" showZero />
              <Text type="secondary">{t("learningGraph.insights")}</Text>
              <Badge count={stats.totalEntities} color="#722ed1" showZero />
              <Text type="secondary">{t("learningGraph.entities")}</Text>
              <Badge count={stats.totalEdges} color="#888" showZero />
              <Text type="secondary">{t("learningGraph.edges")}</Text>
            </div>
          )}
        </div>

        <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <Input
            prefix={<SearchOutlined />}
            placeholder={t("learningGraph.searchPlaceholder")}
            value={searchText}
            onChange={(e) => setSearchText(e.target.value)}
            style={{ width: 200 }}
            allowClear
          />
          <Select
            value={filterKind}
            onChange={setFilterKind}
            style={{ width: 120 }}
            options={[
              { value: "all", label: t("learningGraph.all") },
              { value: "skill", label: t("learningGraph.skills") },
              { value: "memory", label: t("learningGraph.memories") },
              { value: "insight", label: t("learningGraph.insights") },
              { value: "entity", label: t("learningGraph.entities") },
            ]}
          />
          <Button icon={<ReloadOutlined />} onClick={handleRefresh} loading={isRefreshing}>
            {t("learningGraph.refresh")}
          </Button>
        </div>
      </div>

      {/* ── Main content ── */}
      <div style={{ flex: 1, display: "flex", overflow: "hidden" }}>
        {/* Graph area */}
        <div style={{ flex: 1, position: "relative" }}>
          {showInitialLoading && (
            <div
              style={{
                display: "flex",
                justifyContent: "center",
                alignItems: "center",
                height: "100%",
              }}
            >
              <Spin size="large" />
            </div>
          )}
          {showError && (
            <div
              style={{
                display: "flex",
                justifyContent: "center",
                alignItems: "center",
                height: "100%",
                padding: 40,
              }}
            >
              <Alert
                type="error"
                message={t("learningGraph.title")}
                description={errorMsg}
                showIcon
                action={
                  <Button size="small" onClick={handleRefresh} loading={isRefreshing}>
                    {t("learningGraph.refresh")}
                  </Button>
                }
              />
            </div>
          )}
          {showEmpty && (
            <div
              style={{
                display: "flex",
                justifyContent: "center",
                alignItems: "center",
                height: "100%",
              }}
            >
              <Empty description={t("learningGraph.emptyDescription")} />
            </div>
          )}
          {showGraph && (
            <ReactFlow
              nodes={nodes}
              edges={edges}
              onNodesChange={onNodesChange}
              onEdgesChange={onEdgesChange}
              onNodeClick={onNodeClick}
              nodeTypes={nodeTypes}
              fitView
            >
              <Background />
              <Controls />
              <MiniMap
                nodeStrokeColor={token.colorBorder}
                nodeColor={(n: { type?: string }) => KIND_COLORS[n.type || "skill"] || "#888"}
                style={{ border: `1px solid ${token.colorBorderSecondary}` }}
              />
            </ReactFlow>
          )}
        </div>

        {/* Detail panel */}
        {selectedNode && (
          <div
            style={{
              width: 300,
              borderLeft: `1px solid ${token.colorBorderSecondary}`,
              padding: 16,
              overflowY: "auto",
            }}
          >
            <Title level={5} style={{ marginTop: 0 }}>
              {selectedNode.label}
            </Title>
            <div style={{ marginBottom: 12 }}>
              <Tag color={KIND_COLORS[selectedNode.kind] || "#888"}>
                {selectedNode.kind === "skill"
                  ? t("learningGraph.skills")
                  : selectedNode.kind === "memory"
                  ? t("learningGraph.memories")
                  : selectedNode.kind === "entity"
                  ? t("learningGraph.entities")
                  : t("learningGraph.insights")}
              </Tag>
              <Tag>{selectedNode.category}</Tag>
              {selectedNode.state !== "active" && <Tag color="orange">{selectedNode.state}</Tag>}
            </div>
            {selectedNode.useCount > 0 && (
              <Text type="secondary" style={{ display: "block", marginBottom: 8 }}>
                {t("learningGraph.useCount", { count: selectedNode.useCount })}
              </Text>
            )}
            {selectedNode.detail && (
              <Card size="small" title={t("learningGraph.detail")} style={{ marginTop: 8 }}>
                <Text style={{ whiteSpace: "pre-wrap", fontSize: 13 }}>
                  {selectedNode.detail}
                </Text>
              </Card>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
