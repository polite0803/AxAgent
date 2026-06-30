// SPDX-License-Identifier: AGPL-3.0-only

import EngineDetailPanel from "@/components/settings/EngineDetailPanel";
import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { EngineStatus } from "@/stores/feature/evolutionStore";
import { Badge, Button, Card, Col, Row, Space, Statistic, Switch, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

const CATEGORY_COLORS: Record<string, string> = {
  core: "blue",
  learning: "green",
  safety: "orange",
  experimental: "purple",
};

function getTopStats(engine: EngineStatus): { key: string; label: string; value: string | number }[] {
  const entries = Object.entries(engine.stats).slice(0, 3);
  return entries.map(([key, value]) => ({
    key,
    label: key.replace(/([A-Z])/g, " $1").replace(/^./, (s) => s.toUpperCase()),
    value: typeof value === "number" && key.toLowerCase().includes("time") && value > 1000000000
      ? new Date(value as number).toLocaleDateString()
      : String(value),
  }));
}

export default function EvolutionSettings() {
  const { t } = useTranslation();
  const engines = useEvolutionStore((s) => s.engines);
  const loading = useEvolutionStore((s) => s.loading);
  const fetchAllEngineStatus = useEvolutionStore((s) => s.fetchAllEngineStatus);
  const startEngine = useEvolutionStore((s) => s.startEngine);
  const stopEngine = useEvolutionStore((s) => s.stopEngine);

  const [detailEngine, setDetailEngine] = useState<string | null>(null);

  useEffect(() => {
    fetchAllEngineStatus();
  }, [fetchAllEngineStatus]);

  const engineList = Object.values(engines);
  const runningCount = engineList.filter((e) => e.running).length;

  const handleStartAll = useCallback(() => {
    for (const e of engineList) {
      if (!e.running) { startEngine(e.name); }
    }
  }, [engineList, startEngine]);

  const handleStopAll = useCallback(() => {
    for (const e of engineList) {
      if (e.running) { stopEngine(e.name); }
    }
  }, [engineList, stopEngine]);

  return (
    <div style={{ padding: 24 }}>
      {/* Global control bar */}
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
          <Button type="primary" onClick={handleStartAll}>
            {t("settings.evolution.startAll", "全部启动")}
          </Button>
          <Button onClick={handleStopAll}>
            {t("settings.evolution.stopAll", "全部停止")}
          </Button>
          <Button onClick={fetchAllEngineStatus} loading={loading}>
            {t("settings.evolution.refresh", "刷新状态")}
          </Button>
        </Space>
        <Text>
          {t("settings.evolution.engineCount", "运行中")}: {runningCount} / {engineList.length}
        </Text>
      </div>

      {/* Engine card grid */}
      <Row gutter={[16, 16]}>
        {engineList.map((engine) => {
          const topStats = getTopStats(engine);
          return (
            <Col key={engine.name} xs={24} sm={12} lg={8}>
              <Card
                size="small"
                hoverable
                title={
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <Badge status={engine.running ? "processing" : "default"} />
                    <span style={{ fontSize: 14, fontWeight: 600 }}>{engine.displayName}</span>
                    <span
                      style={{
                        fontSize: 11,
                        padding: "1px 6px",
                        borderRadius: 4,
                        background: CATEGORY_COLORS[engine.category] ?? "#888",
                        color: "#fff",
                      }}
                    >
                      {engine.category}
                    </span>
                  </div>
                }
                extra={
                  <Switch
                    checked={engine.running}
                    size="small"
                    onChange={(checked) => {
                      if (checked) { startEngine(engine.name); }
                      else { stopEngine(engine.name); }
                    }}
                  />
                }
              >
                <Paragraph
                  type="secondary"
                  ellipsis={{ rows: 2 }}
                  style={{ fontSize: 12, marginBottom: 12, minHeight: 36 }}
                >
                  {engine.description}
                </Paragraph>

                <Row gutter={8}>
                  {topStats.map((s) => (
                    <Col key={s.key} span={8}>
                      <Statistic
                        title={s.label}
                        value={s.value}
                        valueStyle={{ fontSize: 14 }}
                      />
                    </Col>
                  ))}
                </Row>

                <div style={{ marginTop: 12, textAlign: "right" }}>
                  <Button
                    type="link"
                    size="small"
                    onClick={() => setDetailEngine(engine.name)}
                  >
                    {t("common.details", "详情")}
                  </Button>
                </div>
              </Card>
            </Col>
          );
        })}
      </Row>

      {/* Detail drawer */}
      {detailEngine && (
        <EngineDetailPanel
          engineName={detailEngine}
          open={detailEngine !== null}
          onClose={() => setDetailEngine(null)}
        />
      )}
    </div>
  );
}
