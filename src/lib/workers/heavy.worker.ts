// Web Worker for CPU-intensive operations.
//
// Offloads token estimation and large text processing from the main thread
// to keep the UI responsive during long conversations and exports.
//
// Message protocol:
//   { type: "estimateTokens", payload: { text: string } } → { tokens: number }
//   { type: "processMarkdown", payload: { text: string, stripTags: boolean } } → { result: string }
//   { type: "formatExport", payload: { messages: Message[], format: string } } → { result: string }

import type { Message } from "@/types";

interface WorkerRequest {
  id: number;
  type: "estimateTokens" | "processMarkdown" | "formatExport";
  payload: unknown;
}

interface WorkerResponse {
  id: number;
  type: string;
  result?: unknown;
  error?: string;
}

// ─── Token estimation (replicated from tokenEstimator.ts for worker context) ───

function estimateTokens(text: string): number {
  if (!text) return 0;
  // Simple heuristic: character count / 4, with adjustments
  let count = 0;
  for (const ch of text) {
    const code = ch.charCodeAt(0);
    if (code <= 0x7F) {
      count += 0.25; // ASCII
    } else if (code <= 0x7FF) {
      count += 0.5; // Latin extended
    } else if (ch.match(/[\u4e00-\u9fff\u3400-\u4dbf]/)) {
      count += 1.5; // CJK characters (1-2 tokens each)
    } else {
      count += 1.0; // Other Unicode
    }
  }
  // Add per-message overhead
  count += 4; // role encoding
  return Math.max(1, Math.ceil(count));
}

// ─── Markdown processing ───

const AXAGENT_TAG_REGEX = /<think\s+data-axagent=.+?<\/think>|<\/?web-search[^>]*>/gs;

function processMarkdown(text: string, stripTags: boolean): string {
  if (!stripTags) return text;
  return text.replace(AXAGENT_TAG_REGEX, "").trim();
}

// ─── Export formatting ───

function formatExportMarkdown(messages: Message[]): string {
  const lines: string[] = ["# Chat Export\n"];
  for (const msg of messages) {
    const role = msg.role === "user" ? "User" : msg.role === "assistant" ? "Assistant" : msg.role;
    const content = msg.content.replace(AXAGENT_TAG_REGEX, "").trim();
    lines.push(`## ${role}\n\n${content}\n`);
  }
  return lines.join("\n");
}

function formatExportText(messages: Message[]): string {
  const lines: string[] = [];
  for (const msg of messages) {
    const role = msg.role === "user" ? "User" : msg.role === "assistant" ? "Assistant" : msg.role;
    const content = msg.content.replace(AXAGENT_TAG_REGEX, "").trim();
    lines.push(`[${role}] ${content}`);
  }
  return lines.join("\n\n");
}

// ─── Message handler ───

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const { id, type, payload } = event.data;

  try {
    let result: unknown;

    switch (type) {
      case "estimateTokens": {
        const { text } = payload as { text: string };
        result = { tokens: estimateTokens(text || "") };
        break;
      }
      case "processMarkdown": {
        const { text, stripTags } = payload as { text: string; stripTags: boolean };
        result = { result: processMarkdown(text || "", stripTags) };
        break;
      }
      case "formatExport": {
        const { messages, format } = payload as { messages: Message[]; format: string };
        if (format === "markdown") {
          result = { result: formatExportMarkdown(messages) };
        } else {
          result = { result: formatExportText(messages) };
        }
        break;
      }
      default:
        throw new Error(`Unknown worker task type: ${type}`);
    }

    const response: WorkerResponse = { id, type, result };
    self.postMessage(response);
  } catch (e) {
    const response: WorkerResponse = { id, type, error: String(e) };
    self.postMessage(response);
  }
};
