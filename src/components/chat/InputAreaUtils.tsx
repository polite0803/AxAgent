// SPDX-License-Identifier: AGPL-3.0-only

import type { AttachmentInput } from "@/types";
import { File, FileText, Film, Image as ImageIcon, Music } from "lucide-react";

export type FileTypeCategory = "image" | "video" | "audio" | "document" | "other";

export async function fileToAttachmentInput(file: File): Promise<AttachmentInput> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const base64 = (reader.result as string).split(",")[1] || "";
      resolve({
        file_name: file.name,
        file_type: file.type || "application/octet-stream",
        file_size: file.size,
        data: base64,
      });
    };
    reader.onerror = () => {
      reject(new Error(`Failed to read file: ${file.name}`));
    };
    reader.readAsDataURL(file);
  });
}

export function getFileTypeCategory(mimeType: string): FileTypeCategory {
  if (mimeType.startsWith("image/")) {
    return "image";
  }
  if (mimeType.startsWith("video/")) {
    return "video";
  }
  if (mimeType.startsWith("audio/")) {
    return "audio";
  }
  if (
    mimeType.startsWith("text/")
    || mimeType === "application/pdf"
    || mimeType.includes("document")
    || mimeType.includes("spreadsheet")
    || mimeType.includes("presentation")
    || mimeType.includes("word")
  ) {
    return "document";
  }
  return "other";
}

export function formatFileSize(bytes: number): string {
  if (bytes === 0) {
    return "0 B";
  }
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export function getFileIcon(category: FileTypeCategory) {
  switch (category) {
    case "image":
      return <ImageIcon size={16} />;
    case "video":
      return <Film size={16} />;
    case "audio":
      return <Music size={16} />;
    case "document":
      return <FileText size={16} />;
    default:
      return <File size={16} />;
  }
}
