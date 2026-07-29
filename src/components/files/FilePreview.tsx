// SPDX-License-Identifier: AGPL-3.0-only

import { type FileInfo, getFileInfo, readTextFile } from "@/lib/fileBrowserApi";
import { invoke, logIpcError } from "@/lib/invoke";
import { Empty, Spin, theme, Typography } from "antd";
import { File as FileIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 可作为文本预览的扩展名集合 */
const TEXT_EXTENSIONS = new Set([
  "txt",
  "md",
  "markdown",
  "json",
  "yaml",
  "yml",
  "toml",
  "xml",
  "html",
  "htm",
  "css",
  "scss",
  "less",
  "rs",
  "ts",
  "tsx",
  "js",
  "jsx",
  "mjs",
  "cjs",
  "py",
  "go",
  "java",
  "kt",
  "swift",
  "c",
  "h",
  "cpp",
  "hpp",
  "cc",
  "cs",
  "rb",
  "php",
  "sh",
  "bash",
  "zsh",
  "fish",
  "ps1",
  "bat",
  "cmd",
  "sql",
  "graphql",
  "proto",
  "ini",
  "cfg",
  "conf",
  "log",
  "csv",
  "tsv",
  "env",
  "vue",
  "svelte",
]);

/** 可作为图片预览的扩展名集合 */
const IMAGE_EXTENSIONS = new Set([
  "png",
  "jpg",
  "jpeg",
  "gif",
  "webp",
  "svg",
  "bmp",
  "ico",
]);

function getExtension(path: string): string {
  const base = path.split(/[\\/]/).pop() ?? "";
  const idx = base.lastIndexOf(".");
  if (idx < 0) { return ""; }
  return base.slice(idx + 1).toLowerCase();
}

function formatSize(bytes?: number): string {
  if (bytes == null) { return "—"; }
  if (bytes === 0) { return "0 B"; }
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  return `${(bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0)} ${units[i]}`;
}

function formatTime(ts?: number): string {
  if (ts == null || ts <= 0) { return "—"; }
  try {
    return new Date(ts * 1000).toLocaleString();
  } catch {
    return "—";
  }
}

interface FilePreviewProps {
  /** 待预览的文件路径；为 null 时显示空状态 */
  path: string | null;
}

export function FilePreview({ path }: FilePreviewProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  const [loading, setLoading] = useState(false);
  const [info, setInfo] = useState<FileInfo | null>(null);
  const [text, setText] = useState<string | null>(null);
  // 图片预览的 base64 data URL（通过后端命令读取，避免 file:// 协议被 CSP 拦截 + Windows 路径格式问题）
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [imageError, setImageError] = useState(false);
  const [errorText, setErrorText] = useState<string | null>(null);

  useEffect(() => {
    setInfo(null);
    setText(null);
    setImageSrc(null);
    setImageError(false);
    setErrorText(null);
    if (!path) { return; }
    setLoading(true);
    let cancelled = false;
    (async () => {
      try {
        const fi = await getFileInfo(path);
        if (cancelled) { return; }
        setInfo(fi);
        if (fi.isDir) { return; }
        const ext = getExtension(path);
        if (TEXT_EXTENSIONS.has(ext)) {
          try {
            const content = await readTextFile(path);
            if (!cancelled) { setText(content); }
          } catch {
            // 文本读取失败不致命，回退到信息展示
          }
        } else if (IMAGE_EXTENSIONS.has(ext)) {
          // 通过后端命令读取 base64 data URL，规避 CSP 拦截 + Windows file:// 路径格式问题
          try {
            const dataUrl = await invoke<string>("read_attachment_preview", { filePath: path });
            if (!cancelled) { setImageSrc(dataUrl); }
          } catch (e) {
            logIpcError("FilePreview read_attachment_preview")(e);
            if (!cancelled) { setImageError(true); }
          }
        }
      } catch (e) {
        if (!cancelled) {
          const msg = e instanceof Error ? e.message : String(e);
          setErrorText(msg);
        }
      } finally {
        if (!cancelled) { setLoading(false); }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [path]);

  if (!path) {
    return (
      <div className="h-full flex items-center justify-center p-6">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={t("files.previewEmpty")}
        />
      </div>
    );
  }
  if (loading) {
    return (
      <div className="h-full flex items-center justify-center p-6">
        <Spin size="small" />
      </div>
    );
  }
  if (errorText || !info) {
    return (
      <div className="h-full flex items-center justify-center p-6">
        <Empty
          image={Empty.PRESENTED_IMAGE_SIMPLE}
          description={errorText ?? t("files.previewFailed")}
        />
      </div>
    );
  }

  const ext = getExtension(path);
  const isImage = !info.isDir && IMAGE_EXTENSIONS.has(ext);
  // 图片 src 优先使用后端读取的 base64 data URL；imageError 时回退到图标占位
  const showImage = isImage && !imageError && imageSrc !== null;
  const showImagePlaceholder = isImage && (imageError || imageSrc === null);
  const showText = text !== null;
  const typeLabel = info.isDir
    ? t("files.previewDirType")
    : (ext || t("files.previewUnknownType"));

  return (
    <div
      className="h-full overflow-auto p-4 flex flex-col gap-3"
      data-testid="file-preview"
    >
      {/* 标题 */}
      <div className="flex items-center gap-2">
        <FileIcon size={16} style={{ color: token.colorPrimary, flexShrink: 0 }} />
        <Text strong className="truncate" title={info.name}>
          {info.name}
        </Text>
      </div>

      {/* 图片预览（base64 data URL，规避 CSP 拦截 + Windows 路径格式问题） */}
      {showImage && (
        <div className="flex justify-center">
          <img
            src={imageSrc ?? undefined}
            alt={info.name}
            onError={() => setImageError(true)}
            style={{
              maxWidth: "100%",
              borderRadius: 8,
              border: `1px solid ${token.colorBorderSecondary}`,
            }}
          />
        </div>
      )}
      {showImagePlaceholder && (
        <div
          className="flex flex-col items-center justify-center gap-2 py-8"
          style={{
            color: token.colorTextTertiary,
            backgroundColor: token.colorFillQuaternary,
            borderRadius: 8,
            border: `1px solid ${token.colorBorderSecondary}`,
          }}
        >
          <FileIcon size={32} style={{ color: token.colorTextTertiary }} />
          <span className="text-xs">
            {imageError ? t("files.previewImageFailed") : t("files.previewLoading")}
          </span>
        </div>
      )}

      {/* 文本预览 */}
      {showText && (
        <pre
          className="text-xs p-3 rounded-md overflow-auto"
          style={{
            backgroundColor: token.colorFillQuaternary,
            color: token.colorText,
            maxHeight: 400,
            whiteSpace: "pre-wrap",
            wordBreak: "break-word",
          }}
        >
          {text}
        </pre>
      )}

      {/* 文件信息 */}
      <div
        className="text-xs flex flex-col gap-1"
        style={{ color: token.colorTextSecondary }}
      >
        <div>
          <span style={{ color: token.colorTextTertiary }}>
            {t("files.previewSize")}:
          </span>{" "}
          {formatSize(info.size)}
        </div>
        <div>
          <span style={{ color: token.colorTextTertiary }}>
            {t("files.previewType")}:
          </span>{" "}
          {typeLabel}
        </div>
        <div>
          <span style={{ color: token.colorTextTertiary }}>
            {t("files.previewModified")}:
          </span>{" "}
          {formatTime(info.modified)}
        </div>
        <div className="truncate">
          <span style={{ color: token.colorTextTertiary }}>
            {t("files.previewPath")}:
          </span>{" "}
          <span title={info.path}>{info.path}</span>
        </div>
      </div>
    </div>
  );
}
