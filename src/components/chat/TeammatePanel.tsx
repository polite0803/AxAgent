// SPDX-License-Identifier: AGPL-3.0-only

import { useExecutionStore } from "@/stores/feature/executionStore";
import type { AgentPoolItem, TeammateStatus, WorkerMessage } from "@/types";
import { CheckCircleOutlined, CloseCircleOutlined, LoadingOutlined, TeamOutlined } from "@ant-design/icons";

const _EMPTY: never[] = [];
import { Button, Collapse, message, Tag, theme, Typography } from "antd";
import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { type CreateTeamData, CreateTeamModal, type TeammateBackendType } from "./CreateTeamModal";

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

function getStatusConfig(status: TeammateStatus, t: (key: string) => string) {
  const configs: Record<
    TeammateStatus,
    { color: string; label: string; icon: React.ReactNode }
  > = {
    idle: {
      color: "default",
      label: t("teammatePanel.statusIdle"),
      icon: <CheckCircleOutlined style={{ fontSize: 12 }} />,
    },
    busy: {
      color: "processing",
      label: t("teammatePanel.statusBusy"),
      icon: <LoadingOutlined spin style={{ fontSize: 12 }} />,
    },
    offline: {
      color: "default",
      label: t("teammatePanel.statusOffline"),
      icon: <CloseCircleOutlined style={{ fontSize: 12 }} />,
    },
    error: {
      color: "error",
      label: t("teammatePanel.statusError"),
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
  const pool = useExecutionStore((s) => s.agentPool[conversationId] || _EMPTY);
  const upsertPoolItem = useExecutionStore((s) => s.upsertPoolItem);
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [teamModalOpen, setTeamModalOpen] = useState(false);
  const [creatingTeam, setCreatingTeam] = useState(false);

  const handleCreateTeam = useCallback((data: CreateTeamData) => {
    setCreatingTeam(true);
    const teamName = data.teamName || t("teammatePanel.newTeam");
    for (const tm of data.teammates) {
      upsertPoolItem({
        id: `${teamName}-${tm.name}-${Date.now()}`,
        conversationId,
        type: "worker",
        name: tm.name,
        status: "pending",
        teamName,
        agentType: tm.backendType as TeammateBackendType,
        currentTask: t("teammatePanel.waitingForTask"),
      });
    }
    message.success(
      t("teammatePanel.teamCreated", {
        name: teamName,
        count: data.teammates.length,
      }),
    );
    setCreatingTeam(false);
    setTeamModalOpen(false);
  }, [conversationId, t, upsertPoolItem]);

  // 按团队分组
  const grouped = useMemo(() => {
    const teams: Record<string, AgentPoolItem[]> = {};
    for (const item of pool) {
      if (item.type !== "worker") {
        continue;
      }
      const team = item.teamName || t("teammatePanel.defaultTeam");
      if (!teams[team]) {
        teams[team] = [];
      }
      teams[team].push(item);
    }
    return teams;
  }, [pool, t]);

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
          <Tag>
            {t("teammatePanel.memberCount", { count: teammates.length })}
          </Tag>
        </span>
      ),
      children: (
        <div className="flex flex-col gap-2">
          {teammates.map((tm) => {
            const ts = getTeammateStatus(tm);
            const sc = getStatusConfig(ts, t);

            return (
              <div
                key={tm.id}
                className="rounded border p-2"
                style={{ borderColor: token.colorBorderSecondary }}
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
                    {tm.currentTask
                      || tm.taskDescription
                      || t("teammatePanel.idle")}
                  </Text>
                </div>

                {/* 消息列表 */}
                {tm.messages && tm.messages.length > 0 && (
                  <div
                    className="mt-1 max-h-40 overflow-y-auto rounded p-1"
                    style={{ backgroundColor: token.colorFillQuaternary }}
                  >
                    {tm.messages.map((msg, i) => (
                      <div
                        key={`${msg.workerId}-${msg.timestamp || i}`}
                        className="border-b py-0.5"
                        style={{ fontSize: 12, lineHeight: "18px", borderColor: token.colorBorderSecondary }}
                      >
                        {formatMessage(msg)}
                      </div>
                    ))}
                  </div>
                )}

                {/* 持续时长 */}
                {tm.duration !== undefined && tm.status === "completed" && (
                  <div className="mt-1">
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {t("teammatePanel.duration", {
                        seconds: (tm.duration / 1000).toFixed(1),
                      })}
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
      style={{
        borderColor: token.colorBorderSecondary,
        backgroundColor: token.colorBgContainer,
      }}
    >
      <div
        className="border-b px-3 py-2 flex items-center justify-between"
        style={{ borderColor: token.colorBorderSecondary }}
      >
        <Text strong style={{ fontSize: 13 }}>
          <TeamOutlined className="mr-1" />
          {t("teammatePanel.title")} (
          {teamNames.reduce((acc, t) => acc + grouped[t].length, 0)})
        </Text>
        <Button
          size="small"
          type="primary"
          ghost
          icon={<TeamOutlined />}
          onClick={() => setTeamModalOpen(true)}
        >
          {t("teammatePanel.createTeam")}
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
        onCreate={handleCreateTeam}
      />
    </div>
  );
}
