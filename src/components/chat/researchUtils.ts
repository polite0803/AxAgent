// SPDX-License-Identifier: AGPL-3.0-only

export interface SearchResult {
  id: string;
  sourceType: string;
  url: string;
  title: string;
  snippet: string;
  credibilityScore: number | null;
  relevanceScore: number;
}

export function getSourceTypeColor(sourceType: string): string {
  const colorMap: Record<string, string> = {
    web: "blue",
    academic: "green",
    wikipedia: "cyan",
    github: "purple",
    documentation: "orange",
    news: "magenta",
    blog: "gold",
  };
  return colorMap[sourceType.toLowerCase()] || "default";
}

export function getSourceTypeName(
  sourceType: string,
  t: (key: string) => string,
): string {
  const nameMap: Record<string, string> = {
    web: t("research.sourceTypeWeb"),
    academic: t("research.sourceTypeAcademic"),
    wikipedia: t("research.sourceTypeWikipedia"),
    github: t("research.sourceTypeGithub"),
    documentation: t("research.sourceTypeDocumentation"),
    news: t("research.sourceTypeNews"),
    blog: t("research.sourceTypeBlog"),
    forum: t("research.sourceTypeForum"),
    unknown: t("research.sourceTypeUnknown"),
  };
  return nameMap[sourceType.toLowerCase()] || sourceType;
}
