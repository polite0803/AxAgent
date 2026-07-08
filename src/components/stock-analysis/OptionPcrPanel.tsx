import { invoke } from "@/lib/invoke";
import { Button, Card, Spin } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { PanelEmpty, type PanelEmptyKind } from "./PanelEmpty";
import { useStockAnalysisPage } from "./StockAnalysisPageContext";

interface OptionPCR {
  stockCode: string;
  date: string;
  callVolume: number;
  putVolume: number;
  callOi: number;
  putOi: number;
  volumePcr: number;
  oiPcr: number;
}

export function OptionPcrPanel({ stockCode }: { stockCode: string }) {
  const { t } = useTranslation();
  const { openDataSourceSettings } = useStockAnalysisPage();
  const [pcr, setPcr] = useState<OptionPCR | null>(null);
  const [loading, setLoading] = useState(false);
  const [emptyKind, setEmptyKind] = useState<PanelEmptyKind | null>(null);

  const load = async () => {
    if (!stockCode) {
      setPcr(null);
      setEmptyKind("noStock");
      return;
    }
    setLoading(true);
    setEmptyKind(null);
    try {
      const result = await invoke<OptionPCR | null>("get_stock_option_pcr", { stockCode });
      if (result && (result.callVolume > 0 || result.putVolume > 0)) {
        setPcr(result);
      } else {
        setPcr(null);
        setEmptyKind("noData");
      }
    } catch {
      setPcr(null);
      setEmptyKind("connectionFailed");
    }
    setLoading(false);
  };

  useEffect(() => {
    let cancelled = false;
    if (!stockCode) {
      Promise.resolve().then(() => {
        setPcr(null);
        setEmptyKind("noStock");
      });
      return;
    }
    Promise.resolve().then(() => {
      if (cancelled) { return; }
      setLoading(true);
      setEmptyKind(null);
      return invoke<OptionPCR | null>("get_stock_option_pcr", { stockCode });
    })
      .then((result) => {
        if (cancelled || !result) { return; }
        if (result && (result.callVolume > 0 || result.putVolume > 0)) {
          setPcr(result);
        } else {
          setPcr(null);
          setEmptyKind("noData");
        }
      })
      .catch(() => {
        if (!cancelled) {
          setPcr(null);
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

  // PCR > 1 表示看空力量更强（红色）；PCR < 1 表示看多（绿色）
  const pcrColor = (v: number) => v > 1 ? "var(--sa-red)" : "var(--sa-green)";

  return (
    <Card
      size="small"
      title={`📊 ${t("stockAnalysis.optionPcr")}`}
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
            description={emptyKind === "noData" ? t("stockAnalysis.optionsEmpty") : undefined}
            onOpenSettings={openDataSourceSettings}
          />
        )
        : pcr && (
          <div className="space-y-2">
            <div className="text-xs text-gray-500">{pcr.date}</div>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <span className="text-xs">{t("stockAnalysis.optionPcrVolume")}</span>
                <div className="text-lg font-bold" style={{ color: pcrColor(pcr.volumePcr) }}>
                  {pcr.volumePcr.toFixed(2)}
                </div>
              </div>
              <div>
                <span className="text-xs">{t("stockAnalysis.optionPcrOi")}</span>
                <div className="text-lg font-bold" style={{ color: pcrColor(pcr.oiPcr) }}>
                  {pcr.oiPcr.toFixed(2)}
                </div>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-1 text-xs text-gray-500">
              <span>Call Vol: {pcr.callVolume.toLocaleString()}</span>
              <span>Put Vol: {pcr.putVolume.toLocaleString()}</span>
              <span>Call OI: {pcr.callOi.toLocaleString()}</span>
              <span>Put OI: {pcr.putOi.toLocaleString()}</span>
            </div>
          </div>
        )}
    </Card>
  );
}
