import type { FileCategory } from "@/components/files/fileCategories";
import { FilesSidebar } from "@/components/files/FilesSidebar";
import { theme } from "antd";
import { useState } from "react";

export function FilesPage() {
  const { token } = theme.useToken();
  const [category, setCategory] = useState<FileCategory>("images");

  return (
    <div className="h-full flex" style={{ backgroundColor: token.colorBgElevated }}>
      <div style={{ width: 200, flexShrink: 0, borderRight: `1px solid ${token.colorBorderSecondary}` }}>
        <FilesSidebar activeCategory={category} onSelect={setCategory} />
      </div>
      <div
        className="flex-1 flex items-center justify-center"
        style={{ color: token.colorTextSecondary }}
      >
        <span style={{ fontSize: 14 }}>{category} — coming soon</span>
      </div>
    </div>
  );
}
