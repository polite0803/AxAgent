import { useLocalToolStore } from "@/stores";
import { Spin, Switch, Tabs, Tag, Typography } from "antd";
import {
  BookOpen,
  Brain,
  FileEdit,
  FileSearch,
  Globe,
  HardDrive,
  MessageSquare,
  Search,
  Terminal,
  Wrench,
  Zap,
} from "lucide-react";
import { useEffect } from "react";
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
