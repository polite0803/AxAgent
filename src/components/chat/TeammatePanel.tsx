import { useAgentStore } from "@/stores";
import type { AgentPoolItem, TeammateStatus, WorkerMessage } from "@/types/agent";
import { CheckCircleOutlined, CloseCircleOutlined, LoadingOutlined, TeamOutlined } from "@ant-design/icons";

const _EMPTY: never[] = [];
import { Button, Collapse, message, Tag, Typography } from "antd";
import { useMemo, useState } from "react";
import { type CreateTeamData, CreateTeamModal } from "./CreateTeamModal";

const { Text } = Typography;

// ---------------------------------------------------------------------------
// 队友状态映射
// ---------------------------------------------------------------------------

function getTeammateStatus(item: AgentPoolItem): TeammateStatus {
  switch (item.status) {
    case "running":
      return "busy";
    case "completed":
    case "pending":
      return "idle";
    case "failed":
    case "cancelled":
      return "error";
    default:
      return "offline";
  }
}

function getStatusConfig(status: TeammateStatus) {
  const configs: Record<
    TeammateStatus,
    { color: string; label: string; icon: React.ReactNode }
  > = {
    idle: {
      color: "default",
      label: "空闲",
      icon: <CheckCircleOutlined style={{ fontSize: 12 }} />,
    },
    busy: {
      color: "processing",
      label: "工作中",
      icon: <LoadingOutlined spin style={{ fontSize: 12 }} />,
    },
    offline: {
      color: "default",
      label: "离线",
      icon: <CloseCircleOutlined style={{ fontSize: 12 }} />,
    },
    error: {
      color: "error",
      label: "异常",
      icon: <CloseCircleOutlined style={{ fontSize: 12 }} />,
    },
  };
  return configs[status] || configs.offline;
}

// ---------------------------------------------------------------------------
// 消息格式化
// ---------------------------------------------------------------------------

function formatMessage(msg: WorkerMessage): string {
  const time = msg.timestamp
    ? new Date(msg.timestamp).toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    })
    : "";
  const prefix = time ? `[${time}] ` : "";
  return `${prefix}${msg.content}`;
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

interface TeammatePanelProps {
  conversationId: string;
  /** 是否可见 */
  visible?: boolean;
}

export function TeammatePanel({
  conversationId,
  visible = true,
}: TeammatePanelProps) {
  const pool = useAgentStore((s) => s.agentPool[conversationId] || _EMPTY);
  const upsertPoolItem = useAgentStore((s) => s.upsertPoolItem);
  const [teamModalOpen, setTeamModalOpen] = useState(false);
  const [creatingTeam, setCreatingTeam] = useState(false);

  // 按团队分组
  const grouped = useMemo(() => {
    const teams: Record<string, AgentPoolItem[]> = {};
    for (const item of pool) {
      if (item.type !== "worker") {
        continue;
      }
      const team = item.teamName || "默认团队";
      if (!teams[team]) {
        teams[team] = [];
      }
      teams[team].push(item);
    }
    return teams;
  }, [pool]);

  const teamNames = Object.keys(grouped);

  if (!visible) {
    return null;
  }

  // 构建折叠面板数据
  const collapseItems = teamNames.map((teamName) => {
    const teammates = grouped[teamName];

    return {
      key: teamName,
      label: (
        <span className="flex items-center gap-2">
          <TeamOutlined />
          <span>{teamName}</span>
          <Tag>{teammates.length} 名队友</Tag>
        </span>
      ),
      children: (
        <div className="flex flex-col gap-2">
          {teammates.map((tm) => {
            const ts = getTeammateStatus(tm);
            const sc = getStatusConfig(ts);

            return (
              <div
                key={tm.id}
                className="rounded border p-2"
                style={{ borderColor: "#e8e8e8" }}
              >
                {/* 头部：状态 + 名称 */}
                <div className="mb-1 flex items-center gap-2">
                  <Tag color={sc.color}>{sc.label}</Tag>
                  <Text strong style={{ fontSize: 13 }}>
                    {tm.name}
                  </Text>
                </div>

                {/* 当前任务 */}
                <div className="mb-1">
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {tm.currentTask || tm.taskDescription || "空闲中"}
                  </Text>
                </div>

                {/* 消息列表 */}
                {tm.messages && tm.messages.length > 0 && (
                  <div className="mt-1 max-h-40 overflow-y-auto rounded bg-gray-50 p-1">
                    {tm.messages.map((msg, i) => (
                      <div
                        key={i}
                        className="border-b border-gray-100 py-0.5"
                        style={{ fontSize: 12, lineHeight: "18px" }}
                      >
                        {formatMessage(msg)}
                      </div>
                    ))}
                  </div>
                )}

                {/* 持续时长 */}
                {tm.duration !== undefined && tm.status === "completed" && (
                  <div className="mt-1">
                    <Text type="secondary" style={{ fontSize: 11 }}>
                      耗时 {(tm.duration / 1000).toFixed(1)}s
                    </Text>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      ),
    };
  });

  return (
    <div
      className="mb-2 rounded border"
      style={{ borderColor: "#d9d9d9", backgroundColor: "#fff" }}
    >
      <div
        className="border-b px-3 py-2 flex items-center justify-between"
        style={{ borderColor: "#f0f0f0" }}
      >
        <Text strong style={{ fontSize: 13 }}>
          <TeamOutlined className="mr-1" />
          Swarm 队友 ({teamNames.reduce((acc, t) => acc + grouped[t].length, 0)})
        </Text>
        <Button
          size="small"
          type="primary"
          ghost
          icon={<TeamOutlined />}
          onClick={() => setTeamModalOpen(true)}
        >
          创建团队
        </Button>
      </div>
      <div className="px-2 py-1">
        <Collapse
          size="small"
          ghost
          items={collapseItems}
          defaultActiveKey={teamNames}
        />
      </div>

      <CreateTeamModal
        open={teamModalOpen}
        onCancel={() => setTeamModalOpen(false)}
        loading={creatingTeam}
        onCreate={(data: CreateTeamData) => {
          setCreatingTeam(true);
          // 将队友添加到 agentPool
          const teamName = data.teamName || "新团队";
          for (const tm of data.teammates) {
            upsertPoolItem({
              id: `${teamName}-${tm.name}-${Date.now()}`,
              conversationId,
              type: "worker",
              name: tm.name,
              status: "pending",
              teamName,
              // eslint-disable-next-line @typescript-eslint/no-explicit-any
              agentType: tm.backendType as any,
              currentTask: "等待分配任务",
            });
          }
          message.success(`团队 "${teamName}" 已创建，共 ${data.teammates.length} 名队友`);
          setCreatingTeam(false);
          setTeamModalOpen(false);
        }}
      />
    </div>
  );
}

export default TeammatePanel;
