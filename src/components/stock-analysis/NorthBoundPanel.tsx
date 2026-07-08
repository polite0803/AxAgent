import { invoke } from "@/lib/invoke";
import { Button, Card, Spin, Statistic } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";
import { checkVendorEnabled, PANEL_VENDORS } from "./vendorCheck";

interface NbFlow {
  date: string;
  shFlow: number;
  szFlow: number;
  totalFlow: number;
}

export function NorthBoundPanel() {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const [flow, setFlow] = useState<NbFlow | null>(null);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);
  const [emptyVendors, setEmptyVendors] = useState<string[] | undefined>(undefined);

  const load = async (silent = false) => {
    setLoading(true);
    setEmptyKind(null);
    setEmptyVendors(undefined);
    try {
      const check = await checkVendorEnabled("north", { silent });
      if (check.status === "disabled") {
        setFlow(null);
        setEmptyKind("vendorDisabled");
        setEmptyVendors(check.vendors);
        setLoading(false);
        return;
      }
      if (check.status === "backend_offline") {
        setFlow(null);
        setEmptyKind("backendOffline");
        setLoading(false);
        return;
      }
      const f = await invoke("get_north_bound_flow") as Record<string, unknown>;
      if (!f) {
        setFlow(null);
        setEmptyKind("noData");
        return;
      }
      const next: NbFlow = {
        date: (f.date ?? "") as string,
        shFlow: Number(f.shFlow ?? f.sh_flow ?? 0),
        szFlow: Number(f.szFlow ?? f.sz_flow ?? 0),
        totalFlow: Number(f.totalFlow ?? f.total_flow ?? 0),
      };
      // 后端有时返回全 0（盘前/节假日），按"无数据"处理
      if (next.totalFlow === 0 && next.shFlow === 0 && next.szFlow === 0) {
        setFlow(null);
        setEmptyKind("noData");
      } else {
        setFlow(next);
      }
    } catch {
      setFlow(null);
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
      return checkVendorEnabled("north", { silent: true });
    })
      .then((check) => {
        if (cancelled || !check) { return; }
        if (check.status === "disabled") {
          setFlow(null);
          setEmptyKind("vendorDisabled");
          setEmptyVendors(check.vendors);
          return;
        }
        if (check.status === "backend_offline") {
          setFlow(null);
          setEmptyKind("backendOffline");
          return;
        }
        return invoke<Record<string, unknown>>("get_north_bound_flow");
      })
      .then((f) => {
        if (cancelled) { return; }
        if (!f) {
          setFlow(null);
          setEmptyKind("noData");
          return;
        }
        const next: NbFlow = {
          date: (f.date ?? "") as string,
          shFlow: Number(f.shFlow ?? f.sh_flow ?? 0),
          szFlow: Number(f.szFlow ?? f.sz_flow ?? 0),
          totalFlow: Number(f.totalFlow ?? f.total_flow ?? 0),
        };
        if (next.totalFlow === 0 && next.shFlow === 0 && next.szFlow === 0) {
          setFlow(null);
          setEmptyKind("noData");
        } else {
          setFlow(next);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setFlow(null);
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

  const total = flow?.totalFlow ?? 0;
  const dir = total >= 0 ? t("stockAnalysis.settings.panels.inflow") : t("stockAnalysis.settings.panels.outflow");
  const color = total >= 0 ? "var(--sa-red)" : "var(--sa-green)";

  return (
    <Card
      size="small"
      title={`🧭 ${t("stockAnalysis.settings.panels.northBound")}`}
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
            vendorNames={emptyVendors ?? PANEL_VENDORS.north}
            description={emptyKind === "noData" ? t("stockAnalysis.settings.panels.noNorthBound") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : flow && (
          <div className="text-center">
            <Statistic
              title={t("stockAnalysis.settings.panels.northTitle", { dir, date: flow.date })}
              value={Math.abs(total / 1e4).toFixed(1)}
              suffix={t("stockAnalysis.settings.panels.yiDisplay")}
              valueStyle={{ fontSize: 20, color, fontWeight: "bold" }}
            />
            <div className="grid grid-cols-2 gap-1 mt-1 text-xs text-gray-500">
              <span>
                {t("stockAnalysis.settings.panels.shFlow")}: {(flow.shFlow / 1e4).toFixed(1)}
                {t("stockAnalysis.settings.panels.yiDisplay")}
              </span>
              <span>
                {t("stockAnalysis.settings.panels.szFlow")}: {(flow.szFlow / 1e4).toFixed(1)}
                {t("stockAnalysis.settings.panels.yiDisplay")}
              </span>
            </div>
          </div>
        )}
    </Card>
  );
}
