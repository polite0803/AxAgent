// SPDX-License-Identifier: AGPL-3.0-only

import { EngineDetailPanel } from "@/components/settings/EngineDetailPanel";
import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { EngineStatus } from "@/stores/feature/evolutionStore";
import { Badge, Button, Card, Col, Empty, Row, Space, Statistic, Switch, Typography } from "antd";
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

export function EvolutionSettings() {
  const { t } = useTranslation();
  const engines = useEvolutionStore((s) => s.engines);
  const loading = useEvolutionStore((s) => s.loading);
  const fetchAllEngineStatus = useEvolutionStore((s) => s.fetchAllEngineStatus);
  const startEngine = useEvolutionStore((s) => s.startEngine);
  const stopEngine = useEvolutionStore((s) => s.stopEngine);

  const [detailEngine, setDetailEngine] = useState<string | null>(null);

  // S-P1-1: 添加错误处理
  useEffect(() => {
    fetchAllEngineStatus().catch(() => {
      // store 内部已降级使用 mock 数据
    });
  }, [fetchAllEngineStatus]);

  const engineList = Object.values(engines);
  const runningCount = engineList.filter((e) => e.running).length;

  // S-P1-2: 改为 async + await,避免并发状态竞态
  const handleStartAll = useCallback(async () => {
    for (const e of engineList) {
      if (!e.running) { await startEngine(e.name); }
    }
  }, [engineList, startEngine]);

  const handleStopAll = useCallback(async () => {
    for (const e of engineList) {
      if (e.running) { await stopEngine(e.name); }
    }
  }, [engineList, stopEngine]);

  // S-P1-2: Switch onChange 改为 async
  const handleToggleEngine = useCallback(
    async (checked: boolean, name: string) => {
      if (checked) { await startEngine(name); }
      else { await stopEngine(name); }
    },
    [startEngine, stopEngine],
  );

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
            {t("settings.evolution.startAll")}
          </Button>
          <Button onClick={handleStopAll}>
            {t("settings.evolution.stopAll")}
          </Button>
          <Button onClick={fetchAllEngineStatus} loading={loading}>
            {t("settings.evolution.refresh")}
          </Button>
        </Space>
        <Text>
          {t("settings.evolution.engineCount")}: {runningCount} / {engineList.length}
        </Text>
      </div>

      {/* Engine card grid */}
      {engineList.length === 0
        ? (
          <Empty
            description={t("settings.evolution.empty")}
            image={Empty.PRESENTED_IMAGE_SIMPLE}
          />
        )
        : (
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
                        onChange={(checked) => handleToggleEngine(checked, engine.name)}
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
                            styles={{ content: { fontSize: 14 } }}
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
                        {t("common.details")}
                      </Button>
                    </div>
                  </Card>
                </Col>
              );
            })}
          </Row>
        )}

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
