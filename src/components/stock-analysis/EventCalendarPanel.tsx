import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { Button, Card, Segmented, Spin, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";
import { checkVendorEnabled, PANEL_VENDORS } from "./vendorCheck";

type EventType = "lockup" | "dividend" | "earnings";
type FilterKey = "all" | "earnings" | "lockup" | "dividend";

interface EventItem {
  type: EventType;
  code: string;
  name: string;
  date: string;
  detail: string;
  /// 仅 earnings 类型有
  earningsType?: "preliminary" | "express" | "formal" | "shareholders_meeting" | "other";
  period?: string;
}

export function EventCalendarPanel() {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const stockName = useStockAnalysisStore((s) => s.stockName);
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);
  const [events, setEvents] = useState<EventItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);
  const [emptyVendors, setEmptyVendors] = useState<string[] | undefined>(undefined);
  const [filter, setFilter] = useState<FilterKey>("all");

  const fetchOneStock = async (code: string, name: string, items: EventItem[]) => {
    try {
      const lu: Record<string, unknown>[] = await invoke<Record<string, unknown>[]>("get_lockup_schedule", {
        stockCode: code,
      }) as Record<
        string,
        unknown
      >[];
      if (Array.isArray(lu)) {
        for (const l of lu.slice(0, 5)) {
          const date = (l.unlockDate ?? l.unlock_date ?? "") as string;
          if (!date) { continue; }
          items.push({
            type: "lockup",
            code,
            name,
            date,
            detail: `${(Number(l.unlockRatio ?? l.unlock_ratio ?? 0)).toFixed(1)}% ${
              t("stockAnalysis.settings.panels.lockup")
            }`,
          });
        }
      }
    } catch { /* 单只失败不影响其他 */ }
    try {
      const dv: Record<string, unknown>[] = await invoke<Record<string, unknown>[]>("get_dividend_records", {
        stockCode: code,
      }) as Record<
        string,
        unknown
      >[];
      if (Array.isArray(dv)) {
        for (const d of dv.slice(0, 3)) {
          const ex = (d.exDate ?? d.ex_date ?? "") as string;
          if (!ex) { continue; }
          items.push({
            type: "dividend",
            code,
            name,
            date: ex,
            detail: `${(Number(d.dividendPerShare ?? d.dividend_per_share ?? 0)).toFixed(2)}${
              t("stockAnalysis.settings.panels.perShare")
            }`,
          });
        }
      }
    } catch { /* */ }
    // R3-B 接入:财报披露事件
    try {
      const evs: Record<string, unknown>[] = await invoke<Record<string, unknown>[]>("get_earnings_calendar", {
        stockCode: code,
      }) as Record<
        string,
        unknown
      >[];
      if (Array.isArray(evs)) {
        for (const e of evs.slice(0, 6)) {
          const ed = (e.eventDate ?? e.event_date ?? "") as string;
          if (!ed) { continue; }
          const evType = (e.eventType ?? e.event_type ?? "other") as string;
          items.push({
            type: "earnings",
            code,
            name: (e.stockName ?? e.stock_name ?? name) as string,
            date: ed,
            detail: (e.detail ?? "") as string,
            earningsType: evType as "formal" | "preliminary" | "express" | "shareholders_meeting" | "other" | undefined,
            period: e.period as string | undefined,
          });
        }
      }
    } catch { /* */ }
  };

  const load = async () => {
    setLoading(true);
    setEmptyKind(null);
    setEmptyVendors(undefined);
    const check = await checkVendorEnabled("events", { silent: true });
    if (check.status === "disabled") {
      setEvents([]);
      setEmptyKind("vendorDisabled");
      setEmptyVendors(check.vendors);
      setLoading(false);
      return;
    }
    if (check.status === "backend_offline") {
      setEvents([]);
      setEmptyKind("backendOffline");
      setLoading(false);
      return;
    }

    const items: EventItem[] = [];
    // 优先：当前正在分析的股票（即使没在自选里也能看到）
    if (stockCode) {
      await fetchOneStock(stockCode, stockName ?? stockCode, items);
    }
    // 补充：自选股列表
    try {
      const wl: Record<string, unknown>[] = await invoke("list_watchlist") as Record<string, unknown>[];
      if (Array.isArray(wl)) {
        for (const w of wl.slice(0, 8)) {
          if (stockCode && (w.stockCode as string) === stockCode) { continue; }
          await fetchOneStock(w.stockCode as string, (w.stockName ?? w.stockCode) as string, items);
        }
      }
    } catch { /* 无自选或后端不可用时跳过 */ }

    items.sort((a, b) => a.date.localeCompare(b.date));
    setEvents(items.slice(0, 30));
    if (items.length === 0) { setEmptyKind("noData"); }
    setLoading(false);
  };

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      setEmptyKind(null);
      setEmptyVendors(undefined);
      return (async () => {
        const check = await checkVendorEnabled("events", { silent: true });
        if (cancelled) { return; }
        if (check.status === "disabled") {
          setEvents([]);
          setEmptyKind("vendorDisabled");
          setEmptyVendors(check.vendors);
          setLoading(false);
          return;
        }
        if (check.status === "backend_offline") {
          setEvents([]);
          setEmptyKind("backendOffline");
          setLoading(false);
          return;
        }

        const items: EventItem[] = [];
        // 优先：当前正在分析的股票
        if (stockCode) {
          await (async () => {
            try {
              const lu = await invoke<unknown[]>("get_lockup_schedule", { stockCode: stockCode });
              if (Array.isArray(lu)) {
                for (const l of lu.slice(0, 5)) {
                  const item = l as Record<string, unknown>;
                  const date = (item.unlockDate ?? item.unlock_date ?? "") as string;
                  if (!date) { continue; }
                  items.push({
                    type: "lockup",
                    code: stockCode,
                    name: stockName ?? stockCode,
                    date,
                    detail: `${(Number(item.unlockRatio ?? item.unlock_ratio ?? 0)).toFixed(1)}% ${
                      t("stockAnalysis.settings.panels.lockup")
                    }`,
                  });
                }
              }
            } catch { /* 单只失败不影响其他 */ }
            try {
              const dv = await invoke<unknown[]>("get_dividend_records", { stockCode: stockCode });
              if (Array.isArray(dv)) {
                for (const d of dv.slice(0, 3)) {
                  const item = d as Record<string, unknown>;
                  const ex = (item.exDate ?? item.ex_date ?? "") as string;
                  if (!ex) { continue; }
                  items.push({
                    type: "dividend",
                    code: stockCode,
                    name: stockName ?? stockCode,
                    date: ex,
                    detail: `${(Number(item.dividendPerShare ?? item.dividend_per_share ?? 0)).toFixed(2)}${
                      t("stockAnalysis.settings.panels.perShare")
                    }`,
                  });
                }
              }
            } catch { /* */ }
            try {
              const evs = await invoke<unknown[]>("get_earnings_calendar", { stockCode: stockCode });
              if (Array.isArray(evs)) {
                for (const e of evs.slice(0, 6)) {
                  const item = e as Record<string, unknown>;
                  const ed = (item.eventDate ?? item.event_date ?? "") as string;
                  if (!ed) { continue; }
                  items.push({
                    type: "earnings",
                    code: stockCode,
                    name: (item.stockName ?? item.stock_name ?? stockName ?? stockCode) as string,
                    date: ed,
                    detail: (item.detail ?? "") as string,
                    earningsType: (item.eventType ?? item.event_type ?? "other") as EventItem["earningsType"],
                    period: item.period as string | undefined,
                  });
                }
              }
            } catch { /* */ }
          })();
        }
        // 补充：自选股列表
        try {
          const wl: Record<string, unknown>[] = await invoke("list_watchlist") as Record<string, unknown>[];
          if (Array.isArray(wl)) {
            for (const w of wl.slice(0, 8)) {
              if (cancelled) { return; }
              if (stockCode && (w.stockCode as string) === stockCode) { continue; }
              const code = w.stockCode as string;
              const name = (w.stockName ?? w.stockCode) as string;
              try {
                const lu: Record<string, unknown>[] = await invoke<Record<string, unknown>[]>("get_lockup_schedule", {
                  stockCode: code,
                }) as Record<string, unknown>[];
                if (Array.isArray(lu)) {
                  for (const l of lu.slice(0, 5)) {
                    const date = (l.unlockDate ?? l.unlock_date ?? "") as string;
                    if (!date) { continue; }
                    items.push({
                      type: "lockup",
                      code,
                      name,
                      date,
                      detail: `${(Number(l.unlockRatio ?? l.unlock_ratio ?? 0)).toFixed(1)}% ${
                        t("stockAnalysis.settings.panels.lockup")
                      }`,
                    });
                  }
                }
              } catch { /* 单只失败不影响其他 */ }
              try {
                const dv: Record<string, unknown>[] = await invoke<Record<string, unknown>[]>("get_dividend_records", {
                  stockCode: code,
                }) as Record<string, unknown>[];
                if (Array.isArray(dv)) {
                  for (const d of dv.slice(0, 3)) {
                    const ex = (d.exDate ?? d.ex_date ?? "") as string;
                    if (!ex) { continue; }
                    items.push({
                      type: "dividend",
                      code,
                      name,
                      date: ex,
                      detail: `${(Number(d.dividendPerShare ?? d.dividend_per_share ?? 0)).toFixed(2)}${
                        t("stockAnalysis.settings.panels.perShare")
                      }`,
                    });
                  }
                }
              } catch { /* */ }
              try {
                const evs: Record<string, unknown>[] = await invoke<Record<string, unknown>[]>(
                  "get_earnings_calendar",
                  {
                    stockCode: code,
                  },
                ) as Record<string, unknown>[];
                if (Array.isArray(evs)) {
                  for (const e of evs.slice(0, 6)) {
                    const ed = (e.eventDate ?? e.event_date ?? "") as string;
                    if (!ed) { continue; }
                    items.push({
                      type: "earnings",
                      code,
                      name: (e.stockName ?? e.stock_name ?? name) as string,
                      date: ed,
                      detail: (e.detail ?? "") as string,
                      earningsType: (e.eventType ?? e.event_type ?? "other") as EventItem["earningsType"],
                      period: e.period as string | undefined,
                    });
                  }
                }
              } catch { /* */ }
            }
          }
        } catch { /* 无自选或后端不可用时跳过 */ }

        if (cancelled) { return; }
        items.sort((a, b) => a.date.localeCompare(b.date));
        setEvents(items.slice(0, 30));
        if (items.length === 0) { setEmptyKind("noData"); }
        setLoading(false);
      })();
    })
      .catch(() => {})
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [stockCode, stockName, t]);

  const analyze = async (code: string) => {
    await getStockQuote(code);
    await getStockKline(code, "daily", 120);
    startAnalysis(code);
  };

  const filtered = filter === "all" ? events : events.filter((e) => e.type === filter);

  const typeLabel = (it: EventItem) => {
    if (it.type === "lockup") { return t("stockAnalysis.settings.panels.lockup"); }
    if (it.type === "dividend") { return t("stockAnalysis.settings.panels.dividend"); }
    if (it.type === "earnings") {
      const sub = it.earningsType;
      if (sub === "preliminary") { return t("stockAnalysis.calendar.earningsPre"); }
      if (sub === "express") { return t("stockAnalysis.calendar.earningsExpress"); }
      if (sub === "formal") { return t("stockAnalysis.calendar.earningsFormal"); }
      if (sub === "shareholders_meeting") { return t("stockAnalysis.calendar.shareholdersMeeting"); }
      return t("stockAnalysis.calendar.earningsFormal");
    }
    return it.type;
  };

  const typeColor = (it: EventItem) => {
    if (it.type === "lockup") { return "orange"; }
    if (it.type === "dividend") { return "blue"; }
    if (it.type === "earnings") {
      if (it.earningsType === "preliminary") { return "magenta"; }
      if (it.earningsType === "express") { return "purple"; }
      if (it.earningsType === "shareholders_meeting") { return "cyan"; }
      return "geekblue";
    }
    return "default";
  };

  return (
    <Card
      size="small"
      title={`📅 ${t("stockAnalysis.settings.panels.eventCalendar")}`}
      styles={{ body: { padding: "4px 8px" } }}
      extra={
        <Button size="small" loading={loading} onClick={load}>
          {t("stockAnalysis.settings.panels.refresh")}
        </Button>
      }
    >
      <div style={{ marginBottom: 4 }}>
        <Segmented
          size="small"
          value={filter}
          onChange={(v) => setFilter(v as FilterKey)}
          options={[
            { label: t("stockAnalysis.calendar.filter.all"), value: "all" },
            { label: t("stockAnalysis.calendar.filter.earnings"), value: "earnings" },
            { label: t("stockAnalysis.calendar.filter.lockup"), value: "lockup" },
            { label: t("stockAnalysis.calendar.filter.dividend"), value: "dividend" },
          ]}
        />
      </div>
      {loading
        ? <Spin size="small" style={{ display: "block", margin: "16px auto" }} />
        : emptyKind && events.length === 0
        ? (
          <PanelEmpty
            kind={emptyKind}
            vendorNames={emptyVendors ?? PANEL_VENDORS.events}
            description={emptyKind === "noData"
              ? (!stockCode ? t("stockAnalysis.selectStockFirst") : t("stockAnalysis.settings.panels.noEvents"))
              : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : filtered.length === 0
        ? (
          <div className="text-xs text-gray-400 text-center py-3">
            {t("stockAnalysis.calendar.noDataForFilter")}
          </div>
        )
        : (
          <List
            size="small"
            dataSource={filtered}
            renderItem={(ev) => (
              <List.Item
                style={{ cursor: "pointer", padding: "3px 0" }}
                onClick={() => analyze(ev.code)}
                actions={[
                  <Tag key="type" color={typeColor(ev)} className="text-xs m-0">
                    {typeLabel(ev)}
                  </Tag>,
                ]}
              >
                <div className="flex items-center gap-2 text-xs w-full">
                  <Tag className="m-0 text-xs">{ev.code}</Tag>
                  <span className="flex-1 truncate">
                    {ev.name}
                    {ev.type === "earnings" && ev.period
                      ? <span className="ml-1 text-gray-400">· {ev.period}</span>
                      : null}
                  </span>
                  <span className="text-gray-400">{ev.date}</span>
                  <span className="text-gray-500 truncate max-w-[40%]">{ev.detail}</span>
                </div>
              </List.Item>
            )}
          />
        )}
    </Card>
  );
}
