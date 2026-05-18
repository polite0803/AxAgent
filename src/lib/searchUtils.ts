// i18n-exempt: LLM prompt templates used for search result formatting. Not user-facing UI.
import type { SearchResultItem } from "@/types";

const SEARCH_MARKER_START = "<!-- search:";
const SEARCH_MARKER_END = " -->";
const SEARCH_SEPARATOR = "\n---\n\n";

export interface SearchSourceTag {
  title: string;
  url: string;
  credibility?: "high" | "medium" | "low";
}

export function formatSearchContent(
  results: SearchResultItem[],
  userContent: string,
): string {
  const sourceTags: SearchSourceTag[] = results.map((r) => ({
    title: r.title,
    url: r.url,
    credibility: assessCredibility(r.url),
  }));
  const metadata = JSON.stringify({ sources: sourceTags });

  let block = `${SEARCH_MARKER_START}${metadata}${SEARCH_MARKER_END}\n`;
  block += "以下是与问题相关的网络搜索结果，请参考回答。优先使用高可信度来源，标注来源编号：\n\n";

  results.forEach((r, i) => {
    const cred = assessCredibility(r.url);
    const credLabel = cred === "high" ? " [高可信度]" : cred === "medium" ? " [中可信度]" : "";
    block += `${i + 1}. **${r.title}**${credLabel} - ${r.url}\n   ${r.content}\n\n`;
  });

  return `${block}${SEARCH_SEPARATOR}${userContent}`;
}

function assessCredibility(url: string): "high" | "medium" | "low" {
  const highDomains = [
    "github.com",
    "docs.microsoft.com",
    "developer.mozilla.org",
    "python.org",
    "rust-lang.org",
    "nodejs.org",
    "react.dev",
    "angular.io",
    "vuejs.org",
    "tensorflow.org",
    "pytorch.org",
    "openai.com",
    "anthropic.com",
    "arxiv.org",
    "wikipedia.org",
    "stackoverflow.com",
    "nginx.org",
    "docker.com",
    "kubernetes.io",
  ];
  const mediumDomains = [
    "medium.com",
    "dev.to",
    "hackernoon.com",
    "reddit.com",
    "csdn.net",
    "juejin.cn",
    "zhihu.com",
    "segmentfault.com",
    "infoq.cn",
    "cnblogs.com",
  ];
  try {
    const hostname = new URL(url).hostname.toLowerCase();
    const isMatch = (domain: string) => hostname === domain || hostname.endsWith("." + domain);
    if (highDomains.some(isMatch)) {
      return "high";
    }
    if (mediumDomains.some(isMatch)) {
      return "medium";
    }
  } catch {
    /* invalid URL */
  }
  return "low";
}

export function buildSearchTag(
  status: "searching" | "done" | "error",
  results?: SearchResultItem[],
): string {
  if (status === "searching") {
    return '<web-search status="searching" data-axagent="1"></web-search>';
  }
  if (status === "error") {
    return '<web-search status="error" data-axagent="1"></web-search>';
  }
  const json = JSON.stringify(
    (results ?? []).map((r) => ({
      title: r.title,
      url: r.url,
      content: r.content,
      credibility: assessCredibility(r.url),
    })),
  );
  return `<web-search status="done" data-axagent="1">\n${json}\n</web-search>\n\n`;
}

export function parseSearchContent(content: string): {
  hasSearch: boolean;
  sources: SearchSourceTag[];
  userContent: string;
} {
  if (!content.startsWith(SEARCH_MARKER_START)) {
    return { hasSearch: false, sources: [], userContent: content };
  }

  const markerEndIdx = content.indexOf(SEARCH_MARKER_END);
  if (markerEndIdx === -1) {
    return { hasSearch: false, sources: [], userContent: content };
  }

  const jsonStr = content.substring(SEARCH_MARKER_START.length, markerEndIdx);
  let sources: SearchSourceTag[] = [];
  try {
    const data = JSON.parse(jsonStr);
    sources = data.sources ?? [];
  } catch {}

  const separatorIdx = content.indexOf(SEARCH_SEPARATOR);
  const userContent = separatorIdx !== -1
    ? content.substring(separatorIdx + SEARCH_SEPARATOR.length)
    : content.substring(markerEndIdx + SEARCH_MARKER_END.length);

  return { hasSearch: true, sources, userContent };
}

export function deduplicateResults(
  results: SearchResultItem[],
): SearchResultItem[] {
  const seen = new Set<string>();
  return results.filter((r) => {
    const key = r.url.toLowerCase().replace(/\/+$/, "");
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

export function sortResultsByRelevance(
  results: SearchResultItem[],
  query: string,
): SearchResultItem[] {
  const queryTerms = query
    .toLowerCase()
    .split(/\s+/)
    .filter((w) => w.length > 1);

  return results.toSorted((a, b) => {
    const scoreA = computeRelevanceScore(a, queryTerms);
    const scoreB = computeRelevanceScore(b, queryTerms);
    return scoreB - scoreA;
  });
}

function computeRelevanceScore(
  result: SearchResultItem,
  queryTerms: string[],
): number {
  const titleLower = result.title.toLowerCase();
  const contentLower = result.content.toLowerCase();

  let score = 0;

  // js-set-map-lookups: 子串匹配无法用 Set.has 替代，必须逐 term 扫描
  for (const term of queryTerms) {
    if (titleLower.includes(term)) {
      score += 3;
    }
    if (contentLower.includes(term)) {
      score += 1;
    }
  }

  if (assessCredibility(result.url) === "high") {
    score += 2;
  }
  if (result.content.length > 100) {
    score += 1;
  }

  return score;
}
