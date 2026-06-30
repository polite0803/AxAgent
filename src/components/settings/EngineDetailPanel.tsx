// SPDX-License-Identifier: AGPL-3.0-only

import type { ConfigField } from "@/components/settings/EngineConfigForm";
import EngineConfigForm from "@/components/settings/EngineConfigForm";
import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { EngineStatus } from "@/stores/feature/evolutionStore";
import { Badge, Button, Descriptions, Drawer, Tabs, Tag, Typography } from "antd";
import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

const CATEGORY_COLORS: Record<string, string> = {
  core: "blue",
  learning: "green",
  safety: "orange",
  experimental: "purple",
};

const CATEGORY_LABELS: Record<string, string> = {
  core: "核心",
  learning: "学习",
  safety: "安全",
  experimental: "实验",
};

function buildConfigFields(engine: EngineStatus): ConfigField[] {
  const fields: ConfigField[] = [];
  for (const [key, value] of Object.entries(engine.config)) {
    const label = key.replace(/([A-Z])/g, " $1").replace(/^./, (s) => s.toUpperCase());
    if (typeof value === "boolean") {
      fields.push({ key, label, type: "switch" });
    } else if (typeof value === "number") {
      if (key.toLowerCase().includes("rate") || key.toLowerCase().includes("ratio") || key.toLowerCase().includes("threshold")) {
        fields.push({ key, label, type: "number", min: 0, max: 1, step: 0.01 });
      } else if (key.toLowerCase().includes("count") || key.toLowerCase().includes("size") || key.toLowerCase().includes("budget")) {
        fields.push({ key, label, type: "number", min: 1, max: 10000, step: 1 });
      } else if (key.toLowerCase().includes("interval") || key.toLowerCase().includes("timeout")) {
        fields.push({ key, label, type: "number", min: 0, max: 86400000, step: 1000 });
      } else {
        fields.push({ key, label, type: "number", min: 0, step: 0.001 });
      }
    } else if (typeof value === "string") {
      if (key.toLowerCase().includes("language") || key.toLowerCase().includes("optimizer") || key.toLowerCase().includes("type")) {
        fields.push({
          key,
          label,
          type: "select",
          options: (key === "optimizer"
            ? ["adam", "sgd", "rmsprop", "adagrad"]
            : key.toLowerCase().includes("complexity")
            ? ["low", "medium", "high"]
            : ["python", "javascript", "bash"]
          ).map((v) => ({ label: v, value: v })),
        });
      } else {
        fields.push({ key, label, type: "text" });
      }
    }
  }
  return fields;
}

interface EngineDetailPanelProps {
  engineName: string;
  open: boolean;
  onClose: () => void;
}

export default function EngineDetailPanel({ engineName, open, onClose }: EngineDetailPanelProps) {
  const { t } = useTranslation();
  const engines = useEvolutionStore((s) => s.engines);
  const startEngine = useEvolutionStore((s) => s.startEngine);
  const stopEngine = useEvolutionStore((s) => s.stopEngine);
  const updateEngineConfig = useEvolutionStore((s) => s.updateEngineConfig);
  const fetchEngineLogs = useEvolutionStore((s) => s.fetchEngineLogs);

  const engine = engines[engineName];
  const logsEndRef = useRef<HTMLDivElement>(null);

  const configFields = useMemo(() => (engine ? buildConfigFields(engine) : []), [engine]);

  useEffect(() => {
    if (open && engine) {
      fetchEngineLogs(engineName);
    }
  }, [open, engineName, engine, fetchEngineLogs]);

  useEffect(() => {
    logsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [engine?.logs]);

  if (!engine) {
    return (
      <Drawer title="引擎未找到" open={open} onClose={onClose} width={640}>
        <Text type="secondary">未找到引擎 "{engineName}"</Text>
      </Drawer>
    );
  }

  const handleSaveConfig = (config: Record<string, unknown>) => {
    updateEngineConfig(engineName, config);
  };

  const runningCount = engine.logs.length > 0 ? engine.logs.length : 0;
  const warnCount = engine.logs.filter((l) => l.level === "warn").length;
  const errorCount = engine.logs.filter((l) => l.level === "error").length;

  return (
    <Drawer
      title={
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <span>{engine.displayName}</span>
          <Badge status={engine.running ? "processing" : "default"} text={engine.running ? "运行中" : "已停止"} />
          <Tag color={CATEGORY_COLORS[engine.category]}>{CATEGORY_LABELS[engine.category] ?? engine.category}</Tag>
        </div>
      }
      open={open}
      onClose={onClose}
      width={720}
      extra={
        <Button
          type={engine.running ? "default" : "primary"}
          danger={engine.running}
          onClick={() => (engine.running ? stopEngine(engineName) : startEngine(engineName))}
        >
          {engine.running ? "停止" : "启动"}
        </Button>
      }
    >
      <Paragraph type="secondary" style={{ marginBottom: 16 }}>{engine.description}</Paragraph>

      <Tabs
        defaultActiveKey="config"
        items={[
          {
            key: "config",
            label: t("settings.evolution.config", "配置"),
            children: (
              <EngineConfigForm
                config={engine.config}
                fields={configFields}
                onSave={handleSaveConfig}
              />
            ),
          },
          {
            key: "stats",
            label: t("settings.evolution.stats", "统计"),
            children: (
              <Descriptions column={2} size="small" bordered>
                {Object.entries(engine.stats).map(([key, value]) => (
                  <Descriptions.Item key={key} label={key.replace(/([A-Z])/g, " $1").replace(/^./, (s) => s.toUpperCase())}>
                    {typeof value === "number" && key.toLowerCase().includes("time") && value > 1000000000
                      ? new Date(value as number).toLocaleString()
                      : String(value)}
                  </Descriptions.Item>
                ))}
              </Descriptions>
            ),
          },
          {
            key: "logs",
            label: (
              <span>
                {t("settings.evolution.logs", "日志")}
                {runningCount > 0 && (
                  <span style={{ marginLeft: 4, fontSize: 12, color: "#888" }}>
                    ({runningCount})
                  </span>
                )}
                {warnCount > 0 && (
                  <span style={{ marginLeft: 4, fontSize: 12, color: "#faad14" }}>
                    W{warnCount}
                  </span>
                )}
                {errorCount > 0 && (
                  <span style={{ marginLeft: 4, fontSize: 12, color: "#ff4d4f" }}>
                    E{errorCount}
                  </span>
                )}
              </span>
            ),
            children: (
              <div style={{ maxHeight: 400, overflow: "auto", fontFamily: "monospace", fontSize: 12 }}>
                {engine.logs.length === 0 ? (
                  <Text type="secondary">暂无日志</Text>
                ) : (
                  engine.logs.map((log, i) => (
                    <div
                      key={i}
                      style={{
                        padding: "2px 0",
                        color: log.level === "error" ? "#ff4d4f" : log.level === "warn" ? "#faad14" : "#888",
                      }}
                    >
                      <span style={{ color: "#999", marginRight: 8 }}>
                        {new Date(log.timestamp).toLocaleTimeString()}
                      </span>
                      [{log.level.toUpperCase()}] {log.message}
                    </div>
                  ))
                )}
                <div ref={logsEndRef} />
              </div>
            ),
          },
        ]}
      />
    </Drawer>
  );
}
