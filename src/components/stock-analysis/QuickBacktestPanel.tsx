import { invoke } from "@/lib/invoke";
import { App, Button, Card, Col, InputNumber, Row, Spin, Statistic, Table, Tag } from "antd";
import { BarChart3, Clock, RefreshCw, TrendingDown, TrendingUp } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface QuickBacktestSample {
  analysisDate: string;
  entryPrice: number;
  exitPrice: number;
  returnPct: number;
  wasCorrect: boolean;
  decisionAction: string;
  decisionConfidence: number;
}

interface QuickBacktestResult {
  stockCode: string;
  totalSamples: number;
  correctCount: number;
  accuracyPct: number;
  avgReturnPct: number;
  winRate: number;
  samples: QuickBacktestSample[];
  error: string | null;
}

/** 快速回测面板 — 借鉴 TradingAgents 采样+持有期模式 */
export function QuickBacktestPanel() {
  const { t } = useTranslation();
  const { message: messageApi } = App.useApp();
  const [stockCode, setStockCode] = useState("");
  const [startDate, setStartDate] = useState<string>("");
  const [endDate, setEndDate] = useState<string>("");
  const [sampleInterval, setSampleInterval] = useState<number>(10);
  const [holdDays, setHoldDays] = useState<number>(20);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<QuickBacktestResult | null>(null);

  const handleRun = async () => {
    if (!stockCode.trim()) { return messageApi.warning(t("stockAnalysis.searchPlaceholder")); }
    if (!startDate || !endDate) { return messageApi.warning(t("common.required")); }

    setLoading(true);
    setResult(null);
    try {
      const res = await invoke<QuickBacktestResult>("quick_backtest", {
        request: {
          stockCode: stockCode.trim(),
          startDate,
          endDate,
          sampleInterval,
          holdDays,
          asOfDate: null,
        },
      });
      setResult(res);
      if (res.totalSamples === 0) {
        messageApi.info(t("stockAnalysis.backtest.noData"));
      } else {
        messageApi.success(t("stockAnalysis.backtest.completed", { count: res.totalSamples }));
      }
    } catch (e) {
      messageApi.error(`${t("common.error")}: ${e}`);
    }
    setLoading(false);
  };

  const columns = [
    {
      title: t("stockAnalysis.backtest.date"),
      dataIndex: "analysisDate",
      key: "analysisDate",
      width: 120,
    },
    {
      title: t("stockAnalysis.backtest.entryPrice"),
      dataIndex: "entryPrice",
      key: "entryPrice",
      width: 100,
      render: (v: number) => `¥${v.toFixed(2)}`,
    },
    {
      title: t("stockAnalysis.backtest.exitPrice"),
      dataIndex: "exitPrice",
      key: "exitPrice",
      width: 100,
      render: (v: number) => `¥${v.toFixed(2)}`,
    },
    {
      title: t("stockAnalysis.backtest.returnRate"),
      dataIndex: "returnPct",
      key: "returnPct",
      width: 100,
      render: (v: number) => (
        <span className={`font-mono ${v >= 0 ? "text-red-500" : "text-green-500"}`}>
          {v >= 0 ? "+" : ""}
          {v.toFixed(2)}%
        </span>
      ),
    },
    {
      title: t("stockAnalysis.backtest.result"),
      dataIndex: "wasCorrect",
      key: "wasCorrect",
      width: 80,
      render: (v: boolean) => <Tag color={v ? "green" : "red"}>{v ? "✓" : "✗"}</Tag>,
    },
    {
      title: t("stockAnalysis.backtest.decisionAction"),
      dataIndex: "decisionAction",
      key: "decisionAction",
      width: 120,
    },
    {
      title: t("stockAnalysis.backtest.confidence"),
      dataIndex: "decisionConfidence",
      key: "decisionConfidence",
      width: 80,
      render: (v: number) => `${v.toFixed(0)}%`,
    },
  ];

  return (
    <Card
      size="small"
      title={
        <div className="flex items-center gap-2">
          <BarChart3 size={16} />
          <span>{t("stockAnalysis.backtest.quickBacktest")}</span>
        </div>
      }
    >
      {/* 参数配置 */}
      <div className="flex flex-wrap gap-3 mb-4 items-end">
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
            {t("stockAnalysis.stockCode")}
          </label>
          <input
            className="border rounded px-2 py-1 text-sm font-mono w-28"
            placeholder="600519"
            value={stockCode}
            onChange={(e) => setStockCode(e.target.value)}
            style={{ background: "var(--color-bg)", color: "var(--color-text)", borderColor: "var(--color-border)" }}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
            {t("stockAnalysis.backtest.startDate")}
          </label>
          <input
            className="border rounded px-2 py-1 text-sm w-28"
            type="date"
            value={startDate}
            onChange={(e) => setStartDate(e.target.value)}
            style={{ background: "var(--color-bg)", color: "var(--color-text)", borderColor: "var(--color-border)" }}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
            {t("stockAnalysis.backtest.endDate")}
          </label>
          <input
            className="border rounded px-2 py-1 text-sm w-28"
            type="date"
            value={endDate}
            onChange={(e) => setEndDate(e.target.value)}
            style={{ background: "var(--color-bg)", color: "var(--color-text)", borderColor: "var(--color-border)" }}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
            {t("stockAnalysis.backtest.sampleInterval")}
          </label>
          <InputNumber
            size="small"
            min={1}
            max={100}
            value={sampleInterval}
            onChange={(v) => setSampleInterval(v ?? 10)}
            style={{ width: 80 }}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--color-text-secondary)" }}>
            {t("stockAnalysis.backtest.holdDays")}
          </label>
          <InputNumber
            size="small"
            min={1}
            max={250}
            value={holdDays}
            onChange={(v) => setHoldDays(v ?? 20)}
            style={{ width: 80 }}
          />
        </div>
        <Button
          type="primary"
          size="small"
          icon={loading ? <RefreshCw className="animate-spin" size={14} /> : <BarChart3 size={14} />}
          loading={loading}
          onClick={handleRun}
        >
          {loading ? t("common.loading") : t("stockAnalysis.backtest.run")}
        </Button>
      </div>

      {/* 结果 */}
      {loading && (
        <div className="flex justify-center py-8">
          <Spin description={t("stockAnalysis.backtest.running")}>
            <div style={{ padding: 50 }} />
          </Spin>
        </div>
      )}

      {result && !loading && (
        <>
          {/* 汇总统计 */}
          <Row gutter={16} className="mb-4">
            <Col span={6}>
              <Card size="small">
                <Statistic
                  title={t("stockAnalysis.backtest.totalRuns")}
                  value={result.totalSamples}
                  prefix={<BarChart3 size={14} />}
                  styles={{ content: { fontSize: 18 } }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small">
                <Statistic
                  title={t("stockAnalysis.backtest.winRate")}
                  value={result.winRate}
                  precision={1}
                  suffix="%"
                  styles={{
                    content: { color: result.winRate >= 50 ? "var(--color-up)" : "var(--color-down)", fontSize: 18 },
                  }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small">
                <Statistic
                  title={t("stockAnalysis.backtest.avgReturn")}
                  value={result.avgReturnPct}
                  precision={2}
                  suffix="%"
                  prefix={result.avgReturnPct >= 0 ? <TrendingUp size={14} /> : <TrendingDown size={14} />}
                  styles={{
                    content: {
                      color: result.avgReturnPct >= 0 ? "var(--color-up)" : "var(--color-down)",
                      fontSize: 18,
                    },
                  }}
                />
              </Card>
            </Col>
            <Col span={6}>
              <Card size="small">
                <Statistic
                  title={t("stockAnalysis.backtest.correctCount")}
                  value={`${result.correctCount}/${result.totalSamples}`}
                  prefix={<Clock size={14} />}
                  styles={{ content: { fontSize: 18 } }}
                />
              </Card>
            </Col>
          </Row>

          {/* 明细表格 */}
          <Table
            dataSource={result.samples}
            columns={columns}
            rowKey="analysisDate"
            pagination={{ pageSize: 15, size: "small" }}
            size="small"
            className="quick-backtest-table"
          />
        </>
      )}
    </Card>
  );
}
