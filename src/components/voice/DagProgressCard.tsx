// SPDX-License-Identifier: AGPL-3.0-only

import { Card, Progress, Space, Tag, Timeline, Typography } from "antd";
import { CheckCircle2, Clock, Loader, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface DagNodeProgress {
  nodeId: string;
  nodeName: string;
  status: "pending" | "running" | "completed" | "failed" | "skipped";
  durationMs?: number;
}

interface DagProgressCardProps {
  visible: boolean;
  executionId?: string;
  nodes: DagNodeProgress[];
  overallStatus: "idle" | "running" | "completed" | "failed";
  progressPercent?: number;
}

/**
 * DAG 执行进度卡
 *
 * 在语音驾驶舱中实时展示工作流执行进度，
 * 呼应 HomeRail 的 progress_brief 概念：
 * running / done / error / cancelled 四态播报。
 */
export function DagProgressCard({
  visible,
  executionId,
  nodes,
  overallStatus,
  progressPercent = 0,
}: DagProgressCardProps) {
  const { t } = useTranslation();

  if (!visible) {
    return null;
  }

  const statusTagColor: Record<string, string> = {
    idle: "default",
    running: "processing",
    completed: "success",
    failed: "error",
  };

  const statusIcon: Record<string, React.ReactNode> = {
    idle: <Clock size={14} />,
    running: <Loader size={14} className="animate-spin" />,
    completed: <CheckCircle2 size={14} />,
    failed: <XCircle size={14} />,
  };

  const timelineColor = (status: DagNodeProgress["status"]) => {
    switch (status) {
      case "completed":
        return "green";
      case "running":
        return "blue";
      case "failed":
        return "red";
      case "skipped":
        return "gray";
      default:
        return "gray";
    }
  };

  const nodeIcon = (status: DagNodeProgress["status"]) => {
    switch (status) {
      case "completed":
        return <CheckCircle2 size={14} />;
      case "running":
        return <Loader size={14} className="animate-spin" />;
      case "failed":
        return <XCircle size={14} />;
      default:
        return <Clock size={14} />;
    }
  };

  const statusLabel: Record<string, string> = {
    idle: t("voice.intent.dagRunning"),
    running: t("voice.intent.dagRunning"),
    completed: t("voice.intent.dagComplete"),
    failed: t("voice.intent.dagFailed"),
  };

  return (
    <Card
      size="small"
      className="w-full"
      styles={{ body: { padding: 12 } }}
      title={
        <div className="flex items-center justify-between w-full">
          <Space>
            <Tag color={statusTagColor[overallStatus]}>
              {statusIcon[overallStatus]}
              <span className="ml-1">{t("voice.intent.dagProgress")}</span>
            </Tag>
          </Space>
          {executionId && (
            <Typography.Text type="secondary" style={{ fontSize: 12 }}>
              {executionId}
            </Typography.Text>
          )}
        </div>
      }
    >
      <div className="flex flex-col gap-3">
        {/* 总体进度条 */}
        {overallStatus !== "idle" && (
          <div>
            <div className="flex justify-between mb-1">
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {statusLabel[overallStatus]}
              </Typography.Text>
              <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                {progressPercent}%
              </Typography.Text>
            </div>
            <Progress
              percent={progressPercent}
              size="small"
              status={overallStatus === "running"
                ? "active"
                : overallStatus === "completed"
                ? "success"
                : "exception"}
              showInfo={false}
            />
          </div>
        )}

        {/* 节点时间线 */}
        {nodes.length > 0 && (
          <Timeline
            items={nodes.map((node) => ({
              color: timelineColor(node.status),
              dot: nodeIcon(node.status),
              children: (
                <div className="flex items-center justify-between">
                  <Typography.Text
                    delete={node.status === "skipped"}
                    style={{ fontSize: 13 }}
                  >
                    {node.nodeName}
                  </Typography.Text>
                  {node.durationMs !== undefined && (
                    <Typography.Text type="secondary" style={{ fontSize: 11 }}>
                      {node.durationMs}ms
                    </Typography.Text>
                  )}
                </div>
              ),
            }))}
          />
        )}

        {nodes.length === 0 && overallStatus === "idle" && (
          <Typography.Text type="secondary" style={{ fontSize: 13 }}>
            {t("voice.intent.dagRunning")}
          </Typography.Text>
        )}
      </div>
    </Card>
  );
}
