import { invoke } from "@/lib/invoke";
import { Button, Card, Spin, Table, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";

interface Announcement {
  title: string;
  stockCode: string;
  stockName: string | null;
  announceDate: string;
  annType: string | null;
  pdfUrl: string | null;
}

export function AnnouncementsPanel({ stockCode }: { stockCode: string }) {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const [items, setItems] = useState<Announcement[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);

  const load = async () => {
    if (!stockCode) {
      setItems([]);
      setEmptyKind("noStock");
      return;
    }
    setLoading(true);
    setEmptyKind(null);
    try {
      const data = await invoke<Announcement[]>("get_stock_announcements", { stockCode });
      if (Array.isArray(data) && data.length > 0) {
        setItems(data);
      } else {
        setItems([]);
        setEmptyKind("noData");
      }
    } catch {
      setItems([]);
      setEmptyKind("connectionFailed");
    }
    setLoading(false);
  };

  useEffect(() => {
    let cancelled = false;
    if (!stockCode) {
      Promise.resolve().then(() => {
        setItems([]);
        setEmptyKind("noStock");
      });
      return;
    }
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      setEmptyKind(null);
      return invoke<Announcement[]>("get_stock_announcements", { stockCode });
    })
      .then((data) => {
        if (cancelled || !data) { return; }
        if (Array.isArray(data) && data.length > 0) {
          setItems(data);
        } else {
          setItems([]);
          setEmptyKind("noData");
        }
      })
      .catch(() => {
        if (!cancelled) {
          setItems([]);
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
    {
      title: t("stockAnalysis.alert.name"),
      dataIndex: "title",
      key: "title",
      ellipsis: true,
      render: (v: string, r: Announcement) =>
        r.pdfUrl ? <a href={r.pdfUrl} target="_blank" rel="noreferrer">{v}</a> : v,
    },
    { title: t("stockAnalysis.alert.date"), dataIndex: "announceDate", key: "date", width: 100 },
    {
      title: t("stockAnalysis.type"),
      dataIndex: "annType",
      key: "type",
      width: 80,
      render: (v: string | null) => v ? <Tag>{v}</Tag> : "-",
    },
  ];

  return (
    <Card
      size="small"
      title={`📋 ${t("stockAnalysis.announcements")}`}
      styles={{ body: { padding: 0 } }}
      extra={
        <Button size="small" loading={loading} onClick={load}>
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
            description={emptyKind === "noData" ? t("stockAnalysis.announcementsEmpty") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : (
          <Table
            dataSource={items}
            columns={columns}
            rowKey={(r) => r.title + r.announceDate}
            size="small"
            pagination={{ pageSize: 5 }}
          />
        )}
    </Card>
  );
}
