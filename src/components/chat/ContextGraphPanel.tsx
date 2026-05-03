import { theme, Typography } from "antd";
import { BookOpen, Brain, GitBranch, Link2, Puzzle, Search, Wrench, Zap } from "lucide-react";
import React, { useMemo } from "react";
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
  Position,
  ReactFlow,
  useEdgesState,
  useNodesState,
} from "reactflow";
import "reactflow/dist/style.css";
import { useKnowledgeStore, useMcpStore, useMemoryStore, useSkillExtensionStore } from "@/stores";

// ── Types ────────────────────────────────────────────────────────────────

type ContextNodeType = "conversation" | "model" | "knowledge" | "memory" | "mcp" | "search" | "skill";

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

const nodeTypeStyles: Record<ContextNodeType, { icon: React.ReactNode; bg: string; border: string }> = {
  conversation: { icon: <GitBranch size={12} />, bg: "rgba(24,144,255,0.08)", border: "#1890ff" },
  model: { icon: <Zap size={12} />, bg: "rgba(114,46,209,0.08)", border: "#722ed1" },
  knowledge: { icon: <BookOpen size={12} />, bg: "rgba(82,196,26,0.08)", border: "#52c41a" },
  memory: { icon: <Brain size={12} />, bg: "rgba(250,140,22,0.08)", border: "#fa8c16" },
  mcp: { icon: <Wrench size={12} />, bg: "rgba(19,194,194,0.08)", border: "#13c2c2" },
  search: { icon: <Search size={12} />, bg: "rgba(47,84,235,0.08)", border: "#2f54eb" },
  skill: { icon: <Puzzle size={12} />, bg: "rgba(235,47,150,0.08)", border: "#eb2f96" },
};

function ContextNode({ data }: { data: { label: string; detail?: string; nodeType: ContextNodeType } }) {
  const { token } = theme.useToken();
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
      <Handle type="target" position={Position.Top} style={{ background: style.border }} />
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span style={{ color: style.border, display: "flex" }}>{style.icon}</span>
        <Typography.Text strong style={{ fontSize: 12 }} ellipsis>
          {data.label}
        </Typography.Text>
      </div>
      {data.detail && (
        <Typography.Text type="secondary" style={{ fontSize: 10 }} ellipsis>
          {data.detail}
        </Typography.Text>
      )}
      <Handle type="source" position={Position.Bottom} style={{ background: style.border }} />
    </div>
  );
}

const nodeTypes: NodeTypes = { contextNode: ContextNode };

// ── Layout helper ────────────────────────────────────────────────────────

function layoutGraph(nodes: ContextGraphNode[], edges: ContextGraphEdge[]): { nodes: Node[]; edges: Edge[] } {
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
      data: { label: convNode.label, detail: convNode.detail, nodeType: convNode.type },
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
      markerEnd: { type: MarkerType.ArrowClosed, width: 8, height: 8, color: "#888" },
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

  // Get detail info from various stores
  const knowledgeBases = useKnowledgeStore((s) => s.bases ?? []);
  const memoryNamespaces = useMemoryStore((s) => s.namespaces ?? []);
  const mcpServers = useMcpStore((s) => s.servers ?? []);
  const installedSkills = useSkillExtensionStore((s) => s.skills ?? []);

  const graphData = useMemo(() => {
    const nodes: ContextGraphNode[] = [];
    const edges: ContextGraphEdge[] = [];

    // Conversation node (center)
    const convName = conversationTitle || conversationId?.slice(0, 8)
      || t("chat.contextGraph.conversation", "当前对话");
    nodes.push({ id: "conversation", type: "conversation", label: convName, detail: conversationId?.slice(0, 16) });

    // Model node
    if (modelName) {
      const modelLabel = providerName ? `${providerName} / ${modelName}` : modelName;
      nodes.push({ id: "model", type: "model", label: modelLabel });
      edges.push({ source: "conversation", target: "model", label: "使用" });
    }

    // Knowledge bases
    for (const kbId of knowledgeBaseIds) {
      const kb = knowledgeBases.find((k: any) => k.id === kbId);
      const label = kb?.name || kbId.slice(0, 12);
      nodes.push({ id: `kb:${kbId}`, type: "knowledge", label, detail: kb?.description });
      edges.push({ source: "conversation", target: `kb:${kbId}`, label: "检索" });
    }

    // Memory namespaces
    for (const nsId of memoryNamespaceIds) {
      const ns = memoryNamespaces.find((n: any) => n.id === nsId);
      const label = ns?.name || nsId.slice(0, 12);
      nodes.push({ id: `mem:${nsId}`, type: "memory", label });
      edges.push({ source: "conversation", target: `mem:${nsId}`, label: "读写" });
    }

    // MCP servers
    for (const srvId of mcpServerIds) {
      const srv = mcpServers.find((s: any) => s.id === srvId);
      const label = srv?.name || srvId.slice(0, 12);
      nodes.push({ id: `mcp:${srvId}`, type: "mcp", label });
      edges.push({ source: "conversation", target: `mcp:${srvId}`, label: "调用" });
    }

    // Search
    if (searchEnabled) {
      nodes.push({ id: "search", type: "search", label: t("chat.context.search", "联网搜索") });
      edges.push({ source: "conversation", target: "search", label: "搜索" });
    }

    // Skills
    for (const skillId of enabledSkillIds) {
      const sk = installedSkills.find((s: any) => s.id === skillId);
      const label = sk?.name || skillId.slice(0, 12);
      nodes.push({ id: `skill:${skillId}`, type: "skill", label });
      edges.push({ source: "conversation", target: `skill:${skillId}`, label: "启用" });
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

  // Update nodes/edges when layout changes
  const prevLayoutRef = React.useRef<string>("");
  const layoutKey = JSON.stringify(layout);
  if (layoutKey !== prevLayoutRef.current) {
    prevLayoutRef.current = layoutKey;
    setTimeout(() => {
      setRfNodes(layout.nodes);
      setRfEdges(layout.edges);
    }, 0);
  }

  const totalSources = knowledgeBaseIds.length + memoryNamespaceIds.length + mcpServerIds.length
    + (searchEnabled ? 1 : 0) + enabledSkillIds.length;

  return (
    <div
      style={{
        border: `1px solid ${token.colorBorderSecondary}`,
        borderRadius: token.borderRadius,
        overflow: "hidden",
        marginBottom: 12,
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "8px 12px",
          backgroundColor: token.colorFillQuaternary,
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Link2 size={14} style={{ color: token.colorPrimary }} />
          <Typography.Text strong style={{ fontSize: 13 }}>
            {t("chat.contextGraph.title", "上下文图谱")}
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 11 }}>
            {totalSources} 个上下文源
          </Typography.Text>
        </div>
        {/* Legend */}
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
          {Object.entries(nodeTypeStyles).slice(0, 5).map(([type, style]) => (
            <span
              key={type}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 3,
                fontSize: 10,
                color: style.border,
              }}
            >
              {style.icon} {type === "conversation"
                ? "对话"
                : type === "model"
                ? "模型"
                : type === "knowledge"
                ? "知识"
                : type === "memory"
                ? "记忆"
                : "MCP"}
            </span>
          ))}
        </div>
      </div>

      {/* Graph canvas */}
      <div style={{ height: 280, width: "100%" }}>
        {totalSources > 0
          ? (
            <ReactFlow
              nodes={rfNodes}
              edges={rfEdges}
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
                  const style = nodeData?.nodeType ? nodeTypeStyles[nodeData.nodeType] : undefined;
                  return style?.border || "#ddd";
                }}
              />
            </ReactFlow>
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
              {t("chat.contextGraph.empty", "未启用上下文源")}
            </div>
          )}
      </div>
    </div>
  );
});
