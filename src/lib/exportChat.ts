import { stripAxAgentTags } from "@/lib/chatMarkdown";
import { isTauri } from "@/lib/invoke";
import { formatExportAsync } from "@/lib/workers";
import type { Message } from "@/types";

function browserDownload(filename: string, content: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

async function saveFile(
  defaultName: string,
  content: string | Uint8Array,
  filters: { name: string; extensions: string[] }[],
) {
  if (isTauri()) {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { writeTextFile, writeFile } = await import("@tauri-apps/plugin-fs");
    const filePath = await save({ defaultPath: defaultName, filters });
    if (!filePath) { return false; }
    try {
      if (typeof content === "string") {
        await writeTextFile(filePath, content);
      } else {
        await writeFile(filePath, content);
      }
    } catch (e) {
      console.error("Failed to write file:", filePath, e);
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
    URL.revokeObjectURL(url);
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

function getExportMessageContent(message: Message, options?: TranscriptExportOptions) {
  if (options?.includeThinking === false) {
    return stripAxAgentTags(message.content);
  }
  return message.content;
}

export function buildMarkdownTranscript(messages: Message[], title: string, options?: TranscriptExportOptions) {
  const lines: string[] = [`# ${title}`, ""];
  for (const m of messages) {
    const role = m.role === "user" ? "User" : m.role === "system" ? "System" : "Assistant";
    lines.push(`## ${role}`, "", getExportMessageContent(m, options), "", "---", "");
  }
  return lines.join("\n");
}

export function buildTextTranscript(messages: Message[], title: string, options?: TranscriptExportOptions) {
  const lines: string[] = [title, "=".repeat(title.length), ""];
  for (const m of messages) {
    const role = m.role === "user" ? "User" : m.role === "system" ? "System" : "Assistant";
    lines.push(`[${role}]`, "", getExportMessageContent(m, options), "", "---", "");
  }
  return lines.join("\n");
}

export async function exportAsPNG(element: HTMLElement | null, title: string) {
  if (!element) { return false; }
  const { default: html2canvas } = await import("html2canvas");
  const canvas = await html2canvas(element, { useCORS: true, scale: 2, backgroundColor: "#fff" });

  if (isTauri()) {
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
    if (!blob) { return false; }
    const buffer = new Uint8Array(await blob.arrayBuffer());
    return saveFile(`${title}.png`, buffer, [{ name: "PNG Image", extensions: ["png"] }]);
  }

  // Browser fallback
  const link = document.createElement("a");
  link.download = `${title}.png`;
  link.href = canvas.toDataURL("image/png");
  link.click();
  return true;
}

export function buildJsonTranscript(messages: Message[], title: string, options?: TranscriptExportOptions) {
  const data = {
    title,
    exported_at: new Date().toISOString(),
    messages: messages.map((m) => ({
      role: m.role,
      content: getExportMessageContent(m, options),
      ...(options?.includeThinking === false ? {} : { thinking: m.thinking }),
      created_at: m.created_at,
    })),
  };
  return JSON.stringify(data, null, 2);
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

export async function exportAsMarkdown(messages: Message[], title: string, options?: TranscriptExportOptions) {
  return saveFile(`${title}.md`, buildMarkdownTranscript(messages, title, options), [{
    name: "Markdown",
    extensions: ["md"],
  }]);
}

export async function exportAsText(messages: Message[], title: string, options?: TranscriptExportOptions) {
  return saveFile(`${title}.txt`, buildTextTranscript(messages, title, options), [{
    name: "Text",
    extensions: ["txt"],
  }]);
}

export async function exportAsJSON(messages: Message[], title: string, options?: TranscriptExportOptions) {
  return saveFile(`${title}.json`, buildJsonTranscript(messages, title, options), [{
    name: "JSON",
    extensions: ["json"],
  }]);
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
  return saveFile(`${title}.md`, `# ${title}\n\n${content}`, [{
    name: "Markdown",
    extensions: ["md"],
  }]);
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
  return saveFile(`${title}.txt`, fullContent, [{
    name: "Text",
    extensions: ["txt"],
  }]);
}
