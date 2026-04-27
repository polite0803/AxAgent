import type { FileCategory } from "@/types";
import { FileText, Image } from "lucide-react";
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
];
