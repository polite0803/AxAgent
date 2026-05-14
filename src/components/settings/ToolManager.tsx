import { useLocalToolStore } from "@/stores";
import type { LocalToolGroupInfo, LocalToolInfo } from "@/types";
import { Alert, Collapse, Empty, Spin, Switch, Tabs, Tag, Tooltip, Typography } from "antd";
import {
  BookOpen,
  Bot,
  ExternalLink,
  FileEdit,
  FileSearch,
  GitBranch,
  Globe,
  HardDrive,
  Image,
  MessageSquare,
  MousePointer,
  Shield,
  Terminal,
  Timer,
  Wrench,
  Zap,
} from "lucide-react";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { McpServerSettings } from "./McpServerSettings";
import ToolSemanticCheck from "./ToolSemanticCheck";

const { Text, Paragraph } = Typography;

const GROUP_ICONS: Record<string, React.ReactNode> = {
  "builtin-file-read": <FileSearch size={16} />,
  "builtin-file-write": <FileEdit size={16} />,
  "builtin-shell": <Terminal size={16} />,
  "builtin-network": <Globe size={16} />,
  "builtin-system-tools": <Wrench size={16} />,
  "builtin-agent": <Bot size={16} />,
  "builtin-vcs": <GitBranch size={16} />,
  "builtin-automation": <Timer size={16} />,
  "builtin-communication": <MessageSquare size={16} />,
  "builtin-ai-media": <Image size={16} />,
  "builtin-integration": <ExternalLink size={16} />,
  "builtin-storage": <HardDrive size={16} />,
  "builtin-knowledge": <BookOpen size={16} />,
  "builtin-browser": <Globe size={16} />,
  "builtin-desktop": <MousePointer size={16} />,
};

function ToolItem({
  tool,
  groupEnabled,
  onToggle,
}: {
  tool: LocalToolInfo;
  groupEnabled: boolean;
  onToggle: (name: string) => void;
}) {
  return (
    <div className="flex items-start justify-between py-2.5 px-3 border-b border-border/50 last:border-b-0 hover:bg-bg-container-hover transition-colors">
      <div className="flex-1 min-w-0 mr-3">
        <div className="flex items-center gap-1.5 flex-wrap">
          <Text strong className="text-sm">{tool.name}</Text>
          {tool.isDestructive && (
            <Tooltip title="破坏性操作——执行后不可逆">
              <Tag color="red" className="text-[10px] leading-none px-1 py-0">
                <Shield size={10} className="inline mr-0.5" />破坏性
              </Tag>
            </Tooltip>
          )}
          {tool.isReadOnly && <Tag color="green" className="text-[10px] leading-none px-1 py-0">只读</Tag>}
        </div>
        <Paragraph type="secondary" className="text-xs mt-0.5 mb-0 leading-snug" ellipsis={{ rows: 2 }}>
          {tool.description}
        </Paragraph>
      </div>
      <Tooltip title={groupEnabled ? (tool.enabled ? "禁用此工具" : "启用此工具") : "分类已禁用，无法单独控制"}>
        <Switch
          id="tool-manager-switch-177"
          size="small"
          checked={tool.enabled && groupEnabled}
          disabled={!groupEnabled}
          onChange={() => onToggle(tool.name)}
        />
      </Tooltip>
    </div>
  );
}

function GroupHeader({
  group,
  onToggleGroup,
}: {
  group: LocalToolGroupInfo;
  onToggleGroup: (id: string) => void;
}) {
  const icon = GROUP_ICONS[group.groupId] ?? <Wrench size={16} />;
  const enabledCount = group.tools.filter((t) => t.enabled).length;
  const totalCount = group.tools.length;

  return (
    <div className="flex items-center gap-3 w-full">
      <span className="text-text-secondary shrink-0">{icon}</span>
      <div className="flex-1 min-w-0">
        <Text strong>{group.groupName}</Text>
        <Text type="secondary" className="text-xs ml-2">
          {enabledCount}/{totalCount} 已启用
        </Text>
        <Paragraph type="secondary" className="text-xs mt-0.5 mb-0 leading-snug">
          {group.description}
        </Paragraph>
      </div>
      <Tooltip title={group.enabled ? "禁用整个分类" : "启用整个分类"}>
        <Switch
          id="tool-manager-switch-178"
          checked={group.enabled}
          onChange={() => onToggleGroup(group.groupId)}
          onClick={(_, e) => e.stopPropagation()}
        />
      </Tooltip>
    </div>
  );
}

function BuiltinToolsTab() {
  const { t } = useTranslation();
  const { groups, loading, error, loadGroups, toggleGroup, toggleTool } = useLocalToolStore();

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
    <div className="flex-1 overflow-auto">
      {error && <Alert message={error} type="error" showIcon className="mb-3" closable />}
      {groups.length === 0 ? <Empty description={t("settings.localTools.empty")} /> : (
        <Collapse
          size="small"
          expandIconPosition="end"
          items={groups.map((group) => ({
            key: group.groupId,
            label: <GroupHeader group={group} onToggleGroup={toggleGroup} />,
            children: (
              <div className="border border-border rounded-lg overflow-hidden -mt-2">
                {group.tools.map((tool) => (
                  <ToolItem key={tool.name} tool={tool} groupEnabled={group.enabled} onToggle={toggleTool} />
                ))}
              </div>
            ),
          }))}
        />
      )}
    </div>
  );
}

export function ToolManager() {
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
      children: <McpServerSettings />,
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
    <div className="flex flex-col flex-1" style={{ padding: 24, height: "100%", minHeight: 0 }}>
      <Typography.Title level={4} style={{ marginTop: 0, marginBottom: 16, flexShrink: 0 }}>
        {t("settings.tools.title")}
      </Typography.Title>
      <Tabs
        defaultActiveKey="builtin"
        items={tabItems}
        style={{ flex: 1, display: "flex", flexDirection: "column", minHeight: 0 }}
        tabBarStyle={{ marginBottom: 16, flexShrink: 0 }}
      />
      <style>
        {`
        .ant-tabs-content-holder, .ant-tabs-content, .ant-tabs-tabpane-active {
          flex: 1 !important; min-height: 0 !important; display: flex !important; flex-direction: column !important;
        }
      `}
      </style>
    </div>
  );
}
