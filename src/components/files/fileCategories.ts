// SPDX-License-Identifier: AGPL-3.0-only

import type { FileCategory } from "@/types";
import { Archive, FileText, Image } from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type { FileCategory };

export interface FileCategoryMeta {
  id: FileCategory;
  labelKey: string;
  icon: LucideIcon;
}

export const FILE_CATEGORIES: FileCategoryMeta[] = [
  { id: "images", labelKey: "files.images", icon: Image },
  { id: "files", labelKey: "files.files", icon: FileText },
  { id: "backups", labelKey: "files.backups", icon: Archive },
];

// F-P2-6: 当前仅支持三类（images/files/backups）。如需扩展（audio/video/documents/archives/code），
// 需同步：1) @/types FileCategory 类型；2) 本数组；3) 11 种语言 locale 的 files.* 翻译；
// 4) 后端 list_files_by_category 命令的 category 过滤逻辑。
