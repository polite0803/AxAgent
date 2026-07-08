import { invoke } from "@/lib/invoke";
import { Button, Card, Spin, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";

interface BlockItem {
  name: string;
  changePct: number | null;
}

interface ConceptBlocks {
  stockCode: string;
  industry: string;
  concepts: BlockItem[];
  regions: BlockItem[];
}

export function ConceptBlocksPanel({ stockCode }: { stockCode: string }) {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const [data, setData] = useState<ConceptBlocks | null>(null);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);

  const load = async () => {
    if (!stockCode) {
      setData(null);
      setEmptyKind("noStock");
      return;
    }
    setLoading(true);
    setEmptyKind(null);
    try {
      const result = await invoke<ConceptBlocks | null>("get_stock_concept_blocks", { stockCode });
      // 没有行业/概念/地区数据，按"无数据"处理
      if (!result || (result.industry === "未知" && result.concepts.length === 0 && result.regions.length === 0)) {
        setData(null);
        setEmptyKind("noData");
      } else {
        setData(result);
      }
    } catch {
      setData(null);
      setEmptyKind("connectionFailed");
    }
    setLoading(false);
  };

  useEffect(() => {
    let cancelled = false;
    if (!stockCode) {
      Promise.resolve().then(() => {
        setData(null);
        setEmptyKind("noStock");
      });
      return;
    }
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      setEmptyKind(null);
      return invoke<ConceptBlocks | null>("get_stock_concept_blocks", { stockCode });
    })
      .then((result) => {
        if (cancelled || !result) { return; }
        // 没有行业/概念/地区数据，按"无数据"处理
        if (!result || (result.industry === "未知" && result.concepts.length === 0 && result.regions.length === 0)) {
          setData(null);
          setEmptyKind("noData");
        } else {
          setData(result);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setData(null);
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

  const renderBlock = (items: BlockItem[], color: string) => (
    <div className="flex flex-wrap gap-1 mt-1">
      {items.map((b) => (
        <Tag key={b.name} color={color} style={{ fontSize: 11, margin: 0 }}>
          {b.name}
          {b.changePct != null ? ` ${b.changePct >= 0 ? "+" : ""}${b.changePct.toFixed(1)}%` : ""}
        </Tag>
      ))}
    </div>
  );

  return (
    <Card
      size="small"
      title={`🏷️ ${t("stockAnalysis.conceptBlocks")}`}
      styles={{ body: { padding: "8px 10px" } }}
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
            description={emptyKind === "noData" ? t("stockAnalysis.conceptBlocksEmpty") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : data && (
          <div className="space-y-2">
            <div>
              <span className="text-xs font-medium">{t("stockAnalysis.industry")}:</span>
              <Tag color="blue" style={{ marginLeft: 4 }}>{data.industry}</Tag>
            </div>
            {data.concepts.length > 0 && (
              <div>
                <span className="text-xs font-medium">{t("stockAnalysis.concepts")}:</span>
                {renderBlock(data.concepts, "purple")}
              </div>
            )}
            {data.regions.length > 0 && (
              <div>
                <span className="text-xs font-medium">{t("stockAnalysis.regions")}:</span>
                {renderBlock(data.regions, "cyan")}
              </div>
            )}
          </div>
        )}
    </Card>
  );
}
