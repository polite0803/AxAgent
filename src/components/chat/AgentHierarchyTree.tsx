// Agent 层级树 — 展示 fork/子 Agent 的父子关系

import { useAgentStore } from "@/stores";

const _EMPTY: never[] = [];
import type { AgentPoolItem } from "@/types/agent";
import { BranchesOutlined, CaretRightOutlined, RobotOutlined } from "@ant-design/icons";
import { Tag, Tree, Typography } from "antd";
import type { DataNode } from "antd/es/tree";
import { useMemo } from "react";

const { Text } = Typography;

interface AgentHierarchyTreeProps {
  conversationId: string;
}

// 将 AgentPoolItem 转换为 Tree DataNode
function toTreeNode(item: AgentPoolItem, allItems: AgentPoolItem[]): DataNode {
  const isFork = (item as { isFork?: boolean }).isFork;
  const children = allItems
    .filter((child) => child.dependsOn?.includes(item.id))
    .map((child) => toTreeNode(child, allItems));

  return {
    key: item.id,
    title: (
      <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
        {isFork
          ? <BranchesOutlined style={{ color: "#722ed1", fontSize: 13 }} />
          : <RobotOutlined style={{ color: "#1890ff", fontSize: 13 }} />}
        <Text style={{ fontSize: 13 }}>{item.name}</Text>
        <Tag
          color={item.status === "running"
            ? "processing"
            : item.status === "completed"
            ? "success"
            : item.status === "failed"
            ? "error"
            : "default"}
          style={{ fontSize: 10, lineHeight: "16px", margin: 0 }}
        >
          {item.status === "running"
            ? "运行中"
            : item.status === "completed"
            ? "完成"
            : item.status === "failed"
            ? "失败"
            : item.status}
        </Tag>
        {item.agentType && item.agentType !== "general-purpose" && (
          <Text type="secondary" style={{ fontSize: 11 }}>{item.agentType}</Text>
        )}
      </span>
    ),
    icon: isFork ? <BranchesOutlined /> : item.type === "worker" ? undefined : <CaretRightOutlined />,
    children: children.length > 0 ? children : undefined,
    selectable: false,
  };
}

export function AgentHierarchyTree({ conversationId }: AgentHierarchyTreeProps) {
  const pool = useAgentStore((s) => s.agentPool[conversationId] || _EMPTY);

  const treeData = useMemo(() => {
    // 找根节点（没有 dependsOn 或 dependsOn 为空的节点）
    const roots = pool.filter((item) => !item.dependsOn || item.dependsOn.length === 0);
    if (roots.length === 0 && pool.length > 0) {
      // 如果所有节点都有依赖，取前两个作为根
      return pool.slice(0, 2).map((item) => toTreeNode(item, pool));
    }
    return roots.map((item) => toTreeNode(item, pool));
  }, [pool]);

  if (treeData.length === 0) { return null; }

  return (
    <div style={{ padding: "8px 12px", borderBottom: "1px solid #f0f0f0", background: "#fafafa" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
        <BranchesOutlined style={{ fontSize: 13, color: "#722ed1" }} />
        <Text style={{ fontSize: 12, fontWeight: 600 }}>Agent 层级</Text>
        <Text type="secondary" style={{ fontSize: 11 }}>({pool.length} 个)</Text>
      </div>
      {treeData.length > 0 && (
        <Tree
          treeData={treeData}
          defaultExpandAll
          showIcon
          blockNode
          style={{ fontSize: 12, background: "transparent" }}
        />
      )}
    </div>
  );
}
