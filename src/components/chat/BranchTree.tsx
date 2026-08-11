// SPDX-License-Identifier: AGPL-3.0-only

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
  type OnEdgesChange,
  type OnNodesChange,
  Position,
  ReactFlow,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from "@xyflow/react";
import { Empty, Tag, theme, Typography } from "antd";
import { ChevronDown, ChevronUp, GitBranch, Sparkles } from "lucide-react";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import "@xyflow/react/dist/style.css";

import type { ConversationBranch } from "@/types";

// ── Types ───────────────────────────────────────────────────────────────

type BranchNodeData = {
  label: string;
  detail?: string;
  isActive: boolean;
  isMain: boolean;
  branchIndex: number;
  branchId: string;
  parentMessageId: string;
};

// ── Custom Node Component ────────────────────────────────────────────────

function getBranchColors(token: ReturnType<typeof theme.useToken>["token"]) {
  return {
    main: {
      bg: token.colorPrimaryBg,
      border: token.colorPrimary,
      text: token.colorPrimary,
    },
    branch: {
      bg: "rgba(114,46,209,0.08)",
      border: "#722ed1",
      text: "#722ed1",
    },
    active: {
      ring: token.colorPrimary,
    },
  };
}

function BranchNode({ data }: NodeProps) {
  const nodeData = data as BranchNodeData;
  const { token } = theme.useToken();
  const colors = getBranchColors(token);
  const typeColors = nodeData.isMain ? colors.main : colors.branch;

  return (
    <div
      style={{
        padding: "10px 14px",
        borderRadius: token.borderRadiusLG,
        border: `2px solid ${typeColors.border}`,
        backgroundColor: nodeData.isActive
          ? `${typeColors.border}22`
          : typeColors.bg,
        fontSize: 12,
        minWidth: 140,
        maxWidth: 200,
        cursor: "pointer",
        boxShadow: nodeData.isActive
          ? `0 0 0 2px ${colors.active.ring}44, 0 2px 8px ${typeColors.border}33`
          : "0 1px 3px rgba(0,0,0,0.06)",
        transition: "box-shadow 0.2s, transform 0.15s",
        transform: nodeData.isActive ? "scale(1.02)" : "scale(1)",
      }}
      data-branch-id={nodeData.branchId}
      data-branch-label={nodeData.label}
    >
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: typeColors.border, width: 6, height: 6 }}
      />
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 6,
          marginBottom: 4,
        }}
      >
        <GitBranch size={14} style={{ color: typeColors.text }} />
        <Typography.Text
          strong
          style={{ fontSize: 13, color: typeColors.text }}
          ellipsis
        >
          {nodeData.label}
        </Typography.Text>
        {nodeData.isActive && (
          <Tag color="green" style={{ margin: 0, fontSize: 10 }}>
            Active
          </Tag>
        )}
      </div>
      {nodeData.detail && (
        <Typography.Text
          type="secondary"
          style={{ fontSize: 11, color: token.colorTextSecondary }}
          ellipsis
        >
          {nodeData.detail}
        </Typography.Text>
      )}
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: typeColors.border, width: 6, height: 6 }}
      />
    </div>
  );
}

const nodeTypes: NodeTypes = { branchNode: BranchNode };

// ── Layout ──────────────────────────────────────────────────────────────

function buildBranchTree(
  branches: ConversationBranch[],
  activeBranchId: string | null | undefined,
  mainConversationId: string,
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = [];
  const edges: Edge[] = [];

  // Main conversation (root)
  const mainNodeId = `main_${mainConversationId}`;
  nodes.push({
    id: mainNodeId,
    type: "branchNode",
    position: { x: 0, y: 0 },
    data: {
      label: "Main",
      detail: mainConversationId.slice(0, 8),
      isActive: !activeBranchId,
      isMain: true,
      branchIndex: 0,
      branchId: mainConversationId,
      parentMessageId: "",
    },
  });

  if (branches.length === 0) {
    return { nodes, edges };
  }

  // Sort branches by branch_index
  const sorted = [...branches].sort(
    (a, b) => a.branchIndex - b.branchIndex,
  );

  // Group branches by parent_message_id to build tree
  const childrenByParent = new Map<string, ConversationBranch[]>();
  for (const branch of sorted) {
    const key = branch.parentMessageId || "root";
    if (!childrenByParent.has(key)) {
      childrenByParent.set(key, []);
    }
    childrenByParent.get(key)!.push(branch);
  }

  // Layout: breadth-first, left-to-right
  const layoutNodes: { id: string; x: number; y: number; depth: number }[] = [
    { id: mainNodeId, x: 0, y: 0, depth: 0 },
  ];

  const visited = new Set<string>();

  function layoutChildren(
    parentId: string,
    parentX: number,
    parentDepth: number,
  ) {
    const children = childrenByParent.get(
      parentId === mainNodeId ? "root" : parentId,
    ) || childrenByParent.get(parentId) || [];

    if (children.length === 0) { return; }

    const childSpacing = 200;
    const totalWidth = (children.length - 1) * childSpacing;
    const startX = parentX - totalWidth / 2;

    children.forEach((child, idx) => {
      if (visited.has(child.id)) { return; }
      visited.add(child.id);

      const x = startX + idx * childSpacing;
      const y = (parentDepth + 1) * 140;

      layoutNodes.push({
        id: child.id,
        x,
        y,
        depth: parentDepth + 1,
      });

      nodes.push({
        id: child.id,
        type: "branchNode",
        position: { x, y },
        data: {
          label: child.branchLabel || `Branch ${child.branchIndex}`,
          detail: `#${child.branchIndex} · ${
            child.createdAt?.slice(
              0,
              10,
            ) ?? ""
          }`,
          isActive: activeBranchId === child.id,
          isMain: false,
          branchIndex: child.branchIndex,
          branchId: child.id,
          parentMessageId: child.parentMessageId,
        },
      });

      edges.push({
        id: `${parentId}-${child.id}`,
        source: parentId,
        target: child.id,
        type: "smoothstep",
        animated: false,
        style: {
          stroke: child.id === activeBranchId ? "#722ed1" : "#999",
          strokeWidth: child.id === activeBranchId ? 2 : 1.5,
          opacity: child.id === activeBranchId ? 1 : 0.7,
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 6,
          height: 6,
          color: "#999",
        },
      });

      layoutChildren(child.id, x, parentDepth + 1);
    });
  }

  visited.add(mainNodeId);
  layoutChildren(mainNodeId, 0, 0);

  // If some branches have parentMessageId that doesn't match any known node,
  // attach them to the main node
  for (const branch of sorted) {
    if (!visited.has(branch.id)) {
      const y = 140;
      const x = (nodes.length - 1) * 200;
      nodes.push({
        id: branch.id,
        type: "branchNode",
        position: { x, y },
        data: {
          label: branch.branchLabel || `Branch ${branch.branchIndex}`,
          detail: `#${branch.branchIndex} · orphan`,
          isActive: activeBranchId === branch.id,
          isMain: false,
          branchIndex: branch.branchIndex,
          branchId: branch.id,
          parentMessageId: branch.parentMessageId,
        },
      });
      edges.push({
        id: `${mainNodeId}-${branch.id}`,
        source: mainNodeId,
        target: branch.id,
        type: "smoothstep",
        animated: false,
        style: {
          stroke: branch.id === activeBranchId ? "#722ed1" : "#999",
          strokeWidth: 1.5,
          opacity: 0.7,
        },
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 6,
          height: 6,
          color: "#999",
        },
      });
    }
  }

  return { nodes, edges };
}

// ── Component ────────────────────────────────────────────────────────────

interface BranchTreeProps {
  conversationId: string;
  branches: ConversationBranch[];
  activeBranchId: string | null | undefined;
  onSelectBranch?: (branchId: string | null) => void;
}

export const BranchTree = React.memo(function BranchTree({
  conversationId,
  branches,
  activeBranchId,
  onSelectBranch,
}: BranchTreeProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [collapsed, setCollapsed] = useState(false);

  const { nodes: initialNodes, edges: initialEdges } = useMemo(
    () => buildBranchTree(branches, activeBranchId, conversationId),
    [branches, activeBranchId, conversationId],
  );

  const [rfNodes, setRfNodes, onNodesChange] = useNodesState(initialNodes);
  const [rfEdges, setRfEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Sync when data changes
  const prevDataRef = React.useRef("");
  useEffect(() => {
    const dataKey = JSON.stringify({ initialNodes, initialEdges });
    if (dataKey === prevDataRef.current) {
      return;
    }
    prevDataRef.current = dataKey;
    setRfNodes(initialNodes);
    setRfEdges(initialEdges);
  }, [initialNodes, initialEdges, setRfNodes, setRfEdges]);

  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      if (!onSelectBranch) { return; }
      const data = node.data as BranchNodeData;
      if (data.isMain) {
        onSelectBranch(null);
      } else {
        onSelectBranch(data.branchId);
      }
    },
    [onSelectBranch],
  );

  const branchCount = branches.length;

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
          <Sparkles size={14} style={{ color: token.colorPrimary }} />
          <Typography.Text strong style={{ fontSize: 13 }}>
            {t("chat.branchTree.title")}
          </Typography.Text>
          <Typography.Text type="secondary" style={{ fontSize: 12 }}>
            {t("chat.branchTree.branchCount", { count: branchCount })}
          </Typography.Text>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {collapsed
            ? <ChevronDown size={14} style={{ color: token.colorTextSecondary }} />
            : <ChevronUp size={14} style={{ color: token.colorTextSecondary }} />}
        </div>
      </div>

      {/* Graph canvas */}
      {!collapsed && (
        <div style={{ height: 240, width: "100%" }}>
          {branchCount > 0
            ? (
              <BranchTreeCanvas
                nodes={rfNodes}
                edges={rfEdges}
                nodeTypes={nodeTypes}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onNodeClick={onNodeClick}
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
                <Empty description={t("chat.branchTree.noBranches")} />
              </div>
            )}
        </div>
      )}
    </div>
  );
});

// ── Inner canvas component ──────────────────────────────────────────────

interface BranchTreeCanvasProps {
  nodes: Node[];
  edges: Edge[];
  nodeTypes: NodeTypes;
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onNodeClick: (event: React.MouseEvent, node: Node) => void;
  token: ReturnType<typeof theme.useToken>["token"];
}

function FitViewOnUpdate({ nodeCount }: { nodeCount: number }) {
  const { fitView } = useReactFlow();
  const fitViewRef = React.useRef(fitView);
  fitViewRef.current = fitView;
  const prevRef = React.useRef(nodeCount);
  const timerRef = React.useRef<ReturnType<typeof setTimeout>>(undefined);

  React.useEffect(() => {
    if (nodeCount !== prevRef.current) {
      prevRef.current = nodeCount;
      timerRef.current = setTimeout(
        () => fitViewRef.current({ padding: 0.4 }),
        50,
      );
    }
    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [nodeCount]);
  return null;
}

function BranchTreeCanvas({
  nodes,
  edges,
  nodeTypes,
  onNodesChange,
  onEdgesChange,
  onNodeClick,
  token,
}: BranchTreeCanvasProps) {
  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onNodeClick={onNodeClick}
      fitView
      fitViewOptions={{ padding: 0.4 }}
      attributionPosition="bottom-left"
      nodesDraggable={true}
      nodesConnectable={false}
      elementsSelectable={true}
      proOptions={{ hideAttribution: true }}
      minZoom={0.3}
      maxZoom={2}
    >
      <Background color={token.colorBorderSecondary} gap={16} />
      <Controls showInteractive={false} />
      <MiniMap
        style={{ height: 60 }}
        nodeColor={(n: Node) => {
          const data = n.data as BranchNodeData | undefined;
          if (data?.isActive) { return "#52c41a"; }
          if (data?.isMain) { return token.colorPrimary; }
          return "#722ed1";
        }}
      />
      <FitViewOnUpdate nodeCount={nodes.length} />
    </ReactFlow>
  );
}
