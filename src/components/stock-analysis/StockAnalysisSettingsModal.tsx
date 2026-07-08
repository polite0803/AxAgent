import { StockAnalysisSettings } from "@/components/settings/StockAnalysisSettings";
import { Drawer } from "antd";
import { useTranslation } from "react-i18next";

export function StockAnalysisSettingsModal(
  { open, onClose, defaultTab }: { open: boolean; onClose: () => void; defaultTab?: string },
) {
  const { t } = useTranslation();

  return (
    <Drawer
      title={t("stockAnalysis.settings.title")}
      placement="right"
      width={900}
      rootClassName="sacp-drawer"
      open={open}
      onClose={onClose}
    >
      <StockAnalysisSettings key={defaultTab ?? "default"} defaultTab={defaultTab} />
    </Drawer>
  );
}
