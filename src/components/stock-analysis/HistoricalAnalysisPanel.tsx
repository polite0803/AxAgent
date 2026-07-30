import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { getActionTagStyle, getActionTKey, parseAction } from "@/lib/stock-analysis-utils";
import { SearchOutlined } from "@ant-design/icons";
import { App, Button, Card, Checkbox, Collapse, Empty, Input, Spin, Statistic, Tag } from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface AnalysisRecord {
  id: string;
  stockCode: string;
  stockName: string;
  analysisDate: string;
  /** 决策动作（后端直返，如 BUY/SELL/HOLD/WAIT/UNCERTAIN） */
  decisionAction: string | null;
  /** 决策仓位百分比（后端直返，0-100） */
  decisionPositionPct: number | null;
  /** 完整决策 JSON（含 confidence 等，部分旧数据可能为 null） */
  decisionJson: string | null;
  /** 列表场景不返回，详情页通过 get_stock_analysis 单独获取 */
  blackboardSnapshot?: string | null;
  createdAt: number;
  updatedAt?: number;
  status: string;
  /** 版本化分析：指向原始记录 ID，null 表示首次分析 */
  parentAnalysisId: string | null;
}

interface BacktestResult {
  stockCode: string;
  analysisDate: string;
  decisionAction: string;
  decisionConfidence: number;
  entryPrice?: number;
  exitPrice: number;
  holdingDays: number;
  returnPct: number;
  wasCorrect: boolean;
  maxDrawdownPct: number;
}

interface Props {
  analysisId?: string;
}

export function HistoricalAnalysisPanel({ analysisId = "" }: Props) {
  const { message } = App.useApp();
  const { t } = useTranslation();
  const [records, setRecords] = useState<AnalysisRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState("");
  const [snapshot, setSnapshot] = useState<Record<string, string> | null>(null);
  const [btResult, setBtResult] = useState<BacktestResult | null>(null);
  const [btAllResults, setBtAllResults] = useState<BacktestResult[] | null>(null);
  const [btLoading, setBtLoading] = useState(false);
  const [selectMode, setSelectMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      return invoke("list_stock_analyses", { limit: 30 }) as Promise<AnalysisRecord[]>;
    })
      .then((list) => {
        if (cancelled || !list) { return; }
        if (Array.isArray(list)) { setRecords(list); }
      })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!analysisId) { return; }
    let cancelled = false;
    invoke<{ blackboardSnapshot: string | null }>("get_stock_analysis", { analysisId })
      .then((r) => {
        if (cancelled) { return; }
        if (r.blackboardSnapshot) { setSnapshot(JSON.parse(r.blackboardSnapshot)); }
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [analysisId]);

  const runBacktest = async (record: AnalysisRecord) => {
    setBtLoading(true);
    try {
      const r = await invoke<BacktestResult>("backtest_analysis", { analysisId: record.id });
      setBtResult(r);
    } catch {
      message.error(t("stockAnalysis.backtest.failed"));
    }
    setBtLoading(false);
  };

  const runBacktestAll = async () => {
    setBtLoading(true);
    try {
      const r = await invoke<BacktestResult[]>("backtest_all_history");
      if (Array.isArray(r)) { setBtAllResults(r); }
    } catch {
      message.error(t("stockAnalysis.backtest.allFailed"));
    }
    setBtLoading(false);
  };

  const filtered = useMemo(() => {
    if (!search) { return records; }
    const q = search.toLowerCase();
    return records.filter((r) =>
      r.stockCode.toLowerCase().includes(q) || r.stockName.toLowerCase().includes(q)
      || (r.decisionJson && r.decisionJson.toLowerCase().includes(q))
    );
  }, [records, search]);

  // 按 stockCode 分组：组名 "股票名称(股票代码)"，组内按时间倒序
  const grouped = useMemo(() => {
    const map = new Map<string, { stockName: string; stockCode: string; items: AnalysisRecord[] }>();
    for (const r of filtered) {
      if (!map.has(r.stockCode)) {
        map.set(r.stockCode, { stockName: r.stockName, stockCode: r.stockCode, items: [] });
      }
      map.get(r.stockCode)!.items.push(r);
    }
    for (const g of map.values()) {
      g.items.sort((a, b) => b.createdAt - a.createdAt);
    }
    return Array.from(map.values());
  }, [filtered]);

  const reportEntries = Object.entries(snapshot ?? {}).filter(([k]) => k.startsWith("report."));
  const debateEntries = Object.entries(snapshot ?? {}).filter(([k]) => k.startsWith("debate."));

  // V66 修复(2026-07-29): 提取数据质量诊断，与 DecisionBanner 展示能力对齐
  const dataQualityInfo = useMemo(() => {
    if (!snapshot) { return null; }
    const raw = snapshot["data_quality_summary"];
    if (!raw) { return null; }
    try {
      // snapshot 中的 data_quality_summary 可能是 JSON 字符串或对象
      const parsed = typeof raw === "string" ? JSON.parse(raw) : raw;
      if (parsed && typeof parsed === "object" && typeof parsed.grade === "string") {
        return {
          grade: parsed.grade as string,
          score: Number(parsed.score) || 0,
          missingFactors: Array.isArray(parsed.missing_factors) ? parsed.missing_factors as string[] : [],
          summary: typeof parsed.summary === "string" ? parsed.summary : "",
        };
      }
    } catch {
      // 解析失败返回 null
    }
    return null;
  }, [snapshot]);

  // 全量回测汇总
  const btStats = btAllResults && btAllResults.length > 0
    ? {
      total: btAllResults.length,
      correct: btAllResults.filter((r) => r.wasCorrect).length,
      avgReturn: (btAllResults.reduce((s, r) => s + r.returnPct, 0) / btAllResults.length).toFixed(2),
    }
    : null;

  return (
    <div className="flex flex-col gap-2">
      {reportEntries.length > 0 && (
        <Card size="small" title={t("stockAnalysis.history")} styles={{ body: { padding: "6px 8px" } }}>
          <Collapse
            size="small"
            items={[
              ...reportEntries.slice(0, 6).map(([key, value]) => ({
                key,
                label: (
                  <span className="text-xs">
                    {key.replace("report.", "")}
                    <Tag style={{ marginLeft: 6, fontSize: 10 }}>
                      {t("stockAnalysis.charCount", { count: value.length })}
                    </Tag>
                  </span>
                ),
                children: (
                  <pre
                    className="text-xs"
                    style={{ whiteSpace: "pre-wrap", maxHeight: 200, overflow: "auto", margin: 0 }}
                  >{value}</pre>
                ),
              })),
              ...(debateEntries.length > 0
                ? [{
                  key: "debates",
                  label: <span className="text-xs">{t("stockAnalysis.debateHistory")}</span>,
                  children: (
                    <pre
                      className="text-xs"
                      style={{ whiteSpace: "pre-wrap", maxHeight: 200, overflow: "auto", margin: 0 }}
                    >{debateEntries.map(([k, v]) => `### ${k}\n${v}`).join("\n\n")}</pre>
                  ),
                }]
                : []),
            ]}
          />
        </Card>
      )}

      {/* V66 修复(2026-07-29): 数据质量诊断展示，与 DecisionBanner 对齐 */}
      {dataQualityInfo && (
        <Card size="small" title={t("stockAnalysis.dataQuality") as string} styles={{ body: { padding: "4px 8px" } }}>
          <div className="flex items-center gap-2 flex-wrap text-xs">
            <Tag
              color={dataQualityInfo.grade === "A"
                ? "green"
                : dataQualityInfo.grade === "B"
                ? "blue"
                : dataQualityInfo.grade === "C"
                ? "gold"
                : dataQualityInfo.grade === "D"
                ? "orange"
                : "red"}
            >
              {dataQualityInfo.grade}
              {t("stockAnalysis.gradeSuffix")}
            </Tag>
            <span style={{ color: "var(--muted)" }}>
              {t("stockAnalysis.scoreLabel", { score: dataQualityInfo.score })}
            </span>
            {dataQualityInfo.missingFactors.length > 0 && (
              <span style={{ color: "var(--sa-amber, #f59e0b)" }}>
                {t("stockAnalysis.missingFactorsLabel", { factors: dataQualityInfo.missingFactors.join("、") })}
              </span>
            )}
          </div>
          {dataQualityInfo.summary && (
            <div className="mt-1 text-xs" style={{ color: "var(--muted)", lineHeight: 1.5 }}>
              {dataQualityInfo.summary}
            </div>
          )}
        </Card>
      )}

      {/* 全量回测汇总 */}
      {btStats && (
        <Card size="small" title={t("stockAnalysis.backtest.summary")} styles={{ body: { padding: "4px 8px" } }}>
          <div className="grid grid-cols-3 gap-1 text-center">
            <Statistic
              title={t("stockAnalysis.backtest.total")}
              value={btStats.total}
              styles={{ content: { fontSize: 14 } }}
            />
            <Statistic
              title={t("stockAnalysis.backtest.accuracy")}
              value={btStats.correct}
              suffix={`/${btStats.total}`}
              styles={{ content: { fontSize: 14, color: "var(--sa-green)" } }}
            />
            <Statistic
              title={t("stockAnalysis.backtest.avgReturn")}
              value={btStats.avgReturn}
              suffix="%"
              styles={{ content: { fontSize: 14 } }}
            />
          </div>
        </Card>
      )}

      {/* 单次回测结果 */}
      {btResult && (
        <Card
          size="small"
          title={t("stockAnalysis.backtest.title", { code: btResult.stockCode, action: btResult.decisionAction })}
          styles={{ body: { padding: "4px 8px" } }}
        >
          <div className="grid grid-cols-3 gap-1 text-center text-xs">
            <div>
              <span className="text-gray-400">{t("stockAnalysis.backtest.holdingDays")}</span>
              <br />
              <b>{btResult.holdingDays}</b>
            </div>
            <div>
              <span className="text-gray-400">{t("stockAnalysis.backtest.returnRate")}</span>
              <br />
              <b className={btResult.returnPct >= 0 ? "text-red-500" : "text-green-500"}>
                {btResult.returnPct >= 0 ? "+" : ""}
                {btResult.returnPct.toFixed(2)}%
              </b>
            </div>
            <div>
              <span className="text-gray-400">{t("stockAnalysis.backtest.maxDrawdown")}</span>
              <br />
              <b>{btResult.maxDrawdownPct.toFixed(2)}%</b>
            </div>
            <div className="col-span-3 mt-1">
              <Tag color={btResult.wasCorrect ? "green" : "red"}>
                {btResult.wasCorrect ? t("stockAnalysis.backtest.correct") : t("stockAnalysis.backtest.wrong")}
              </Tag>
            </div>
          </div>
        </Card>
      )}

      {/* 历史列表 */}
      <Card
        size="small"
        title={t("stockAnalysis.history")}
        styles={{ body: { padding: "6px 8px" } }}
        extra={
          <div className="flex gap-1">
            <Input
              size="small"
              prefix={<SearchOutlined />}
              placeholder={t("stockAnalysis.search")}
              style={{ width: 100 }}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              allowClear
            />
            {selectMode
              ? (
                <>
                  <Button
                    size="small"
                    onClick={() => {
                      setSelectMode(false);
                      setSelectedIds([]);
                    }}
                  >
                    {t("stockAnalysis.historyBatchExit")}
                  </Button>
                  {selectedIds.length > 0 && (
                    <Button
                      size="small"
                      danger
                      loading={deleting}
                      onClick={async () => {
                        setDeleting(true);
                        try {
                          await invoke("batch_delete_stock_analyses", { analysisIds: selectedIds });
                          message.success(t("stockAnalysis.historyDeleteSuccess", { count: selectedIds.length }));
                          setRecords((prev) => prev.filter((r) => !selectedIds.includes(r.id)));
                          setSelectedIds([]);
                          setSelectMode(false);
                        } catch (e) {
                          message.error(String(e));
                        }
                        setDeleting(false);
                      }}
                    >
                      {t("stockAnalysis.historyBatchDelete")}
                    </Button>
                  )}
                </>
              )
              : (
                <>
                  <Button size="small" loading={btLoading} onClick={runBacktestAll}>
                    {t("stockAnalysis.backtest.runAll")}
                  </Button>
                  <Button size="small" onClick={() => setSelectMode(true)}>
                    {t("stockAnalysis.historySelectMode")}
                  </Button>
                </>
              )}
          </div>
        }
      >
        {loading
          ? <Spin size="small" />
          : filtered.length === 0
          ? <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description={t("stockAnalysis.noRecords")} />
          : grouped.map((g) => (
            <div key={g.stockCode} className="mb-2">
              {/* 组标题：股票名称(股票代码) - 名称用主题色，代码用次要色 */}
              <div
                className="text-xs font-semibold flex items-center gap-1 px-1 py-1"
                style={{
                  borderBottom: "1px solid var(--color-border, #333)",
                }}
              >
                <span style={{ color: "var(--accent, #7c3aed)" }}>{g.stockName}</span>
                <span style={{ color: "var(--color-text-secondary, #888)", fontSize: 10 }}>
                  ({g.stockCode})
                </span>
              </div>
              {/* 组内记录：日期为记录名 */}
              <List
                size="small"
                dataSource={g.items}
                renderItem={(r) => {
                  // 优先使用后端直返字段 decisionAction / decisionPositionPct，
                  // decisionJson 仅用于提取 confidence 等额外字段（兼容旧数据）。
                  let action = r.decisionAction ? parseAction(r.decisionAction) : "";
                  let posPct: number | null = r.decisionPositionPct;
                  let conf: number | null = null;
                  if (r.decisionJson) {
                    try {
                      const d = JSON.parse(r.decisionJson) as Record<string, unknown>;
                      if (!action && d.action) { action = parseAction(d.action as string); }
                      if (posPct == null && typeof d.positionPct === "number") { posPct = d.positionPct; }
                      if (typeof d.confidence === "number") { conf = d.confidence; }
                    } catch { /* */ }
                  }
                  const hasDecision = !!action;
                  return (
                    <List.Item
                      style={{ cursor: "pointer", padding: "4px 0 4px 12px" }}
                      onClick={() => {
                        if (selectMode) {
                          setSelectedIds((prev) =>
                            prev.includes(r.id) ? prev.filter((id) => id !== r.id) : [...prev, r.id]
                          );
                        } else {
                          runBacktest(r);
                        }
                      }}
                      actions={[
                        selectMode
                          ? (
                            <Checkbox
                              key="select"
                              checked={selectedIds.includes(r.id)}
                              onClick={(e) => e.stopPropagation()}
                              onChange={() => {
                                setSelectedIds((prev) =>
                                  prev.includes(r.id) ? prev.filter((id) => id !== r.id) : [...prev, r.id]
                                );
                              }}
                            />
                          )
                          : (
                            <>
                              {hasDecision && (
                                <Tag
                                  key="act"
                                  style={getActionTagStyle(action)}
                                >
                                  {t(getActionTKey(action))}
                                </Tag>
                              )}
                              <Button
                                key="bt"
                                size="small"
                                type="link"
                                className="text-xs px-1"
                                loading={btLoading}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  runBacktest(r);
                                }}
                              >
                                {t("stockAnalysis.backtest.run")}
                              </Button>
                            </>
                          ),
                      ]}
                    >
                      <div className="flex flex-col gap-0.5">
                        <div className="flex items-center gap-2 text-xs">
                          <span>{r.analysisDate || new Date(r.createdAt).toLocaleDateString()}</span>
                          {r.parentAnalysisId && (
                            <Tag
                              className="m-0"
                              style={{
                                margin: 0,
                                fontSize: 10,
                                lineHeight: "16px",
                                padding: "0 4px",
                                borderRadius: 3,
                                border: "1px solid var(--color-accent)",
                                color: "var(--color-accent)",
                                background: "color-mix(in oklch, var(--color-accent) 12%, transparent)",
                              }}
                            >
                              ↻
                            </Tag>
                          )}
                        </div>
                        {r.createdAt > 0 && (
                          <div className="text-[10px]" style={{ color: "var(--color-t-tertiary, #888)" }}>
                            {t("stockAnalysis.analysisTime")}{" "}
                            {new Date(r.createdAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                          </div>
                        )}
                        {/* 重要要素：仓位 + 置信度 */}
                        {(() => {
                          const parts: string[] = [];
                          if (posPct != null) {
                            parts.push(`${t("stockAnalysis.decision.positionPct")}${posPct}%`);
                          }
                          if (conf != null) {
                            parts.push(`${t("stockAnalysis.decision.confidence")}${conf.toFixed(0)}%`);
                          }
                          if (parts.length === 0) { return null; }
                          return (
                            <div className="text-[10px]" style={{ color: "var(--color-t-tertiary, #888)" }}>
                              {parts.join(" · ")}
                            </div>
                          );
                        })()}
                      </div>
                    </List.Item>
                  );
                }}
              />
            </div>
          ))}
      </Card>
    </div>
  );
}
