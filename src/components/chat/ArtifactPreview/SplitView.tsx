import type { ArtifactLanguage } from "@/types/artifact";
import { Segmented } from "antd";
import { memo, useState } from "react";
import { CodePreview } from "./CodePreview";
import { HtmlPreview } from "./HtmlPreview";

interface SplitViewProps {
  code: string;
  language: ArtifactLanguage;
  splitDirection?: "horizontal" | "vertical";
  showPreview?: boolean;
  onChange?: (code: string) => void;
}

export const SplitView = memo(function SplitView({
  code,
  language,
  splitDirection = "horizontal",
  showPreview = true,
  onChange,
}: SplitViewProps) {
  const [activeTab, setActiveTab] = useState<"code" | "preview">(
    showPreview ? "code" : "code",
  );
  const [htmlParts, setHtmlParts] = useState<{ html: string; css: string; js: string }>({
    html: code,
    css: "",
    js: "",
  });

  const isHtml = language === "html" || language === "jsx" || language === "tsx";

  const handleCodeChange = (newCode: string) => {
    onChange?.(newCode);
    if (isHtml) {
      setHtmlParts((prev) => ({ ...prev, html: newCode }));
    }
  };

  const flexDirection = splitDirection === "horizontal" ? "row" : "column";

  return (
    <div style={{ display: "flex", flexDirection, height: "100%" }}>
      <div style={{ flex: 1, display: "flex", flexDirection: "column" }}>
        {showPreview && (
          <div style={{ padding: "4px 8px", borderBottom: "1px solid #eee" }}>
            <Segmented
              size="small"
              value={activeTab}
              onChange={(v) => setActiveTab(v as "code" | "preview")}
              options={[
                { label: "Code", value: "code" },
                { label: "Preview", value: "preview" },
              ]}
            />
          </div>
        )}
        <div style={{ flex: 1 }}>
          <CodePreview
            code={code}
            language={language}
            onChange={handleCodeChange}
          />
        </div>
      </div>

      {activeTab === "preview" && showPreview && isHtml && (
        <div style={{ flex: 1, borderLeft: "1px solid #eee" }}>
          <HtmlPreview
            html={htmlParts.html}
            css={htmlParts.css}
            js={htmlParts.js}
            previewMode="preview"
          />
        </div>
      )}
    </div>
  );
});
