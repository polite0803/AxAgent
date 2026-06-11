// SPDX-License-Identifier: AGPL-3.0-only

import { useTranslation } from "react-i18next";
import { FILE_CATEGORIES, type FileCategory } from "./fileCategories";

interface FilesSidebarProps {
  activeCategory: FileCategory;
  onSelect: (category: FileCategory) => void;
}

export function FilesSidebar({ activeCategory, onSelect }: FilesSidebarProps) {
  const { t } = useTranslation();

  return (
    <nav data-testid="files-sidebar">
      {FILE_CATEGORIES.map(({ id, labelKey, icon: Icon }) => (
        <button
          key={id}
          type="button"
          className={`fl-cat${activeCategory === id ? " active" : ""}`}
          onClick={() => onSelect(id)}
        >
          <Icon size={16} />
          {t(labelKey)}
        </button>
      ))}
    </nav>
  );
}
