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
  Handle,
  MarkerType,
  MiniMap,
  type Node,
  type NodeProps,
  type NodeTypes,
  Position,
  ReactFlow,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import { Alert, Badge, Button, Card, Empty, Input, Select, Spin, Tag, theme, Typography } from "antd";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import "@xyflow/react/dist/style.css";

const { Text, Title } = Typography;

// F-P1-3: 浏览器开发模式（npm run dev）下的 mock 数据，遵循 AGENTS.md 约定的 localStorage mock 模式
const MOCK_GRAPH: LearningGraph = {
  nodes: [
    {
      id: "skill-1",
      kind: "skill",
      label: "Mock Skill A",
      detail: "Dev mode sample",
      category: "coding",
      timestampMs: Date.now(),
      useCount: 5,
      state: "active",
    },
    {
      id: "skill-2",
      kind: "skill",
      label: "Mock Skill B",
      detail: "Dev mode sample",
      category: "writing",
      timestampMs: Date.now(),
      useCount: 2,
      state: "active",
    },
    {
      id: "memory-1",
      kind: "memory",
      label: "Mock Memory",
      detail: "Dev mode sample",
      category: "preference",
      timestampMs: Date.now(),
      useCount: 8,
      state: "active",
    },
    {
      id: "insight-1",
      kind: "insight",
      label: "Mock Insight",
      detail: "Dev mode sample",
      category: "improvement",
      timestampMs: Date.now(),
      useCount: 0,
      state: "active",
    },
    {
      id: "entity-1",
      kind: "entity",
      label: "Mock Entity",
      detail: "Dev mode sample",
      category: "concept",
      timestampMs: Date.now(),
      useCount: 3,
      state: "active",
    },
  ],
  edges: [
    { source: "skill-1", target: "memory-1", relation: "category_match", weight: 0.8 },
    { source: "skill-1", target: "entity-1", relation: "lexical_overlap", weight: 0.5 },
    { source: "skill-2", target: "entity-1", relation: "related", weight: 0.3 },
    { source: "memory-1", target: "insight-1", relation: "derived", weight: 0.7 },
  ],
  stats: {
    totalSkills: 2,
    totalMemories: 1,
    totalInsights: 1,
    totalEntities: 1,
    totalEdges: 4,
    linkedNodes: 5,
    categories: [
      { category: "coding", count: 1 },
      { category: "writing", count: 1 },
      { category: "preference", count: 1 },
      { category: "improvement", count: 1 },
      { category: "concept", count: 1 },
    ],
  },
};

// F-P1-5: localStorage 持久化节点 position 的 key（按 stats 哈希区分不同图）
const LAYOUT_STORAGE_KEY = "axagent.learningGraph.layout";

function loadSavedLayout(graphStats: LearningGraph["stats"] | undefined): Record<string, { x: number; y: number }> {
  if (!graphStats) { return {}; }
  try {
    const key = `${LAYOUT_STORAGE_KEY}.${graphStats.totalSkills}-${graphStats.totalMemories}-${graphStats.totalEdges}`;
    const raw = localStorage.getItem(key);
    if (!raw) { return {}; }
    const parsed = JSON.parse(raw) as Record<string, { x: number; y: number }>;
    return parsed && typeof parsed === "object" ? parsed : {};
  } catch {
    return {};
  }
}

function saveLayout(
  graphStats: LearningGraph["stats"] | undefined,
  positions: Record<string, { x: number; y: number }>,
): void {
  if (!graphStats) { return; }
  try {
    const key = `${LAYOUT_STORAGE_KEY}.${graphStats.totalSkills}-${graphStats.totalMemories}-${graphStats.totalEdges}`;
    localStorage.setItem(key, JSON.stringify(positions));
  } catch {
    // localStorage 满或禁用时静默忽略
  }
}

// F-P0-2: 简单分层布局算法（基于入度的 BFS 拓扑分层，比纯索引网格更能体现关系）
function computeLayeredLayout(
  nodes: GraphNode[],
  edges: Array<{ source: string; target: string }>,
): Record<string, { x: number; y: number }> {
  const positions: Record<string, { x: number; y: number }> = {};
  const inDegree = new Map<string, number>();
  const adj = new Map<string, string[]>();
  nodes.forEach((n) => {
    inDegree.set(n.id, 0);
    adj.set(n.id, []);
  });
  edges.forEach((e) => {
    if (inDegree.has(e.target) && adj.has(e.source)) {
      inDegree.set(e.target, (inDegree.get(e.target) ?? 0) + 1);
      adj.get(e.source)?.push(e.target);
    }
  });
  // BFS 分层
  const layers = new Map<string, number>();
  const queue: string[] = [];
  inDegree.forEach((deg, id) => {
    if (deg === 0) {
      layers.set(id, 0);
      queue.push(id);
    }
  });
  // 处理孤立节点（无入度也无出度）和环
  if (queue.length === 0 && nodes.length > 0) {
    nodes.forEach((n, i) => layers.set(n.id, Math.floor(i / 5)));
  }
  let head = 0;
  while (head < queue.length) {
    const cur = queue[head++];
    const curLayer = layers.get(cur) ?? 0;
    const neighbors = adj.get(cur) ?? [];
    neighbors.forEach((nb) => {
      const newLayer = curLayer + 1;
      if (!layers.has(nb) || (layers.get(nb) ?? 0) < newLayer) {
        layers.set(nb, newLayer);
        queue.push(nb);
      }
    });
  }
  // 兜底：未分层的节点放到第 0 层
  nodes.forEach((n) => {
    if (!layers.has(n.id)) { layers.set(n.id, 0); }
  });
  // 按层摆放
  const byLayer = new Map<number, string[]>();
  layers.forEach((layer, id) => {
    if (!byLayer.has(layer)) { byLayer.set(layer, []); }
    byLayer.get(layer)?.push(id);
  });
  const LAYER_HEIGHT = 160;
  const NODE_WIDTH = 260;
  const X_OFFSET = 200;
  const Y_OFFSET = 80;
  byLayer.forEach((ids, layer) => {
    ids.forEach((id, idxInLayer) => {
      positions[id] = {
        x: X_OFFSET + idxInLayer * NODE_WIDTH,
        y: Y_OFFSET + layer * LAYER_HEIGHT,
      };
    });
  });
  return positions;
}

// ── Node type components ──────────────────────────────────────────────

function SkillNode({ data }: NodeProps) {
  const d = data as Record<string, unknown>;
  const color = (d.color as string) || "#4a9eff";
  return (
    <>
      {/* 隐藏的 target/source handle，确保 React Flow 能正确创建 edge 连接 */}
      <Handle
        type="target"
        id="target"
        position={Position.Left}
        style={{ opacity: 0, width: 1, height: 1 }}
      />
      <Handle
        type="source"
        id="source"
        position={Position.Right}
        style={{ opacity: 0, width: 1, height: 1 }}
      />
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
    </>
  );
}

function MemoryNode({ data }: NodeProps) {
  const d = data as Record<string, unknown>;
  const color = (d.color as string) || "#52c41a";
  return (
    <>
      <Handle
        type="target"
        id="target"
        position={Position.Left}
        style={{ opacity: 0, width: 1, height: 1 }}
      />
      <Handle
        type="source"
        id="source"
        position={Position.Right}
        style={{ opacity: 0, width: 1, height: 1 }}
      />
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
    </>
  );
}

function InsightNode({ data }: NodeProps) {
  const d = data as Record<string, unknown>;
  const color = (d.color as string) || "#faad14";
  return (
    <>
      <Handle
        type="target"
        id="target"
        position={Position.Left}
        style={{ opacity: 0, width: 1, height: 1 }}
      />
      <Handle
        type="source"
        id="source"
        position={Position.Right}
        style={{ opacity: 0, width: 1, height: 1 }}
      />
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
    </>
  );
}

function EntityNode({ data }: NodeProps) {
  const d = data as Record<string, unknown>;
  const color = (d.color as string) || "#722ed1";
  return (
    <>
      <Handle
        type="target"
        id="target"
        position={Position.Left}
        style={{ opacity: 0, width: 1, height: 1 }}
      />
      <Handle
        type="source"
        id="source"
        position={Position.Right}
        style={{ opacity: 0, width: 1, height: 1 }}
      />
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
    </>
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
      // F-P1-3: 浏览器开发模式（npm run dev）下加载 mock 数据，避免空白页违反 AGENTS.md 约定
      if (!isTauri()) {
        setGraph(MOCK_GRAPH);
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
    // F-P1-3: 浏览器开发模式下也允许刷新（重新加载 mock）
    if (!isTauri()) {
      setIsRefreshing(true);
      setGraph(MOCK_GRAPH);
      setErrorMsg(null);
      setIsRefreshing(false);
      return;
    }
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

  // F-P0-1: edge label 翻译移到 useMemo 中，避免 useEffect 依赖 t 导致语言切换时重置布局
  const edgeLabelMap = useMemo(() => {
    return {
      lexical_overlap: t("learningGraph.lexicalOverlap"),
      category_match: t("learningGraph.categoryMatch"),
      related: t("learningGraph.related"),
      derived: t("learningGraph.derived"),
    } as Record<string, string>;
  }, [t]);

  useEffect(() => {
    if (!graph || filteredNodes.length === 0) {
      if (filteredNodes.length === 0 && graph) {
        setNodes([]);
        setEdges([]);
      }
      return;
    }
    const nodeMap = new Map(filteredNodes.map((n) => [n.id, n]));

    // F-P0-2: 用基于入度的 BFS 拓扑分层算法替代纯索引网格
    // F-P1-5: 优先读取 localStorage 中保存的 position，无则用计算出的布局
    const computedLayout = computeLayeredLayout(filteredNodes, graph.edges);
    const savedLayout = loadSavedLayout(graph.stats);
    const finalLayout = { ...computedLayout, ...savedLayout };

    const rfNodes = filteredNodes.map((n) => ({
      id: n.id,
      type: n.kind,
      position: finalLayout[n.id] ?? { x: 200, y: 80 },
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
        // F-P1-4: category_match 用独立 i18n key 而非复用 insights
        label: edgeLabelMap[e.relation] ?? e.relation,
      }));

    setNodes(rfNodes);
    setEdges(rfEdges);
  }, [filteredNodes, graph, setNodes, setEdges, edgeLabelMap]);

  // F-P1-5: 监听 nodes position 变化时持久化到 localStorage
  useEffect(() => {
    if (!graph || nodes.length === 0) { return; }
    const positions: Record<string, { x: number; y: number }> = {};
    nodes.forEach((n) => {
      positions[n.id] = n.position;
    });
    saveLayout(graph.stats, positions);
  }, [nodes, graph]);

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
                title={t("learningGraph.title")}
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
                // F-P1-7: node.type 大小写不敏感，避免 "Skill" vs "skill" 导致回退到灰色
                nodeColor={(n: { type?: string }) => {
                  const type = (n.type || "skill").toLowerCase();
                  return KIND_COLORS[type] || "#888";
                }}
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
              {/* F-P1-6: category/state 用 i18n 翻译，未知值回退到原始字符串 */}
              <Tag>
                {t(`learningGraph.categories.${selectedNode.category}`, { defaultValue: selectedNode.category })}
              </Tag>
              {selectedNode.state !== "active" && (
                <Tag color="orange">
                  {t(
                    `learningGraph.state${selectedNode.state.charAt(0).toUpperCase()}${selectedNode.state.slice(1)}`,
                    { defaultValue: selectedNode.state },
                  )}
                </Tag>
              )}
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
