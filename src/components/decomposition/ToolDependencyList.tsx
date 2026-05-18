import { CheckCircleOutlined, ThunderboltOutlined, ToolOutlined, WarningOutlined } from "@ant-design/icons";
import { Button, Table, Tag } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { ToolDependency } from "../../types";

interface ToolDependencyListProps {
  dependencies: ToolDependency[];
  onAction?: (dep: ToolDependency) => void;
  actionLoading?: string | null;
}

export const ToolDependencyList: React.FC<ToolDependencyListProps> = ({
  dependencies,
  onAction,
  actionLoading,
}) => {
  const { t } = useTranslation();

  const STATUS_CONFIG: Record<string, { color: string; label: string; icon: React.ReactNode }> = {
    satisfied: { color: "success", label: t("decomposition.statusSatisfied"), icon: <CheckCircleOutlined /> },
    auto_installable: { color: "processing", label: t("decomposition.statusAutoInstallable"), icon: <ToolOutlined /> },
    manual_installable: {
      color: "warning",
      label: t("decomposition.statusManualInstallable"),
      icon: <WarningOutlined />,
    },
    needs_generation: {
      color: "error",
      label: t("decomposition.statusNeedsGeneration"),
      icon: <ThunderboltOutlined />,
    },
  };

  const columns = [
    { title: t("decomposition.toolName"), dataIndex: "name", key: "name" },
    {
      title: t("decomposition.colType"),
      dataIndex: "tool_type",
      key: "tool_type",
      width: 80,
      render: (t: string) => <Tag>{t}</Tag>,
    },
    {
      title: t("decomposition.colStatus"),
      dataIndex: "status",
      key: "status",
      width: 120,
      render: (status: string) => {
        const config = STATUS_CONFIG[status] || { color: "default", label: status, icon: null };
        return <Tag color={config.color} icon={config.icon}>{config.label}</Tag>;
      },
    },
    {
      title: t("decomposition.colInstallInstructions"),
      dataIndex: "install_instructions",
      key: "install_instructions",
      ellipsis: true,
      render: (text: string) => text || "-",
    },
    {
      title: t("decomposition.colAction"),
      key: "action",
      width: 100,
      render: (_: unknown, record: ToolDependency) => {
        const isSatisfied = record.status === "satisfied";
        if (isSatisfied) { return null; }
        if (!onAction) { return null; }
        return (
          <Button type="link" size="small" onClick={() => onAction(record)}>
            {actionLoading === record.name ? t("decomposition.processingLabel") : t("decomposition.processLabel")}
          </Button>
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
