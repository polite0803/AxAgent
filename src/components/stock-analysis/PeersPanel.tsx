import { invoke } from "@/lib/invoke";
import { useStockAnalysisStore } from "@/stores";
import { Button, Card, Spin, Table } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";

interface PeerComparison {
  stockCode: string;
  stockName: string;
  pe: number | null;
  pb: number | null;
  roe: number | null;
  changePct: number;
  marketCap: number | null;
}

export function PeersPanel() {
  const { t } = useTranslation();
  const stockCode = useStockAnalysisStore((s) => s.stockCode);
  const { openDataSourceSettings } = useStockAnalysisPage();
  const [peers, setPeers] = useState<PeerComparison[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);

  const load = async () => {
    if (!stockCode) {
      setPeers([]);
      setEmptyKind("noStock");
      return;
    }
    setLoading(true);
    setEmptyKind(null);
    try {
      const data = await invoke<PeerComparison[]>("get_stock_peers", { stockCode });
      if (Array.isArray(data) && data.length > 0) {
        setPeers(data);
      } else {
        setPeers([]);
        setEmptyKind("noData");
      }
    } catch {
      setPeers([]);
      setEmptyKind("connectionFailed");
    }
    setLoading(false);
  };

  useEffect(() => {
    let cancelled = false;
    if (!stockCode) {
      Promise.resolve().then(() => {
        setPeers([]);
        setEmptyKind("noStock");
      });
      return;
    }
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      setEmptyKind(null);
      return invoke<PeerComparison[]>("get_stock_peers", { stockCode });
    })
      .then((data) => {
        if (cancelled || !data) { return; }
        if (Array.isArray(data) && data.length > 0) {
          setPeers(data);
        } else {
          setPeers([]);
          setEmptyKind("noData");
        }
      })
      .catch(() => {
        if (!cancelled) {
          setPeers([]);
          setEmptyKind("connectionFailed");
        }
      })
      .finally(() => {
        if (!cancelled) { setLoading(false); }
      });
    return () => {
      cancelled = true;
    };
  }, [stockCode]);

  const columns = [
    { title: t("stockAnalysis.alert.code"), dataIndex: "stockCode", key: "code", width: 70 },
    { title: t("stockAnalysis.alert.name"), dataIndex: "stockName", key: "name", width: 80 },
    {
      title: "PE",
      dataIndex: "pe",
      key: "pe",
      width: 60,
      render: (v: number | null) => v != null ? v.toFixed(1) : "-",
    },
    {
      title: "PB",
      dataIndex: "pb",
      key: "pb",
      width: 60,
      render: (v: number | null) => v != null ? v.toFixed(1) : "-",
    },
    {
      title: "ROE",
      dataIndex: "roe",
      key: "roe",
      width: 60,
      render: (v: number | null) => v != null ? `${v.toFixed(1)}%` : "-",
    },
    {
      title: t("stockAnalysis.change"),
      dataIndex: "changePct",
      key: "change",
      width: 70,
      render: (v: number | undefined) => {
        if (v == null) { return <span>-</span>; }
        const color = v >= 0 ? "var(--sa-red)" : "var(--sa-green)";
        return <span style={{ color, fontWeight: "bold" }}>{v >= 0 ? "+" : ""}{v.toFixed(2)}%</span>;
      },
    },
  ];

  return (
    <Card
      size="small"
      title={`🏢 ${t("stockAnalysis.peers")}`}
      styles={{ body: { padding: 0 } }}
      extra={
        <Button size="small" loading={loading} onClick={load}>{t("stockAnalysis.settings.panels.refresh")}</Button>
      }
    >
      {loading
        ? <Spin size="small" style={{ display: "block", margin: "16px auto" }} />
        : emptyKind
        ? (
          <PanelEmpty
            kind={emptyKind}
            description={emptyKind === "noData" ? t("stockAnalysis.peersEmpty") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : <Table dataSource={peers} columns={columns} rowKey="stockCode" size="small" pagination={false} />}
    </Card>
  );
}
