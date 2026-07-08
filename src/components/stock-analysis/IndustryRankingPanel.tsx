import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { Button, Card, Spin, Tag, Tooltip } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";
import { checkVendorEnabled, PANEL_VENDORS } from "./vendorCheck";

interface IndustryRow {
  rank: number;
  industryName: string;
  industryCode: string;
  changePct: number;
  mainInflow: number | null;
  leaderCode: string;
  leaderName: string;
  leaderChangePct: number;
}

function fmtYi(v: number): string {
  if (Math.abs(v) >= 1e8) { return `${(v / 1e8).toFixed(2)}亿`; }
  if (Math.abs(v) >= 1e4) { return `${(v / 1e4).toFixed(0)}万`; }
  return `${v.toFixed(0)}`;
}

export function IndustryRankingPanel() {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);
  const [rows, setRows] = useState<IndustryRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);
  const [emptyVendors, setEmptyVendors] = useState<string[] | undefined>(undefined);

  const load = async (silent = false) => {
    setLoading(true);
    setEmptyKind(null);
    setEmptyVendors(undefined);
    try {
      const check = await checkVendorEnabled("sectors", { silent });
      if (check.status === "disabled") {
        setRows([]);
        setEmptyKind("vendorDisabled");
        setEmptyVendors(check.vendors);
        setLoading(false);
        return;
      }
      if (check.status === "backend_offline") {
        setRows([]);
        setEmptyKind("backendOffline");
        setLoading(false);
        return;
      }
      const data = await invoke("get_industry_ranking") as Record<string, unknown>[];
      if (!Array.isArray(data)) { throw new Error("bad data"); }
      const list: IndustryRow[] = data.slice(0, 20).map((e, i) => {
        const r = e as Record<string, unknown>;
        return {
          rank: i + 1,
          industryName: (r.industryName ?? r.industry_name ?? "") as string,
          industryCode: (r.industryCode ?? r.industry_code ?? "") as string,
          changePct: Number(r.changePct ?? r.change_pct ?? 0),
          mainInflow: r.mainInflow != null
            ? Number(r.mainInflow)
            : (r.main_inflow != null ? Number(r.main_inflow) : null),
          leaderCode: (r.leaderCode ?? r.leader_code ?? "") as string,
          leaderName: (r.leaderName ?? r.leader_name ?? "") as string,
          leaderChangePct: Number(r.leaderChangePct ?? r.leader_change_pct ?? 0),
        };
      }).map((item, i) => ({ ...item, rank: i + 1 }));
      list.sort((a, b) => b.changePct - a.changePct);
      list.forEach((r, i) => {
        r.rank = i + 1;
      });
      setRows(list);
      if (list.length === 0) { setEmptyKind("noData"); }
    } catch {
      setRows([]);
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
      return checkVendorEnabled("sectors", { silent: true });
    })
      .then((check) => {
        if (cancelled || !check) { return; }
        if (check.status === "disabled") {
          setRows([]);
          setEmptyKind("vendorDisabled");
          setEmptyVendors(check.vendors);
          return;
        }
        if (check.status === "backend_offline") {
          setRows([]);
          setEmptyKind("backendOffline");
          return;
        }
        return invoke("get_industry_ranking") as Promise<Record<string, unknown>[]>;
      })
      .then((data) => {
        if (cancelled || !data) { return; }
        if (!Array.isArray(data)) { throw new Error("bad data"); }
        const list: IndustryRow[] = data.slice(0, 20).map((e, i: number) => {
          const r = e as Record<string, unknown>;
          return {
            rank: i + 1,
            industryName: (r.industryName ?? r.industry_name ?? "") as string,
            industryCode: (r.industryCode ?? r.industry_code ?? "") as string,
            changePct: Number(r.changePct ?? r.change_pct ?? 0),
            mainInflow: r.mainInflow != null
              ? Number(r.mainInflow)
              : (r.main_inflow != null ? Number(r.main_inflow) : null),
            leaderCode: (r.leaderCode ?? r.leader_code ?? "") as string,
            leaderName: (r.leaderName ?? r.leader_name ?? "") as string,
            leaderChangePct: Number(r.leaderChangePct ?? r.leader_change_pct ?? 0),
          };
        });
        list.sort((a, b) => b.changePct - a.changePct);
        list.forEach((r, i) => {
          r.rank = i + 1;
        });
        setRows(list);
        if (list.length === 0) { setEmptyKind("noData"); }
      })
      .catch(() => {
        if (!cancelled) {
          setRows([]);
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
    if (!code) { return; }
    await getStockQuote(code);
    await getStockKline(code, "daily", 120);
    startAnalysis(code);
  };

  return (
    <Card
      size="small"
      title={`🏆 ${t("stockAnalysis.settings.panels.industryRank")}`}
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
            vendorNames={emptyVendors ?? PANEL_VENDORS.sectors}
            description={emptyKind === "noData" ? t("stockAnalysis.settings.panels.noIndustry") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : (
          <List
            size="small"
            dataSource={rows}
            renderItem={(r) => {
              const up = r.changePct >= 0;
              return (
                <List.Item
                  style={{ padding: "3px 0", cursor: r.leaderCode ? "pointer" : "default" }}
                  onClick={() => analyze(r.leaderCode)}
                >
                  <div className="flex items-center gap-2 text-xs w-full">
                    <span className="w-5 text-right text-gray-400 font-mono shrink-0">{r.rank}</span>
                    <Tag
                      className="m-0 text-xs shrink-0"
                      color={r.rank <= 3 ? (up ? "red" : "green") : "default"}
                    >
                      {r.industryName}
                    </Tag>
                    <span
                      className={up ? "text-red-500" : "text-green-500"}
                      style={{ minWidth: 50 }}
                    >
                      {up ? "+" : ""}
                      {r.changePct.toFixed(2)}%
                    </span>
                    {r.mainInflow != null && (
                      <span className="text-gray-400 text-xs">
                        {t("stockAnalysis.settings.panels.main")} {fmtYi(r.mainInflow)}
                      </span>
                    )}
                    {r.leaderName && (
                      <Tooltip
                        title={`${r.leaderName} (${r.leaderChangePct >= 0 ? "+" : ""}${r.leaderChangePct.toFixed(2)}%)`}
                      >
                        <span className="text-gray-400 truncate">👑 {r.leaderName}</span>
                      </Tooltip>
                    )}
                  </div>
                </List.Item>
              );
            }}
          />
        )}
    </Card>
  );
}
