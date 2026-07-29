// SPDX-License-Identifier: AGPL-3.0-only

import { FILE_CATEGORIES, type FileCategory } from "@/components/files/fileCategories";
import { FilesContent } from "@/components/files/FilesContent";
import { FilesSidebar } from "@/components/files/FilesSidebar";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function FilesPage() {
  const { t } = useTranslation();
  const [category, setCategory] = useState<FileCategory>(FILE_CATEGORIES[0].id);

  return (
    <div style={{ flex: 1, minHeight: 0, display: "flex", flexDirection: "column" }}>
      <div style={{ display: "flex", flex: 1, minHeight: 0, overflow: "hidden" }}>
        <div
          style={{
            width: 200,
            minWidth: 200,
            borderRight: "1px solid var(--color-border-secondary)",
            display: "flex",
            flexDirection: "column",
            padding: "8px",
            flexShrink: 0,
          }}
        >
          <div
            style={{
              padding: "8px 10px 4px",
              fontSize: 11,
              fontWeight: 600,
              color: "var(--color-text-tertiary)",
              letterSpacing: "0.04em",
              textTransform: "uppercase",
            }}
          >
            {t("appHeader.filesContext")}
          </div>
          <FilesSidebar activeCategory={category} onSelect={setCategory} />
        </div>
        <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden", minWidth: 0 }}>
          <FilesContent activeCategory={category} />
        </div>
      </div>
    </div>
  );
}
