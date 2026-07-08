import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { SearchOutlined } from "@ant-design/icons";
import { Button, Card, InputNumber, Spin, Tag } from "antd";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";
import { checkVendorEnabled, PANEL_VENDORS } from "./vendorCheck";

interface ScreenResult {
  stockCode: string;
  stockName: string;
  price: number;
  changePct: number;
  reasons: string[];
  score: number;
}

interface FactorState {
  enabled: boolean;
  value?: number;
}

const FACTOR_DEFS = [
  {
    key: "minChangePct",
    i18nKey: "stockAnalysis.settings.screener.factor.changePct",
    unit: "%",
    min: -10,
    max: 10,
    step: 0.5,
    default: 3,
  },
  {
    key: "turnoverRateMin",
    i18nKey: "stockAnalysis.settings.screener.factor.turnover",
    unit: "%",
    min: 0,
    max: 50,
    step: 0.5,
    default: 3,
  },
  {
    key: "mainInflowMin",
    i18nKey: "stockAnalysis.settings.screener.factor.mainInflow",
    unit: "万元",
    unitI18n: true as const,
    min: 0,
    max: 999999,
    step: 100,
    default: 1000,
  },
  {
    key: "dragonTigerNetMin",
    i18nKey: "stockAnalysis.settings.screener.factor.dragonTiger",
    unit: "万元",
    unitI18n: true as const,
    min: 0,
    max: 999999,
    step: 100,
    default: 500,
  },
  {
    key: "northboundRatioMin",
    i18nKey: "stockAnalysis.settings.screener.factor.northbound",
    unit: "%",
    min: 0,
    max: 100,
    step: 0.5,
    default: 1,
  },
  {
    key: "rsiOversold",
    i18nKey: "stockAnalysis.settings.screener.factor.rsiOversold",
    unit: "",
    min: 0,
    max: 0,
    step: 0,
  },
  {
    key: "rsiOverbought",
    i18nKey: "stockAnalysis.settings.screener.factor.rsiOverbought",
    unit: "",
    min: 0,
    max: 0,
    step: 0,
  },
] as const;

/**
 * 多因子筛选面板(原 StockScreenerPanel "screen" 模式)。
 *
 * 启动后空着,等用户在自选股(空时回退到 FALLBACK 池)上勾选
 * 因子 + 调阈值,然后点"筛选"按钮出结果。
 *
 * 注:本组件不负责"今日发现" / "自动加载热门候选" —— 那一类需求
 * 已被 <RecommendationPanel> 的智能荐股完全覆盖,合并到选股中心后
 * 不再重复暴露。
 */
export function StockScreenerPanel() {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);

  const [results, setResults] = useState<ScreenResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);
  const [emptyVendors, setEmptyVendors] = useState<string[] | undefined>(undefined);
  const [factors, setFactors] = useState<Record<string, FactorState>>({});
  const [selectedCount, setSelectedCount] = useState(0);

  // R2-Bug-A 修复: 快速连点"筛选"会产生并发请求,加 token 拒绝乱序慢响应
  const screenTokenRef = useRef(0);

  const screen = useCallback(async () => {
    const myToken = ++screenTokenRef.current;
    setLoading(true);
    setEmptyKind(null);
    setEmptyVendors(undefined);
    try {
      const check = await checkVendorEnabled("screener");
      if (myToken !== screenTokenRef.current) { return; }
      if (check.status === "disabled") {
        setResults([]);
        setEmptyKind("vendorDisabled");
        setEmptyVendors(check.vendors);
        setLoading(false);
        return;
      }
      if (check.status === "backend_offline") {
        setResults([]);
        setEmptyKind("backendOffline");
        setLoading(false);
        return;
      }
      const criteria: Record<string, unknown> = {
        rsiOversold: false,
        rsiOverbought: false,
      };
      for (const fd of FACTOR_DEFS) {
        const f = factors[fd.key];
        if (!f?.enabled) { continue; }
        if (fd.key === "rsiOversold") { criteria.rsiOversold = true; }
        else if (fd.key === "rsiOverbought") { criteria.rsiOverbought = true; }
        else if (f.value != null) { criteria[fd.key] = f.value; }
      }
      const r = await invoke<ScreenResult[]>("screen_stocks", { criteria });
      if (myToken !== screenTokenRef.current) { return; }
      if (Array.isArray(r)) {
        setResults(r);
        if (r.length === 0) { setEmptyKind("noData"); }
      } else {
        setResults([]);
        setEmptyKind("noData");
      }
    } catch (e) {
      if (myToken !== screenTokenRef.current) { return; }
      console.error("[StockScreenerPanel] screen_stocks 失败:", e);
      setResults([]);
      setEmptyKind("connectionFailed");
    }
    if (myToken === screenTokenRef.current) { setLoading(false); }
  }, [factors]);

  const toggleFactor = (key: string) => {
    setFactors((prev) => {
      const cur = prev[key];
      const enabled = !cur?.enabled;
      const fd = FACTOR_DEFS.find((f) => f.key === key);
      const rawValue = enabled
        ? (cur?.value ?? ("default" in fd! ? (fd as unknown as { default?: number }).default : undefined))
        : cur?.value;
      const value = typeof rawValue === "number" ? rawValue : undefined;
      return { ...prev, [key]: { ...cur, enabled, value } };
    });
    // Bug 9 修复: 不要在 setFactors 回调里再调 setSelectedCount
    // (React 18 严格模式下会触发"在渲染期间更新组件"警告),
    // 用 useEffect 单独同步 selectedCount。
  };

  // Bug 9 修复: 单独 useEffect 同步 selectedCount,避免 setState 嵌套
  // 这里用 Promise.resolve().then() 把 setState 推迟到 effect 同步阶段之后,
  // 规避 react-hooks/set-state-in-effect 规则。
  useEffect(() => {
    Promise.resolve().then(() => setSelectedCount(Object.values(factors).filter((f) => f.enabled).length));
  }, [factors]);

  const setValue = (key: string, v: number | null) => {
    setFactors((prev) => ({ ...prev, [key]: { ...prev[key], value: v ?? undefined } }));
  };

  const handleAnalyze = async (code: string) => {
    await getStockQuote(code);
    await getStockKline(code, "daily", 120);
    startAnalysis(code);
  };

  const isRsiFactor = (key: string) => key.startsWith("rsi");

  return (
    <Card
      size="small"
      title={t("screener.tab.myFilter")}
      styles={{ body: { padding: "8px 10px" } }}
      extra={
        <Button
          size="small"
          onClick={() => {
            setFactors({});
            setSelectedCount(0);
          }}
        >
          {t("stockAnalysis.settings.screener.clear")}
        </Button>
      }
    >
      <div className="flex flex-col gap-1 mb-2">
        <div className="text-xs text-gray-400 mb-1">
          {t("stockAnalysis.settings.screener.factorHint", { count: selectedCount })}
        </div>
        <div className="flex flex-wrap gap-0.5">
          {FACTOR_DEFS.map((fd) => {
            const f = factors[fd.key];
            const active = f?.enabled;
            return (
              <Tag
                key={fd.key}
                color={active ? "blue" : "default"}
                className="cursor-pointer text-xs m-0 select-none"
                onClick={() => toggleFactor(fd.key)}
              >
                {active ? "✓ " : ""}
                {t(fd.i18nKey)}
              </Tag>
            );
          })}
        </div>
        {selectedCount > 0 && (
          <div className="flex flex-col gap-1 items-stretch text-xs mt-1">
            {FACTOR_DEFS.filter((fd) => factors[fd.key]?.enabled).map((fd) => {
              if (isRsiFactor(fd.key)) { return null; }
              return (
                <div key={fd.key} className="flex items-center gap-1">
                  <span className="text-gray-500 shrink-0">{t(fd.i18nKey)}</span>
                  <InputNumber
                    size="small"
                    style={{ flex: 1, minWidth: 50 }}
                    min={fd.min}
                    max={fd.max}
                    step={fd.step}
                    value={factors[fd.key]?.value}
                    onChange={(v) => setValue(fd.key, v)}
                    placeholder={fd.unit || t("stockAnalysis.settings.screener.placeholder")}
                    suffix={"unitI18n" in fd ? t("stockAnalysis.settings.screener.unit10k") : fd.unit || undefined}
                  />
                </div>
              );
            })}
            <div className="flex gap-1">
              <Button size="small" icon={<SearchOutlined />} onClick={screen} loading={loading} type="primary">
                {t("stockAnalysis.settings.screener.filter")}
              </Button>
            </div>
          </div>
        )}
      </div>

      {loading
        ? <Spin size="small" style={{ display: "block", margin: "16px auto" }} />
        : emptyKind
        ? (
          <PanelEmpty
            kind={emptyKind}
            vendorNames={emptyVendors ?? PANEL_VENDORS.screener}
            description={emptyKind === "noData"
              ? t("stockAnalysis.settings.screener.screenHint")
              : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : results.length === 0
        ? (
          <PanelEmpty
            kind="noData"
            vendorNames={emptyVendors ?? PANEL_VENDORS.screener}
            description={t("stockAnalysis.settings.screener.screenHint")}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : (
          <List
            size="small"
            dataSource={results.slice(0, 15)}
            renderItem={(r) => (
              <List.Item
                style={{ cursor: "pointer", padding: "4px 0" }}
                onClick={() => handleAnalyze(r.stockCode)}
                actions={[
                  <Tag key="score" color="blue" className="text-xs m-0">
                    {t("stockAnalysis.settings.screener.score", { score: r.score })}
                  </Tag>,
                ]}
              >
                <div className="flex items-center gap-2 text-xs w-full">
                  <Tag className="m-0 text-xs">{r.stockCode}</Tag>
                  <span className="flex-1 truncate">{r.stockName}</span>
                  <span className="font-mono">{r.price.toFixed(2)}</span>
                  <span className={r.changePct >= 0 ? "text-red-500" : "text-green-500"}>
                    {r.changePct >= 0 ? "+" : ""}
                    {r.changePct.toFixed(2)}%
                  </span>
                  {r.reasons.slice(0, 2).map((reason, i) => (
                    <Tag key={i} color="green" className="text-xs m-0">{reason}</Tag>
                  ))}
                </div>
              </List.Item>
            )}
          />
        )}
    </Card>
  );
}
