// SPDX-License-Identifier: AGPL-3.0-only

/**
 * TrajectoryPanel — agent 轨迹面板。
 *
 * 展示最近的 dispatcher 事件流（routing / process / token_usage /
 * agent_status）以时间线形式呈现，便于观察 agent 的协作过程。
 *
 * 与 ChatPanel 共享 dispatchEvents，但只展示「过程类」事件，
 * 过滤掉 agent_message 和 complete。
 */

import { useOfficeStore } from "@/stores";
import type { DispatchEvent } from "@/types";
import { Empty, Tag, theme, Timeline } from "antd";
import { useTranslation } from "react-i18next";

export function TrajectoryPanel() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const events = useOfficeStore((s) => s.dispatchEvents);

  const trajectoryEvents = events.filter((e) =>
    e.type === "routing" || e.type === "process" || e.type === "agent_status"
    || e.type === "token_usage"
  );

  if (trajectoryEvents.length === 0) {
    return (
      <div style={{ padding: 24, height: "100%" }}>
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("office.trajectory.empty")}
          styles={{ description: { fontSize: 12, color: token.colorTextQuaternary } }}
        />
      </div>
    );
  }

  const items = trajectoryEvents.map((e, i) => ({
    key: i,
    children: <TrajectoryItem event={e} />,
    color: getEventColor(e),
  }));

  return (
    <div style={{ padding: 12, height: "100%", overflow: "auto" }}>
      <Timeline items={items} />
    </div>
  );
}

function TrajectoryItem({ event }: { event: DispatchEvent }) {
  const { t } = useTranslation();
  switch (event.type) {
    case "routing":
      return (
        <div style={{ fontSize: 12 }}>
          <Tag color="blue" style={{ fontSize: 10 }}>
            {t("office.trajectory.tagRouting")}
          </Tag>
          <span>
            {t("office.trajectory.routing", {
              slug: event.agentSlug,
              summary: event.taskSummary.slice(0, 60),
            })}
          </span>
        </div>
      );
    case "process":
      return (
        <div style={{ fontSize: 12 }}>
          <Tag color="purple" style={{ fontSize: 10 }}>
            {t("office.trajectory.tagProcess")}
          </Tag>
          <span>
            {event.agentSlug}: {event.status}
          </span>
        </div>
      );
    case "agent_status":
      return (
        <div style={{ fontSize: 12 }}>
          <Tag color="orange" style={{ fontSize: 10 }}>
            {t("office.trajectory.tagStatus")}
          </Tag>
          <span>
            {event.agentSlug}: {t(`office.memberStatus.${event.status}`)}
          </span>
        </div>
      );
    case "token_usage":
      return (
        <div style={{ fontSize: 12 }}>
          <Tag color="green" style={{ fontSize: 10 }}>
            {t("office.trajectory.tagToken")}
          </Tag>
          <span>
            {event.agentSlug}: +{event.inputTokens}↑ / +{event.outputTokens}↓
          </span>
        </div>
      );
    default:
      return null;
  }
}

function getEventColor(event: DispatchEvent): string {
  switch (event.type) {
    case "routing":
      return "blue";
    case "process":
      return "purple";
    case "agent_status":
      return "orange";
    case "token_usage":
      return "green";
    default:
      return "gray";
  }
}
