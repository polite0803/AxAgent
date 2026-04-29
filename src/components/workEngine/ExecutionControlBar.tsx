import {
  HistoryOutlined,
  PauseCircleOutlined,
  PlayCircleOutlined,
  ReloadOutlined,
  StopOutlined,
} from "@ant-design/icons";
import { Button, Popconfirm, Space, Tag, Tooltip, message } from "antd";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "../../lib/invoke";
import { useWorkEngineStore } from "../../stores/feature/workEngineStore";
import { useSettingsStore } from "../../stores";

interface ExecutionControlBarProps {
  workflowId: string;
}

export const ExecutionControlBar: React.FC<ExecutionControlBarProps> = ({ workflowId }) => {
  const { t } = useTranslation();
  const {
    executionId,
    status,
    pause,
    resume,
    cancel,
    loadHistory,
    setupEventListeners,
  } = useWorkEngineStore();
  const defaultProviderId = useSettingsStore((s) => s.settings.default_provider_id);
  const [starting, setStarting] = useState(false);

  useEffect(() => {
    const cleanup = setupEventListeners();
    return () => {
      cleanup.then((fn) => fn());
    };
  }, [setupEventListeners]);

  const isRunning = status?.status === "running";
  const isPaused = status?.status === "paused";
  const isCompleted = status?.status === "completed" || status?.status === "failed" || status?.status === "cancelled";

  const handleStart = async () => {
    if (!defaultProviderId) {
      message.error(t("workEngine.configureProviderFirst"));
      return;
    }
    setStarting(true);
    try {
      await invoke<string>("workflow_execute", {
        workflow_id: workflowId,
        provider_id: defaultProviderId,
      });
      message.success(t("workEngine.workflowStarted"));
    } catch (e) {
      message.error(`${t("workEngine.startFailed")}: ${e}`);
    } finally {
      setStarting(false);
    }
  };

  return (
    <Space>
      {!executionId || isCompleted
        ? (
          <Tooltip title={t("workEngine.startExecution")}>
            <Button
              type="primary"
              icon={<PlayCircleOutlined />}
              loading={starting}
              onClick={handleStart}
              size="small"
            >
              {t("workEngine.start")}
            </Button>
          </Tooltip>
        )
        : null}

      {isRunning && (
        <Tooltip title={t("workEngine.pauseExecution")}>
          <Button icon={<PauseCircleOutlined />} onClick={pause} size="small">{t("workEngine.pause")}</Button>
        </Tooltip>
      )}

      {isPaused && (
        <Tooltip title={t("workEngine.resumeExecution")}>
          <Button type="primary" icon={<ReloadOutlined />} onClick={resume} size="small">{t("workEngine.resume")}</Button>
        </Tooltip>
      )}

      {(isRunning || isPaused) && (
        <Popconfirm title={t("workEngine.confirmCancelExecution")} onConfirm={cancel}>
          <Tooltip title={t("workEngine.cancelExecution")}>
            <Button danger icon={<StopOutlined />} size="small">{t("workEngine.cancel")}</Button>
          </Tooltip>
        </Popconfirm>
      )}

      {status && (
        <Tag
          color={isRunning
            ? "processing"
            : isPaused
            ? "warning"
            : status.status === "completed"
            ? "success"
            : status.status === "failed"
            ? "error"
            : "default"}
        >
          {status.status === "running"
            ? t("workEngine.statusRunning")
            : status.status === "paused"
            ? t("workEngine.statusPaused")
            : status.status === "completed"
            ? t("workEngine.statusCompleted")
            : status.status === "failed"
            ? t("workEngine.statusFailed")
            : status.status === "cancelled"
            ? t("workEngine.statusCancelled")
            : status.status}
        </Tag>
      )}

      {status && status.total_time_ms > 0 && (
        <span style={{ fontSize: 12, color: "#999" }}>{status.total_time_ms}ms</span>
      )}

      <Tooltip title={t("workEngine.executionHistory")}>
        <Button icon={<HistoryOutlined />} onClick={() => loadHistory(workflowId)} size="small" />
      </Tooltip>
    </Space>
  );
};
