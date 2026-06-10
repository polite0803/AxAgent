import { theme, Typography } from "antd";
import type { GlobalToken } from "antd/es/theme/interface";
import { BookOpen, Brain, ChevronDown, ChevronUp, GitBranch, Link2, Puzzle, Search, Wrench, Zap } from "lucide-react";
import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Background,
  Controls,
  type Edge,
  Handle,
  MarkerType,
  MiniMap,
  type Node,
  type NodeTypes,
  type OnEdgesChange,
  type OnNodesChange,
  Position,
  ReactFlow,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "reactflow";
import "reactflow/dist/style.css";
import { useKnowledgeStore, useMcpStore, useMemoryStore, useSkillExtensionStore } from "@/stores";

// ── Types ────────────────────────────────────────────────────────────────

type ContextNodeType =
  | "conversation"
  | "model"
  | "knowledge"
  | "memory"
  | "mcp"
  | "search"
  | "skill";

interface ContextGraphNode {
  id: string;
  type: ContextNodeType;
  label: string;
  detail?: string;
}

interface ContextGraphEdge {
  source: string;
  target: string;
  label?: string;
}

// ── Custom React Flow Node ───────────────────────────────────────────────

function getNodeTypeStyles(token: GlobalToken): Record<
  ContextNodeType,
  { icon: React.ReactNode; bg: string; border: string }
> {
  return {
    conversation: {
      icon: <GitBranch size={12} />,
      bg: token.colorPrimaryBg,
      border: token.colorPrimary,
    },
    model: {
      icon: <Zap size={12} />,
      bg: "rgba(114,46,209,0.08)",
      border: "#722ed1",
    },
    knowledge: {
      icon: <BookOpen size={12} />,
      bg: token.colorSuccessBg,
      border: token.colorSuccess,
    },
    memory: {
      icon: <Brain size={12} />,
      bg: token.colorWarningBg,
      border: token.colorWarning,
    },
    mcp: {
      icon: <Wrench size={12} />,
      bg: "rgba(19,194,194,0.08)",
      border: "#13c2c2",
    },
    search: {
      icon: <Search size={12} />,
      bg: token.colorPrimaryBg,
      border: token.colorPrimary,
    },
    skill: {
      icon: <Puzzle size={12} />,
      bg: "rgba(235,47,150,0.08)",
      border: "#eb2f96",
    },
  };
}

function ContextNode({
  data,
}: {
  data: { label: string; detail?: string; nodeType: ContextNodeType };
}) {
  const { token } = theme.useToken();
  const nodeTypeStyles = getNodeTypeStyles(token);
  const style = nodeTypeStyles[data.nodeType] || nodeTypeStyles.conversation;

  return (
    <div
      style={{
        padding: "8px 12px",
        borderRadius: token.borderRadius,
        border: `1.5px solid ${style.border}`,
        backgroundColor: style.bg,
        fontSize: 12,
        minWidth: 100,
        maxWidth: 180,
        cursor: "default",
      }}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: style.border }}
      />
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ color: style.border, display: "flex" }}>
          {style.icon}
        </span>
        <Typography.Text strong style={{ fontSize: 12 }} ellipsis>
          {data.label}
        </Typography.Text>
      </div>
      {data.detail && (
        <Typography.Text type="secondary" style={{ fontSize: 10 }} ellipsis>
          {data.detail}
        </Typography.Text>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: style.border }}
      />
    </div>
  );
}

const nodeTypes: NodeTypes = { contextNode: ContextNode };

// ── Layout helper ────────────────────────────────────────────────────────

function layoutGraph(
  nodes: ContextGraphNode[],
  edges: ContextGraphEdge[],
): { nodes: Node[]; edges: Edge[] } {
  const rfNodes: Node[] = [];
  const rfEdges: Edge[] = [];

  // Simple radial-ish layout: conversation at center, everything else around
  const centerX = 300;
  const centerY = 250;
  const radius = 180;
  const nonConvNodes = nodes.filter((n) => n.type !== "conversation");
  const convNode = nodes.find((n) => n.type === "conversation");

  if (convNode) {
    rfNodes.push({
      id: convNode.id,
      type: "contextNode",
      position: { x: centerX - 60, y: centerY - 30 },
      data: {
        label: convNode.label,
        detail: convNode.detail,
        nodeType: convNode.type,
      },
    });
  }

  nonConvNodes.forEach((node, idx) => {
    const angle = (idx / Math.max(1, nonConvNodes.length)) * 2 * Math.PI;
    const x = centerX + radius * Math.cos(angle) - 60;
    const y = centerY + radius * Math.sin(angle) - 30;
    rfNodes.push({
      id: node.id,
      type: "contextNode",
      position: { x, y },
      data: { label: node.label, detail: node.detail, nodeType: node.type },
    });
  });

  for (const edge of edges) {
    rfEdges.push({
      id: `${edge.source}-${edge.target}`,
      source: edge.source,
      target: edge.target,
      label: edge.label,
      type: "smoothstep",
      animated: true,
      style: { stroke: "#888", strokeWidth: 1 },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        width: 8,
        height: 8,
        color: "#888",
      },
    });
  }

  return { nodes: rfNodes, edges: rfEdges };
}

// ── Component ────────────────────────────────────────────────────────────

interface ContextGraphPanelProps {
  conversationTitle?: string;
  conversationId?: string;
  modelName?: string;
  providerName?: string;
  knowledgeBaseIds: string[];
  memoryNamespaceIds: string[];
  mcpServerIds: string[];
  searchEnabled: boolean;
  enabledSkillIds: string[];
}

export const ContextGraphPanel = React.memo(function ContextGraphPanel({
  conversationTitle,
  conversationId,
  modelName,
  providerName,
  knowledgeBaseIds,
  memoryNamespaceIds,
  mcpServerIds,
  searchEnabled,
  enabledSkillIds,
}: ContextGraphPanelProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const nodeTypeStyles = getNodeTypeStyles(token);

  const [collapsed, setCollapsed] = useState(true);
  // 选中的图例类型（用于筛选图谱展示），null = 全部显示
  const [hiddenTypes, setHiddenTypes] = useState<Set<string>>(new Set());

  // Get detail info from various stores
  const knowledgeBases = useKnowledgeStore((s) => s.bases ?? []);
  const memoryNamespaces = useMemoryStore((s) => s.namespaces ?? []);
  const mcpServers = useMcpStore((s) => s.servers ?? []);
  const installedSkills = useSkillExtensionStore((s) => s.skills ?? []);

  const graphData = useMemo(() => {
    const nodes: ContextGraphNode[] = [];
    const edges: ContextGraphEdge[] = [];

    // 预构建查找映射，避免在循环中调用 find
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const kbMap = new Map(knowledgeBases.map((k: any) => [k.id, k]));
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const nsMap = new Map(memoryNamespaces.map((n: any) => [n.id, n]));
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const srvMap = new Map(mcpServers.map((s: any) => [s.id, s]));
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const skillMap = new Map(installedSkills.map((s: any) => [s.id, s]));

    // Conversation node (center)
    const convName = conversationTitle
      || conversationId?.slice(0, 8)
      || t("chat.contextGraph.conversation");
    nodes.push({
      id: "conversation",
      type: "conversation",
      label: convName,
      detail: conversationId?.slice(0, 16),
    });

    // Model node
    if (modelName) {
      const modelLabel = providerName
        ? `${providerName} / ${modelName}`
        : modelName;
      nodes.push({ id: "model", type: "model", label: modelLabel });
      edges.push({
        source: "conversation",
        target: "model",
        label: t("chat.contextGraph.edges.uses"),
      });
    }

    // Knowledge bases
    for (const kbId of knowledgeBaseIds) {
      const kb = kbMap.get(kbId);
      const label = kb?.name || kbId.slice(0, 12);
      nodes.push({
        id: `kb:${kbId}`,
        type: "knowledge",
        label,
        detail: kb?.description,
      });
      edges.push({
        source: "conversation",
        target: `kb:${kbId}`,
        label: t("chat.contextGraph.edges.retrieves"),
      });
    }

    // Memory namespaces
    for (const nsId of memoryNamespaceIds) {
      const ns = nsMap.get(nsId);
      const label = ns?.name || nsId.slice(0, 12);
      nodes.push({ id: `mem:${nsId}`, type: "memory", label });
      edges.push({
        source: "conversation",
        target: `mem:${nsId}`,
        label: t("chat.contextGraph.edges.readWrite"),
      });
    }

    // MCP servers
    for (const srvId of mcpServerIds) {
      const srv = srvMap.get(srvId);
      const label = srv?.name || srvId.slice(0, 12);
      nodes.push({ id: `mcp:${srvId}`, type: "mcp", label });
      edges.push({
        source: "conversation",
        target: `mcp:${srvId}`,
        label: t("chat.contextGraph.edges.calls"),
      });
    }

    // Search
    if (searchEnabled) {
      nodes.push({
        id: "search",
        type: "search",
        label: t("chat.contextGraph.legend.search"),
      });
      edges.push({
        source: "conversation",
        target: "search",
        label: t("chat.contextGraph.edges.searches"),
      });
    }

    // Skills
    for (const skillId of enabledSkillIds) {
      const sk = skillMap.get(skillId);
      const label = sk?.name || skillId.slice(0, 12);
      nodes.push({ id: `skill:${skillId}`, type: "skill", label });
      edges.push({
        source: "conversation",
        target: `skill:${skillId}`,
        label: t("chat.contextGraph.edges.enables"),
      });
    }

    return { nodes, edges };
  }, [
    conversationTitle,
    conversationId,
    modelName,
    providerName,
    knowledgeBaseIds,
    memoryNamespaceIds,
    mcpServerIds,
    searchEnabled,
    enabledSkillIds,
    knowledgeBases,
    memoryNamespaces,
    mcpServers,
    installedSkills,
    t,
  ]);

  const layout = useMemo(
    () => layoutGraph(graphData.nodes, graphData.edges),
    [graphData],
  );

  const [rfNodes, setRfNodes, onNodesChange] = useNodesState(layout.nodes);
  const [rfEdges, setRfEdges, onEdgesChange] = useEdgesState(layout.edges);

  // 根据 hiddenTypes 筛选可见节点和边
  const visibleNodes = useMemo(() => {
    if (hiddenTypes.size === 0) {
      return rfNodes;
    }
    return rfNodes.filter((n) => {
      const nodeData = n.data as { nodeType?: ContextNodeType } | undefined;
      const nt = nodeData?.nodeType;
      if (!nt) {
        return true;
      }
      return !hiddenTypes.has(nt);
    });
  }, [rfNodes, hiddenTypes]);
  const visibleNodeIds = useMemo(
    () => new Set(visibleNodes.map((n) => n.id)),
    [visibleNodes],
  );
  const visibleEdges = useMemo(() => {
    if (hiddenTypes.size === 0) {
      return rfEdges;
    }
    return rfEdges.filter(
      (e) => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target),
    );
  }, [rfEdges, visibleNodeIds, hiddenTypes]);

  const toggleType = (type: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setHiddenTypes((prev) => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
      }
      return next;
    });
  };

  // Update nodes/edges when layout changes
  const prevLayoutRef = React.useRef<string>("");
  const layoutKey = JSON.stringify(layout);
  // eslint-disable-next-line react-hooks/refs
  if (layoutKey !== prevLayoutRef.current) {
    // eslint-disable-next-line react-hooks/refs
    prevLayoutRef.current = layoutKey;
    setTimeout(() => {
      setRfNodes(layout.nodes);
      setRfEdges(layout.edges);
    }, 0);
  }

  const totalSources = knowledgeBaseIds.length
    + memoryNamespaceIds.length
    + mcpServerIds.length
    + (searchEnabled ? 1 : 0)
    + enabledSkillIds.length;

  return (
    <div
      style={{
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: token.borderRadius,
        overflow: "hidden",
        marginBottom: 12,
      }}
    >
      {/* Header — click to toggle */}
      <div
        onClick={() => setCollapsed(!collapsed)}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            setCollapsed(!collapsed);
          }
        }}
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "6px 12px",
          backgroundColor: token.colorFillQuaternary,
          borderBottom: collapsed
            ? "none"
            : `1px solid ${token.colorBorderSecondary}`,
          cursor: "pointer",
          userSelect: "none",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Link2 size={14} style={{ color: token.colorPrimary }} />
          <Typography.Text strong style={{ fontSize: 13 }}>
            {t("chat.contextGraph.title")}
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {t("chat.contextGraph.sourceCount", { count: totalSources })}
          </Typography.Text>
          {/* Inline source pills when collapsed — compact one-line overview */}
          {collapsed && totalSources > 0 && (
            <div
              style={{
                display: "flex",
                gap: 4,
                marginLeft: 4,
                flexWrap: "wrap",
                maxWidth: 260,
                overflow: "hidden",
              }}
            >
              {(() => {
                const pills: { label: string; color: string }[] = [];
                // 预构建查找映射，避免在循环中调用 find
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const kbMap = new Map(knowledgeBases.map((k: any) => [k.id, k]));
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const nsMap = new Map(memoryNamespaces.map((n: any) => [n.id, n]));
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const srvMap = new Map(mcpServers.map((s: any) => [s.id, s]));
                // eslint-disable-next-line @typescript-eslint/no-explicit-any
                const skillMap = new Map(installedSkills.map((s: any) => [s.id, s]));
                if (modelName) {
                  pills.push({
                    label: modelName.slice(0, 12),
                    color: nodeTypeStyles.model.border,
                  });
                }
                for (const kbId of knowledgeBaseIds.slice(0, 2)) {
                  const kb = kbMap.get(kbId);
                  pills.push({
                    label: (kb?.name || kbId).slice(0, 10),
                    color: nodeTypeStyles.knowledge.border,
                  });
                }
                if (knowledgeBaseIds.length > 2) {
                  pills.push({
                    label: `+${knowledgeBaseIds.length - 2}`,
                    color: nodeTypeStyles.knowledge.border,
                  });
                }
                for (const nsId of memoryNamespaceIds.slice(0, 1)) {
                  const ns = nsMap.get(nsId);
                  pills.push({
                    label: (ns?.name || nsId).slice(0, 10),
                    color: nodeTypeStyles.memory.border,
                  });
                }
                if (memoryNamespaceIds.length > 1) {
                  pills.push({
                    label: `+${memoryNamespaceIds.length - 1}`,
                    color: nodeTypeStyles.memory.border,
                  });
                }
                for (const srvId of mcpServerIds.slice(0, 1)) {
                  const srv = srvMap.get(srvId);
                  pills.push({
                    label: (srv?.name || srvId).slice(0, 10),
                    color: nodeTypeStyles.mcp.border,
                  });
                }
                if (mcpServerIds.length > 1) {
                  pills.push({
                    label: `+${mcpServerIds.length - 1}`,
                    color: nodeTypeStyles.mcp.border,
                  });
                }
                if (searchEnabled) {
                  pills.push({
                    label: t("chat.contextGraph.legend.search"),
                    color: nodeTypeStyles.search.border,
                  });
                }
                for (const skillId of enabledSkillIds.slice(0, 1)) {
                  const sk = skillMap.get(skillId);
                  pills.push({
                    label: (sk?.name || skillId).slice(0, 10),
                    color: nodeTypeStyles.skill.border,
                  });
                }
                if (enabledSkillIds.length > 1) {
                  pills.push({
                    label: `+${enabledSkillIds.length - 1}`,
                    color: nodeTypeStyles.skill.border,
                  });
                }
                return pills.map((p, _i) => (
                  <span
                    key={p.label}
                    style={{
                      fontSize: 10,
                      padding: "0 5px",
                      borderRadius: 8,
                      border: `1px solid ${p.color}`,
                      color: p.color,
                      whiteSpace: "nowrap",
                      lineHeight: "18px",
                    }}
                  >
                    {p.label}
                  </span>
                ));
              })()}
            </div>
          )}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {/* Legend — only when expanded, click to toggle visibility */}
          {!collapsed && (
            <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
              {Object.entries(nodeTypeStyles).map(([type, style]) => {
                const isHidden = hiddenTypes.has(type);
                const label = type === "conversation"
                  ? t("chat.contextGraph.legend.conversation")
                  : type === "model"
                  ? t("chat.contextGraph.legend.model")
                  : type === "knowledge"
                  ? t("chat.contextGraph.legend.knowledge")
                  : type === "memory"
                  ? t("chat.contextGraph.legend.memory")
                  : type === "mcp"
                  ? t("chat.contextGraph.legend.mcp")
                  : type === "search"
                  ? t("chat.contextGraph.legend.search")
                  : t("chat.contextGraph.legend.skill");
                return (
                  <span
                    key={type}
                    role="button"
                    tabIndex={0}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        toggleType(type, e as unknown as React.MouseEvent);
                      }
                    }}
                    onClick={(e) => toggleType(type, e)}
                    style={{
                      display: "inline-flex",
                      alignItems: "center",
                      gap: 3,
                      fontSize: 10,
                      color: isHidden
                        ? token.colorTextQuaternary
                        : style.border,
                      cursor: "pointer",
                      opacity: isHidden ? 0.4 : 1,
                      padding: "1px 4px",
                      borderRadius: 4,
                      transition: "opacity 0.15s",
                      userSelect: "none",
                    }}
                    title={isHidden
                      ? t("chat.contextGraph.showType")
                      : t("chat.contextGraph.hideType")}
                  >
                    {style.icon} {label}
                  </span>
                );
              })}
            </div>
          )}
          {collapsed
            ? (
              <ChevronDown
                size={14}
                style={{ color: token.colorTextSecondary }}
              />
            )
            : <ChevronUp size={14} style={{ color: token.colorTextSecondary }} />}
        </div>
      </div>

      {/* Graph canvas — only when expanded */}
      {!collapsed && (
        <div style={{ height: 280, width: "100%" }}>
          {totalSources > 0
            ? (
              <GraphCanvas
                nodes={visibleNodes}
                edges={visibleEdges}
                nodeTypes={nodeTypes}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                token={token}
              />
            )
            : (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  height: "100%",
                  color: token.colorTextQuaternary,
                  fontSize: 13,
                }}
              >
                {t("chat.contextGraph.empty")}
              </div>
            )}
        </div>
      )}
    </div>
  );
});

// ── Inner graph canvas (separate component so useReactFlow is safe) ──────

interface GraphCanvasProps {
  nodes: Node[];
  edges: Edge[];
  nodeTypes: NodeTypes;
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  token: GlobalToken;
}

/// 放在 <ReactFlow> 内部，筛选变化时自动 fitView
function FitViewOnNodeChange({ nodeCount }: { nodeCount: number }) {
  const { fitView } = useReactFlow();
  const prevRef = React.useRef(nodeCount);
  const timerRef = React.useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  React.useEffect(() => {
    if (nodeCount !== prevRef.current) {
      prevRef.current = nodeCount;
      timerRef.current = setTimeout(() => fitView({ padding: 0.3 }), 50);
    }
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [nodeCount, fitView]);
  return null;
}

function GraphCanvas({
  nodes,
  edges,
  nodeTypes,
  onNodesChange,
  onEdgesChange,
  token,
}: GraphCanvasProps) {
  const nodeTypeStyles = getNodeTypeStyles(token);
  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      fitView
      fitViewOptions={{ padding: 0.3 }}
      attributionPosition="bottom-left"
      nodesDraggable={false}
      nodesConnectable={false}
      elementsSelectable={false}
      proOptions={{ hideAttribution: true }}
    >
      <Background color={token.colorBorderSecondary} gap={16} />
      <Controls showInteractive={false} />
      <MiniMap
        style={{ height: 60 }}
        nodeColor={(n: Node) => {
          const nodeData = n.data as { nodeType?: ContextNodeType } | undefined;
          const style = nodeData?.nodeType
            ? nodeTypeStyles[nodeData.nodeType]
            : undefined;
          return style?.border || "#ddd";
        }}
      />
      <FitViewOnNodeChange nodeCount={nodes.length} />
    </ReactFlow>
  );
}
