// SPDX-License-Identifier: AGPL-3.0-only

import type { FileCategory } from "@/components/files/fileCategories";
import { FilesContent } from "@/components/files/FilesContent";
import { FilesSidebar } from "@/components/files/FilesSidebar";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function FilesPage() {
  const { t } = useTranslation();
  const [category, setCategory] = useState<FileCategory>("images");

  return (
    <div className="fl-layout">
      <div className="fl-sidebar">
        <div className="fl-sidebar-title">{t("appHeader.filesContext")}</div>
        <FilesSidebar activeCategory={category} onSelect={setCategory} />
      </div>
      <div className="fl-body">
        <FilesContent activeCategory={category} />
      </div>
    </div>
  );
}
