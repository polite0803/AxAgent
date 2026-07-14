// SPDX-License-Identifier: AGPL-3.0-only

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
  // 后端 Citation 实际返回（research_state.rs），此前缺失
  quotedText?: string;
  pageNumber?: number;
}

export interface CitationStatsData {
  total: number;
  inReport: number;
  byType: Partial<Record<CitationSourceType, number>>;
  avgCredibility: number;
}
