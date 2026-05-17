// Agent 层级树 — 展示 fork/子 Agent 的父子关系

import { useExecutionStore } from "@/stores/feature/executionStore";

const _EMPTY: never[] = [];
import type { AgentPoolItem } from "@/types";
import { BranchesOutlined, CaretRightOutlined, RobotOutlined } from "@ant-design/icons";
import { Tag, Tree, Typography } from "antd";
import type { DataNode } from "antd/es/tree";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

interface AgentHierarchyTreeProps {
  conversationId: string;
}

// 将 AgentPoolItem 转换为 Tree DataNode
function toTreeNode(
  item: AgentPoolItem & { isFork?: boolean },
  allItems: AgentPoolItem[],
  t: (key: string, options?: Record<string, unknown>) => string,
): DataNode {
  const isFork = item.isFork;
  const children = allItems
    .filter((child) => child.dependsOn?.includes(item.id))
    .map((child) => toTreeNode(child, allItems, t));

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
            ? t("agentHierarchy.running")
            : item.status === "completed"
            ? t("agentHierarchy.completed")
            : item.status === "failed"
            ? t("agentHierarchy.failed")
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
  const { t } = useTranslation();
  const pool = useExecutionStore((s) => s.agentPool[conversationId] || _EMPTY);

  const treeData = useMemo(() => {
    const roots = pool.filter((item) => !item.dependsOn || item.dependsOn.length === 0);
    if (roots.length === 0 && pool.length > 0) {
      return pool.slice(0, 2).map((item) => toTreeNode(item, pool, t));
    }
    return roots.map((item) => toTreeNode(item, pool, t));
  }, [pool, t]);

  if (treeData.length === 0) { return null; }

  return (
    <div style={{ padding: "8px 12px", borderBottom: "1px solid #f0f0f0", background: "#fafafa" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
        <BranchesOutlined style={{ fontSize: 13, color: "#722ed1" }} />
        <Text style={{ fontSize: 12, fontWeight: 600 }}>{t("agentHierarchy.title")}</Text>
        <Text type="secondary" style={{ fontSize: 11 }}>
          {t("agentHierarchy.count", { count: pool.length })}
        </Text>
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
