import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { Button, Card, Spin, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";
import { checkVendorEnabled, PANEL_VENDORS } from "./vendorCheck";

interface DragonTigerEntry {
  code: string;
  name: string;
  date: string;
  netBuy: number;
  buyAmount: number;
  sellAmount: number;
  reason?: string;
}

function fmtYi(v: number): string {
  if (Math.abs(v) >= 1e8) { return `${(v / 1e8).toFixed(2)}亿`; }
  if (Math.abs(v) >= 1e4) { return `${(v / 1e4).toFixed(0)}万`; }
  return `${v.toFixed(0)}`;
}

interface DragonTigerPanelProps {
  /** 是否显示外边框(ScreenerPage Collapse 内嵌时传 false) */
  bordered?: boolean;
}

export function DragonTigerPanel({ bordered = true }: DragonTigerPanelProps = {}) {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);
  const [entries, setEntries] = useState<DragonTigerEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);
  const [emptyVendors, setEmptyVendors] = useState<string[] | undefined>(undefined);

  const load = async (silent = false) => {
    setLoading(true);
    setEmptyKind(null);
    setEmptyVendors(undefined);
    try {
      const check = await checkVendorEnabled("dragontiger", { silent });
      if (check.status === "disabled") {
        setEntries([]);
        setEmptyKind("vendorDisabled");
        setEmptyVendors(check.vendors);
        setLoading(false);
        return;
      }
      if (check.status === "backend_offline") {
        setEntries([]);
        setEmptyKind("backendOffline");
        setLoading(false);
        return;
      }
      const data = await invoke("get_market_dragon_tiger") as Record<string, unknown>[];
      if (!Array.isArray(data)) { throw new Error("bad data"); }
      const list: DragonTigerEntry[] = data.slice(0, 30).map((e) => ({
        code: String((e as Record<string, unknown>).stockCode ?? (e as Record<string, unknown>).stock_code ?? ""),
        name: String((e as Record<string, unknown>).stockName ?? (e as Record<string, unknown>).stock_name ?? ""),
        date: String((e as Record<string, unknown>).date ?? ""),
        netBuy: Number((e as Record<string, unknown>).netBuy ?? (e as Record<string, unknown>).net_buy ?? 0),
        buyAmount: Number((e as Record<string, unknown>).buyAmount ?? (e as Record<string, unknown>).buy_amount ?? 0),
        sellAmount: Number(
          (e as Record<string, unknown>).sellAmount ?? (e as Record<string, unknown>).sell_amount ?? 0,
        ),
        reason: (e as Record<string, unknown>).reason as string | undefined,
      }));
      // 按净买额降序
      list.sort((a, b) => b.netBuy - a.netBuy);
      setEntries(list);
      if (list.length === 0) { setEmptyKind("noData"); }
    } catch {
      setEntries([]);
      setEmptyKind("connectionFailed");
    }
    setLoading(false);
  };

  useEffect(() => {
    let cancelled = false;
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      setEmptyKind(null);
      setEmptyVendors(undefined);
      return checkVendorEnabled("dragontiger", { silent: true });
    })
      .then((check) => {
        if (cancelled || !check) { return; }
        if (check.status === "disabled") {
          setEntries([]);
          setEmptyKind("vendorDisabled");
          setEmptyVendors(check.vendors);
          return;
        }
        if (check.status === "backend_offline") {
          setEntries([]);
          setEmptyKind("backendOffline");
          return;
        }
        return invoke("get_market_dragon_tiger") as Promise<Record<string, unknown>[]>;
      })
      .then((data) => {
        if (cancelled || !data) { return; }
        if (!Array.isArray(data)) { throw new Error("bad data"); }
        const list: DragonTigerEntry[] = data.slice(0, 30).map((e) => {
          const r = e as Record<string, unknown>;
          return {
            code: String(r.stockCode ?? r.stock_code ?? ""),
            name: String(r.stockName ?? r.stock_name ?? ""),
            date: String(r.date ?? ""),
            netBuy: Number(r.netBuy ?? r.net_buy ?? 0),
            buyAmount: Number(r.buyAmount ?? r.buy_amount ?? 0),
            sellAmount: Number(r.sellAmount ?? r.sell_amount ?? 0),
            reason: r.reason as string | undefined,
          };
        });
        // 按净买额降序
        list.sort((a, b) => b.netBuy - a.netBuy);
        setEntries(list);
        if (list.length === 0) { setEmptyKind("noData"); }
      })
      .catch(() => {
        if (!cancelled) {
          setEntries([]);
          setEmptyKind("connectionFailed");
        }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const analyze = async (code: string) => {
    await getStockQuote(code);
    await getStockKline(code, "daily", 120);
    startAnalysis(code);
  };

  return (
    <Card
      size="small"
      bordered={bordered}
      title={`🐉 ${t("stockAnalysis.settings.panels.dragonTiger")}`}
      styles={{ body: { padding: "4px 8px" } }}
      extra={
        <Button size="small" loading={loading} onClick={() => load()}>
          {t("stockAnalysis.settings.panels.refresh")}
        </Button>
      }
    >
      {loading
        ? <Spin size="small" style={{ display: "block", margin: "16px auto" }} />
        : emptyKind
        ? (
          <PanelEmpty
            kind={emptyKind}
            vendorNames={emptyVendors ?? PANEL_VENDORS.dragontiger}
            description={emptyKind === "noData" ? t("stockAnalysis.settings.panels.noDragonTiger") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : (
          <List
            size="small"
            dataSource={entries}
            renderItem={(e) => {
              const up = e.netBuy >= 0;
              return (
                <List.Item
                  style={{ cursor: "pointer", padding: "3px 0" }}
                  onClick={() => analyze(e.code)}
                  actions={[
                    <Tag
                      key="net"
                      color={up ? "red" : "green"}
                      className="text-xs m-0"
                    >
                      {up ? t("stockAnalysis.settings.panels.netBuy") : t("stockAnalysis.settings.panels.netSell")}{" "}
                      {fmtYi(e.netBuy)}
                    </Tag>,
                  ]}
                >
                  <div className="flex items-center gap-2 text-xs w-full">
                    <Tag className="m-0 text-xs">{e.code}</Tag>
                    <span className="flex-1 truncate">{e.name}</span>
                    <span className="text-gray-500 truncate max-w-[140px]">{e.reason ?? ""}</span>
                  </div>
                </List.Item>
              );
            }}
          />
        )}
    </Card>
  );
}
