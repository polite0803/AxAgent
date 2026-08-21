import { invoke } from "@/lib/invoke";
import type { McRunRequest, RobustnessResult } from "@/types/market-sim";
import {
  Button,
  Card,
  Checkbox,
  Col,
  Descriptions,
  Divider,
  InputNumber,
  Row,
  Spin,
  Statistic,
  Table,
  Tag,
} from "antd";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface ScenarioConfig {
  key: string;
  label: string;
  enabled: boolean;
  paths: number;
}

const DEFAULT_SCENARIOS: ScenarioConfig[] = [
  { key: "normal", label: "stockAnalysis.monteCarlo.normal", enabled: true, paths: 20 },
  { key: "bull", label: "stockAnalysis.monteCarlo.bull", enabled: true, paths: 20 },
  { key: "bear", label: "stockAnalysis.monteCarlo.bear", enabled: true, paths: 20 },
  { key: "flash_crash", label: "stockAnalysis.monteCarlo.flashCrash", enabled: false, paths: 15 },
  { key: "high_vol", label: "stockAnalysis.monteCarlo.highVol", enabled: false, paths: 15 },
];

export function MonteCarloPanel() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<RobustnessResult | null>(null);
  const [stockCode, setStockCode] = useState("000001");
  const [refPrice, setRefPrice] = useState(1000);
  const [simMs, setSimMs] = useState(50);
  const [scenarios, setScenarios] = useState<ScenarioConfig[]>(DEFAULT_SCENARIOS);
  const tokenRef = useRef(0);

  const toggleScenario = (key: string) => {
    setScenarios((prev) => prev.map((s) => (s.key === key ? { ...s, enabled: !s.enabled } : s)));
  };

  const setPaths = (key: string, paths: number) => {
    setScenarios((prev) => prev.map((s) => (s.key === key ? { ...s, paths } : s)));
  };

  const handleRun = async () => {
    const activeScenarios = scenarios.filter((s) => s.enabled);
    if (activeScenarios.length === 0) {
      setError(t("stockAnalysis.monte-carlo-panel.select-at-least-one-scenario"));
      return;
    }

    const myToken = ++tokenRef.current;
    setLoading(true);
    setError(null);
    setReport(null);

    try {
      const request: McRunRequest = {
        stockCode,
        referencePrice: refPrice,
        maxSimTimeNs: simMs * 1_000_000,
        scenarios: activeScenarios.map((s) => ({
          scenario: s.key,
          paths: s.paths,
        })),
      };

      const result = await invoke<RobustnessResult>("market_sim_run_mc", { request });
      if (myToken !== tokenRef.current) {
        return;
      }
      setReport(result);
    } catch (e: unknown) {
      if (myToken !== tokenRef.current) {
        return;
      }
      setError(typeof e === "string" ? e : e instanceof Error ? e.message : String(e));
    } finally {
      if (myToken === tokenRef.current) {
        setLoading(false);
      }
    }
  };

  const totalPaths = scenarios.filter((s) => s.enabled).reduce((sum, s) => sum + s.paths, 0);

  return (
    <div className="space-y-4">
      {/* 配置区 */}
      <Card size="small" title={t("stockAnalysis.monte-carlo-panel.robustness-test-config")}>
        <div className="mb-3 flex flex-wrap items-center gap-4">
          <label className="text-sm font-medium">
            {t("stockAnalysis.monte-carlo-panel.stock-code")}
            <InputNumber
              className="ml-2"
              style={{ width: 110 }}
              value={stockCode}
              onChange={(v) => setStockCode(v ?? "000001")}
            />
          </label>
          <label className="text-sm font-medium">
            {t("stockAnalysis.monte-carlo-panel.reference-price")}
            <InputNumber
              className="ml-2"
              style={{ width: 120 }}
              min={1}
              value={refPrice}
              onChange={(v) => setRefPrice(v ?? 1000)}
            />
          </label>
          <label className="text-sm font-medium">
            {t("stockAnalysis.monte-carlo-panel.duration-ms")}
            <InputNumber
              className="ml-2"
              style={{ width: 100 }}
              min={1}
              max={1000}
              value={simMs}
              onChange={(v) => setSimMs(v ?? 50)}
            />
          </label>
        </div>

        <Divider style={{ margin: "8px 0" }} />

        <div className="mb-3 flex flex-wrap gap-4">
          {scenarios.map((sc) => (
            <div key={sc.key} className="flex items-center gap-2 rounded-lg border px-3 py-1.5">
              <Checkbox checked={sc.enabled} onChange={() => toggleScenario(sc.key)} />
              <span className="text-sm">{t(sc.label)}</span>
              <InputNumber
                size="small"
                style={{ width: 65 }}
                min={1}
                max={100}
                value={sc.paths}
                disabled={!sc.enabled}
                onChange={(v) => setPaths(sc.key, v ?? 10)}
              />
            </div>
          ))}
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm text-secondary">
            {t("stockAnalysis.monte-carlo-panel.total-summary", {
              totalPaths,
              simTime: ((totalPaths * simMs) / 1000).toFixed(1),
            })}
          </span>
          <Button type="primary" onClick={handleRun} loading={loading}>
            {loading
              ? t("stockAnalysis.monte-carlo-panel.running")
              : t("stockAnalysis.monte-carlo-panel.run-robustness-test")}
          </Button>
        </div>
      </Card>

      {/* 加载态 */}
      {loading && (
        <Card size="small">
          <div className="flex items-center justify-center py-8">
            <Spin
              size="large"
              description={t("stockAnalysis.monte-carlo-panel.running-simulation-tip", { totalPaths })}
            />
          </div>
        </Card>
      )}

      {/* 错误态 */}
      {error && (
        <Card size="small">
          <div className="py-4 text-center text-red">{error}</div>
        </Card>
      )}

      {/* 结果区 */}
      {report && !loading && (
        <>
          {/* 核心指标 */}
          <Row gutter={[12, 12]}>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title={t("stockAnalysis.monte-carlo-panel.total-paths-stat")}
                  value={report.totalPaths}
                  suffix={t("stockAnalysis.monte-carlo-panel.paths-unit")}
                  styles={{ content: { fontSize: 22 } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title={t("stockAnalysis.monte-carlo-panel.cross-scenario-survival-rate")}
                  value={report.survivalRate}
                  suffix="%"
                  precision={1}
                  styles={{ content: { fontSize: 22, color: report.survivalRate >= 50 ? "#52c41a" : "#f5222d" } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title={t("stockAnalysis.monte-carlo-panel.consistency-score")}
                  value={report.consistencyScore}
                  precision={2}
                  suffix={report.consistencyScore < 1.0
                    ? t("stockAnalysis.monte-carlo-panel.consistency-stable-suffix")
                    : t("stockAnalysis.monte-carlo-panel.consistency-volatile-suffix")}
                  styles={{ content: { fontSize: 22 } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <div className="text-sm text-secondary">{t("stockAnalysis.monte-carlo-panel.best-worst-scenario")}</div>
                <div className="mt-1">
                  <Tag color="green">{report.bestScenario}</Tag>
                  <Tag color="red">{report.worstScenario}</Tag>
                </div>
              </Card>
            </Col>
          </Row>

          {/* 场景详情表格 */}
          <Card
            size="small"
            title={
              <span>
                📊 {t("stockAnalysis.monte-carlo-panel.scenario-detail")} · <Tag color="blue">{report.stockCode}</Tag>
                {" "}
                {t("stockAnalysis.monte-carlo-panel.reference-price-label", { price: report.referencePrice })}
              </span>
            }
          >
            <Table
              dataSource={report.scenarioResults}
              rowKey="scenario"
              size="small"
              pagination={false}
              columns={[
                {
                  title: t("stockAnalysis.monte-carlo-panel.column-scenario"),
                  dataIndex: "label",
                  key: "label",
                  render: (label: string, record: McScenarioResult) => (
                    <span>
                      {label}
                      <Tag className="ml-2" color="default">{record.scenario}</Tag>
                    </span>
                  ),
                },
                {
                  title: t("stockAnalysis.monte-carlo-panel.column-paths"),
                  dataIndex: "paths",
                  key: "paths",
                  width: 80,
                },
                {
                  title: t("stockAnalysis.monte-carlo-panel.column-avg-trades"),
                  dataIndex: "avgTotalTrades",
                  key: "avgTotalTrades",
                  width: 100,
                  render: (v: number) => v.toFixed(1),
                },
                {
                  title: t("stockAnalysis.monte-carlo-panel.column-final-price"),
                  dataIndex: "avgFinalMidPrice",
                  key: "avgFinalMidPrice",
                  width: 120,
                  render: (v: number | null) => (v ?? "—"),
                },
                {
                  title: t("stockAnalysis.monte-carlo-panel.column-price-change"),
                  dataIndex: "priceChangePct",
                  key: "priceChangePct",
                  width: 100,
                  render: (v: number | null) => {
                    if (v == null) {
                      return "—";
                    }
                    const color = v >= 0 ? "#52c41a" : "#f5222d";
                    return <span style={{ color }}>{v >= 0 ? "+" : ""}{v.toFixed(2)}%</span>;
                  },
                },
              ]}
            />
          </Card>

          {/* 解读 */}
          <Card size="small" title={t("stockAnalysis.monte-carlo-panel.interpretation")}>
            <Descriptions column={1} size="small">
              <Descriptions.Item label={t("stockAnalysis.monte-carlo-panel.survival-rate-analysis")}>
                {report.survivalRate >= 70
                  ? t("stockAnalysis.monte-carlo-panel.survival-rate-high")
                  : report.survivalRate >= 40
                  ? t("stockAnalysis.monte-carlo-panel.survival-rate-medium")
                  : t("stockAnalysis.monte-carlo-panel.survival-rate-low")}
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.monte-carlo-panel.consistency")}>
                {report.consistencyScore < 0.5
                  ? t("stockAnalysis.monte-carlo-panel.consistency-high")
                  : report.consistencyScore < 1.0
                  ? t("stockAnalysis.monte-carlo-panel.consistency-acceptable")
                  : t("stockAnalysis.monte-carlo-panel.consistency-environment-dependent")}
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.monte-carlo-panel.advice")}>
                {report.bestScenario === report.worstScenario
                  ? t("stockAnalysis.monte-carlo-panel.advice-consistent")
                  : t("stockAnalysis.monte-carlo-panel.advice-different", {
                    best: report.bestScenario,
                    worst: report.worstScenario,
                  })}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        </>
      )}

      {/* 初始提示 */}
      {!report && !loading && !error && (
        <Card size="small">
          <div className="py-8 text-center text-secondary">
            <p className="mb-2 text-base">{t("stockAnalysis.monte-carlo-panel.empty-state-title")}</p>
            <p className="text-sm">
              {t("stockAnalysis.monte-carlo-panel.empty-state-desc")}
            </p>
          </div>
        </Card>
      )}
    </div>
  );
}

// 辅助接口（Table 用）
interface McScenarioResult {
  scenario: string;
  label: string;
  paths: number;
  avgTotalTrades: number;
  avgFinalMidPrice: number | null;
  priceChangePct: number | null;
}
