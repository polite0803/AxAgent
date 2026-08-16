// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import { stripAxAgentTags } from "@/lib/chatMarkdown";
import { invoke, isTauri, logIpcError } from "@/lib/invoke";
import { formatExportAsync } from "@/lib/workers";
import type { Message } from "@/types";

function browserDownload(filename: string, content: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 10000);
}

async function saveFile(
  defaultName: string,
  content: string | Uint8Array,
  filters: { name: string; extensions: string[] }[],
) {
  if (isTauri()) {
    const [{ save }, { writeTextFile, writeFile }] = await Promise.all([
      import("@tauri-apps/plugin-dialog"),
      import("@tauri-apps/plugin-fs"),
    ]);
    const filePath = await save({ defaultPath: defaultName, filters });
    if (!filePath) {
      return false;
    }
    try {
      if (typeof content === "string") {
        await writeTextFile(filePath, content);
      } else {
        await writeFile(filePath, content);
      }
    } catch (e) {
      logIpcError("exportChat.writeFile")(e);
      throw e;
    }
    return true;
  }
  // Browser fallback
  const mimeType = filters[0]?.extensions[0] === "png" ? "image/png" : "text/plain";
  if (typeof content === "string") {
    browserDownload(defaultName, content, mimeType);
  } else {
    const blob = new Blob([content.buffer as ArrayBuffer], { type: mimeType });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = defaultName;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 10000);
  }
  return true;
}

async function writeToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
    await writeText(text);
  }
}

export interface TranscriptExportOptions {
  includeThinking?: boolean;
}

function getExportMessageContent(
  message: Message,
  options?: TranscriptExportOptions,
) {
  if (options?.includeThinking === false) {
    return stripAxAgentTags(message.content);
  }
  return message.content;
}

export function buildMarkdownTranscript(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
) {
  const lines: string[] = [`# ${title}`, ""];
  for (const m of messages) {
    const role = m.role === "user" ? "User" : m.role === "system" ? "System" : "Assistant";
    lines.push(
      `## ${role}`,
      "",
      getExportMessageContent(m, options),
      "",
      "---",
      "",
    );
  }
  return lines.join("\n");
}

export function buildTextTranscript(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
) {
  const lines: string[] = [title, "=".repeat(title.length), ""];
  for (const m of messages) {
    const role = m.role === "user" ? "User" : m.role === "system" ? "System" : "Assistant";
    lines.push(
      `[${role}]`,
      "",
      getExportMessageContent(m, options),
      "",
      "---",
      "",
    );
  }
  return lines.join("\n");
}

export async function exportAsPNG(element: HTMLElement | null, title: string) {
  if (!element) {
    return false;
  }
  const { default: html2canvas } = await import("html2canvas");
  const canvas = await html2canvas(element, {
    useCORS: true,
    scale: 2,
    backgroundColor: "#fff",
  });

  if (isTauri()) {
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
    if (!blob) {
      return false;
    }
    const buffer = new Uint8Array(await blob.arrayBuffer());
    return saveFile(`${title}.png`, buffer, [
      { name: "PNG Image", extensions: ["png"] },
    ]);
  }

  // Browser fallback
  const link = document.createElement("a");
  link.download = `${title}.png`;
  link.href = canvas.toDataURL("image/png");
  link.click();
  return true;
}

export function buildJsonTranscript(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
) {
  const data = {
    title,
    exported_at: new Date().toISOString(),
    messages: messages.map((m) => ({
      role: m.role,
      content: getExportMessageContent(m, options),
      ...(options?.includeThinking === false ? {} : { thinking: m.thinking }),
      createdAt: m.createdAt,
    })),
  };
  return JSON.stringify(data, null, 2);
}

/** 构建 HTML 格式的对话抄本，带内联 CSS 样式保持聊天气泡外观 */
export function buildHtmlTranscript(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
) {
  const escapedTitle = title
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
  const now = new Date().toISOString().replace("T", " ").substring(0, 19);

  const messageHtml = messages
    .map((m) => {
      const roleLabel = m.role === "user"
        ? i18n.t("chat.role.user")
        : m.role === "system"
        ? i18n.t("chat.role.system")
        : i18n.t("chat.role.assistant");
      const alignClass = m.role === "user"
        ? "user"
        : m.role === "system"
        ? "system"
        : "assistant";
      const escapedContent = getExportMessageContent(m, options)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        // 将代码块转换为 <pre><code> 以保持格式
        .replace(/```(\w*)\n([\s\S]*?)```/g, (_m, lang, code) => {
          const escapedCode = code
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
          return `<pre><code class="language-${lang || "text"}">${escapedCode}</code></pre>`;
        })
        // 行内代码
        .replace(/`([^`]+)`/g, (_m, code: string) => {
          const escapedCode = code
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;");
          return `<code>${escapedCode}</code>`;
        })
        // 换行转 <br>
        .replace(/\n/g, "<br>");

      const time = m.createdAt
        ? new Date(m.createdAt).toLocaleString(i18n.language)
        : "";

      return `<div class="message ${alignClass}">
  <div class="message-header">
    <span class="role">${roleLabel}</span>
    <span class="time">${time}</span>
  </div>
  <div class="message-body">${escapedContent}</div>
</div>`;
    })
    .join("\n");

  return `<!DOCTYPE html>
<html lang="${i18n.language}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>${escapedTitle}</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Noto Sans SC", sans-serif;
    background: #f5f5f5;
    color: #1a1a1a;
    line-height: 1.6;
    padding: 20px;
    max-width: 900px;
    margin: 0 auto;
  }
  .export-header {
    text-align: center;
    padding: 24px 0;
    border-bottom: 2px solid #e0e0e0;
    margin-bottom: 24px;
  }
  .export-header h1 { font-size: 1.6em; color: #1677ff; }
  .export-header .meta { font-size: 0.85em; color: #888; margin-top: 4px; }
  .message {
    margin-bottom: 20px;
    padding: 16px;
    border-radius: 12px;
    max-width: 85%;
  }
  .message.user {
    background: #1677ff;
    color: #fff;
    margin-left: auto;
    margin-right: 0;
  }
  .message.assistant {
    background: #fff;
    border: 1px solid #e8e8e8;
    margin-left: 0;
  }
  .message.system {
    background: #fafafa;
    border: 1px dashed #d9d9d9;
    margin: 0 auto;
    text-align: center;
    max-width: 90%;
    color: #888;
  }
  .message-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 8px;
    font-size: 0.8em;
    opacity: 0.7;
  }
  .message.user .message-header { color: rgba(255,255,255,0.85); }
  .message-body { font-size: 0.95em; word-wrap: break-word; }
  .message-body pre {
    background: rgba(0,0,0,0.06);
    padding: 12px;
    border-radius: 8px;
    overflow-x: auto;
    font-size: 0.85em;
    line-height: 1.5;
    margin: 8px 0;
    white-space: pre-wrap;
  }
  .message.user .message-body pre { background: rgba(255,255,255,0.15); color: #fff; }
  .message-body code {
    background: rgba(0,0,0,0.06);
    padding: 1px 4px;
    border-radius: 4px;
    font-size: 0.9em;
  }
  .message.user .message-body code { background: rgba(255,255,255,0.2); }
  .export-footer {
    text-align: center;
    margin-top: 32px;
    padding: 16px;
    border-top: 1px solid #e0e0e0;
    color: #aaa;
    font-size: 0.8em;
  }
</style>
</head>
<body>
<div class="export-header">
  <h1>${escapedTitle}</h1>
  <div class="meta">${i18n.t("export.exportTime")}: ${now} · ${messages.length} ${i18n.t("export.messageCount")}</div>
</div>
${messageHtml}
<div class="export-footer">${i18n.t("export.exportedBy")}</div>
</body>
</html>`;
}

export async function copyTranscript(
  messages: Message[],
  title: string,
  format: "markdown" | "text",
  options?: TranscriptExportOptions,
) {
  const content = format === "markdown"
    ? buildMarkdownTranscript(messages, title, options)
    : buildTextTranscript(messages, title, options);
  await writeToClipboard(content);
  return true;
}

export async function exportAsMarkdown(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
) {
  return saveFile(
    `${title}.md`,
    buildMarkdownTranscript(messages, title, options),
    [
      {
        name: "Markdown",
        extensions: ["md"],
      },
    ],
  );
}

export async function exportAsText(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
) {
  return saveFile(
    `${title}.txt`,
    buildTextTranscript(messages, title, options),
    [
      {
        name: "Text",
        extensions: ["txt"],
      },
    ],
  );
}

export async function exportAsJSON(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
) {
  return saveFile(
    `${title}.json`,
    buildJsonTranscript(messages, title, options),
    [
      {
        name: "JSON",
        extensions: ["json"],
      },
    ],
  );
}

export async function exportAsHTML(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
) {
  return saveFile(
    `${title}.html`,
    buildHtmlTranscript(messages, title, options),
    [
      {
        name: "HTML",
        extensions: ["html"],
      },
    ],
  );
}

// ── Worker-accelerated exports (P2) ──

/** Export as Markdown using a Web Worker for large conversations (>50 messages). */
export async function exportAsMarkdownAsync(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
): Promise<boolean> {
  if (messages.length < 50) {
    return exportAsMarkdown(messages, title, options);
  }
  const content = await formatExportAsync(
    messages.map((m) => ({
      role: m.role,
      content: getExportMessageContent(m, options),
    })),
    "markdown",
  );
  return saveFile(`${title}.md`, `# ${title}\n\n${content}`, [
    {
      name: "Markdown",
      extensions: ["md"],
    },
  ]);
}

/** Export as Text using a Web Worker for large conversations (>50 messages). */
export async function exportAsTextAsync(
  messages: Message[],
  title: string,
  options?: TranscriptExportOptions,
): Promise<boolean> {
  if (messages.length < 50) {
    return exportAsText(messages, title, options);
  }
  const content = await formatExportAsync(
    messages.map((m) => ({
      role: m.role,
      content: getExportMessageContent(m, options),
    })),
    "text",
  );
  const fullContent = `${title}\n${"=".repeat(title.length)}\n\n${content}`;
  return saveFile(`${title}.txt`, fullContent, [
    {
      name: "Text",
      extensions: ["txt"],
    },
  ]);
}

// ── G18: 通过后端 sessions_export 命令导出 ──────────────────────────────
//
// 后端支持 4 种格式：messages_jsonl / openai_dataset_jsonl / markdown / manifest_json
// 后端会写入临时文件并返回 ExportResult（含 file_path），前端通过 save 对话框下载。

/** 后端 `export_session` 命令的返回类型（camelCase） */
export interface BackendExportResult {
  /** 导出文件路径 */
  filePath: string;
  /** 导出格式 */
  format: string;
  /** 导出的会话数量 */
  conversationCount: number;
  /** 导出的消息总数 */
  messageCount: number;
  /** 文件大小（字节） */
  fileSize: number;
}

/** 后端支持的导出格式 */
export type BackendExportFormat =
  | "messages_jsonl"
  | "openai_dataset_jsonl"
  | "markdown"
  | "manifest_json";

/**
 * 调用后端 `export_session` 命令导出会话。
 *
 * 与前端纯导出函数（exportAsJSON/exportAsMarkdown 等）的区别：
 * - 后端支持 OpenAI 微调数据集 JSONL 格式（前端无法生成）
 * - 后端支持 manifest_json 元数据导出
 * - 后端写入临时文件后由前端 save 对话框下载，避免大文件内存占用
 *
 * @param conversationId 会话 ID
 * @param format 导出格式
 * @returns 成功返回 true，失败抛出异常
 */
export async function exportSessionViaBackend(
  conversationId: string,
  format: BackendExportFormat,
): Promise<boolean> {
  // 浏览器模式下后端命令不可用，回退到前端实现
  if (!isTauri()) {
    throw new Error("Backend export_session requires Tauri environment");
  }

  const result = await invoke<BackendExportResult>("export_session", {
    conversationId,
    format,
    outputDir: null,
  });

  // 后端已写入临时文件，前端通过 save 对话框让用户选择保存位置
  const [{ save }, { copyFile, remove }] = await Promise.all([
    import("@tauri-apps/plugin-dialog"),
    import("@tauri-apps/plugin-fs"),
  ]);

  const extension = format === "messages_jsonl"
    ? "jsonl"
    : format === "openai_dataset_jsonl"
    ? "jsonl"
    : format === "markdown"
    ? "md"
    : "json";
  const defaultName = `${conversationId.substring(0, 8)}_${format}.${extension}`;

  const destPath = await save({
    defaultPath: defaultName,
    filters: [{ name: format, extensions: [extension] }],
  });

  if (!destPath) {
    // 用户取消，清理临时文件
    try {
      await remove(result.filePath);
    } catch {
      // 忽略清理失败
    }
    return false;
  }

  try {
    await copyFile(result.filePath, destPath);
  } catch (e) {
    logIpcError("exportSessionViaBackend.copyFile")(e);
    throw e;
  } finally {
    // 清理后端临时文件
    try {
      await remove(result.filePath);
    } catch {
      // 忽略清理失败
    }
  }

  return true;
}
