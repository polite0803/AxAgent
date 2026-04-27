import { CheckCircleOutlined, ThunderboltOutlined, ToolOutlined, WarningOutlined } from "@ant-design/icons";
import { Table, Tag } from "antd";
import React from "react";
import type { ToolDependency } from "../../types";

interface ToolDependencyListProps {
  dependencies: ToolDependency[];
  onAction?: (dep: ToolDependency) => void;
  actionLoading?: string | null;
}

const STATUS_CONFIG: Record<string, { color: string; label: string; icon: React.ReactNode }> = {
  satisfied: { color: "success", label: "已满足", icon: <CheckCircleOutlined /> },
  auto_installable: { color: "processing", label: "可自动安装", icon: <ToolOutlined /> },
  manual_installable: { color: "warning", label: "需手动安装", icon: <WarningOutlined /> },
  needs_generation: { color: "error", label: "需生成", icon: <ThunderboltOutlined /> },
};

export const ToolDependencyList: React.FC<ToolDependencyListProps> = ({
  dependencies,
  onAction,
  actionLoading,
}) => {
  const columns = [
    { title: "工具名称", dataIndex: "name", key: "name" },
    {
      title: "类型",
      dataIndex: "tool_type",
      key: "tool_type",
      width: 80,
      render: (t: string) => <Tag>{t}</Tag>,
    },
    {
      title: "状态",
      dataIndex: "status",
      key: "status",
      width: 120,
      render: (status: string) => {
        const config = STATUS_CONFIG[status] || { color: "default", label: status, icon: null };
        return <Tag color={config.color} icon={config.icon}>{config.label}</Tag>;
      },
    },
    {
      title: "安装说明",
      dataIndex: "install_instructions",
      key: "install_instructions",
      ellipsis: true,
      render: (text: string) => text || "-",
    },
    {
      title: "操作",
      key: "action",
      width: 100,
      render: (_: unknown, record: ToolDependency) => {
        const isSatisfied = record.status === "satisfied";
        if (isSatisfied) { return null; }
        if (!onAction) { return null; }
        return (
          <a onClick={() => onAction(record)} style={{ fontSize: 12 }}>
            {actionLoading === record.name ? "处理中..." : "处理"}
          </a>
        );
      },
    },
  ];

  return (
    <Table
      dataSource={dependencies}
      columns={columns}
      rowKey="name"
      size="small"
      pagination={false}
    />
  );
};
