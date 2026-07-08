import { useStockAnalysisStore } from "@/stores";
import { Button, Card, Tag } from "antd";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";

export function StockQuoteCard() {
  const { t } = useTranslation();
  const quote = useStockAnalysisStore((s) => s.quote);
  const stockName = useStockAnalysisStore((s) => s.stockName);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const autoRefresh = useStockAnalysisStore((s) => s.autoRefresh);
  const setAutoRefresh = useStockAnalysisStore((s) => s.setAutoRefresh);
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // Auto-refresh: 每 30 秒轮询一次
  useEffect(() => {
    if (autoRefresh && stockCode) {
      timerRef.current = setInterval(() => {
        const currentCode = useStockAnalysisStore.getState().stockCode;
        if (currentCode === stockCode) {
          getStockQuote(stockCode);
        }
      }, 30000);
    }
    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [autoRefresh, stockCode, getStockQuote]);

  if (!quote) { return null; }

  // 严格 > 0 才算"涨",0% 视为平盘不带"+"号(中国股市惯例)
  const isUp = quote.changePct > 0;
  const isDown = quote.changePct < 0;
  const color = isUp ? "var(--sa-red)" : isDown ? "var(--sa-green)" : "var(--muted)";
  // 涨跌额 = 当前价 - 昨收价(中国股市惯例),不是 price - open
  const changeAmount = quote.price - quote.preClose;

  return (
    <Card
      size="small"
      title={`${quote.name || stockName}（${quote.code}）`}
      extra={
        <Button
          size="small"
          type={autoRefresh ? "primary" : "text"}
          onClick={() => setAutoRefresh(!autoRefresh)}
          style={{ fontSize: 11, padding: "0 6px", height: 22 }}
        >
          {autoRefresh
            ? `● ${t("stockAnalysis.refreshing")}`
            : `○ ${t("stockAnalysis.autoRefresh")}`}
        </Button>
      }
      styles={{ body: { padding: "8px 12px" } }}
    >
      <div className="flex items-end gap-4 flex-wrap">
        {/* 当前价 + 涨跌额 + 涨跌幅 */}
        <div className="flex items-baseline gap-2">
          <span className="text-[28px] font-semibold font-mono" style={{ color }}>
            {quote.price.toFixed(2)}
          </span>
          <Tag color={isUp ? "red" : isDown ? "green" : "default"}>
            {isUp ? "+" : ""}
            {quote.changePct.toFixed(2)}%
          </Tag>
          <span className="text-xs font-mono" style={{ color }}>
            {changeAmount >= 0 ? "+" : ""}
            {changeAmount.toFixed(2)}
          </span>
        </div>

        {/* 核心数据：4 列紧凑网格 */}
        <div className="grid grid-cols-4 gap-x-3 gap-y-0.5 text-xs" style={{ color: "var(--muted)" }}>
          <span>
            {t("stockAnalysis.open")}: <b className="font-mono">{quote.open}</b>
          </span>
          <span>
            {t("stockAnalysis.high")}: <b className="font-mono">{quote.high}</b>
          </span>
          <span>
            {t("stockAnalysis.low")}: <b className="font-mono">{quote.low}</b>
          </span>
          <span>
            {t("stockAnalysis.volume")}:{" "}
            <b className="font-mono">{(quote.volume / 10000).toFixed(1)}{t("stockAnalysis.volumeUnit")}</b>
          </span>
          {quote.pe != null && (
            <span>
              PE: <b className="font-mono">{quote.pe}</b>
            </span>
          )}
          {quote.pb != null && (
            <span>
              PB: <b className="font-mono">{quote.pb}</b>
            </span>
          )}
          {quote.totalMv != null && (
            <span>
              {t("stockAnalysis.marketCap")}:{" "}
              <b className="font-mono">{(quote.totalMv / 1e8).toFixed(1)}{t("stockAnalysis.yiUnit")}</b>
            </span>
          )}
          {quote.turnoverRate != null && (
            <span>
              {t("stockAnalysis.turnoverRate")}: <b className="font-mono">{quote.turnoverRate.toFixed(2)}%</b>
            </span>
          )}
        </div>
      </div>
    </Card>
  );
}
