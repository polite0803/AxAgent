// SPDX-License-Identifier: AGPL-3.0-only

import { FILE_CATEGORIES, type FileCategory } from "@/components/files/fileCategories";
import { FilesContent } from "@/components/files/FilesContent";
import { FilesSidebar } from "@/components/files/FilesSidebar";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function FilesPage() {
  const { t } = useTranslation();
  // F-P1-5: 默认分类从 FILE_CATEGORIES 派生，避免硬编码 "images" 与数组顺序脱钩
  const [category, setCategory] = useState<FileCategory>(FILE_CATEGORIES[0].id);

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
