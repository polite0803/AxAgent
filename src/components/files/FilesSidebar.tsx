import { useTranslation } from "react-i18next";
import { FILE_CATEGORIES, type FileCategory } from "./fileCategories";

interface FilesSidebarProps {
  activeCategory: FileCategory;
  onSelect: (category: FileCategory) => void;
}

export function FilesSidebar({ activeCategory, onSelect }: FilesSidebarProps) {
  const { t } = useTranslation();

  return (
    <nav className="settings-menu" data-testid="files-sidebar">
      {FILE_CATEGORIES.map(({ id, labelKey, icon: Icon }) => (
        <button
          key={id}
          className={`settings-menu-item${activeCategory === id ? " active" : ""}`}
          onClick={() => onSelect(id)}
        >
          <span className="settings-menu-item-icon">
            <Icon size={16} />
          </span>
          <span className="settings-menu-item-label">{t(labelKey)}</span>
        </button>
      ))}
    </nav>
  );
}
