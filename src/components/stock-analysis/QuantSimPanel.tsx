import { invoke } from "@/lib/invoke";
import type { QuantSimResult } from "@/types/market-sim";
import { Button, Card, Descriptions, InputNumber, Select, Spin, Statistic, Tag } from "antd";
import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";

export function QuantSimPanel() {
  const { t } = useTranslation();

  const STRATEGIES = [
    { value: "ma_cross", label: t("quant.strategySelect.maCross") },
    { value: "macd", label: t("quant.strategySelect.macd") },
    { value: "rsi", label: t("quant.strategySelect.rsi") },
    { value: "boll", label: t("quant.strategySelect.boll") },
    { value: "turtle", label: t("quant.strategySelect.turtle") },
  ];
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<QuantSimResult | null>(null);
  const [stockCode, setStockCode] = useState("000001");
  const [refPrice, setRefPrice] = useState(1000);
  const [simMs, setSimMs] = useState(500);
  const [strategy, setStrategy] = useState("ma_cross");
  const tokenRef = useRef(0);

  const handleRun = async () => {
    const myToken = ++tokenRef.current;
    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await invoke<QuantSimResult>("market_sim_run_strategy", {
        request: {
          stockCode,
          referencePrice: refPrice,
          strategyName: strategy,
          maxSimTimeMs: simMs,
        },
      });
      if (myToken !== tokenRef.current) {
        return;
      }
      setResult(res);
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

  return (
    <div className="space-y-4">
      <Card size="small" title={t("stockAnalysis.quantSim.title")}>
        <div className="mb-3 flex flex-wrap items-center gap-4">
          <label className="text-sm font-medium">
            {t("stockAnalysis.quantSim.stockCode")}
            <InputNumber
              className="ml-2"
              style={{ width: 110 }}
              value={stockCode}
              onChange={(v) => setStockCode(v ?? "000001")}
            />
          </label>
          <label className="text-sm font-medium">
            {t("stockAnalysis.quantSim.referencePrice")}
            <InputNumber
              className="ml-2"
              style={{ width: 120 }}
              min={1}
              value={refPrice}
              onChange={(v) => setRefPrice(v ?? 1000)}
            />
          </label>
          <label className="text-sm font-medium">
            {t("stockAnalysis.quantSim.duration")}
            <InputNumber
              className="ml-2"
              style={{ width: 100 }}
              min={1}
              max={5000}
              value={simMs}
              onChange={(v) => setSimMs(v ?? 500)}
            />
          </label>
        </div>

        <div className="mb-3 flex items-center gap-4">
          <label className="text-sm font-medium">{t("stockAnalysis.quantSim.strategy")}</label>
          <Select
            style={{ width: 240 }}
            value={strategy}
            onChange={setStrategy}
            options={STRATEGIES}
          />
          <Button type="primary" onClick={handleRun} loading={loading}>
            {loading ? t("stockAnalysis.quantSim.running") : t("stockAnalysis.quantSim.run")}
          </Button>
        </div>

        <div className="text-xs text-secondary">
          {t("stockAnalysis.quantSim.description")}
        </div>
      </Card>

      {loading && (
        <Card size="small">
          <div className="flex items-center justify-center py-6">
            <Spin description={t("stockAnalysis.quantSim.spinTip")} />
          </div>
        </Card>
      )}

      {error && (
        <Card size="small">
          <div className="py-3 text-center text-red">{error}</div>
        </Card>
      )}

      {result && !loading && (
        <>
          <div className="grid grid-cols-4 gap-3">
            <Card size="small" hoverable>
              <Statistic
                title={t("stockAnalysis.quantSim.events")}
                value={result.totalEvents}
                suffix={t("stockAnalysis.quantSim.eventsSuffix")}
              />
            </Card>
            <Card size="small" hoverable>
              <Statistic
                title={t("stockAnalysis.quantSim.trades")}
                value={result.totalTrades}
                suffix={t("stockAnalysis.quantSim.tradesSuffix")}
              />
            </Card>
            <Card size="small" hoverable>
              <Statistic
                title={t("stockAnalysis.quantSim.finalPrice")}
                value={result.finalMidPrice ?? "—"}
                suffix={t("stockAnalysis.quantSim.fenSuffix")}
              />
            </Card>
            <Card size="small" hoverable>
              <Statistic title={t("stockAnalysis.quantSim.wallClock")} value={result.wallClockMs} suffix="ms" />
            </Card>
          </div>

          <Card size="small" title={t("stockAnalysis.quantSim.interpretation")}>
            <Descriptions column={1} size="small">
              <Descriptions.Item label={t("stockAnalysis.quantSim.strategyLabel")}>
                <Tag color="blue">
                  {STRATEGIES.find((s) => s.value === strategy)?.label ?? strategy}
                </Tag>
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.quantSim.marketActivity")}>
                {result.totalEvents > 0
                  ? t("stockAnalysis.quantSim.marketActivityDetail", {
                    simMs,
                    totalEvents: result.totalEvents,
                    totalTrades: result.totalTrades,
                  })
                  : t("stockAnalysis.quantSim.noEvents")}
              </Descriptions.Item>
              <Descriptions.Item label={t("stockAnalysis.quantSim.quote")}>
                {result.finalMidPrice
                  ? t("stockAnalysis.quantSim.quoteDetail", { finalMidPrice: result.finalMidPrice, refPrice })
                  : t("stockAnalysis.quantSim.noQuote")}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        </>
      )}

      {!result && !loading && !error && (
        <Card size="small">
          <div className="py-6 text-center text-secondary">
            <p className="mb-1 text-base">{t("stockAnalysis.quantSim.emptyHint")}</p>
            <p className="text-sm">
              {t("stockAnalysis.quantSim.emptyDesc")}
            </p>
          </div>
        </Card>
      )}
    </div>
  );
}
