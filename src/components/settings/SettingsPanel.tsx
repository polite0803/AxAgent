import { invoke, logIpcError } from "@/lib/invoke";
import { useAppConfigStore } from "@/stores/feature/appConfigStore";
import type { FeatureFlags } from "@/stores/feature/appConfigStore";
import type { SubAgent } from "@/types";
import {
  Badge,
  Button,
  Card,
  Descriptions,
  Divider,
  Empty,
  InputNumber,
  List,
  message,
  Popconfirm,
  Radio,
  Space,
  Spin,
  Switch,
  Tabs,
  Tag,
  theme,
  Typography,
} from "antd";
import {
  AlertTriangle,
  Bot,
  ChevronDown,
  ChevronRight,
  Code,
  Gauge,
  Play,
  Plus,
  Puzzle,
  ScrollText,
  Shield,
  SlidersHorizontal,
  Terminal,
  Trash2,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AgentProfileManager } from "./AgentProfileManager";
import { HookExecutionLog } from "./HookExecutionLog";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

interface HookEventItem {
  event: string;
  labelKey: string;
  descKey: string;
  icon: React.ReactNode;
}

const HOOK_EVENTS: HookEventItem[] = [
  {
    event: "PreToolUse",
    labelKey: "settings.agent.hookEvents.preToolUse",
    descKey: "settings.agent.hookEvents.preToolUseDesc",
    icon: <Play size={14} />,
  },
  {
    event: "PostToolUse",
    labelKey: "settings.agent.hookEvents.postToolUse",
    descKey: "settings.agent.hookEvents.postToolUseDesc",
    icon: <Code size={14} />,
  },
  {
    event: "PostToolUseFailure",
    labelKey: "settings.agent.hookEvents.postToolUseFailure",
    descKey: "settings.agent.hookEvents.postToolUseFailureDesc",
    icon: <AlertTriangle size={14} />,
  },
  {
    event: "Notification",
    labelKey: "settings.agent.hookEvents.notification",
    descKey: "settings.agent.hookEvents.notificationDesc",
    icon: <Zap size={14} />,
  },
  {
    event: "UserPromptSubmit",
    labelKey: "settings.agent.hookEvents.userPromptSubmit",
    descKey: "settings.agent.hookEvents.userPromptSubmitDesc",
    icon: <Terminal size={14} />,
  },
  {
    event: "SessionStart",
    labelKey: "settings.agent.hookEvents.sessionStart",
    descKey: "settings.agent.hookEvents.sessionStartDesc",
    icon: <Play size={14} />,
  },
  {
    event: "SessionEnd",
    labelKey: "settings.agent.hookEvents.sessionEnd",
    descKey: "settings.agent.hookEvents.sessionEndDesc",
    icon: <Bot size={14} />,
  },
  {
    event: "Stop",
    labelKey: "settings.agent.hookEvents.stop",
    descKey: "settings.agent.hookEvents.stopDesc",
    icon: <AlertTriangle size={14} />,
  },
  {
    event: "StopFailure",
    labelKey: "settings.agent.hookEvents.stopFailure",
    descKey: "settings.agent.hookEvents.stopFailureDesc",
    icon: <AlertTriangle size={14} />,
  },
  {
    event: "SubagentStart",
    labelKey: "settings.agent.hookEvents.subagentStart",
    descKey: "settings.agent.hookEvents.subagentStartDesc",
    icon: <Puzzle size={14} />,
  },
  {
    event: "SubagentStop",
    labelKey: "settings.agent.hookEvents.subagentStop",
    descKey: "settings.agent.hookEvents.subagentStopDesc",
    icon: <Puzzle size={14} />,
  },
  {
    event: "PreCompact",
    labelKey: "settings.agent.hookEvents.preCompact",
    descKey: "settings.agent.hookEvents.preCompactDesc",
    icon: <SlidersHorizontal size={14} />,
  },
  {
    event: "PostCompact",
    labelKey: "settings.agent.hookEvents.postCompact",
    descKey: "settings.agent.hookEvents.postCompactDesc",
    icon: <SlidersHorizontal size={14} />,
  },
  {
    event: "TeammateIdle",
    labelKey: "settings.agent.hookEvents.teammateIdle",
    descKey: "settings.agent.hookEvents.teammateIdleDesc",
    icon: <Bot size={14} />,
  },
  {
    event: "TaskCreated",
    labelKey: "settings.agent.hookEvents.taskCreated",
    descKey: "settings.agent.hookEvents.taskCreatedDesc",
    icon: <Plus size={14} />,
  },
  {
    event: "TaskCompleted",
    labelKey: "settings.agent.hookEvents.taskCompleted",
    descKey: "settings.agent.hookEvents.taskCompletedDesc",
    icon: <Play size={14} />,
  },
  {
    event: "Elicitation",
    labelKey: "settings.agent.hookEvents.elicitation",
    descKey: "settings.agent.hookEvents.elicitationDesc",
    icon: <Terminal size={14} />,
  },
  {
    event: "ElicitationResult",
    labelKey: "settings.agent.hookEvents.elicitationResult",
    descKey: "settings.agent.hookEvents.elicitationResultDesc",
    icon: <Terminal size={14} />,
  },
  {
    event: "ConfigChange",
    labelKey: "settings.agent.hookEvents.configChange",
    descKey: "settings.agent.hookEvents.configChangeDesc",
    icon: <SlidersHorizontal size={14} />,
  },
  {
    event: "InstructionsLoaded",
    labelKey: "settings.agent.hookEvents.instructionsLoaded",
    descKey: "settings.agent.hookEvents.instructionsLoadedDesc",
    icon: <Code size={14} />,
  },
  {
    event: "FileChanged",
    labelKey: "settings.agent.hookEvents.fileChanged",
    descKey: "settings.agent.hookEvents.fileChangedDesc",
    icon: <Code size={14} />,
  },
  {
    event: "CwdChanged",
    labelKey: "settings.agent.hookEvents.cwdChanged",
    descKey: "settings.agent.hookEvents.cwdChangedDesc",
    icon: <Terminal size={14} />,
  },
  {
    event: "PermissionRequest",
    labelKey: "settings.agent.hookEvents.permissionRequest",
    descKey: "settings.agent.hookEvents.permissionRequestDesc",
    icon: <Shield size={14} />,
  },
  {
    event: "PermissionDenied",
    labelKey: "settings.agent.hookEvents.permissionDenied",
    descKey: "settings.agent.hookEvents.permissionDeniedDesc",
    icon: <Shield size={14} />,
  },
  {
    event: "WorktreeCreate",
    labelKey: "settings.agent.hookEvents.worktreeCreate",
    descKey: "settings.agent.hookEvents.worktreeCreateDesc",
    icon: <Plus size={14} />,
  },
  {
    event: "WorktreeRemove",
    labelKey: "settings.agent.hookEvents.worktreeRemove",
    descKey: "settings.agent.hookEvents.worktreeRemoveDesc",
    icon: <Trash2 size={14} />,
  },
];

interface HookCommand {
  id: string;
  command: string;
}

interface HookState {
  event: string;
  enabled: boolean;
  commands: HookCommand[];
}

interface AgentDisplay {
  id: string;
  name: string;
  description: string;
  status: string;
  agentType: string;
  tools: string[];
  capabilities: string[];
}

const FEATURE_FLAG_META: Array<{
  key: keyof FeatureFlags;
  labelKey: string;
  descKey: string;
  icon: React.ReactNode;
}> = [
  {
    key: "forkSubagent",
    labelKey: "settings.agent.featureFlags.forkSubagent",
    descKey: "settings.agent.featureFlags.forkSubagentDesc",
    icon: <Puzzle size={14} />,
  },
  {
    key: "coordinatorMode",
    labelKey: "settings.agent.featureFlags.coordinatorMode",
    descKey: "settings.agent.featureFlags.coordinatorModeDesc",
    icon: <Bot size={14} />,
  },
  {
    key: "proactiveMode",
    labelKey: "settings.agent.featureFlags.proactiveMode",
    descKey: "settings.agent.featureFlags.proactiveModeDesc",
    icon: <Zap size={14} />,
  },
  {
    key: "swarmMode",
    labelKey: "settings.agent.featureFlags.swarmMode",
    descKey: "settings.agent.featureFlags.swarmModeDesc",
    icon: <Shield size={14} />,
  },
  {
    key: "toolConcurrency",
    labelKey: "settings.agent.featureFlags.toolConcurrency",
    descKey: "settings.agent.featureFlags.toolConcurrencyDesc",
    icon: <Play size={14} />,
  },
  {
    key: "verificationAgent",
    labelKey: "settings.agent.featureFlags.verificationAgent",
    descKey: "settings.agent.featureFlags.verificationAgentDesc",
    icon: <Code size={14} />,
  },
  {
    key: "dreamTask",
    labelKey: "settings.agent.featureFlags.dreamTask",
    descKey: "settings.agent.featureFlags.dreamTaskDesc",
    icon: <Bot size={14} />,
  },
];

function GeneralTab() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { permissionMode, maxIterations, setPermissionMode, setMaxIterations } = useAppConfigStore();

  const rowStyle = { padding: "6px 0" };

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title={t("settings.agent.agentConfig")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span className="flex items-center gap-2">
            <Gauge size={14} /> {t("settings.agent.maxIterations")}
          </span>
          <InputNumber
            id="panel-inputnumber-166"
            min={1}
            max={100}
            value={maxIterations}
            onChange={(v) => v != null && setMaxIterations(v)}
            size="small"
            style={{ width: 120 }}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("settings.agent.permissionControl")}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span className="flex items-center gap-2">
            <Shield size={14} /> {t("settings.agent.permissionMode")}
          </span>
          <Radio.Group
            value={permissionMode}
            onChange={(e) => setPermissionMode(e.target.value)}
            size="small"
            optionType="button"
            buttonStyle="solid"
          >
            <Radio.Button value="read-only">
              {t("settings.agent.readOnly")}
            </Radio.Button>
            <Radio.Button value="workspace-write">
              {t("settings.agent.workspaceWrite")}
            </Radio.Button>
            <Radio.Button
              value="danger-full-access"
              style={{
                borderColor: permissionMode === "danger-full-access"
                  ? token.colorError
                  : undefined,
                color: permissionMode === "danger-full-access"
                  ? token.colorError
                  : undefined,
              }}
            >
              {t("settings.agent.fullAccess")}
            </Radio.Button>
          </Radio.Group>
        </div>
        {permissionMode === "danger-full-access" && (
          <div
            style={{
              marginTop: 8,
              padding: "8px 12px",
              borderRadius: 6,
              backgroundColor: token.colorErrorBg,
              border: `1px solid ${token.colorErrorBorder}`,
              fontSize: 12,
              color: token.colorError,
            }}
          >
            <AlertTriangle
              size={14}
              style={{ display: "inline", marginRight: 6 }}
            />
            {t("settings.agent.fullAccessWarning")}
          </div>
        )}
      </SettingsGroup>
    </div>
  );
}

function FeaturesTab() {
  const { t } = useTranslation();
  const { features, toggleFeature } = useAppConfigStore();

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title="Feature Flags">
        {FEATURE_FLAG_META.map((item, idx) => (
          <div key={item.key}>
            {idx > 0 && <Divider style={{ margin: "2px 0" }} />}
            <div
              style={{ padding: "8px 0" }}
              className="flex items-center justify-between"
            >
              <div className="flex flex-col" style={{ flex: 1 }}>
                <span
                  className="flex items-center gap-2"
                  style={{ fontSize: 13, fontWeight: 500 }}
                >
                  {item.icon} {t(item.labelKey)}
                  {features[item.key] && (
                    <Tag
                      color="green"
                      style={{
                        marginLeft: 4,
                        fontSize: 10,
                        lineHeight: "16px",
                        padding: "0 4px",
                      }}
                    >
                      {t("settings.agent.enabled")}
                    </Tag>
                  )}
                </span>
                <Text type="secondary" style={{ fontSize: 12, marginTop: 2 }}>
                  {t(item.descKey)}
                </Text>
              </div>
              <Switch
                id="panel-switch-167"
                checked={features[item.key]}
                onChange={() => toggleFeature(item.key)}
                style={{ flexShrink: 0, marginLeft: 16 }}
              />
            </div>
          </div>
        ))}
      </SettingsGroup>
    </div>
  );
}

function AgentsTab() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [agents, setAgents] = useState<AgentDisplay[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchAgents = useCallback(async () => {
    setLoading(true);
    try {
      const rawList = await invoke<SubAgent[]>("sub_agent_list");
      const list: AgentDisplay[] = (rawList || []).map((a: SubAgent) => ({
        id: a.id,
        name: a.name,
        description: a.description || t("settings.agent.noDescription"),
        status: a.status,
        agentType: a.metadata?.agent_type ?? "unknown",
        tools: a.metadata?.tools ?? [],
        capabilities: a.metadata?.capabilities ?? [],
      }));
      setAgents(list);
    } catch (e) {
      logIpcError("获取 Agent 列表")(e);
      setAgents([]);
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect
    fetchAgents();
  }, [fetchAgents]);

  const statusColor: Record<string, string> = {
    pending: "default",
    running: "processing",
    completed: "success",
    failed: "error",
    cancelled: "warning",
  };

  const statusLabel: Record<string, string> = {
    pending: t("settings.agent.statusPending"),
    running: t("settings.agent.statusRunning"),
    completed: t("settings.agent.statusCompleted"),
    failed: t("settings.agent.statusFailed"),
    cancelled: t("settings.agent.statusCancelled"),
  };

  return (
    <div className="p-6 pb-12">
      <AgentProfileManager />
      <Divider style={{ margin: "20px 0" }} />
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 16,
        }}
      >
        <Text strong style={{ fontSize: 13, color: token.colorTextSecondary }}>
          {t("settings.agent.runtimeAgentList")}
        </Text>
        <Button
          size="small"
          icon={<ChevronRight size={14} />}
          onClick={fetchAgents}
          loading={loading}
        >
          {t("settings.agent.refresh")}
        </Button>
      </div>

      {loading
        ? (
          <div style={{ textAlign: "center", padding: 48 }}>
            <Spin />
            <div
              style={{
                marginTop: 12,
                color: token.colorTextDescription,
                fontSize: 12,
              }}
            >
              {t("settings.agent.loading")}
            </div>
          </div>
        )
        : agents.length === 0
        ? (
          <Card
            size="small"
            style={{ borderRadius: 10, textAlign: "center", padding: 32 }}
          >
            <Empty
              description={t("settings.agent.noRegisteredAgents")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          </Card>
        )
        : (
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            {agents.map((agent) => (
              <Card
                key={agent.id}
                size="small"
                style={{
                  borderRadius: 10,
                  border: "none",
                  boxShadow: `0 0 0 0.5px ${token.colorBorderSecondary}`,
                }}
                title={
                  <div
                    className="flex items-center justify-between"
                    style={{ width: "100%" }}
                  >
                    <Space size={8}>
                      <Bot size={16} color={token.colorPrimary} />
                      <Text strong>{agent.name}</Text>
                      <Tag color={statusColor[agent.status] || "default"}>
                        {statusLabel[agent.status] || agent.status}
                      </Tag>
                    </Space>
                  </div>
                }
              >
                <Descriptions size="small" column={1} colon={false}>
                  <Descriptions.Item label={t("settings.agent.descLabel")}>
                    {agent.description}
                  </Descriptions.Item>
                  <Descriptions.Item label={t("settings.agent.typeLabel")}>
                    <Tag>{agent.agentType}</Tag>
                  </Descriptions.Item>
                  {agent.tools.length > 0 && (
                    <Descriptions.Item label={t("settings.agent.toolsLabel")}>
                      <Space size={4} wrap>
                        {agent.tools.map((tool) => (
                          <Tag key={tool} color="blue" style={{ fontSize: 12 }}>
                            {tool}
                          </Tag>
                        ))}
                      </Space>
                    </Descriptions.Item>
                  )}
                  {agent.capabilities.length > 0 && (
                    <Descriptions.Item
                      label={t("settings.agent.capabilitiesLabel")}
                    >
                      <Space size={4} wrap>
                        {agent.capabilities.map((cap) => (
                          <Tag key={cap} color="purple" style={{ fontSize: 12 }}>
                            {cap}
                          </Tag>
                        ))}
                      </Space>
                    </Descriptions.Item>
                  )}
                </Descriptions>
              </Card>
            ))}
          </div>
        )}
    </div>
  );
}

function HooksTab() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [hooks, setHooks] = useState<HookState[]>(() =>
    HOOK_EVENTS.map((e) => ({
      event: e.event,
      enabled: e.event === "PreToolUse" || e.event === "UserPromptSubmit",
      commands: [],
    }))
  );
  const [expandedEvents, setExpandedEvents] = useState<Set<string>>(new Set());

  const toggleExpand = (event: string) => {
    setExpandedEvents((prev) => {
      const next = new Set(prev);
      if (next.has(event)) {
        next.delete(event);
      } else {
        next.add(event);
      }
      return next;
    });
  };

  const toggleHook = (event: string) => {
    setHooks((prev) => prev.map((h) => (h.event === event ? { ...h, enabled: !h.enabled } : h)));
  };

  const addCommand = (event: string) => {
    const cmd = window.prompt(t("settings.agent.enterShellCommand"));
    if (!cmd || !cmd.trim()) {
      return;
    }
    setHooks((prev) =>
      prev.map((h) =>
        h.event === event
          ? {
            ...h,
            commands: [
              ...h.commands,
              { id: crypto.randomUUID(), command: cmd.trim() },
            ],
          }
          : h
      )
    );
    message.success(t("settings.agent.commandAddedForEvent", { event }));
  };

  const removeCommand = (event: string, cmdId: string) => {
    setHooks((prev) =>
      prev.map((h) =>
        h.event === event
          ? { ...h, commands: h.commands.filter((c) => c.id !== cmdId) }
          : h
      )
    );
    message.success(t("settings.agent.commandRemoved"));
  };

  const hookMeta = (event: string) => HOOK_EVENTS.find((e) => e.event === event);

  const eventsContent = (
    <div className="pt-2">
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 16,
        }}
      >
        <Text strong style={{ fontSize: 13, color: token.colorTextSecondary }}>
          {t("settings.agent.hookEventConfig")} ({HOOK_EVENTS.length})
        </Text>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {hooks.map((hook) => {
          const meta = hookMeta(hook.event);
          const isExpanded = expandedEvents.has(hook.event);

          return (
            <Card
              key={hook.event}
              size="small"
              style={{
                borderRadius: 10,
                border: "none",
                boxShadow: `0 0 0 0.5px ${token.colorBorderSecondary}`,
              }}
            >
              <div
                className="flex items-center justify-between cursor-pointer"
                style={{ padding: "2px 0" }}
                role="button"
                tabIndex={0}
                onClick={() => toggleExpand(hook.event)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    toggleExpand(hook.event);
                  }
                }}
              >
                <Space size={8}>
                  <span style={{ color: token.colorTextQuaternary }}>
                    {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  </span>
                  {meta?.icon}
                  <Text strong style={{ fontSize: 13 }}>
                    {meta ? t(meta.labelKey) : hook.event}
                  </Text>
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {hook.event}
                  </Text>
                </Space>
                <Space size={8} onClick={(e) => e.stopPropagation()}>
                  {hook.commands.length > 0 && (
                    <Badge
                      count={hook.commands.length}
                      size="small"
                      style={{ marginRight: 4 }}
                    />
                  )}
                  <Switch
                    id="panel-switch-168"
                    checked={hook.enabled}
                    onChange={() => toggleHook(hook.event)}
                    size="small"
                  />
                </Space>
              </div>

              {meta && (
                <Text type="secondary" style={{ fontSize: 12, marginLeft: 28 }}>
                  {t(meta.descKey)}
                </Text>
              )}

              {isExpanded && (
                <div style={{ marginTop: 12, marginLeft: 28 }}>
                  <Divider style={{ margin: "4px 0 10px" }} />

                  {hook.commands.length === 0
                    ? (
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        {t("settings.agent.noShellCommands")}
                      </Text>
                    )
                    : (
                      <List
                        size="small"
                        dataSource={hook.commands}
                        renderItem={(cmd) => (
                          <List.Item
                            actions={[
                              <Popconfirm
                                key="del"
                                title={t("settings.agent.confirmRemove")}
                                onConfirm={() => removeCommand(hook.event, cmd.id)}
                                okText={t("common.confirm")}
                                cancelText={t("common.cancel")}
                              >
                                <Button
                                  size="small"
                                  type="text"
                                  danger
                                  icon={<Trash2 size={13} />}
                                />
                              </Popconfirm>,
                            ]}
                          >
                            <Code
                              size={12}
                              style={{ marginRight: 8, opacity: 0.5 }}
                            />
                            <Text code style={{ fontSize: 12 }}>
                              {cmd.command}
                            </Text>
                          </List.Item>
                        )}
                        style={{ marginTop: 4 }}
                      />
                    )}

                  <Button
                    size="small"
                    type="dashed"
                    icon={<Plus size={13} />}
                    onClick={() => addCommand(hook.event)}
                    style={{ marginTop: 8 }}
                    disabled={!hook.enabled}
                  >
                    {t("settings.agent.addShellCommand")}
                  </Button>
                </div>
              )}
            </Card>
          );
        })}
      </div>
    </div>
  );

  const logsContent = (
    <div className="pt-2">
      <HookExecutionLog />
    </div>
  );

  return (
    <div className="p-6 pb-12">
      <Tabs
        size="small"
        items={[
          {
            key: "events",
            label: (
              <span className="flex items-center gap-1.5">
                <Code size={13} />
                {t("settings.agent.eventConfig")}
              </span>
            ),
            children: eventsContent,
          },
          {
            key: "logs",
            label: (
              <span className="flex items-center gap-1.5">
                <ScrollText size={13} />
                {t("settings.agent.executionLog")}
              </span>
            ),
            children: logsContent,
          },
        ]}
      />
    </div>
  );
}

export function SettingsPanel() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { saveConfig, loadConfig } = useAppConfigStore();

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  useEffect(() => {
    return () => {
      saveConfig();
    };
  }, [saveConfig]);

  const tabItems = [
    {
      key: "general",
      label: (
        <span className="flex items-center gap-1.5">
          <SlidersHorizontal size={14} />
          {t("settings.agent.general")}
        </span>
      ),
      children: <GeneralTab />,
    },
    {
      key: "features",
      label: (
        <span className="flex items-center gap-1.5">
          <Zap size={14} />
          {t("settings.agent.features")}
        </span>
      ),
      children: <FeaturesTab />,
    },
    {
      key: "agents",
      label: (
        <span className="flex items-center gap-1.5">
          <Bot size={14} />
          {t("settings.agent.agents")}
        </span>
      ),
      children: <AgentsTab />,
    },
    {
      key: "hooks",
      label: (
        <span className="flex items-center gap-1.5">
          <Terminal size={14} />
          {t("settings.agent.hooks")}
        </span>
      ),
      children: <HooksTab />,
    },
  ];

  return (
    <div className="h-full" style={{ overflowY: "auto", overflowX: "hidden" }} data-os-scrollbar>
      <div
        style={{
          padding: "20px 24px 16px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
        }}
      >
        <Typography.Title level={5} style={{ margin: 0 }}>
          {t("settings.agent.controlPanel")}
        </Typography.Title>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {t("settings.agent.controlPanelDesc")}
        </Text>
      </div>

      <Tabs
        defaultActiveKey="general"
        items={tabItems}
        tabPlacement="top"
        style={{ padding: "0 24px" }}
        tabBarStyle={{ marginBottom: 0 }}
        onChange={() => {
          saveConfig();
        }}
      />
    </div>
  );
}
