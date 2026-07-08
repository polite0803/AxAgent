import { List } from "@/components/common/AntdList";
import { ReplayBadge, ReplayWatermark } from "@/components/time-travel/ReplayBadge";
import { invoke } from "@/lib/invoke";
import { parseAction } from "@/lib/stock-analysis-utils";
import { useStockAnalysisStore } from "@/stores";
import { useTimeAnchorStore } from "@/stores/feature/timeAnchorStore";
import type {
  BacktestComparisonResponse,
  LatestAnalysisSummary,
  PeriodKey,
  RecoPick,
  RecoResponse,
  StockConsensus,
  StyleKey,
} from "@/types/stock-analysis";

import { Alert, Button, Card, Checkbox, Collapse, Empty, message, Modal, Spin, Tabs, Tag, Tooltip } from "antd";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { RecoHistoryModal } from "./RecoHistoryModal";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";

interface RecommendationPanelProps {
  /**
   * 打开数据源设置的回调。优先于上下文中的实现 —— 让该面板可脱离
   * <StockAnalysisPage> 渲染（例如在选股中心里）。
   * 不传时回退到上下文（默认 no-op）。
   */
  onOpenDataSourceSettings?: () => void;
}

const noop = () => {};

// Bug 10 修复: RecoResponse / StyleKey / PeriodKey / RecoPick 统一从
// @/types/stock-analysis 导入,与后端 crates/stock-analysis/src/recommender/types.rs
// 的 RecoPick schema 一一对应(完整字段版,含 sector/style/period/price/entryLow/
// entryHigh/stopLoss/targetPrice/positionPct/holdingDays/riskNotes 等)。
// 这里不再保留本地 panel-specific 富化类型。

const STYLE_KEYS: StyleKey[] = ["trend", "value", "capital", "reversion", "watchlist", "serenity"];
const STYLE_COLOR: Record<StyleKey, string> = {
  trend: "blue",
  value: "gold",
  capital: "magenta",
  reversion: "green",
  watchlist: "default",
  serenity: "purple",
};

const FALLBACK = "—";
const isFiniteNumber = (n: unknown): n is number => typeof n === "number" && Number.isFinite(n);

/** Format a number with decimals; render FALLBACK for non-finite values. */
function fmt(value: unknown, decimals = 2, fallback = FALLBACK): string {
  if (!isFiniteNumber(value)) { return fallback; }
  return value.toFixed(decimals);
}

export function RecommendationPanel({ onOpenDataSourceSettings }: RecommendationPanelProps = {}) {
  const { t, i18n } = useTranslation();
  const { openDataSourceSettings: ctxOpenSettings } = useStockAnalysisPage();
  const openDataSourceSettings = onOpenDataSourceSettings ?? ctxOpenSettings ?? noop;
  const asOfDate = useTimeAnchorStore((s) => s.asOfDate);
  const anchorMode = useTimeAnchorStore((s) => s.mode);

  const [period, setPeriod] = useState<PeriodKey>("short");
  const [data, setData] = useState<RecoResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);
  const [errorDetail, setErrorDetail] = useState<string | null>(null);
  const [generatedAtText, setGeneratedAtText] = useState<string>("");
  // 显示策略：挂载 + 切 period 时优先读缓存(上一次的荐股结果),
  // 只有用户点击"刷新"按钮才走 recommend_stocks 拉新数据
  const [isCached, setIsCached] = useState(false);
  // P0-1: 荐股面板关联历史分析数据
  const [latestAnalyses, setLatestAnalyses] = useState<Record<string, LatestAnalysisSummary | null>>({});
  // P0-2: 策略回测统计（每个风格的 win rate + Sharpe）
  const [strategyStats, setStrategyStats] = useState<
    Record<string, { winRate: number; sharpe: number | null; signalCount: number }> | null
  >(null);
  const [strategyStatsLoading, setStrategyStatsLoading] = useState(false);

  const reqTokenRef = useRef(0);

  // P0-2: 加载策略回测统计（仅在已有荐股结果时调用，避免空数据库时拉取）
  // 改为手动触发：用户点击"刷新回测"按钮或首次成功 recommend_stocks 后触发
  const backtestTriggeredRef = useRef<boolean>(false);
  const triggerBacktest = useCallback(async () => {
    if (backtestTriggeredRef.current) { return; }
    backtestTriggeredRef.current = true;
    try {
      setStrategyStatsLoading(true);
      const result = await invoke<BacktestComparisonResponse>("backtest_reco_strategies");
      if (!result) { return; }
      // 按 style 聚合所有 period 的统计
      const byStyle: Record<string, { winRates: number[]; sharpes: number[]; signals: number }> = {};
      for (const [, s] of Object.entries(result.positive.strategies)) {
        const style = s.style;
        if (!byStyle[style]) { byStyle[style] = { winRates: [], sharpes: [], signals: 0 }; }
        byStyle[style].winRates.push(s.winRatePct);
        if (s.sharpeRatio != null) { byStyle[style].sharpes.push(s.sharpeRatio); }
        byStyle[style].signals += s.totalSignals;
      }
      const agg: Record<string, { winRate: number; sharpe: number | null; signalCount: number }> = {};
      for (const [style, v] of Object.entries(byStyle)) {
        const avgWr = v.winRates.reduce((a, b) => a + b, 0) / v.winRates.length;
        const avgSh = v.sharpes.length > 0 ? v.sharpes.reduce((a, b) => a + b, 0) / v.sharpes.length : null;
        agg[style] = {
          winRate: Math.round(avgWr * 10) / 10,
          sharpe: avgSh != null ? Math.round(avgSh * 100) / 100 : null,
          signalCount: v.signals,
        };
      }
      setStrategyStats(agg);
    } catch {
      // 回测失败时静默，不打扰用户
      backtestTriggeredRef.current = false;
    } finally {
      setStrategyStatsLoading(false);
    }
  }, []);

  /**
   * 统一数据加载入口。所有数据拉取都走这里,避免 useEffect 与 onClick
   * 各自发起一次重复请求,造成 RPC 浪费。
   *
   * 用 `reqTokenRef` 做请求级取消:每次调用 +1,旧请求的响应会被丢弃,
   * 不会覆盖新一次调用的结果。
   */
  const load = useCallback(async () => {
    const myToken = ++reqTokenRef.current;
    setLoading(true);
    setEmptyKind(null);
    setErrorDetail(null);
    try {
      const r = await invoke<RecoResponse>("recommend_stocks", { period, asOfDate });
      if (myToken !== reqTokenRef.current) { return; }
      if (!r || !r.picks || Object.keys(r.picks).length === 0) {
        setData(r ?? null);
        if (r?.errorDetail) {
          // 后端主动检测到数据源不可用并返回了具体错误原因
          setErrorDetail(r.errorDetail);
          setEmptyKind("noData");
        } else if (r && r.disabledStyles && r.disabledStyles.length >= 4) {
          setEmptyKind("vendorDisabled");
        } else {
          setEmptyKind("noData");
        }
        setLoading(false);
        return;
      }
      setData(r);
      const d = new Date(r.generatedAt);
      setGeneratedAtText(
        d.toLocaleTimeString(i18n.language === "zh-CN" ? "zh-CN" : "en-US", {
          hour: "2-digit",
          minute: "2-digit",
        }),
      );
      // 荐股成功后异步触发策略回测统计（fire-and-forget，不阻塞 UI）
      void triggerBacktest();

      // 同一 token 下串行拉"最近分析",避免与 useEffect 内联请求重复(Bug 7 修复)
      const allCodes = new Set<string>();
      for (const arr of Object.values(r.picks ?? {})) {
        if (!arr) { continue; }
        for (const p of arr) {
          if (!p.synthetic) { allCodes.add(p.stockCode); }
        }
      }
      if (allCodes.size > 0) {
        try {
          const result = await invoke<Record<string, LatestAnalysisSummary | null>>(
            "get_latest_analyses_for_stocks",
            { stockCodes: Array.from(allCodes), asOfDate },
          );
          if (myToken === reqTokenRef.current && result) {
            setLatestAnalyses(result);
          }
        } catch (e) {
          console.warn("[RecommendationPanel] Failed to load latest analyses:", e);
        }
      }
    } catch (e: unknown) {
      console.error("[RecommendationPanel] load failed:", e);
      if (myToken !== reqTokenRef.current) { return; }
      setData(null);
      setErrorDetail(null);
      setEmptyKind("connectionFailed");
    }
    if (myToken === reqTokenRef.current) { setLoading(false); }
  }, [period, asOfDate, i18n.language, triggerBacktest]);

  /**
   * 缓存加载入口。打开页面 / 切换 period 时调用,优先展示上一次的荐股结果
   * (避免用户每次打开都触发一次新的后端推荐任务)。
   *
   * 行为:
   * - live 模式 → 调 get_cached_recommendation(period) 读最新缓存
   * - replay 模式 → 没有缓存概念,直接回退到 load()(同原行为)
   *
   * 缓存命中时 isCached=true,UI 上显示"缓存"灰底小标签;
   * 无缓存(emptyKind=noData)时改用 emptyNoCache 文案,引导用户点"刷新"。
   */
  const loadCache = useCallback(async () => {
    // replay 模式没有缓存(缓存永远是 live 产物),回退到实时
    if (anchorMode !== "live") {
      void load();
      return;
    }
    const myToken = ++reqTokenRef.current;
    setLoading(true);
    setEmptyKind(null);
    setErrorDetail(null);
    try {
      const r = await invoke<RecoResponse | null>("get_cached_recommendation", { period });
      if (myToken !== reqTokenRef.current) { return; }
      if (!r || !r.picks || Object.keys(r.picks).length === 0) {
        setData(r ?? null);
        // 缓存为空时区分文案,提示用户点"刷新"重新拉取
        if (r?.errorDetail) {
          setErrorDetail(r.errorDetail);
        }
        setEmptyKind("noData");
        setIsCached(false);
        setLoading(false);
        return;
      }
      setData(r);
      setIsCached(true);
      const d = new Date(r.generatedAt);
      setGeneratedAtText(
        d.toLocaleTimeString(i18n.language === "zh-CN" ? "zh-CN" : "en-US", {
          hour: "2-digit",
          minute: "2-digit",
        }),
      );
    } catch (e: unknown) {
      console.error("[RecommendationPanel] loadCache failed:", e);
      if (myToken !== reqTokenRef.current) { return; }
      setData(null);
      setErrorDetail(null);
      setIsCached(false);
      setEmptyKind("connectionFailed");
    }
    if (myToken === reqTokenRef.current) { setLoading(false); }
  }, [period, anchorMode, i18n.language, load]);

  // Period 切换时优先读缓存；首次挂载（Tab 切走后 destroyOnHidden 重新渲染）
  // 也走缓存，避免每次切回 Tab 都后台刷新（用户原话："简直就是傻逼逻辑"）。
  // mount 时仍触发一次策略回测统计 (triggerBacktest 内部用 backtestTriggeredRef
  // 去重,不会与 loadCache 内部的调用重复),保证面板 mount 后立即有历史回测数据可见。
  const initialMountRef = useRef(true);
  useEffect(() => {
    if (initialMountRef.current) {
      initialMountRef.current = false;
      void triggerBacktest();
    }
    // eslint-disable-next-line react-hooks/set-state-in-effect
    void loadCache();
  }, [period, loadCache, triggerBacktest]);

  // P0-1: 批量加载所有 picks 的最近分析结果。
  // Bug 7 修复: 该函数已合并进 load() 内部,避免 useEffect 和 onClick
  // 各自发起一次重复请求,造成 RPC 浪费。
  // （保留此注释作为变更记录。）

  const disabledStyleSet = useMemo(() => new Set(data?.disabledStyles ?? []), [data]);
  const disabledStyleNames = useMemo(() => {
    if (!data) { return ""; }
    return data.disabledStyles?.map((s) => t(`stockAnalysis.recommendation.style${capitalize(s)}`))
      .join(" / ");
  }, [data, t]);

  // B15: degraded styles — as-of 截断导致降级(≠ 缺失),前端用橙色"⛔"标识
  // 与 disabled(灰)区分: disabled 是 vendor 完全不可用;degraded 是可用但效果减弱
  const degradedStyleSet = useMemo(() => new Set(data?.degradedStyles ?? []), [data]);
  const hasDegraded = degradedStyleSet.size > 0;

  // 数据质量统计：所有 picks 总数（包含兜底合成）
  const dataQuality = useMemo(() => {
    if (!data) { return { real: 0, synthetic: 0 }; }
    let real = 0;
    let synthetic = 0;
    for (const arr of Object.values(data.picks ?? {})) {
      if (!arr) { continue; }
      for (const p of arr) {
        if (p.synthetic) { synthetic++; }
        else { real++; }
      }
    }
    return { real, synthetic };
  }, [data]);

  const periodItems = [
    { key: "ultra_short", label: t("stockAnalysis.recommendation.periodUltraShort") },
    { key: "short", label: t("stockAnalysis.recommendation.periodShort") },
    { key: "mid", label: t("stockAnalysis.recommendation.periodMid") },
    { key: "long", label: t("stockAnalysis.recommendation.periodLong") },
  ];

  const isReplay = anchorMode === "replay" && asOfDate !== null;

  return (
    <Card
      size="small"
      title={
        <div className="flex items-center gap-2">
          <span>{t("stockAnalysis.recommendation.title")}</span>
          {isReplay && <ReplayBadge />}
        </div>
      }
      styles={{ body: { padding: "8px 10px" } }}
      extra={
        <div className="flex items-center gap-2">
          {generatedAtText && (
            <span className="text-[10px] text-gray-400">
              {isCached
                ? t("stockAnalysis.recommendation.cachedAt", { time: generatedAtText })
                : t("stockAnalysis.recommendation.generatedAt", { time: generatedAtText })}
            </span>
          )}
          {isCached && (
            <Tag
              color="default"
              className="m-0 text-[10px]"
              data-testid="reco-cached-badge"
            >
              {t("stockAnalysis.recommendation.cachedBadge")}
            </Tag>
          )}
          <Button size="small" loading={loading} onClick={load}>
            {t("stockAnalysis.settings.panels.refresh")}
          </Button>
          <RecoHistoryModal />
          <AutoCalibrateButton t={t} />
        </div>
      }
    >
      <Tabs
        size="small"
        activeKey={period}
        onChange={(k) => setPeriod(k as PeriodKey)}
        items={periodItems}
        style={{ marginBottom: 8 }}
      />

      {isReplay && asOfDate && (
        <Alert
          type="warning"
          showIcon
          className="!text-xs !mb-2"
          message={
            <span className="text-xs">
              {t("timeTravel.recommendationBanner", { date: asOfDate })}
            </span>
          }
        />
      )}

      <div style={{ position: "relative" }}>
        {disabledStyleSet.size > 0 && (
          <Alert
            type="warning"
            showIcon
            className="!text-xs !mb-2"
            message={
              <span className="text-xs">
                {t("stockAnalysis.recommendation.bannerVendorDisabled", { styles: disabledStyleNames })}
              </span>
            }
            action={
              <Button size="small" type="link" onClick={openDataSourceSettings}>
                {t("stockAnalysis.recommendation.openSettings")}
              </Button>
            }
          />
        )}

        {/* 当所有推荐均为兜底合成时提示用户（已自动过滤兜底数据） */}
        {data && dataQuality.real === 0 && dataQuality.synthetic > 0 && (
          <Alert
            type="info"
            showIcon
            className="!text-xs !mb-2"
            message={
              <span className="text-xs">
                {t("stockAnalysis.recommendation.dataQualitySummary", {
                  real: dataQuality.real,
                  synthetic: dataQuality.synthetic,
                })}
              </span>
            }
          />
        )}

        {/* B15: 降级风格提示 —— 与 disabled 不同,degraded 是"可用但效果减弱",用橙色 info 区分 */}
        {data && hasDegraded && asOfDate && (
          <Alert
            type="warning"
            showIcon
            className="!text-xs !mb-2"
            message={
              <span className="text-xs">
                {t("stockAnalysis.recommendation.bannerDegraded", {
                  styles: Array.from(degradedStyleSet)
                    .map((s) => t(`stockAnalysis.recommendation.style${capitalize(s)}`))
                    .join(" / "),
                  date: asOfDate,
                })}
              </span>
            }
          />
        )}

        {/* 数据可用性错误详情：后端返回 errorDetail 时显示具体原因 */}
        {errorDetail && (
          <Alert
            type="warning"
            showIcon
            className="!text-xs !mb-2"
            message={
              <span className="text-xs">
                ⚠ {t("stockAnalysis.settings.panels.noData")} — {errorDetail}
              </span>
            }
            action={
              <Button size="small" type="link" onClick={openDataSourceSettings}>
                {t("stockAnalysis.recommendation.openSettings")}
              </Button>
            }
          />
        )}

        {loading
          ? <Spin size="small" style={{ display: "block", margin: "16px auto" }} />
          : emptyKind
          ? (
            <PanelEmpty
              kind={emptyKind}
              description={emptyKind === "noData"
                ? (anchorMode !== "live" || isCached
                  ? t("stockAnalysis.recommendation.empty")
                  : t("stockAnalysis.recommendation.emptyNoCache"))
                : undefined}
              onOpenSettings={openDataSourceSettings}
            />
          )
          : !data
          ? (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={anchorMode !== "live"
                ? t("stockAnalysis.recommendation.empty")
                : t("stockAnalysis.recommendation.emptyNoCache")}
            />
          )
          : (
            // P3-4: key={period} forces the Collapse to remount when period changes,
            // so defaultActiveKey re-applies for the new dataset.
            <Collapse
              key={period}
              ghost
              size="small"
              defaultActiveKey={STYLE_KEYS.filter((s) =>
                !disabledStyleSet.has(s) && (data?.picks?.[s]?.length ?? 0) > 0
              )
                .slice(0, 2)}
              items={STYLE_KEYS.map((style) => {
                const picks = (data?.picks?.[style])?.filter(p => !p.synthetic) ?? [];
                const isDisabled = disabledStyleSet.has(style);
                const isDegraded = degradedStyleSet.has(style);
                // P2-3: when a style is disabled, still show the section (expandable)
                // with a specific empty state explaining why.
                return {
                  key: style,
                  label: (
                    <div className="flex items-center gap-2">
                      <Tag color={STYLE_COLOR[style]} className="m-0 text-xs">
                        {t(`stockAnalysis.recommendation.style${capitalize(style)}`)}
                      </Tag>
                      {/* P0-2: 策略回测徽章 */}
                      {!strategyStatsLoading && strategyStats?.[style] && (
                        <>
                          <Tag
                            className="m-0 text-[10px] leading-4"
                            color={strategyStats[style].winRate >= 55
                              ? "green"
                              : strategyStats[style].winRate >= 45
                              ? "orange"
                              : "red"}
                          >
                            {`${strategyStats[style].winRate}%`}
                          </Tag>
                          {strategyStats[style].sharpe != null && (
                            <Tag
                              className="m-0 text-[10px] leading-4"
                              color={strategyStats[style].sharpe! >= 1
                                ? "green"
                                : strategyStats[style].sharpe! >= 0.5
                                ? "orange"
                                : "red"}
                            >
                              {`S ${strategyStats[style].sharpe!.toFixed(1)}`}
                            </Tag>
                          )}
                        </>
                      )}
                      {/* B15: 降级风格在 label 处加 ⛔ 徽标(橙色),区别于 disabled 的灰 */}
                      {isDegraded && (
                        <Tag color="orange" className="m-0 text-[10px]">
                          ⛔ {t("timeTravel.degradedStyles.title")}
                        </Tag>
                      )}
                      <span className="text-xs text-gray-500">
                        {isDisabled
                          ? t("stockAnalysis.recommendation.styleDisabled")
                          : `(${picks.length})`}
                      </span>
                    </div>
                  ),
                  children: isDisabled
                    ? (
                      <Empty
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                        description={t("stockAnalysis.recommendation.styleDisabledReason", {
                          style: t(`stockAnalysis.recommendation.style${capitalize(style)}`),
                        })}
                      />
                    )
                    : picks.length === 0
                    ? (
                      <Empty
                        image={Empty.PRESENTED_IMAGE_SIMPLE}
                        description={t("stockAnalysis.recommendation.empty")}
                      />
                    )
                    : (
                      <List
                        size="small"
                        dataSource={picks}
                        renderItem={(p) => (
                          <PickRow
                            pick={p}
                            latestAnalysis={latestAnalyses[p.stockCode] ?? null}
                          />
                        )}
                      />
                    ),
                };
              })}
            />
          )}
        {isReplay && <ReplayWatermark />}
      </div>
    </Card>
  );
}

function capitalize(s: string) {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** 荐股 ↔ 分析师共识交叉验证徽章
 *  - 推荐为 BUY，仅在有缓存共识时显示（避免噪音）
 *  - 共识看多 → 绿色 ✓
 *  - 共识看空 / 中性 / 分歧 → 警示色 ⚠
 */
function CrossCheckBadge({
  consensus,
  recAction,
}: {
  consensus: StockConsensus;
  recAction: string;
}) {
  const { t, i18n } = useTranslation();
  if (consensus.total === 0) { return null; }

  // 推荐与共识的对齐：BUY 时要求共识 bullish 才算"一致"；
  // 其他动作（HOLD/SELL）暂不参与交叉验证，留给后续扩展。
  const aligned = recAction === "BUY" ? consensus.consensus === "bullish" : null;

  let color: "green" | "red" | "orange" | "gold";
  let icon: string;
  let label: string;
  let tooltipBody: string;

  if (aligned === true) {
    color = "green";
    icon = "✓";
    label = t("stockAnalysis.recommendation.consensusBullish");
    tooltipBody = t("stockAnalysis.recommendation.crossCheckAligned");
  } else if (consensus.consensus === "bearish") {
    color = "red";
    icon = "⚠";
    label = t("stockAnalysis.recommendation.consensusBearish");
    tooltipBody = t("stockAnalysis.recommendation.crossCheckBearish");
  } else if (consensus.consensus === "divided") {
    color = "gold";
    icon = "⚠";
    label = t("stockAnalysis.recommendation.consensusDivided");
    tooltipBody = t("stockAnalysis.recommendation.crossCheckDivided");
  } else {
    // 共识中性，且推荐为 BUY → 直接提示"未印证 BUY 推荐"
    color = "orange";
    icon = "⚠";
    label = `${consensus.neutral}/${consensus.total} ${t("stockAnalysis.recommendation.neutral")}`;
    tooltipBody = t("stockAnalysis.recommendation.crossCheckNeutral", {
      total: consensus.total,
      neutral: consensus.neutral,
    });
  }

  const updatedAtText = new Date(consensus.updatedAt).toLocaleString(
    i18n.language === "zh-CN" ? "zh-CN" : "en-US",
    { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" },
  );

  return (
    <Tooltip
      title={
        <div className="text-[11px] space-y-0.5">
          <div>{tooltipBody}</div>
          <div style={{ opacity: 0.75 }}>
            {t("stockAnalysis.recommendation.crossCheckTitle", { updatedAt: updatedAtText })}
          </div>
        </div>
      }
    >
      <Tag color={color} className="m-0 text-[10px]">
        {icon} {label}
      </Tag>
    </Tooltip>
  );
}

function PickRow(
  { pick, latestAnalysis }: {
    pick: RecoPick;
    latestAnalysis: LatestAnalysisSummary | null;
  },
) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  // 读荐股 ↔ 分析师交叉验证缓存（仅当该股已有最近一次工作流结果时存在）
  const stockCodeConsensus = useStockAnalysisStore((s) => s.stockCodeConsensus);
  const consensus = stockCodeConsensus[pick.stockCode];

  // P0-1: 上次分析结论的视觉展示
  const historyBadge = useMemo(() => {
    if (!latestAnalysis || latestAnalysis.status !== "completed") { return null; }
    const action = parseAction(latestAnalysis.decisionAction);
    let color: string;
    let label: string;
    switch (action) {
      case "BUY":
      case "INCREASE":
        color = "red";
        label = t("stockAnalysis.actionBuy");
        break;
      case "SELL":
      case "REDUCE":
        color = "green";
        label = t("stockAnalysis.actionSell");
        break;
      case "UNCERTAIN":
        color = "default";
        label = t("stockAnalysis.actionUncertain");
        break;
      default:
        color = "blue";
        label = t("stockAnalysis.actionHold");
    }
    const confText = latestAnalysis.confidence != null ? ` ${latestAnalysis.confidence}` : "";

    return (
      <Tooltip
        title={
          <div className="text-[11px] space-y-0.5">
            <div>
              {t("stockAnalysis.recommendation.lastAnalysis", {
                date: latestAnalysis.analysisDate,
                action: label,
                confidence: confText,
              })}
            </div>
            {latestAnalysis.outcome && latestAnalysis.outcome !== "pending" && (
              <div>
                {t("stockAnalysis.recommendation.outcome")}: {latestAnalysis.outcome === "win"
                  ? t("stockAnalysis.recommendation.outcomeWin")
                  : t("stockAnalysis.recommendation.outcomeLoss")}
              </div>
            )}
          </div>
        }
      >
        <Tag color={color} className="m-0 text-[10px]" style={{ opacity: 0.8 }}>
          {label}
          {confText}
          {latestAnalysis.outcome === "win" && " ✓"}
          {latestAnalysis.outcome === "loss" && " ✗"}
        </Tag>
      </Tooltip>
    );
  }, [latestAnalysis, t]);
  const content = (
    <div className="text-xs w-full flex flex-col gap-0.5 py-0.5">
      <div className="flex items-center gap-1.5">
        <Tag className="m-0 text-[10px]">{pick.stockCode}</Tag>
        <span className="font-medium truncate flex-1">{pick.stockName}</span>
        <Tag color="volcano" className="m-0 text-[10px]">BUY</Tag>
        {historyBadge}
        {consensus && <CrossCheckBadge consensus={consensus} recAction="BUY" />}
        {pick.synthetic
          ? (
            <Tooltip title={t("stockAnalysis.recommendation.syntheticTooltip")}>
              <Tag color="orange" className="m-0 text-[10px]">
                {t("stockAnalysis.recommendation.tagSynthetic")}
              </Tag>
            </Tooltip>
          )
          : (
            <Tooltip title={t("stockAnalysis.recommendation.realTooltip")}>
              <Tag color="green" className="m-0 text-[10px]">
                {t("stockAnalysis.recommendation.tagReal")}
              </Tag>
            </Tooltip>
          )}
        <span className="font-mono text-[10px] text-gray-500">{fmt(pick.price)}</span>
      </div>
      <div className="flex items-center gap-1.5 text-[10px] text-gray-500">
        <span>
          {t("stockAnalysis.recommendation.row.entry")} {fmt(pick.entryLow)}-{fmt(pick.entryHigh)}
        </span>
        <span className="text-red-500">
          {t("stockAnalysis.recommendation.row.stopLoss")} {fmt(pick.stopLoss)}
        </span>
        <span className="text-green-500">
          {t("stockAnalysis.recommendation.row.target")} {fmt(pick.targetPrice)}
        </span>
      </div>
      <div className="flex items-center gap-1.5 text-[10px] text-gray-500">
        <span>
          {t("stockAnalysis.recommendation.row.position")} {fmt(pick.positionPct, 1)}%
        </span>
        <span>
          {t("stockAnalysis.recommendation.row.holding")} {fmt(pick.holdingDays, 0, fmt(0, 0))}d
        </span>
        <Tag color="blue" className="m-0 text-[10px]">
          {t("stockAnalysis.recommendation.row.confidence")} {fmt(pick.confidence, 0, "0")}
        </Tag>
        {pick.secondaryStyles && pick.secondaryStyles.length > 0 && (
          <span className="text-gray-400">
            ({t("stockAnalysis.recommendation.row.secondaryStyle")}:
            {pick.secondaryStyles?.map((s) => t(`stockAnalysis.recommendation.style${capitalize(s)}`)).join("/")})
          </span>
        )}
      </div>
    </div>
  );
  return (
    <Tooltip
      title={
        <div className="text-xs">
          <div className="font-medium mb-1">{pick.stockName} ({pick.stockCode})</div>
          {pick.reasons.length > 0 && (
            <div className="mb-1">
              {/* P1-1: i18n for "Reasons" label */}
              <div className="text-green-600">
                {t("stockAnalysis.recommendation.row.reasons")}：
              </div>
              <ul className="m-0 pl-4">{pick.reasons?.map((r, i) => <li key={i}>{r}</li>)}</ul>
            </div>
          )}
          {pick.riskNotes.length > 0 && (
            <div>
              <div className="text-red-600">
                {t("stockAnalysis.recommendation.row.risks")}：
              </div>
              <ul className="m-0 pl-4">{pick.riskNotes?.map((r, i) => <li key={i}>{r}</li>)}</ul>
            </div>
          )}
        </div>
      }
    >
      <List.Item
        style={{ cursor: "pointer", padding: "4px 0" }}
        onClick={() => navigate(`/stock-analysis?code=${pick.stockCode}`, { replace: true })}
      >
        {content}
      </List.Item>
    </Tooltip>
  );
}

/** 自动校准按钮：预览 → 确认 → 应用 */
function AutoCalibrateButton({ t }: { t: (key: string) => string }) {
  const [loading, setLoading] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);
  const [diff, setDiff] = useState<
    Array<{
      strategyId: string;
      oldWeight: number;
      newWeight: number;
      delta: number;
    }>
  >([]);
  const [checked, setChecked] = useState<string[]>([]);
  const [applying, setApplying] = useState(false);

  const handlePreview = async () => {
    if (loading) { return; }
    setLoading(true);
    try {
      const result = await invoke<{
        totalStrategies: number;
        changed: number;
        weights: Array<{
          strategyId: string;
          oldWeight: number;
          newWeight: number;
          delta: number;
        }>;
      }>("preview_adjust_reco_weights", {});
      setDiff(result.weights);
      setChecked(result.weights.map((w) => w.strategyId));
      setModalOpen(true);
    } catch (e) {
      message.error(`校准预览失败: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleApply = async () => {
    // R2-Bug-X 修复: 防止极快双击触发并发 apply_reco_weights
    if (applying) { return; }
    const selected = diff.filter((d) => checked.includes(d.strategyId));
    if (selected.length === 0) {
      message.warning("请先勾选要应用的策略");
      return;
    }
    // Bug 8 修复: 防御 weight 为 NaN / undefined,避免后端
    // 返回 "未选中任何权重调整项" 这种误报。
    const validPayload = selected
      .map((d) => ({ strategyId: d.strategyId, weight: d.newWeight }))
      .filter(
        (w) =>
          typeof w.strategyId === "string"
          && w.strategyId.length > 0
          && typeof w.weight === "number"
          && Number.isFinite(w.weight),
      );
    if (validPayload.length === 0) {
      message.error("所选项的权重值无效(NaN/缺失),无法应用");
      return;
    }
    setApplying(true);
    try {
      const result = await invoke<{ applied: number }>("apply_reco_weights", {
        weights: validPayload,
      });
      message.success(`已应用 ${result.applied} 个策略权重调整`);
      setModalOpen(false);
    } catch (e) {
      // 后端在 weights=null 时返回 "请先调用 preview...",
      // 这里把后端字符串错误直接抛到 toast 之外加 prefix,便于排查
      const msg = typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
      message.error(`应用失败: ${msg}`);
    } finally {
      setApplying(false);
    }
  };

  return (
    <>
      <Button
        size="small"
        loading={loading}
        disabled={loading}
        onClick={handlePreview}
        style={{ marginLeft: 4 }}
      >
        ⚡ {t("stockAnalysis.recommendation.calibrate") ?? "权重校准"}
      </Button>
      <Modal
        title="策略权重校准预览"
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        footer={[
          <Button key="cancel" onClick={() => setModalOpen(false)}>取消</Button>,
          <Button
            key="apply"
            type="primary"
            loading={applying}
            disabled={applying || checked.length === 0}
            onClick={handleApply}
          >
            应用选中项 ({checked.length})
          </Button>,
        ]}
        width={600}
      >
        {diff.length === 0
          ? <p style={{ color: "var(--muted)" }}>当前无建议调整</p>
          : (
            <table style={{ width: "100%", borderCollapse: "collapse" }}>
              <thead>
                <tr style={{ borderBottom: "1px solid var(--color-border-tertiary)" }}>
                  <th style={{ padding: 4, textAlign: "left" }}>策略</th>
                  <th style={{ padding: 4, textAlign: "right" }}>当前权重</th>
                  <th style={{ padding: 4, textAlign: "right" }}>建议权重</th>
                  <th style={{ padding: 4, textAlign: "right" }}>变动</th>
                  <th style={{ padding: 4 }}>应用</th>
                </tr>
              </thead>
              <tbody>
                {diff.map((d) => (
                  <tr key={d.strategyId} style={{ borderBottom: "1px solid var(--color-border-tertiary)" }}>
                    <td style={{ padding: 4 }}>{d.strategyId}</td>
                    <td style={{ padding: 4, textAlign: "right" }}>{d.oldWeight.toFixed(2)}</td>
                    <td style={{ padding: 4, textAlign: "right", fontWeight: 600 }}>
                      {d.newWeight.toFixed(2)}
                    </td>
                    <td
                      style={{
                        padding: 4,
                        textAlign: "right",
                        color: d.delta > 0 ? "var(--sa-green)" : d.delta < 0 ? "var(--sa-red)" : undefined,
                      }}
                    >
                      {d.delta > 0 ? "+" : ""}
                      {d.delta.toFixed(2)}
                    </td>
                    <td style={{ padding: 4, textAlign: "center" }}>
                      <Checkbox
                        checked={checked.includes(d.strategyId)}
                        onChange={(e) => {
                          setChecked(
                            e.target.checked
                              ? [...checked, d.strategyId]
                              : checked.filter((id) =>
                                id !== d.strategyId
                              ),
                          );
                        }}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
      </Modal>
    </>
  );
}
