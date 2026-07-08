import { List } from "@/components/common/AntdList";
import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { Button, Card, Spin, Tag, Tooltip } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";
import { checkVendorEnabled, PANEL_VENDORS } from "./vendorCheck";

interface SectorEntry {
  name: string;
  changePct: number;
  turnover: number;
  leaderCode?: string;
  leaderName?: string;
  leaderChange?: number;
}

export function SectorHeatmapPanel() {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);
  const [sectors, setSectors] = useState<SectorEntry[]>([]);
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
        setSectors([]);
        setEmptyKind("vendorDisabled");
        setEmptyVendors(check.vendors);
        setLoading(false);
        return;
      }
      if (check.status === "backend_offline") {
        setSectors([]);
        setEmptyKind("backendOffline");
        setLoading(false);
        return;
      }
      const list = await invoke("get_industry_ranking") as Record<string, unknown>[];
      if (!Array.isArray(list)) { throw new Error("bad data"); }
      const next: SectorEntry[] = list.slice(0, 25).map((e) => {
        const r = e as Record<string, unknown>;
        return {
          name: (r.industryName ?? r.industry_name ?? "") as string,
          changePct: Number(r.changePct ?? r.change_pct ?? 0),
          turnover: Number(r.turnover ?? 0),
          leaderCode: (r.leaderCode ?? r.leader_code) as string | undefined,
          leaderName: (r.leaderName ?? r.leader_name) as string | undefined,
          leaderChange: (r.leaderChangePct ?? r.leader_change_pct) as number | undefined,
        };
      });
      // 涨幅降序
      next.sort((a, b) => b.changePct - a.changePct);
      setSectors(next);
      if (next.length === 0) { setEmptyKind("noData"); }
    } catch {
      setSectors([]);
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
          setSectors([]);
          setEmptyKind("vendorDisabled");
          setEmptyVendors(check.vendors);
          return;
        }
        if (check.status === "backend_offline") {
          setSectors([]);
          setEmptyKind("backendOffline");
          return;
        }
        return invoke("get_industry_ranking") as Promise<Record<string, unknown>[]>;
      })
      .then((list) => {
        if (cancelled || !list) { return; }
        if (!Array.isArray(list)) { throw new Error("bad data"); }
        const next: SectorEntry[] = list.slice(0, 25).map((e) => {
          const r = e as Record<string, unknown>;
          return {
            name: (r.industryName ?? r.industry_name ?? "") as string,
            changePct: Number(r.changePct ?? r.change_pct ?? 0),
            turnover: Number(r.turnover ?? 0),
            leaderCode: (r.leaderCode ?? r.leader_code) as string | undefined,
            leaderName: (r.leaderName ?? r.leader_name) as string | undefined,
            leaderChange: (r.leaderChangePct ?? r.leader_change_pct) as number | undefined,
          };
        });
        next.sort((a, b) => b.changePct - a.changePct);
        setSectors(next);
        if (next.length === 0) { setEmptyKind("noData"); }
      })
      .catch(() => {
        if (!cancelled) {
          setSectors([]);
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

  const getColor = (pct: number) => {
    if (pct >= 3) { return "red"; }
    if (pct >= 1) { return "orange"; }
    if (pct >= 0) { return "gold"; }
    if (pct >= -1) { return "green"; }
    if (pct >= -3) { return "cyan"; }
    return "blue";
  };

  return (
    <Card
      size="small"
      title={`🔥 ${t("stockAnalysis.settings.panels.sectorHeatmap")}`}
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
            description={emptyKind === "noData" ? t("stockAnalysis.settings.panels.noSector") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : (
          <List
            size="small"
            dataSource={sectors}
            renderItem={(s) => {
              const leaderTip = s.leaderName && s.leaderChange != null
                ? `${s.leaderName} (${s.leaderChange >= 0 ? "+" : ""}${s.leaderChange.toFixed(2)}%)`
                : s.leaderName;
              return (
                <List.Item
                  style={{ cursor: s.leaderCode ? "pointer" : "default", padding: "3px 0" }}
                  onClick={() => {
                    if (s.leaderCode) { analyze(s.leaderCode); }
                  }}
                >
                  <div className="flex items-center gap-2 text-xs w-full">
                    <Tag color={getColor(s.changePct)} className="text-xs m-0 min-w-0 truncate">{s.name}</Tag>
                    <span
                      className={s.changePct >= 0 ? "text-red-500" : "text-green-500"}
                      style={{ minWidth: 52 }}
                    >
                      {s.changePct >= 0 ? "+" : ""}
                      {s.changePct.toFixed(2)}%
                    </span>
                    {leaderTip && (
                      <Tooltip title={leaderTip}>
                        <span className="text-gray-400 truncate">🏷 {s.leaderName}</span>
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
