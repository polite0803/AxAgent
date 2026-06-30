// SPDX-License-Identifier: AGPL-3.0-only

import RLTrainingConfig from "@/components/devtools/RLTrainingConfig";
import RLTrainingMonitor from "@/components/devtools/RLTrainingMonitor";
import RLCheckpointManager from "@/components/devtools/RLCheckpointManager";
import { useRlTrainingStore } from "@/stores/feature/rlTrainingStore";
import { Badge, Button, Space, Tabs, Typography } from "antd";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

const STATUS_BADGE: Record<string, "processing" | "success" | "error" | "warning" | "default"> = {
  idle: "default",
  running: "processing",
  paused: "warning",
  completed: "success",
  failed: "error",
};

const STATUS_LABELS: Record<string, string> = {
  idle: "空闲",
  running: "训练中",
  paused: "已暂停",
  completed: "已完成",
  failed: "失败",
};

export default function RLTrainingPanel() {
  const { t } = useTranslation();
  const status = useRlTrainingStore((s) => s.status);
  const startTraining = useRlTrainingStore((s) => s.startTraining);
  const stopTraining = useRlTrainingStore((s) => s.stopTraining);
  const config = useRlTrainingStore((s) => s.config);

  const isRunning = status === "running";

  return (
    <div style={{ padding: 24 }}>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 24,
          padding: "12px 16px",
          background: "var(--ant-color-bg-container, #fff)",
          borderRadius: 8,
          border: "1px solid var(--ant-color-border-secondary, #f0f0f0)",
        }}
      >
        <Space>
          <Text strong>{t("rl.panel.title", "RL 训练面板")}</Text>
          <Badge
            status={STATUS_BADGE[status] ?? "default"}
            text={STATUS_LABELS[status] ?? status}
          />
        </Space>
        <Space>
          {isRunning ? (
            <Button danger onClick={stopTraining}>
              {t("rl.panel.stop", "停止训练")}
            </Button>
          ) : (
            <Button type="primary" onClick={() => startTraining(config)}>
              {t("rl.panel.start", "启动训练")}
            </Button>
          )}
        </Space>
      </div>

      <Tabs
        defaultActiveKey="config"
        items={[
          {
            key: "config",
            label: t("rl.panel.config", "配置"),
            children: <RLTrainingConfig />,
          },
          {
            key: "monitor",
            label: t("rl.panel.monitor", "监控"),
            children: <RLTrainingMonitor />,
          },
          {
            key: "checkpoints",
            label: t("rl.panel.checkpoints", "检查点"),
            children: <RLCheckpointManager />,
          },
        ]}
      />
    </div>
  );
}
