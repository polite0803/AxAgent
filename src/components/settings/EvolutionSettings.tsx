// SPDX-License-Identifier: AGPL-3.0-only

import { EngineDetailPanel } from "@/components/settings/EngineDetailPanel";
import { useEvolutionStore } from "@/stores/feature/evolutionStore";
import type { EngineStatus } from "@/stores/feature/evolutionStore";
import type { RuntimeToolInfo } from "@/types";
import {
  Badge,
  Button,
  Card,
  Col,
  Empty,
  message,
  Popconfirm,
  Row,
  Space,
  Statistic,
  Switch,
  Table,
  Tag,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
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
  // 阶段二 T2.5：运行时动态工具管理
  const runtimeTools = useEvolutionStore((s) => s.runtimeTools);
  const listRuntimeTools = useEvolutionStore((s) => s.listRuntimeTools);
  const unregisterRuntimeTool = useEvolutionStore((s) => s.unregisterRuntimeTool);

  const [detailEngine, setDetailEngine] = useState<string | null>(null);
  const [runtimeLoading, setRuntimeLoading] = useState(false);
  const [unloadingName, setUnloadingName] = useState<string | null>(null);

  // S-P1-1: 添加错误处理
  useEffect(() => {
    fetchAllEngineStatus().catch(() => {
      // store 内部已降级使用 mock 数据
    });
  }, [fetchAllEngineStatus]);

  // 阶段二 T2.5：挂载时加载运行时动态工具列表
  useEffect(() => {
    listRuntimeTools().catch(() => {
      // store 内部已降级使用 mock 数据
    });
  }, [listRuntimeTools]);

  const refreshRuntimeTools = useCallback(async () => {
    setRuntimeLoading(true);
    try {
      await listRuntimeTools();
    } finally {
      setRuntimeLoading(false);
    }
  }, [listRuntimeTools]);

  const handleUnregister = useCallback(
    async (name: string) => {
      setUnloadingName(name);
      try {
        await unregisterRuntimeTool(name);
        message.success(t("settings.evolution.runtimeTools.unloadSuccess", { name }));
      } catch {
        message.error(t("settings.evolution.runtimeTools.unloadFail", { name }));
      } finally {
        setUnloadingName(null);
      }
    },
    [unregisterRuntimeTool, t],
  );

  // 运行时工具表列定义
  const runtimeColumns: ColumnsType<RuntimeToolInfo> = [
    {
      title: t("settings.evolution.runtimeTools.name"),
      dataIndex: "name",
      key: "name",
      render: (name: string) => <Text code>{name}</Text>,
    },
    {
      title: t("settings.evolution.runtimeTools.source"),
      dataIndex: "source",
      key: "source",
      width: 180,
      render: (source: string) => {
        const isSystem = source === "system_evolution";
        return (
          <Tag color={isSystem ? "purple" : "blue"}>
            {isSystem
              ? t("settings.evolution.runtimeTools.sourceSystem")
              : t("settings.evolution.runtimeTools.sourceEvolved")}
          </Tag>
        );
      },
    },
    {
      title: t("settings.evolution.runtimeTools.action"),
      key: "action",
      width: 120,
      align: "right",
      render: (_, record) => {
        // 自指工具（system_evolution）为系统内建能力，不可卸载，仅展示
        if (record.source === "system_evolution") {
          return (
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("settings.evolution.runtimeTools.builtin")}
            </Text>
          );
        }
        return (
          <Popconfirm
            title={t("settings.evolution.runtimeTools.unloadConfirm", { name: record.name })}
            okText={t("common.confirm")}
            cancelText={t("common.cancel")}
            onConfirm={() => handleUnregister(record.name)}
          >
            <Button danger size="small" loading={unloadingName === record.name}>
              {t("settings.evolution.runtimeTools.unload")}
            </Button>
          </Popconfirm>
        );
      },
    },
  ];

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

      {/* ── 阶段二 T2.5：运行时动态工具管理 ── */}
      <Card
        size="small"
        style={{ marginTop: 16 }}
        title={
          <Space size={8}>
            <span>{t("settings.evolution.runtimeTools.title")}</span>
            <Tag color="purple" style={{ margin: 0 }}>
              {t("settings.evolution.runtimeTools.systemChannel")}
            </Tag>
          </Space>
        }
        extra={
          <Button size="small" onClick={refreshRuntimeTools} loading={runtimeLoading}>
            {t("settings.evolution.refresh")}
          </Button>
        }
      >
        <Paragraph type="secondary" style={{ fontSize: 12, marginBottom: 12 }}>
          {t("settings.evolution.runtimeTools.desc")}
        </Paragraph>
        <Table<RuntimeToolInfo>
          rowKey="name"
          size="small"
          columns={runtimeColumns}
          dataSource={runtimeTools}
          loading={runtimeLoading}
          pagination={false}
          locale={{
            emptyText: (
              <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("settings.evolution.runtimeTools.empty")} />
            ),
          }}
        />
      </Card>

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
