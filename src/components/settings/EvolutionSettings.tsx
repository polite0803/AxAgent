// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { invoke } from "@/lib/invoke";
import { Badge, Button, Card, Col, Divider, Row, Spin, Statistic, Tag, theme, Typography } from "antd";
import { Activity, Brain, Dna, FlaskConical, Lightbulb, RefreshCw, Shield, Sparkles, Wrench } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { pushNotification } from "../layout/NotificationBell";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

interface EvolutionEngineStatus {
  name: string;
  running: boolean;
  last_run: string | null;
  items_processed: number;
}

interface EvolutionStats {
  skill_count: number;
  total_trajectories: number;
  evolution_engines: EvolutionEngineStatus[];
  auto_tools_count: number;
  auto_tool_patterns: string[];
  text_grad_nodes: number;
  text_grad_gradients: number;
  constitution_rules: number;
  intrinsic_motivation_active: boolean;
  coevolution_tasks: number;
  dream_knowledge_count: number;
  prm_enabled: boolean;
  sandbox_enabled: boolean;
  llm_provider_connected: boolean;
}

const ENGINE_ICONS: Record<string, React.ReactNode> = {
  "Skill Evolution": <Dna size={16} />,
  "RL Reward": <Activity size={16} />,
  "Process Reward Model": <FlaskConical size={16} />,
  "Auto Tool Creator": <Wrench size={16} />,
  "TextGrad Engine": <Brain size={16} />,
  "Dream Consolidator": <Lightbulb size={16} />,
  "Intrinsic Motivation": <Sparkles size={16} />,
  Coevolution: <Dna size={16} />,
};

const ENGINE_NAME_KEY_MAP: Record<string, string> = {
  "Skill Evolution": "evolution.engineNames.skillEvolution",
  "RL Reward": "evolution.engineNames.rlReward",
  "Process Reward Model": "evolution.engineNames.processRewardModel",
  "Auto Tool Creator": "evolution.engineNames.autoToolCreator",
  "TextGrad Engine": "evolution.engineNames.textGradEngine",
  "Dream Consolidator": "evolution.engineNames.dreamConsolidator",
  "Intrinsic Motivation": "evolution.engineNames.intrinsicMotivation",
  Coevolution: "evolution.engineNames.coevolution",
};

function EngineStatusCard({ engine }: { engine: EvolutionEngineStatus }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const icon = ENGINE_ICONS[engine.name] ?? <Activity size={16} />;

  return (
    <Card
      size="small"
      style={{
        borderRadius: 8,
        border: `1px solid ${engine.running ? token.colorSuccessBorder : token.colorBorderSecondary}`,
        backgroundColor: engine.running
          ? token.colorSuccessBg
          : token.colorBgContainer,
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          marginBottom: 4,
        }}
      >
        <span
          style={{
            color: engine.running
              ? token.colorSuccess
              : token.colorTextQuaternary,
          }}
        >
          {icon}
        </span>
        <Text strong style={{ fontSize: 13, flex: 1 }}>
          {t(ENGINE_NAME_KEY_MAP[engine.name] ?? engine.name, engine.name)}
        </Text>
        <Badge
          status={engine.running ? "success" : "default"}
          text={
            <Text
              style={{
                fontSize: 12,
                color: engine.running
                  ? token.colorSuccess
                  : token.colorTextQuaternary,
              }}
            >
              {engine.running ? t("evolution.running") : t("evolution.idle")}
            </Text>
          }
        />
      </div>
      <Text type="secondary" style={{ fontSize: 12 }}>
        {t("evolution.processed", { count: engine.items_processed })}
      </Text>
    </Card>
  );
}

function InfrastructureStatus({ stats }: { stats: EvolutionStats }) {
  const { t } = useTranslation();

  const items = [
    { label: t("evolution.llmProvider"), ok: stats.llm_provider_connected },
    { label: t("evolution.sandbox"), ok: stats.sandbox_enabled },
    { label: t("evolution.prm"), ok: stats.prm_enabled },
    { label: t("evolution.intrinsic"), ok: stats.intrinsic_motivation_active },
  ];

  return (
    <SettingsGroup title={t("evolution.infrastructure")}>
      <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
        {items.map((item) => (
          <Tag
            key={item.label}
            color={item.ok ? "success" : "default"}
            style={{ margin: 0, borderRadius: 6, padding: "2px 10px" }}
          >
            {item.ok ? "✓" : "○"} {item.label}
          </Tag>
        ))}
      </div>
    </SettingsGroup>
  );
}

function MetricsOverview({ stats }: { stats: EvolutionStats }) {
  const { t } = useTranslation();

  return (
    <SettingsGroup title={t("evolution.metricsOverview")}>
      <Row gutter={[12, 12]}>
        <Col span={6}>
          <Statistic
            title={t("evolution.skillCount")}
            value={stats.skill_count}
            valueStyle={{ fontSize: 20 }}
          />
        </Col>
        <Col span={6}>
          <Statistic
            title={t("evolution.trajectories")}
            value={stats.total_trajectories}
            valueStyle={{ fontSize: 20 }}
          />
        </Col>
        <Col span={6}>
          <Statistic
            title={t("evolution.autoTools")}
            value={stats.auto_tools_count}
            valueStyle={{ fontSize: 20 }}
          />
        </Col>
        <Col span={6}>
          <Statistic
            title={t("evolution.dreamKnowledge")}
            value={stats.dream_knowledge_count}
            valueStyle={{ fontSize: 20 }}
          />
        </Col>
      </Row>
      <Divider style={{ margin: "12px 0" }} />
      <Row gutter={[12, 12]}>
        <Col span={6}>
          <Statistic
            title={t("evolution.textGradNodes")}
            value={stats.text_grad_nodes}
            valueStyle={{ fontSize: 20 }}
          />
        </Col>
        <Col span={6}>
          <Statistic
            title={t("evolution.textGradGradients")}
            value={stats.text_grad_gradients}
            valueStyle={{ fontSize: 20 }}
          />
        </Col>
        <Col span={6}>
          <Statistic
            title={t("evolution.constitutionRules")}
            value={stats.constitution_rules}
            valueStyle={{ fontSize: 20 }}
          />
        </Col>
        <Col span={6}>
          <Statistic
            title={t("evolution.coevolutionTasks")}
            value={stats.coevolution_tasks}
            valueStyle={{ fontSize: 20 }}
          />
        </Col>
      </Row>
    </SettingsGroup>
  );
}

function AutoToolPatterns({ patterns }: { patterns: string[] }) {
  const { t } = useTranslation();

  if (patterns.length === 0) {
    return null;
  }

  return (
    <SettingsGroup title={t("evolution.frequentPatterns")}>
      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
        {patterns.map((p) => (
          <Tag key={p} style={{ borderRadius: 6, margin: 0 }}>
            {p}
          </Tag>
        ))}
      </div>
    </SettingsGroup>
  );
}

export function EvolutionSettings() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [stats, setStats] = useState<EvolutionStats | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchStats = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<EvolutionStats>("get_evolution_stats");
      if (result && !Array.isArray(result)) {
        setStats(result);
        const prevCount = stats?.auto_tools_count ?? 0;
        if (result.auto_tools_count > prevCount) {
          pushNotification("success", t("evolution.newToolDiscovered"));
        }
        const prevKnowledge = stats?.dream_knowledge_count ?? 0;
        if (result.dream_knowledge_count > prevKnowledge) {
          pushNotification("info", t("evolution.newDreamKnowledge"));
        }
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [stats, t]);

  const fetchStatsRef = useRef(fetchStats);
  useEffect(() => {
    fetchStatsRef.current = fetchStats;
  });

  useEffect(() => {
    fetchStatsRef.current();
    const interval = setInterval(() => fetchStatsRef.current(), 30000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div>
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          marginBottom: 16,
        }}
      >
        <div>
          <Text strong style={{ fontSize: 16 }}>
            {t("evolution.title")}
          </Text>
          <br />
          <Text type="secondary" style={{ fontSize: 12 }}>
            {t("evolution.description")}
          </Text>
        </div>
        <Tooltip title={t("evolution.refresh")}>
          <Button
            size="small"
            icon={<RefreshCw size={14} />}
            onClick={fetchStats}
            loading={loading}
            style={{ display: "flex", alignItems: "center", gap: 4 }}
          >
            {t("evolution.refresh")}
          </Button>
        </Tooltip>
      </div>

      {error && (
        <Card
          size="small"
          style={{ marginBottom: 12, borderColor: token.colorErrorBorder }}
        >
          <Text type="danger" style={{ fontSize: 12 }}>
            {error}
          </Text>
        </Card>
      )}

      {loading && !stats && (
        <div style={{ textAlign: "center", padding: 40 }}>
          <Spin />
        </div>
      )}

      {stats && (
        <>
          <InfrastructureStatus stats={stats} />
          <MetricsOverview stats={stats} />

          <SettingsGroup title={t("evolution.engineStatus")}>
            <Row gutter={[8, 8]}>
              {stats.evolution_engines.map((engine) => (
                <Col span={12} key={engine.name}>
                  <EngineStatusCard engine={engine} />
                </Col>
              ))}
            </Row>
          </SettingsGroup>

          <AutoToolPatterns patterns={stats.auto_tool_patterns} />

          <SettingsGroup title={t("evolution.constitutionShield")}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Shield
                size={16}
                style={{
                  color: stats.constitution_rules > 0
                    ? token.colorSuccess
                    : token.colorTextQuaternary,
                }}
              />
              <Text style={{ fontSize: 13 }}>
                {stats.constitution_rules > 0
                  ? t("evolution.constitutionActive", {
                    count: stats.constitution_rules,
                  })
                  : t("evolution.constitutionEmpty")}
              </Text>
            </div>
          </SettingsGroup>
        </>
      )}
    </div>
  );
}
