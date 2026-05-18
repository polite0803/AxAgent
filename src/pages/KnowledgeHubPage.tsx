import { SourceManager } from "@/components/settings/SourceManager";
import { theme } from "antd";
import { useTranslation } from "react-i18next";

export function KnowledgeHubPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <div className="h-full flex flex-col" style={{ overflow: "hidden" }}>
      <div
        className="px-4 pt-3 pb-2 border-b flex items-center gap-2"
        style={{
          borderColor: token.colorBorder,
          backgroundColor: token.colorBgContainer,
        }}
      >
        <span
          style={{
            fontSize: token.fontSizeLG,
            fontWeight: token.fontWeightStrong,
          }}
        >
          {t("nav.knowledge")}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto">
        <SourceManager />
      </div>
    </div>
  );
}
