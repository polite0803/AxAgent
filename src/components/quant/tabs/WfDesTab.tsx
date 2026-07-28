// WfDesTab — WalkForward + DES 对比
//
// 调用后端 `wf_des_integration` 命令，对比同一策略在两条管道上的表现：
//   - WalkForward：基于历史 K 线的滚动窗回测（确定性）
//   - DES：基于 Agent 的市场模拟（种子化随机）
//
// K 线数据自动从 `get_stock_kline` 拉取（与股票分析工作流同源），用户无需手动粘贴。
// 参考价默认取最新收盘价 × 100（单位：分），用户可覆盖。

import {
  Alert,
  Button,
  Card,
  Checkbox,
  Collapse,
  DatePicker,
  Empty,
  Form,
  Input,
  InputNumber,
  Space,
  Table,
  Tag,
  Typography,
} from "antd";
import dayjs from "dayjs";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { QuantMetricsCard } from "@/components/quant/charts/QuantMetricsCard";
import { invoke } from "@/lib/invoke";
import type { Bar, MetricsReport, WfDesFold, WfDesReport, WfDesRequest } from "@/types/quant";
import type { KLine } from "@/types/stock-analysis";
import type { ColumnsType } from "antd/es/table";

const { Title, Text } = Typography;

// ── 默认参数 ──

const DEFAULT_CODE = "600519";
const DEFAULT_KLINE_LIMIT = 600; // ~ 2.5 年日线，足以切 5+ fold
const DEFAULT_TRAIN_DAYS = 300;
const DEFAULT_TEST_DAYS = 100;
const DEFAULT_RISK_FREE = 0.025;
const DEFAULT_SIM_DURATION_S = 30; // 30s
const DEFAULT_SEED = 42;
const DEFAULT_INITIAL_CASH = 1_000_000;
const DEFAULT_WAKEUP_US = 500; // 500μs
const STRATEGY = "ma_cross"; // 后端目前仅支持 ma_cross

// ── 表单状态 ──

interface DraftForm {
  code: string;
  startDate: string;
  endDate: string;
  klineLimit: number;
  trainDays: number;
  testDays: number;
  stepDays: number | null;
  riskFreeAnnual: number;
  simDurationS: number;
  seed: number;
  initialCash: number;
  wakeupUs: number;
  /** 是否自动从最新收盘价推算参考价 */
  autoReferencePrice: boolean;
  /** 手填参考价（元）；autoReferencePrice=false 时生效 */
  manualReferencePrice: number;
}

const INITIAL_DRAFT: DraftForm = {
  code: DEFAULT_CODE,
  startDate: "2023-01-01",
  endDate: dayjs().format("YYYY-MM-DD"),
  klineLimit: DEFAULT_KLINE_LIMIT,
  trainDays: DEFAULT_TRAIN_DAYS,
  testDays: DEFAULT_TEST_DAYS,
  stepDays: null,
  riskFreeAnnual: DEFAULT_RISK_FREE,
  simDurationS: DEFAULT_SIM_DURATION_S,
  seed: DEFAULT_SEED,
  initialCash: DEFAULT_INITIAL_CASH,
  wakeupUs: DEFAULT_WAKEUP_US,
  autoReferencePrice: true,
  manualReferencePrice: 1000,
};

export function WfDesTab() {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<DraftForm>(INITIAL_DRAFT);
  const [isRunning, setIsRunning] = useState(false);
  const [klineLoading, setKlineLoading] = useState(false);
  const [klineError, setKlineError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [report, setReport] = useState<WfDesReport | null>(null);

  // ── 拉取 K 线（与股票分析工作流同源） ──
  const fetchKlines = async (code: string): Promise<KLine[]> => {
    setKlineLoading(true);
    setKlineError(null);
    try {
      const data = await invoke<KLine[]>("get_stock_kline", {
        stockCode: code,
        period: "daily",
        limit: draft.klineLimit,
        // 不传 asOfDate：quant 模块不走时间旅行
        adj: "forward",
      });
      // 按日期升序（WalkForward 要求 klines 已排序）
      data.sort((a, b) => (a.date < b.date ? -1 : a.date > b.date ? 1 : 0));
      return data;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setKlineError(msg);
      throw e;
    } finally {
      setKlineLoading(false);
    }
  };

  // ── KLine → Bar 转换（前端 KLine 缺 code 字段，需补充） ──
  const toBars = (code: string, klines: KLine[]): Bar[] => {
    return klines.map((k) => ({
      date: k.date,
      code,
      open: k.open,
      high: k.high,
      low: k.low,
      close: k.close,
      volume: k.volume,
      amount: k.amount,
      turnoverRate: k.turnoverRate ?? undefined,
      adjFactor: k.adjFactor ?? undefined,
      isSt: false,
    }));
  };

  const onRun = async () => {
    setError(null);
    setReport(null);
    setIsRunning(true);
    try {
      const code = draft.code.trim();
      if (!code) {
        throw new Error(t("quant.wfDes.errorEmptyCode"));
      }

      // 1. 自动拉取 K 线
      const klines = await fetchKlines(code);
      if (klines.length < draft.trainDays + draft.testDays) {
        throw new Error(
          t("quant.wfDes.errorInsufficientKlines", {
            got: klines.length,
            need: draft.trainDays + draft.testDays,
          }),
        );
      }

      // 2. 推算参考价（单位：分）
      const lastClose = klines[klines.length - 1].close;
      const referencePrice = draft.autoReferencePrice
        ? Math.round(lastClose * 100)
        : Math.round(draft.manualReferencePrice * 100);

      // 3. 组装请求
      const req: WfDesRequest = {
        klines: toBars(code, klines),
        wfConfig: {
          trainDays: draft.trainDays,
          testDays: draft.testDays,
          stepDays: draft.stepDays,
          riskFreeAnnual: draft.riskFreeAnnual,
        },
        desConfig: {
          stockCode: code,
          referencePrice,
          simDurationNs: draft.simDurationS * 1_000_000_000,
          seed: draft.seed,
          initialCash: draft.initialCash,
          wakeupIntervalNs: draft.wakeupUs * 1_000,
        },
        strategyName: STRATEGY,
      };

      // 4. 调用后端
      const result = await invoke<WfDesReport>("wf_des_integration", { request: req });
      setReport(result);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
    } finally {
      setIsRunning(false);
    }
  };

  const setField = <K extends keyof DraftForm>(key: K, value: DraftForm[K]) => {
    setDraft((d) => ({ ...d, [key]: value }));
  };

  return (
    <Space orientation="vertical" size="large" style={{ width: "100%" }}>
      <Card title={t("quant.wfDes.title")} size="small">
        <Form layout="vertical" size="small">
          <div
            style={{
              display: "grid",
              gap: 16,
              gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))",
            }}
          >
            <Form.Item label={t("quant.wfDes.code")}>
              <Input
                value={draft.code}
                placeholder={t("quant.wfDes.codePlaceholder")}
                onChange={(e) => setField("code", e.target.value)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.startDate")}>
              <DatePicker
                value={draft.startDate ? dayjs(draft.startDate) : null}
                onChange={(d) => setField("startDate", d ? d.format("YYYY-MM-DD") : "")}
                style={{ width: "100%" }}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.endDate")}>
              <DatePicker
                value={draft.endDate ? dayjs(draft.endDate) : null}
                onChange={(d) => setField("endDate", d ? d.format("YYYY-MM-DD") : "")}
                style={{ width: "100%" }}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.klineLimit")}>
              <InputNumber
                min={100}
                step={100}
                style={{ width: "100%" }}
                value={draft.klineLimit}
                onChange={(v) => setField("klineLimit", typeof v === "number" ? v : DEFAULT_KLINE_LIMIT)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.trainDays")}>
              <InputNumber
                min={30}
                step={50}
                style={{ width: "100%" }}
                value={draft.trainDays}
                onChange={(v) => setField("trainDays", typeof v === "number" ? v : DEFAULT_TRAIN_DAYS)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.testDays")}>
              <InputNumber
                min={10}
                step={20}
                style={{ width: "100%" }}
                value={draft.testDays}
                onChange={(v) => setField("testDays", typeof v === "number" ? v : DEFAULT_TEST_DAYS)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.stepDays")}>
              <InputNumber
                min={1}
                step={20}
                style={{ width: "100%" }}
                placeholder={t("quant.wfDes.stepDaysPlaceholder")}
                value={draft.stepDays ?? undefined}
                onChange={(v) => setField("stepDays", typeof v === "number" ? v : null)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.riskFreeAnnual")}>
              <InputNumber
                min={0}
                max={1}
                step={0.005}
                style={{ width: "100%" }}
                value={draft.riskFreeAnnual}
                onChange={(v) => setField("riskFreeAnnual", typeof v === "number" ? v : DEFAULT_RISK_FREE)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.simDurationS")}>
              <InputNumber
                min={1}
                max={600}
                step={5}
                style={{ width: "100%" }}
                value={draft.simDurationS}
                onChange={(v) => setField("simDurationS", typeof v === "number" ? v : DEFAULT_SIM_DURATION_S)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.seed")}>
              <InputNumber
                min={0}
                step={1}
                style={{ width: "100%" }}
                value={draft.seed}
                onChange={(v) => setField("seed", typeof v === "number" ? v : DEFAULT_SEED)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.initialCash")}>
              <InputNumber
                min={10000}
                step={100000}
                style={{ width: "100%" }}
                value={draft.initialCash}
                onChange={(v) => setField("initialCash", typeof v === "number" ? v : DEFAULT_INITIAL_CASH)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.wakeupUs")}>
              <InputNumber
                min={10}
                step={100}
                style={{ width: "100%" }}
                value={draft.wakeupUs}
                onChange={(v) => setField("wakeupUs", typeof v === "number" ? v : DEFAULT_WAKEUP_US)}
              />
            </Form.Item>
            <Form.Item label={t("quant.wfDes.referencePrice")}>
              <Space orientation="vertical" size="small" style={{ width: "100%" }}>
                <Checkbox
                  checked={draft.autoReferencePrice}
                  onChange={(e) => setField("autoReferencePrice", e.target.checked)}
                >
                  {t("quant.wfDes.autoReferencePrice")}
                </Checkbox>
                {!draft.autoReferencePrice && (
                  <Space.Compact style={{ width: "100%" }}>
                    <InputNumber
                      min={0.01}
                      step={1}
                      style={{ width: "100%" }}
                      value={draft.manualReferencePrice}
                      onChange={(v) =>
                        setField(
                          "manualReferencePrice",
                          typeof v === "number" ? v : 1000,
                        )}
                    />
                    <Button disabled>{t("quant.wfDes.yuan")}</Button>
                  </Space.Compact>
                )}
              </Space>
            </Form.Item>
          </div>

          <div style={{ marginTop: 16 }}>
            <Text type="secondary" style={{ fontSize: 12 }}>
              {t("quant.wfDes.strategyFixed", { strategy: STRATEGY })}
            </Text>
          </div>

          <div style={{ marginTop: 16 }}>
            <Button
              type="primary"
              loading={isRunning || klineLoading}
              onClick={onRun}
              disabled={isRunning || klineLoading}
            >
              {isRunning
                ? t("quant.wfDes.running")
                : klineLoading
                ? t("quant.wfDes.klineLoading")
                : t("quant.wfDes.run")}
            </Button>
          </div>
        </Form>
      </Card>

      {klineError && (
        <Alert
          type="error"
          showIcon
          title={t("quant.wfDes.klineError")}
          description={klineError}
          closable
          onClose={() => setKlineError(null)}
        />
      )}

      {error && (
        <Alert
          type="error"
          showIcon
          title={t("quant.wfDes.errorRunning")}
          description={error}
          closable
          onClose={() => setError(null)}
        />
      )}

      {report && <WfDesReportView report={report} />}

      {!report && !isRunning && !klineLoading && !error && !klineError && (
        <Empty description={t("quant.wfDes.noResult")} />
      )}
    </Space>
  );
}

// ── 报告展示 ──

function WfDesReportView({ report }: { report: WfDesReport }) {
  const { t } = useTranslation();
  const wf = report.walkforward;
  const des = report.desMetrics;
  const dev = report.deviation;

  return (
    <Space orientation="vertical" size="middle" style={{ width: "100%" }}>
      <Card title={t("quant.wfDes.resultTitle")} size="small">
        <Space size="middle" wrap>
          <Tag color="blue">
            {t("quant.wfDes.foldCount")}: {wf.windows.length}
          </Tag>
          <Tag color={wf.stabilityScore > 0.7 ? "green" : wf.stabilityScore > 0.4 ? "orange" : "red"}>
            {t("quant.wfDes.stabilityScore")}: {wf.stabilityScore.toFixed(3)}
          </Tag>
          <Tag color={wf.overfitWarning ? "red" : "green"}>
            {t("quant.wfDes.overfitWarning")}: {wf.overfitWarning
              ? `${wf.overfitWindowCount}/${wf.windows.length}`
              : t("quant.wfDes.noOverfit")}
          </Tag>
          <Tag color="purple">
            {t("quant.wfDes.oosSharpe")}: {wf.aggregatedOosMetrics.sharpe.toFixed(3)}
          </Tag>
          <Tag color="geekblue">
            {t("quant.wfDes.desTotalTrades")}: {report.desTotalTrades}
          </Tag>
        </Space>
      </Card>

      <Card title={t("quant.wfDes.deviationTitle")} size="small">
        <div
          style={{
            display: "grid",
            gap: 10,
            gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))",
          }}
        >
          <QuantMetricsCard
            title={t("quant.wfDes.sharpeDelta")}
            value={dev.sharpeDelta}
            precision={3}
          />
          <QuantMetricsCard
            title={t("quant.wfDes.maxddDelta")}
            value={dev.maxddDelta}
            precision={2}
            suffix="%"
          />
          <QuantMetricsCard
            title={t("quant.wfDes.winRateDelta")}
            value={dev.winRateDelta}
            precision={2}
            suffix="%"
          />
          <QuantMetricsCard
            title={t("quant.wfDes.volumeRatio")}
            value={dev.volumeRatio}
            precision={3}
          />
        </div>
      </Card>

      <Card title={t("quant.wfDes.walkforwardTitle")} size="small">
        <MetricsCompareTable
          title={t("quant.wfDes.oosAggregated")}
          metrics={wf.aggregatedOosMetrics}
        />
        <div style={{ marginTop: 16 }}>
          <Title level={5}>{t("quant.wfDes.foldDetail")}</Title>
          <FoldTable folds={wf.windows.map((w) => w.fold)} />
        </div>
      </Card>

      <Card title={t("quant.wfDes.desTitle")} size="small">
        <MetricsCompareTable title={t("quant.wfDes.desMetricsTitle")} metrics={des} />
      </Card>
    </Space>
  );
}

// ── 子组件：单组 MetricsReport 展示 ──

function MetricsCompareTable({ title, metrics }: { title: string; metrics: MetricsReport }) {
  const { t } = useTranslation();
  const items = [
    {
      key: "totalReturn",
      label: t("quant.metrics.totalReturn"),
      value: metrics.totalReturn * 100,
      suffix: "%",
      good: true,
    },
    {
      key: "annualizedReturn",
      label: t("quant.metrics.annualizedReturn"),
      value: metrics.annualizedReturn * 100,
      suffix: "%",
      good: true,
    },
    { key: "sharpe", label: t("quant.metrics.sharpe"), value: metrics.sharpe, good: true },
    { key: "sortino", label: "Sortino", value: metrics.sortino, good: true },
    {
      key: "maxDrawdownPct",
      label: t("quant.metrics.maxDrawdownPct"),
      value: metrics.maxDrawdownPct * 100,
      suffix: "%",
      good: false,
    },
    { key: "winRate", label: t("quant.metrics.winRate"), value: metrics.winRate * 100, suffix: "%" },
    { key: "totalTrades", label: t("quant.metrics.totalTrades"), value: metrics.totalTrades, prec: 0 },
    { key: "profitFactor", label: t("quant.backtest.profitFactor"), value: metrics.profitFactor, good: true },
  ];
  return (
    <div>
      <Text type="secondary" style={{ fontSize: 12 }}>
        {title}
      </Text>
      <div
        style={{
          display: "grid",
          gap: 10,
          marginTop: 8,
          gridTemplateColumns: "repeat(auto-fit, minmax(160px, 1fr))",
        }}
      >
        {items.map((it) => (
          <QuantMetricsCard
            key={it.key}
            title={it.label}
            value={it.value}
            precision={it.prec ?? 2}
            suffix={it.suffix}
            positiveIsGood={it.good}
          />
        ))}
      </div>
    </div>
  );
}

// ── 子组件：Fold 表 ──

function FoldTable({ folds }: { folds: WfDesFold[] }) {
  const { t } = useTranslation();
  const columns: ColumnsType<WfDesFold> = [
    { title: "Fold", dataIndex: "foldIdx", key: "fi", width: 60, sorter: (a, b) => a.foldIdx - b.foldIdx },
    {
      title: t("quant.wfDes.trainRange"),
      key: "train",
      width: 240,
      render: (_, r) => `${r.trainStart} → ${r.trainEnd} (${r.trainBarsCount})`,
    },
    {
      title: t("quant.wfDes.testRange"),
      key: "test",
      width: 240,
      render: (_, r) => `${r.testStart} → ${r.testEnd} (${r.testBarsCount})`,
    },
  ];
  return (
    <Collapse
      size="small"
      items={[
        {
          key: "folds",
          label: `${t("quant.wfDes.foldDetail")} (${folds.length})`,
          children: (
            <Table<WfDesFold>
              size="small"
              columns={columns}
              dataSource={folds}
              rowKey="foldIdx"
              pagination={false}
              scroll={{ x: 600 }}
            />
          ),
        },
      ]}
    />
  );
}
