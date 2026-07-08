import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { Button, Card, Spin, Table, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";
import { checkVendorEnabled, PANEL_VENDORS } from "./vendorCheck";

interface HotStock {
  stockCode: string;
  stockName: string;
  price: number;
  changePct: number;
  turnoverRate: number | null;
  reasonTags: string[];
  sector: string | null;
}

interface HotStocksPanelProps {
  /** 是否显示外边框(ScreenerPage Collapse 内嵌时传 false) */
  bordered?: boolean;
}

export function HotStocksPanel({ bordered = true }: HotStocksPanelProps = {}) {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const getStockQuote = useStockAnalysisStore((s) => s.getStockQuote);
  const getStockKline = useStockAnalysisStore((s) => s.getStockKline);
  const startAnalysis = useStockAnalysisStore((s) => s.startAnalysis);
  const [stocks, setStocks] = useState<HotStock[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);
  const [emptyVendors, setEmptyVendors] = useState<string[] | undefined>(undefined);

  const load = async (silent = false) => {
    setLoading(true);
    setEmptyKind(null);
    setEmptyVendors(undefined);
    try {
      const check = await checkVendorEnabled("screener", { silent });
      if (check.status === "disabled") {
        setStocks([]);
        setEmptyKind("vendorDisabled");
        setEmptyVendors(check.vendors);
        setLoading(false);
        return;
      }
      if (check.status === "backend_offline") {
        setStocks([]);
        setEmptyKind("backendOffline");
        setLoading(false);
        return;
      }
      const data = await invoke<HotStock[]>("get_hot_stocks");
      if (Array.isArray(data) && data.length > 0) {
        setStocks(data);
      } else {
        setStocks([]);
        setEmptyKind("noData");
      }
    } catch {
      setStocks([]);
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
      return checkVendorEnabled("screener", { silent: true });
    })
      .then((check) => {
        if (cancelled || !check) { return; }
        if (check.status === "disabled") {
          setStocks([]);
          setEmptyKind("vendorDisabled");
          setEmptyVendors(check.vendors);
          return;
        }
        if (check.status === "backend_offline") {
          setStocks([]);
          setEmptyKind("backendOffline");
          return;
        }
        return invoke<HotStock[]>("get_hot_stocks");
      })
      .then((data) => {
        if (cancelled || !data) { return; }
        if (Array.isArray(data) && data.length > 0) {
          setStocks(data);
        } else {
          setStocks([]);
          setEmptyKind("noData");
        }
      })
      .catch(() => {
        if (!cancelled) {
          setStocks([]);
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

  const columns = [
    {
      title: t("stockAnalysis.alert.code"),
      dataIndex: "stockCode",
      key: "stockCode",
      width: 70,
      render: (code: string) => <Tag className="m-0 text-xs">{code}</Tag>,
    },
    {
      title: t("stockAnalysis.alert.name"),
      dataIndex: "stockName",
      key: "stockName",
      width: 80,
      render: (v: string | null) => v ?? "-",
    },
    {
      title: t("stockAnalysis.price"),
      dataIndex: "price",
      key: "price",
      width: 70,
      render: (v: number | null | undefined) => v != null ? v.toFixed(2) : "-",
    },
    {
      title: t("stockAnalysis.change"),
      dataIndex: "changePct",
      key: "changePct",
      width: 70,
      render: (v: number | null | undefined) => {
        if (v == null) { return <span>-</span>; }
        const color = v >= 0 ? "var(--sa-red)" : "var(--sa-green)";
        return <span style={{ color, fontWeight: "bold" }}>{v >= 0 ? "+" : ""}{v.toFixed(2)}%</span>;
      },
    },
    {
      title: t("stockAnalysis.settings.panels.turnover"),
      dataIndex: "turnoverRate",
      key: "turnoverRate",
      width: 60,
      render: (v: number | null | undefined) => v != null ? `${v.toFixed(1)}%` : "-",
    },
    {
      title: t("stockAnalysis.settings.panels.tags"),
      dataIndex: "reasonTags",
      key: "reasonTags",
      render: (tags: string[] | null) => (
        <div className="flex flex-wrap gap-0.5">
          {(tags ?? []).slice(0, 2).map((tag, i) => <Tag key={i} color="volcano" className="text-xs m-0">{tag}</Tag>)}
        </div>
      ),
    },
  ];

  return (
    <Card
      size="small"
      bordered={bordered}
      title={`🔥 ${t("stockAnalysis.settings.panels.hotStocks")}`}
      styles={{ body: { padding: 0 } }}
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
            vendorNames={emptyVendors ?? PANEL_VENDORS.screener}
            description={emptyKind === "noData" ? t("stockAnalysis.settings.panels.noHot") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : (
          <Table
            dataSource={stocks}
            columns={columns}
            rowKey="stockCode"
            size="small"
            pagination={false}
            onRow={(record) => ({ onClick: () => analyze(record.stockCode), style: { cursor: "pointer" } })}
          />
        )}
    </Card>
  );
}
