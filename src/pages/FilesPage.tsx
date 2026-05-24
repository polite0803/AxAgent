import type { FileCategory } from "@/components/files/fileCategories";
import { FilesSidebar } from "@/components/files/FilesSidebar";
import { useState } from "react";

export function FilesPage() {
  const [category, setCategory] = useState<FileCategory>("images");

  return (
    <div className="fl-layout">
      <div className="fl-sidebar">
        <div className="fl-sidebar-title">Files</div>
        <FilesSidebar activeCategory={category} onSelect={setCategory} />
      </div>
      <div className="fl-body">
        <div className="fl-empty">
          <div className="fl-empty-text">{category} — coming soon</div>
        </div>
      </div>
    </div>
  );
}
