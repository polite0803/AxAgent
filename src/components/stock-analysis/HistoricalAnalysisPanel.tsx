import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { getActionColor } from "@/lib/stock-analysis-utils";
import { SearchOutlined } from "@ant-design/icons";
import { App, Button, Card, Checkbox, Collapse, Empty, Input, Spin, Statistic, Tag } from "antd";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

interface AnalysisRecord {
  id: string;
  stockCode: string;
  stockName: string;
  decisionJson: string | null;
  blackboardSnapshot: string | null;
  createdAt: number;
  status: string;
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
  maxDrawdown: number;
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

  const reportEntries = Object.entries(snapshot ?? {}).filter(([k]) => k.startsWith("report."));
  const debateEntries = Object.entries(snapshot ?? {}).filter(([k]) => k.startsWith("debate."));

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

      {/* 全量回测汇总 */}
      {btStats && (
        <Card size="small" title={t("stockAnalysis.backtest.summary")} styles={{ body: { padding: "4px 8px" } }}>
          <div className="grid grid-cols-3 gap-1 text-center">
            <Statistic title={t("stockAnalysis.backtest.total")} value={btStats.total} valueStyle={{ fontSize: 14 }} />
            <Statistic
              title={t("stockAnalysis.backtest.accuracy")}
              value={btStats.correct}
              suffix={`/${btStats.total}`}
              valueStyle={{ fontSize: 14, color: "var(--sa-green)" }}
            />
            <Statistic
              title={t("stockAnalysis.backtest.avgReturn")}
              value={btStats.avgReturn}
              suffix="%"
              valueStyle={{ fontSize: 14 }}
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
              <b>{btResult.maxDrawdown.toFixed(2)}%</b>
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
                    {t("chat.batchExit")}
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
          : (
            <List
              size="small"
              dataSource={filtered}
              renderItem={(r) => {
                let decision: Record<string, unknown> | null = null;
                try {
                  if (r.decisionJson) { decision = JSON.parse(r.decisionJson); }
                } catch { /* */ }
                return (
                  <List.Item
                    style={{ cursor: "pointer", padding: "4px 0" }}
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
                            {decision && (
                              <Tag
                                key="act"
                                color={getActionColor(decision.action as string)}
                                className="text-xs m-0"
                              >
                                {decision.action as string}
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
                    <div className="flex items-center gap-2 text-xs">
                      <Tag className="m-0 text-xs">{r.stockCode}</Tag>
                      <span>{r.stockName}</span>
                    </div>
                  </List.Item>
                );
              }}
            />
          )}
      </Card>
    </div>
  );
}
