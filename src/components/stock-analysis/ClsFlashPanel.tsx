import { invoke } from "@/lib/invoke";
import { Button, Card, Spin, Tag, Timeline } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";

interface ClsFlashItem {
  title: string;
  content: string;
  publishTime: string;
  source: string | null;
  url?: string | null;
}

export function ClsFlashPanel() {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const [items, setItems] = useState<ClsFlashItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);

  const load = async () => {
    setLoading(true);
    setEmptyKind(null);
    try {
      const data = await invoke<Record<string, string | null | undefined>[]>("get_cls_flash");
      if (Array.isArray(data) && data.length > 0) {
        // 兼容后端字段名变化：驼峰/下划线都接受
        const normalized: ClsFlashItem[] = data.slice(0, 20).map((d) => ({
          title: d.title ?? "",
          content: d.content ?? d.summary ?? "",
          publishTime: d.publishTime ?? d.publish_time ?? d.time ?? "",
          source: d.source ?? d.source_name ?? null,
          url: d.url ?? d.link ?? null,
        }));
        setItems(normalized);
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
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      setEmptyKind(null);
      return invoke<Record<string, string | null | undefined>[]>("get_cls_flash");
    })
      .then((data) => {
        if (cancelled || !data) { return; }
        if (Array.isArray(data) && data.length > 0) {
          // 兼容后端字段名变化：驼峰/下划线都接受
          const normalized: ClsFlashItem[] = data.slice(0, 20).map((d) => ({
            title: d.title ?? "",
            content: d.content ?? d.summary ?? "",
            publishTime: d.publishTime ?? d.publish_time ?? d.time ?? "",
            source: d.source ?? d.source_name ?? null,
            url: d.url ?? d.link ?? null,
          }));
          setItems(normalized);
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
  }, []);

  return (
    <Card
      size="small"
      title={`⚡ ${t("stockAnalysis.clsFlash")}`}
      styles={{ body: { padding: "8px 10px", maxHeight: 400, overflowY: "auto" } }}
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
            description={emptyKind === "noData" ? t("stockAnalysis.clsFlashEmpty") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : (
          <Timeline
            items={items.map((item) => ({
              children: (
                <div>
                  <div className="text-xs font-medium">{item.title}</div>
                  {item.content && <div className="text-xs text-gray-500 mt-1">{item.content}</div>}
                  <div className="text-xs text-gray-400 mt-1">
                    {item.publishTime}
                    {item.source && <Tag style={{ marginLeft: 4, fontSize: 10 }}>{item.source}</Tag>}
                  </div>
                </div>
              ),
            }))}
          />
        )}
    </Card>
  );
}
