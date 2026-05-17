export interface MemoryRetrievedItem {
  content: string;
  score: number;
  document_id: string;
  id: string;
  document_name?: string;
}

export interface MemorySourceResult {
  source_type: "knowledge" | "memory" | "wiki";
  container_id: string;
  items: MemoryRetrievedItem[];
}

export interface RagContextRetrievedEvent {
  conversation_id: string;
  sources: MemorySourceResult[];
}

export type MemoryTier = "short_term" | "working" | "long_term" | "core";
export type MemoryNature = "episodic" | "semantic";

export interface MemoryEntry {
  id: string;
  content: string;
  memory_type: string;
  tier: MemoryTier;
  importance: number;
  access_count: number;
  last_accessed: number;
  decay_rate: number;
  created_at: number;
  updated_at: number;
  expires_at: number | null;
  nature: MemoryNature;
  provenance: {
    conversation_id: string | null;
    message_id: string | null;
    extraction_method: string;
  } | null;
  tags: string[];
}

export interface MemoryUsageStats {
  memory_count: number;
  user_count: number;
  total_tokens: number;
  tier_counts: Record<string, number>;
}

/**
 * 记忆层级标签映射（i18n key）。
 * 调用方需用 t() 包装后显示。
 */
export const TIER_LABELS: Record<MemoryTier, string> = {
  short_term: "memoryLabels.tier.shortTerm",
  working: "memoryLabels.tier.working",
  long_term: "memoryLabels.tier.longTerm",
  core: "memoryLabels.tier.core",
};

export const TIER_COLORS: Record<MemoryTier, string> = {
  short_term: "#94a3b8",
  working: "#3b82f6",
  long_term: "#8b5cf6",
  core: "#f59e0b",
};

/**
 * 记忆性质标签映射（i18n key）。
 * 调用方需用 t() 包装后显示。
 */
export const NATURE_LABELS: Record<MemoryNature, string> = {
  episodic: "memoryLabels.nature.episodic",
  semantic: "memoryLabels.nature.semantic",
};

/**
 * 获取记忆层级显示文本的 i18n key。
 * 调用方需用 t() 包装后显示，例如：{t(getTierLabel(tier))}
 */
export function getTierLabel(tier: MemoryTier): string {
  return TIER_LABELS[tier] ?? tier;
}

export function getTierColor(tier: MemoryTier): string {
  return TIER_COLORS[tier] ?? "#6b7280";
}

/**
 * 获取记忆性质显示文本的 i18n key。
 * 调用方需用 t() 包装后显示，例如：{t(getNatureLabel(nature))}
 */
export function getNatureLabel(nature: MemoryNature): string {
  return NATURE_LABELS[nature] ?? nature;
}

/**
 * 返回重要性对应的 i18n key。
 * 调用方需用 t() 包装后显示。
 */
export function formatImportance(importance: number): string {
  if (importance >= 0.9) { return "memoryLabels.importance.critical"; }
  if (importance >= 0.7) { return "memoryLabels.importance.important"; }
  if (importance >= 0.5) { return "memoryLabels.importance.normal"; }
  if (importance >= 0.3) { return "memoryLabels.importance.minor"; }
  return "memoryLabels.importance.low";
}

export function isExpired(entry: MemoryEntry): boolean {
  if (entry.expires_at == null) { return false; }
  let expiresAt = entry.expires_at;
  if (expiresAt > 1e12) { expiresAt = expiresAt / 1000; }
  return Date.now() / 1000 > expiresAt;
}

export function effectiveScore(entry: MemoryEntry): number {
  const now = Math.floor(Date.now() / 1000);
  const hoursElapsed = Math.max(0, now - entry.last_accessed) / 3600;
  const timeDecay = Math.exp(-entry.decay_rate * hoursElapsed);
  const accessBoost = 1.0 + Math.log(1.0 + entry.access_count);
  const tierBonus = { short_term: 0.1, working: 0.2, long_term: 0.3, core: 0.4 }[entry.tier] ?? 0.2;
  return entry.importance * timeDecay * accessBoost + tierBonus;
}

export function buildKnowledgeTag(
  status: "searching" | "done" | "error",
  sources?: MemorySourceResult[],
): string {
  if (status === "searching") {
    return '<knowledge-retrieval status="searching" data-axagent="1"></knowledge-retrieval>';
  }
  if (status === "error") {
    return '<knowledge-retrieval status="error" data-axagent="1"></knowledge-retrieval>';
  }
  const json = JSON.stringify(sources ?? []);
  return `<knowledge-retrieval status="done" data-axagent="1">\n${json}\n</knowledge-retrieval>\n\n`;
}

export function buildMemoryTag(
  status: "searching" | "done" | "error",
  sources?: MemorySourceResult[],
): string {
  if (status === "searching") {
    return '<memory-retrieval status="searching" data-axagent="1"></memory-retrieval>';
  }
  if (status === "error") {
    return '<memory-retrieval status="error" data-axagent="1"></memory-retrieval>';
  }
  const json = JSON.stringify(sources ?? []);
  return `<memory-retrieval status="done" data-axagent="1">\n${json}\n</memory-retrieval>\n\n`;
}

export function buildWikiTag(
  status: "searching" | "done" | "error",
  sources?: MemorySourceResult[],
): string {
  if (status === "searching") {
    return '<wiki-retrieval status="searching" data-axagent="1"></wiki-retrieval>';
  }
  if (status === "error") {
    return '<wiki-retrieval status="error" data-axagent="1"></wiki-retrieval>';
  }
  const json = JSON.stringify(sources ?? []);
  return `<wiki-retrieval status="done" data-axagent="1">\n${json}\n</wiki-retrieval>\n\n`;
}
