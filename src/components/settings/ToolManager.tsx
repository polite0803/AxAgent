import { useGeneratedToolStore, useLocalToolStore } from "@/stores";
import type { GeneratedToolInfo } from "@/types";
import { Button, Empty, message, Popconfirm, Spin, Switch, Table, Tabs, Tag, Typography } from "antd";
import {
  BookOpen,
  Brain,
  Code,
  FileEdit,
  FileSearch,
  Globe,
  HardDrive,
  MessageSquare,
  RefreshCw,
  Search,
  Terminal,
  Trash2,
  Wrench,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import McpServerSettings from "./McpServerSettings";
import ToolSemanticCheck from "./ToolSemanticCheck";

const { Text, Paragraph } = Typography;

// ── Builtin Tool Group Icons ──────────────────────────────

const GROUP_ICONS: Record<string, React.ReactNode> = {
  "builtin-fetch": <Globe size={18} />,
  "builtin-search-file": <FileSearch size={18} />,
  "builtin-filesystem": <FileEdit size={18} />,
  "builtin-system": <Terminal size={18} />,
  "builtin-search": <Search size={18} />,
  "builtin-knowledge": <BookOpen size={18} />,
  "builtin-storage": <HardDrive size={18} />,
  "builtin-skills": <Wrench size={18} />,
  "builtin-session": <MessageSquare size={18} />,
  "builtin-memory": <Brain size={18} />,
};

const GROUP_NAME_KEYS: Record<string, string> = {
  "builtin-fetch": "settings.localTools.groupFetch",
  "builtin-search-file": "settings.localTools.groupSearchFile",
  "builtin-filesystem": "settings.localTools.groupFilesystem",
  "builtin-system": "settings.localTools.groupSystem",
  "builtin-search": "settings.localTools.groupSearch",
  "builtin-knowledge": "settings.localTools.groupKnowledge",
  "builtin-storage": "settings.localTools.groupStorage",
  "builtin-skills": "settings.localTools.groupSkills",
  "builtin-session": "settings.localTools.groupSession",
  "builtin-memory": "settings.localTools.groupMemory",
};

// ── Tab: Builtin Tools ────────────────────────────────────

function BuiltinToolsTab() {
  const { t } = useTranslation();
  const { groups, loading, loadGroups, toggleGroup } = useLocalToolStore();

  useEffect(() => {
    loadGroups();
  }, [loadGroups]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48">
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div className="max-w-2xl">
      <Paragraph type="secondary" className="mb-4">
        {t("settings.localTools.description")}
      </Paragraph>

      <div className="border border-border rounded-lg overflow-hidden">
        {groups.map((group) => {
          const icon = GROUP_ICONS[group.groupId] ?? <Wrench size={18} />;
          const nameKey = GROUP_NAME_KEYS[group.groupId];
          const displayName = nameKey ? t(nameKey) : group.groupName;

          return (
            <div
              key={group.groupId}
              className="flex items-center justify-between py-3 px-4 border-b border-border last:border-b-0"
            >
              <div className="flex items-center gap-3 min-w-0 flex-1">
                <span className="text-text-secondary shrink-0">{icon}</span>
                <div className="min-w-0 flex-1">
                  <Text strong className="block">{displayName}</Text>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {group.tools.map((tool) => (
                      <Tag key={tool.toolName} className="text-xs">
                        {tool.toolName}
                      </Tag>
                    ))}
                  </div>
                </div>
              </div>
              <Switch
                checked={group.enabled}
                onChange={() => toggleGroup(group.groupId)}
                className="shrink-0 ml-3"
              />
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ── Tab: Generated Tools ─────────────────────────────────

function GeneratedToolsTab() {
  const { t } = useTranslation();
  const { tools, loading, loadTools, deleteTool } = useGeneratedToolStore();
  const [deletingId, setDeletingId] = useState<string | null>(null);

  useEffect(() => {
    loadTools();
  }, [loadTools]);

  const handleDelete = async (id: string) => {
    setDeletingId(id);
    try {
      await deleteTool(id);
      message.success(t("settings.generatedTools.deleteSuccess"));
    } catch (e) {
      message.error(String(e));
    } finally {
      setDeletingId(null);
    }
  };

  const columns = [
    {
      title: t("settings.generatedTools.toolName"),
      dataIndex: "toolName",
      key: "toolName",
      width: 200,
      render: (name: string) => <span className="font-mono text-sm">{name}</span>,
    },
    {
      title: t("settings.generatedTools.originalName"),
      dataIndex: "originalName",
      key: "originalName",
      width: 180,
    },
    {
      title: t("settings.generatedTools.description"),
      dataIndex: "originalDescription",
      key: "originalDescription",
      ellipsis: true,
    },
    {
      title: t("settings.generatedTools.createdAt"),
      dataIndex: "createdAt",
      key: "createdAt",
      width: 160,
      render: (ts: number) => new Date(ts).toLocaleString(),
    },
    {
      title: "",
      key: "actions",
      width: 80,
      render: (_: unknown, record: GeneratedToolInfo) => (
        <Popconfirm
          title={t("settings.generatedTools.deleteConfirm")}
          onConfirm={() => handleDelete(record.id)}
          okText={t("common.confirm")}
          cancelText={t("common.cancel")}
          okButtonProps={{ danger: true }}
        >
          <Button
            type="text"
            danger
            size="small"
            icon={<Trash2 size={14} />}
            loading={deletingId === record.id}
          />
        </Popconfirm>
      ),
    },
  ];

  if (loading) {
    return (
      <div className="flex items-center justify-center h-48">
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div className="max-w-3xl">
      <Paragraph type="secondary" className="mb-4">
        {t("settings.generatedTools.description")}
      </Paragraph>

      <div className="flex items-center justify-between mb-3">
        <Typography.Text type="secondary">
          {t("settings.generatedTools.total", { count: tools.length })}
        </Typography.Text>
        <Button
          size="small"
          icon={<RefreshCw size={14} />}
          onClick={loadTools}
        >
          {t("common.refresh")}
        </Button>
      </div>

      {tools.length === 0
        ? <Empty description={t("settings.generatedTools.empty")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
        : (
          <Table
            dataSource={tools}
            columns={columns}
            rowKey="id"
            pagination={false}
            size="small"
          />
        )}
    </div>
  );
}

// ── Tab: MCP Servers ─────────────────────────────────────

function McpServersTab() {
  return <McpServerSettings />;
}

// ── Main ToolManager ─────────────────────────────────────

export default function ToolManager() {
  const { t } = useTranslation();

  const tabItems = [
    {
      key: "builtin",
      label: (
        <span className="flex items-center gap-2">
          <Wrench size={16} />
          {t("settings.tools.tabBuiltin")}
        </span>
      ),
      children: <BuiltinToolsTab />,
    },
    {
      key: "mcp",
      label: (
        <span className="flex items-center gap-2">
          <Globe size={16} />
          {t("settings.tools.tabMcp")}
        </span>
      ),
      children: <McpServersTab />,
    },
    {
      key: "generated",
      label: (
        <span className="flex items-center gap-2">
          <Code size={16} />
          {t("settings.tools.tabGenerated")}
        </span>
      ),
      children: <GeneratedToolsTab />,
    },
    {
      key: "semantic",
      label: (
        <span className="flex items-center gap-2">
          <Zap size={16} />
          {t("settings.tools.tabSemantic")}
        </span>
      ),
      children: <ToolSemanticCheck />,
    },
  ];

  return (
    <div className="p-6 h-full flex flex-col">
      <Typography.Title level={4}>
        {t("settings.tools.title")}
      </Typography.Title>
      <div className="flex-1 min-h-0" style={{ overflow: "hidden" }}>
        <Tabs
          defaultActiveKey="builtin"
          items={tabItems}
          style={{ height: "100%" }}
          tabBarStyle={{ marginBottom: 16 }}
        />
      </div>
      <style>
        {`
        .ant-tabs-content-holder, .ant-tabs-content, .ant-tabs-tabpane-active {
          height: 100% !important;
        }
      `}
      </style>
    </div>
  );
}
