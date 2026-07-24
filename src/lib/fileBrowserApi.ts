// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";

/** 目录条目（与后端 `DirEntry` 对齐，camelCase 序列化） */
export interface DirEntry {
  name: string;
  path: string;
  isDir: boolean;
  /** 文件大小（字节）；目录为 undefined */
  size?: number;
  /** 修改时间（UNIX 秒） */
  modified?: number;
}

/** 文件详细信息（与后端 `FileInfo` 对齐） */
export interface FileInfo {
  name: string;
  path: string;
  isDir: boolean;
  size?: number;
  modified?: number;
  extension?: string;
}

/**
 * 列出指定目录下的文件和文件夹（按名称排序，目录优先）。
 *
 * 后端会拒绝包含 `..` 的路径，并规范化路径。
 */
export function listDirectory(path: string): Promise<DirEntry[]> {
  return invoke<DirEntry[]>("list_directory", { path });
}

/**
 * 重命名文件或文件夹（仅修改最后一段名称，不允许跨目录移动）。
 *
 * `newName` 不允许包含路径分隔符或 `..`。
 */
export function renameEntry(oldPath: string, newName: string): Promise<void> {
  return invoke<void>("rename_entry", { oldPath, newName });
}

/** 移动文件/文件夹到目标目录。 */
export function moveEntry(srcPath: string, dstDir: string): Promise<void> {
  return invoke<void>("move_entry", { srcPath, dstDir });
}

/** 创建目录（含父目录）。 */
export function createDirectory(path: string): Promise<void> {
  return invoke<void>("create_directory", { path });
}

/** 删除文件或目录（recursive=true 时递归删除目录）。 */
export function deleteEntry(path: string, recursive: boolean): Promise<void> {
  return invoke<void>("delete_entry", { path, recursive });
}

/** 获取文件/目录的详细信息。 */
export function getFileInfo(path: string): Promise<FileInfo> {
  return invoke<FileInfo>("get_file_info", { path });
}

/**
 * 读取文本文件内容用于预览（限制 100KB，超出截断）。
 *
 * 仅用于文本类文件预览，非 UTF-8 文件会抛错。
 */
export function readTextFile(path: string): Promise<string> {
  return invoke<string>("read_text_file", { path });
}

/** 返回默认的文件浏览器根目录（documents_root）。 */
export function getDocumentsRoot(): Promise<string> {
  return invoke<string>("get_documents_root");
}
