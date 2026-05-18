export type CitationSourceType =
  | "web"
  | "academic"
  | "wikipedia"
  | "github"
  | "documentation"
  | "news"
  | "blog"
  | "forum"
  | "unknown";

export interface Citation {
  id: string;
  sourceUrl: string;
  sourceTitle: string;
  sourceType: CitationSourceType;
  credibility: number;
  inReport: boolean;
  accessedAt?: string;
  usedInSection?: string;
}

export interface CitationStatsData {
  total: number;
  inReport: number;
  byType: Partial<Record<CitationSourceType, number>>;
  avgCredibility: number;
}
