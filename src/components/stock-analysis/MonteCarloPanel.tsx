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

interface ScenarioConfig {
  key: string;
  label: string;
  enabled: boolean;
  paths: number;
}

const DEFAULT_SCENARIOS: ScenarioConfig[] = [
  { key: "normal", label: "正常市场", enabled: true, paths: 20 },
  { key: "bull", label: "牛市", enabled: true, paths: 20 },
  { key: "bear", label: "熊市", enabled: true, paths: 20 },
  { key: "flash_crash", label: "闪崩", enabled: false, paths: 15 },
  { key: "high_vol", label: "高波动", enabled: false, paths: 15 },
];

export function MonteCarloPanel() {
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
      setError("请至少选择一个场景");
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
      <Card size="small" title="⚙ 鲁棒性测试配置">
        <div className="mb-3 flex flex-wrap items-center gap-4">
          <label className="text-sm font-medium">
            股票代码
            <InputNumber
              className="ml-2"
              style={{ width: 110 }}
              value={stockCode}
              onChange={(v) => setStockCode(v ?? "000001")}
            />
          </label>
          <label className="text-sm font-medium">
            参考价(分)
            <InputNumber
              className="ml-2"
              style={{ width: 120 }}
              min={1}
              value={refPrice}
              onChange={(v) => setRefPrice(v ?? 1000)}
            />
          </label>
          <label className="text-sm font-medium">
            时长(ms)
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
              <span className="text-sm">{sc.label}</span>
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
            合计 {totalPaths} 路径 · 模拟 ~{((totalPaths * simMs) / 1000).toFixed(1)}s 虚拟时间
          </span>
          <Button type="primary" onClick={handleRun} loading={loading}>
            {loading ? "运行中..." : "运行鲁棒性测试"}
          </Button>
        </div>
      </Card>

      {/* 加载态 */}
      {loading && (
        <Card size="small">
          <div className="flex items-center justify-center py-8">
            <Spin size="large" tip="正在运行 {totalPaths} 条模拟路径 ..." />
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
                  title="总路径数"
                  value={report.totalPaths}
                  suffix="条"
                  styles={{ content: { fontSize: 22 } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <Statistic
                  title="跨场景胜率"
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
                  title="一致性评分"
                  value={report.consistencyScore}
                  precision={2}
                  suffix={report.consistencyScore < 1.0 ? " (稳定)" : " (波动)"}
                  styles={{ content: { fontSize: 22 } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small" hoverable>
                <div className="text-sm text-secondary">最佳/最差场景</div>
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
                📊 场景详情 · <Tag color="blue">{report.stockCode}</Tag> 参考价 {report.referencePrice} 分
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
                  title: "场景",
                  dataIndex: "label",
                  key: "label",
                  render: (label: string, record: McScenarioResult) => (
                    <span>
                      {label}
                      <Tag className="ml-2" color="default">{record.scenario}</Tag>
                    </span>
                  ),
                },
                { title: "路径数", dataIndex: "paths", key: "paths", width: 80 },
                {
                  title: "平均成交",
                  dataIndex: "avgTotalTrades",
                  key: "avgTotalTrades",
                  width: 100,
                  render: (v: number) => v.toFixed(1),
                },
                {
                  title: "终止价(分)",
                  dataIndex: "avgFinalMidPrice",
                  key: "avgFinalMidPrice",
                  width: 120,
                  render: (v: number | null) => (v ?? "—"),
                },
                {
                  title: "涨跌幅",
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
          <Card size="small" title="💡 解读">
            <Descriptions column={1} size="small">
              <Descriptions.Item label="胜率解读">
                {report.survivalRate >= 70
                  ? "✅ 策略在不同市场环境下表现稳定"
                  : report.survivalRate >= 40
                  ? "⚠️ 策略对市场环境有一定选择性，需关注当前市场风格"
                  : "❌ 策略仅在特定市场中有效，需谨慎使用"}
              </Descriptions.Item>
              <Descriptions.Item label="一致性">
                {report.consistencyScore < 0.5
                  ? "策略在不同场景下表现高度一致"
                  : report.consistencyScore < 1.0
                  ? "策略表现有一定波动但可接受"
                  : "策略表现高度依赖市场环境"}
              </Descriptions.Item>
              <Descriptions.Item label="建议">
                {report.bestScenario === report.worstScenario
                  ? "策略在所有场景下表现一致"
                  : `策略在 ${report.bestScenario} 场景中最佳，在 ${report.worstScenario} 场景中最差，建议结合当前市场风格使用`}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        </>
      )}

      {/* 初始提示 */}
      {!report && !loading && !error && (
        <Card size="small">
          <div className="py-8 text-center text-secondary">
            <p className="mb-2 text-base">选择多个市场场景，运行蒙特卡洛鲁棒性测试</p>
            <p className="text-sm">
              系统会在每个场景中运行 N 条随机路径，评估策略的跨场景稳定性
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
