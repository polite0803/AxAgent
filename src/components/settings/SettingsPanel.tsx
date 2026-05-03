import { invoke } from "@/lib/invoke";
import { useAppConfigStore } from "@/stores/feature/appConfigStore";
import type { FeatureFlags, ModelTier } from "@/stores/feature/appConfigStore";
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
  Select,
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
import { HookExecutionLog } from "./HookExecutionLog";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

// ─── Hook 事件列表（与后端 runtime hooks.rs 中的 26 个事件一一对应）───

interface HookEventItem {
  event: string;
  label: string;
  description: string;
  icon: React.ReactNode;
}

const HOOK_EVENTS: HookEventItem[] = [
  { event: "PreToolUse", label: "工具使用前", description: "工具调用执行前触发", icon: <Play size={14} /> },
  { event: "PostToolUse", label: "工具使用后", description: "工具调用成功完成后触发", icon: <Code size={14} /> },
  { event: "PostToolUseFailure", label: "工具使用失败", description: "工具调用失败后触发", icon: <AlertTriangle size={14} /> },
  { event: "Notification", label: "通知", description: "系统通知事件", icon: <Zap size={14} /> },
  { event: "UserPromptSubmit", label: "用户提交提示", description: "用户提交消息时触发", icon: <Terminal size={14} /> },
  { event: "SessionStart", label: "会话开始", description: "Agent 会话启动时触发", icon: <Play size={14} /> },
  { event: "SessionEnd", label: "会话结束", description: "Agent 会话结束时触发", icon: <Bot size={14} /> },
  { event: "Stop", label: "停止", description: "Agent 停止时触发", icon: <AlertTriangle size={14} /> },
  { event: "StopFailure", label: "停止失败", description: "Agent 停止失败时触发", icon: <AlertTriangle size={14} /> },
  { event: "SubagentStart", label: "子 Agent 开始", description: "子 Agent 启动时触发", icon: <Puzzle size={14} /> },
  { event: "SubagentStop", label: "子 Agent 停止", description: "子 Agent 停止时触发", icon: <Puzzle size={14} /> },
  { event: "PreCompact", label: "压缩前", description: "上下文压缩前触发", icon: <SlidersHorizontal size={14} /> },
  { event: "PostCompact", label: "压缩后", description: "上下文压缩完成后触发", icon: <SlidersHorizontal size={14} /> },
  { event: "TeammateIdle", label: "队友空闲", description: "队友 Agent 进入空闲时触发", icon: <Bot size={14} /> },
  { event: "TaskCreated", label: "任务创建", description: "新任务创建时触发", icon: <Plus size={14} /> },
  { event: "TaskCompleted", label: "任务完成", description: "任务完成时触发", icon: <Play size={14} /> },
  { event: "Elicitation", label: "信息征询", description: "需要用户提供额外信息时触发", icon: <Terminal size={14} /> },
  { event: "ElicitationResult", label: "征询结果", description: "用户回复征询后触发", icon: <Terminal size={14} /> },
  { event: "ConfigChange", label: "配置变更", description: "系统配置发生变更时触发", icon: <SlidersHorizontal size={14} /> },
  { event: "InstructionsLoaded", label: "指令加载", description: "Agent 指令加载完成时触发", icon: <Code size={14} /> },
  { event: "FileChanged", label: "文件变更", description: "监控的文件发生变更时触发", icon: <Code size={14} /> },
  { event: "CwdChanged", label: "目录切换", description: "当前工作目录切换时触发", icon: <Terminal size={14} /> },
  { event: "PermissionRequest", label: "权限请求", description: "工具调用请求权限时触发", icon: <Shield size={14} /> },
  { event: "PermissionDenied", label: "权限拒绝", description: "权限请求被拒绝时触发", icon: <Shield size={14} /> },
  { event: "WorktreeCreate", label: "工作树创建", description: "Git Worktree 创建时触发", icon: <Plus size={14} /> },
  { event: "WorktreeRemove", label: "工作树移除", description: "Git Worktree 移除时触发", icon: <Trash2 size={14} /> },
];

// ─── Hook 数据接口 ───

interface HookCommand {
  id: string;
  command: string;
}

interface HookState {
  event: string;
  enabled: boolean;
  commands: HookCommand[];
}

// ─── Agent 数据接口 ───

interface AgentDisplay {
  id: string;
  name: string;
  description: string;
  status: string;
  agentType: string;
  tools: string[];
  capabilities: string[];
}

// ─── 通用 Tab ───

function GeneralTab() {
  const { token } = theme.useToken();
  const { model, permissionMode, maxIterations, setModel, setPermissionMode, setMaxIterations } = useAppConfigStore();

  const modelOptions = [
    { label: "Opus (最强推理)", value: "opus" as ModelTier },
    { label: "Sonnet (平衡性能)", value: "sonnet" as ModelTier },
    { label: "Haiku (极速响应)", value: "haiku" as ModelTier },
  ];

  const rowStyle = { padding: "6px 0" };

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title="模型配置">
        <div style={rowStyle} className="flex items-center justify-between">
          <span className="flex items-center gap-2">
            <Bot size={14} /> 模型选择
          </span>
          <Select
            value={model}
            onChange={(v) => setModel(v)}
            options={modelOptions}
            style={{ width: 220 }}
            size="small"
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span className="flex items-center gap-2">
            <Gauge size={14} /> 最大迭代次数
          </span>
          <InputNumber
            min={1}
            max={100}
            value={maxIterations}
            onChange={(v) => v != null && setMaxIterations(v)}
            size="small"
            style={{ width: 120 }}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup title="权限控制">
        <div style={rowStyle} className="flex items-center justify-between">
          <span className="flex items-center gap-2">
            <Shield size={14} /> 权限模式
          </span>
          <Radio.Group
            value={permissionMode}
            onChange={(e) => setPermissionMode(e.target.value)}
            size="small"
            optionType="button"
            buttonStyle="solid"
          >
            <Radio.Button value="read-only">只读</Radio.Button>
            <Radio.Button value="workspace-write">工作区写入</Radio.Button>
            <Radio.Button
              value="danger-full-access"
              style={{
                borderColor: permissionMode === "danger-full-access" ? token.colorError : undefined,
                color: permissionMode === "danger-full-access" ? token.colorError : undefined,
              }}
            >
              完全访问
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
            <AlertTriangle size={14} style={{ display: "inline", marginRight: 6 }} />
            完全访问模式下，Agent 可以执行任意系统命令和文件操作，请谨慎使用。
          </div>
        )}
      </SettingsGroup>
    </div>
  );
}

// ─── Features Tab ───

const FEATURE_FLAG_META: Array<{
  key: keyof FeatureFlags;
  label: string;
  description: string;
  icon: React.ReactNode;
}> = [
  { key: "forkSubagent", label: "Fork 子 Agent", description: "允许 Agent 派生子 Agent 执行并行任务", icon: <Puzzle size={14} /> },
  { key: "coordinatorMode", label: "协调者模式", description: "Agent 以协调者角色运行，调度多个子任务", icon: <Bot size={14} /> },
  { key: "proactiveMode", label: "主动模式", description: "Agent 主动预测用户需求并提前执行操作", icon: <Zap size={14} /> },
  { key: "swarmMode", label: "集群模式", description: "启用多 Agent 集群协作完成复杂任务", icon: <Shield size={14} /> },
  { key: "toolConcurrency", label: "工具并发", description: "允许同时执行多个独立工具调用", icon: <Play size={14} /> },
  { key: "verificationAgent", label: "验证 Agent", description: "启用独立验证 Agent 审查执行结果", icon: <Code size={14} /> },
  { key: "dreamTask", label: "Dream Task", description: "空闲时执行背景优化和反思任务", icon: <Bot size={14} /> },
];

function FeaturesTab() {
  const { token: _t } = theme.useToken();
  const { features, toggleFeature } = useAppConfigStore();

  return (
    <div className="p-6 pb-12">
      <SettingsGroup title="Feature Flags">
        {FEATURE_FLAG_META.map((item, idx) => (
          <div key={item.key}>
            {idx > 0 && <Divider style={{ margin: "2px 0" }} />}
            <div style={{ padding: "8px 0" }} className="flex items-center justify-between">
              <div className="flex flex-col" style={{ flex: 1 }}>
                <span className="flex items-center gap-2" style={{ fontSize: 13, fontWeight: 500 }}>
                  {item.icon} {item.label}
                  {features[item.key] && (
                    <Tag color="green" style={{ marginLeft: 4, fontSize: 10, lineHeight: "16px", padding: "0 4px" }}>
                      开启
                    </Tag>
                  )}
                </span>
                <Text type="secondary" style={{ fontSize: 12, marginTop: 2 }}>
                  {item.description}
                </Text>
              </div>
              <Switch
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

// ─── Agents Tab ───

function AgentsTab() {
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
        description: a.description || "暂无描述",
        status: a.status,
        agentType: a.metadata?.agent_type ?? "unknown",
        tools: a.metadata?.tools ?? [],
        capabilities: a.metadata?.capabilities ?? [],
      }));
      setAgents(list);
    } catch (e) {
      console.warn("获取 Agent 列表失败:", e);
      setAgents([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
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
    pending: "等待中",
    running: "运行中",
    completed: "已完成",
    failed: "失败",
    cancelled: "已取消",
  };

  return (
    <div className="p-6 pb-12">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <Text strong style={{ fontSize: 13, color: token.colorTextSecondary }}>
          已注册 Agent 列表
        </Text>
        <Button size="small" icon={<ChevronRight size={14} />} onClick={fetchAgents} loading={loading}>
          刷新
        </Button>
      </div>

      {loading ? (
        <div style={{ textAlign: "center", padding: 48 }}>
          <Spin />
          <div style={{ marginTop: 12, color: token.colorTextDescription, fontSize: 12 }}>加载中...</div>
        </div>
      ) : agents.length === 0 ? (
        <Card size="small" style={{ borderRadius: 10, textAlign: "center", padding: 32 }}>
          <Empty description="暂无已注册的 Agent" image={Empty.PRESENTED_IMAGE_SIMPLE} />
        </Card>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {agents.map((agent) => (
            <Card
              key={agent.id}
              size="small"
              style={{ borderRadius: 10, border: "none", boxShadow: `0 0 0 0.5px ${token.colorBorderSecondary}` }}
              title={
                <div className="flex items-center justify-between" style={{ width: "100%" }}>
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
                <Descriptions.Item label="描述">
                  {agent.description}
                </Descriptions.Item>
                <Descriptions.Item label="类型">
                  <Tag>{agent.agentType}</Tag>
                </Descriptions.Item>
                {agent.tools.length > 0 && (
                  <Descriptions.Item label="工具">
                    <Space size={4} wrap>
                      {agent.tools.map((t) => (
                        <Tag key={t} color="blue" style={{ fontSize: 11 }}>{t}</Tag>
                      ))}
                    </Space>
                  </Descriptions.Item>
                )}
                {agent.capabilities.length > 0 && (
                  <Descriptions.Item label="能力">
                    <Space size={4} wrap>
                      {agent.capabilities.map((c) => (
                        <Tag key={c} color="purple" style={{ fontSize: 11 }}>{c}</Tag>
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

// ─── Hooks Tab ───

function HooksTab() {
  const { token } = theme.useToken();
  const [hooks, setHooks] = useState<HookState[]>(() =>
    HOOK_EVENTS.map((e) => ({
      event: e.event,
      enabled: e.event === "PreToolUse" || e.event === "UserPromptSubmit",
      commands: [],
    })),
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
    setHooks((prev) =>
      prev.map((h) => (h.event === event ? { ...h, enabled: !h.enabled } : h)),
    );
  };

  const addCommand = (event: string) => {
    const cmd = window.prompt("请输入 Shell 命令:");
    if (!cmd || !cmd.trim()) { return; }
    setHooks((prev) =>
      prev.map((h) =>
        h.event === event
          ? { ...h, commands: [...h.commands, { id: crypto.randomUUID(), command: cmd.trim() }] }
          : h,
      ),
    );
    message.success(`已为 ${event} 添加命令`);
  };

  const removeCommand = (event: string, cmdId: string) => {
    setHooks((prev) =>
      prev.map((h) =>
        h.event === event ? { ...h, commands: h.commands.filter((c) => c.id !== cmdId) } : h,
      ),
    );
    message.success("命令已移除");
  };

  const hookMeta = (event: string) => HOOK_EVENTS.find((e) => e.event === event);

  const eventsContent = (
    <div className="pt-2">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 16 }}>
        <Text strong style={{ fontSize: 13, color: token.colorTextSecondary }}>
          Hook 事件配置 ({HOOK_EVENTS.length} 个事件)
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
                onClick={() => toggleExpand(hook.event)}
              >
                <Space size={8}>
                  <span style={{ color: token.colorTextQuaternary }}>
                    {isExpanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                  </span>
                  {meta?.icon}
                  <Text strong style={{ fontSize: 13 }}>{meta?.label ?? hook.event}</Text>
                  <Text type="secondary" style={{ fontSize: 11 }}>{hook.event}</Text>
                </Space>
                <Space size={8} onClick={(e) => e.stopPropagation()}>
                  {hook.commands.length > 0 && (
                    <Badge count={hook.commands.length} size="small" style={{ marginRight: 4 }} />
                  )}
                  <Switch checked={hook.enabled} onChange={() => toggleHook(hook.event)} size="small" />
                </Space>
              </div>

              {meta && (
                <Text type="secondary" style={{ fontSize: 11, marginLeft: 28 }}>
                  {meta.description}
                </Text>
              )}

              {isExpanded && (
                <div style={{ marginTop: 12, marginLeft: 28 }}>
                  <Divider style={{ margin: "4px 0 10px" }} />

                  {hook.commands.length === 0 ? (
                    <Text type="secondary" style={{ fontSize: 12 }}>暂无配置的 Shell 命令</Text>
                  ) : (
                    <List
                      size="small"
                      dataSource={hook.commands}
                      renderItem={(cmd) => (
                        <List.Item
                          actions={[
                            <Popconfirm
                              key="del"
                              title="确认移除?"
                              onConfirm={() => removeCommand(hook.event, cmd.id)}
                              okText="确认"
                              cancelText="取消"
                            >
                              <Button size="small" type="text" danger icon={<Trash2 size={13} />} />
                            </Popconfirm>,
                          ]}
                        >
                          <Code size={12} style={{ marginRight: 8, opacity: 0.5 }} />
                          <Text code style={{ fontSize: 12 }}>{cmd.command}</Text>
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
                    添加 Shell 命令
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
                事件配置
              </span>
            ),
            children: eventsContent,
          },
          {
            key: "logs",
            label: (
              <span className="flex items-center gap-1.5">
                <ScrollText size={13} />
                执行日志
              </span>
            ),
            children: logsContent,
          },
        ]}
      />
    </div>
  );
}

// ─── 主面板组件 ───

export function SettingsPanel() {
  const { token } = theme.useToken();
  const { saveConfig, loadConfig } = useAppConfigStore();

  // 组件挂载时加载配置
  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  // 离开时自动保存
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
          通用
        </span>
      ),
      children: <GeneralTab />,
    },
    {
      key: "features",
      label: (
        <span className="flex items-center gap-1.5">
          <Zap size={14} />
          Features
        </span>
      ),
      children: <FeaturesTab />,
    },
    {
      key: "agents",
      label: (
        <span className="flex items-center gap-1.5">
          <Bot size={14} />
          Agents
        </span>
      ),
      children: <AgentsTab />,
    },
    {
      key: "hooks",
      label: (
        <span className="flex items-center gap-1.5">
          <Terminal size={14} />
          Hooks
        </span>
      ),
      children: <HooksTab />,
    },
  ];

  return (
    <div className="h-full" style={{ overflowY: "auto" }} data-os-scrollbar>
      <div
        style={{
          padding: "20px 24px 16px",
          borderBottom: `1px solid ${token.colorBorderSecondary}`,
        }}
      >
        <Typography.Title level={5} style={{ margin: 0 }}>
          Agent 控制面板
        </Typography.Title>
        <Text type="secondary" style={{ fontSize: 12 }}>
          模型选择、功能开关、Agent 管理与 Hook 配置
        </Text>
      </div>

      <Tabs
        defaultActiveKey="general"
        items={tabItems}
        tabPosition="top"
        style={{ padding: "0 24px" }}
        tabBarStyle={{ marginBottom: 0 }}
        onChange={() => {
          // Tab 切换时保存当前配置
          saveConfig();
        }}
      />
    </div>
  );
}
