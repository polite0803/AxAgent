import { invoke } from "@/lib/invoke";
import { Button, Card, Spin, Table } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";

interface IndexQuote {
  code: string;
  name: string;
  price: number;
  preClose: number;
  changePct: number;
  volume: number;
  amount: number;
}

export function IndexQuotesPanel() {
  const { t } = useTranslation();
  const [quotes, setQuotes] = useState<IndexQuote[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);

  const load = async () => {
    setLoading(true);
    setEmptyKind(null);
    try {
      const data = await invoke<IndexQuote[]>("get_index_quotes");
      if (Array.isArray(data) && data.length > 0) {
        setQuotes(data);
      } else {
        setQuotes([]);
        setEmptyKind("noData");
      }
    } catch {
      setQuotes([]);
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
      return invoke<IndexQuote[]>("get_index_quotes");
    })
      .then((data) => {
        if (cancelled || !data) { return; }
        if (Array.isArray(data) && data.length > 0) {
          setQuotes(data);
        } else {
          setQuotes([]);
          setEmptyKind("noData");
        }
      })
      .catch(() => {
        if (!cancelled) {
          setQuotes([]);
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

  const columns = [
    { title: t("stockAnalysis.alert.name"), dataIndex: "name", key: "name", width: 100 },
    {
      title: t("stockAnalysis.price"),
      dataIndex: "price",
      key: "price",
      width: 80,
      render: (v: number | undefined) => v != null ? v.toFixed(2) : "-",
    },
    {
      title: t("stockAnalysis.change"),
      dataIndex: "changePct",
      key: "changePct",
      width: 80,
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
      title={`📈 ${t("stockAnalysis.indexQuotes")}`}
      styles={{ body: { padding: 0 } }}
      extra={
        <Button size="small" loading={loading} onClick={load}>{t("stockAnalysis.settings.panels.refresh")}</Button>
      }
    >
      {loading
        ? <Spin size="small" style={{ display: "block", margin: "16px auto" }} />
        : emptyKind
        ? <PanelEmpty kind={emptyKind} description={t("stockAnalysis.indexQuoteEmpty")} />
        : <Table dataSource={quotes} columns={columns} rowKey="code" size="small" pagination={false} />}
    </Card>
  );
}
