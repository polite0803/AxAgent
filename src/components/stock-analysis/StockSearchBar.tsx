import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { Button, Input, Tag } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ParsedIntent {
  rawInput: string;
  stockQuery: string | null;
  stockCode: string | null;
  timeHorizon: string | null;
  actionType: string;
  success: boolean;
  description: string;
}

const HORIZON_LABELS: Record<string, string> = {
  ultra_short: "超短线",
  short: "短线",
  mid: "中线",
  long: "长线",
};

export function StockSearchBar() {
  const { t } = useTranslation();
  const searchKeyword = useStockAnalysisStore((s) => s.searchKeyword);
  const searchResults = useStockAnalysisStore((s) => s.searchResults);
  const searchStock = useStockAnalysisStore((s) => s.searchStock);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const status = useStockAnalysisStore((s) => s.status);
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const [intent, setIntent] = useState<ParsedIntent | null>(null);

  const isRunning = status === "loading" || status === "running";

  // Ctrl+K / Cmd+K 聚焦到搜索框
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        const el = document.querySelector<HTMLInputElement>(".stock-search-input input");
        el?.focus();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // P1: 自然语言意图解析 — 借鉴 TradingAgents intent parser
  // 当用户输入"调研茅台短线""分析宁德时代中线"时自动识别
  const handleSearch = useCallback(async (value: string) => {
    setIntent(null);

    // 先尝试 NL 意图解析
    try {
      const parsed = await invoke<ParsedIntent>("parse_analysis_intent", { input: value });
      if (parsed.success && parsed.stockCode) {
        setIntent(parsed);
        // 直接填充分析参数
        useStockAnalysisStore.setState({
          stockCode: parsed.stockCode,
          stockName: parsed.stockQuery || parsed.stockCode,
          searchKeyword: value,
        });
        // 如果有时间周期，传递到 store
        if (parsed.timeHorizon) {
          useStockAnalysisStore.setState({ selectedHorizon: parsed.timeHorizon } as never);
        }
        // 如果查询词就是股票代码/名称，直接获取行情
        getStockQuote(parsed.stockCode);
        getStockKline(parsed.stockCode, "daily", 120);
        return;
      }
    } catch {
      // NL 解析失败，回退到普通搜索
    }

    searchStock(value, true);
  }, [searchStock, getStockQuote, getStockKline]);

  return (
    <div className="flex flex-col gap-2">
      <div className="flex gap-2 items-center">
        <Input.Search
          className="stock-search-input"
          data-testid="stock-analysis-search-input"
          placeholder={`${t("stockAnalysis.searchPlaceholder")} (Ctrl+K)`}
          value={searchKeyword}
          onChange={(e) => {
            useStockAnalysisStore.setState({ searchKeyword: e.target.value });
            if (e.target.value.length >= 2) {
              searchStock(e.target.value);
            }
          }}
          onSearch={handleSearch}
          style={{ maxWidth: 360 }}
          loading={status === "loading"}
        />
        <Button
          type="primary"
          disabled={!stockCode || isRunning}
          loading={isRunning}
          onClick={() => {
            if (stockCode) {
              startAnalysis(stockCode);
            }
          }}
        >
          {isRunning ? t("stockAnalysis.analyzing") : t("stockAnalysis.startAnalysis")}
        </Button>
      </div>

      {/* P1: 意图解析结果展示 */}
      {intent && intent.success && (
        <div className="flex gap-1 items-center text-xs" style={{ color: "var(--color-text-secondary)" }}>
          <span>{t("stockAnalysis.parsedAs")}:</span>
          <Tag color="blue" className="text-xs">{intent.stockQuery || intent.stockCode}</Tag>
          {intent.timeHorizon && (
            <Tag color="green" className="text-xs">
              {HORIZON_LABELS[intent.timeHorizon] || intent.timeHorizon}
            </Tag>
          )}
          <span className="text-xs">{intent.description}</span>
        </div>
      )}

      {searchResults.length > 0 && (
        <List
          size="small"
          bordered
          dataSource={searchResults}
          style={{ maxWidth: 300 }}
          renderItem={(item) => (
            <List.Item
              style={{ cursor: "pointer" }}
              onClick={() => {
                getStockQuote(item.code);
                getStockKline(item.code, "daily", 120);
                useStockAnalysisStore.setState({ searchResults: [] });
              }}
            >
              {item.code} — {item.name}
            </List.Item>
          )}
        />
      )}
    </div>
  );
}
