// SPDX-License-Identifier: AGPL-3.0-only

import type { Citation, CitationStatsData } from "@/types";
import { create } from "zustand";

interface CitationStore {
  citations: Citation[];
  selectedCitationId: string | null;

  setCitations: (citations: Citation[]) => void;
  addCitation: (citation: Citation) => void;
  removeCitation: (citationId: string) => void;
  toggleInReport: (citationId: string) => void;
  selectCitation: (citationId: string | null) => void;
  clearCitations: () => void;
  getStats: () => CitationStatsData;
}

export const useCitationStore = create<CitationStore>((set, get) => ({
  citations: [],
  selectedCitationId: null,

  setCitations: (citations) => set({ citations }),

  addCitation: (citation) =>
    set((s) => {
      const exists = s.citations.some((c) => c.id === citation.id);
      if (exists) {
        return {
          citations: s.citations.map((c) => c.id === citation.id ? citation : c),
        };
      }
      return { citations: [...s.citations, citation] };
    }),

  removeCitation: (citationId) =>
    set((s) => ({
      citations: s.citations.filter((c) => c.id !== citationId),
      selectedCitationId: s.selectedCitationId === citationId ? null : s.selectedCitationId,
    })),

  toggleInReport: (citationId) =>
    set((s) => ({
      citations: s.citations.map((c) => c.id === citationId ? { ...c, inReport: !c.inReport } : c),
    })),

  selectCitation: (citationId) => set({ selectedCitationId: citationId }),

  clearCitations: () => set({ citations: [], selectedCitationId: null }),

  getStats: () => {
    const citations = get().citations;
    const total = citations.length;
    const inReport = citations.filter((c) => c.inReport).length;
    const byType = citations.reduce<Partial<Record<string, number>>>(
      (acc, c) => {
        acc[c.sourceType] = (acc[c.sourceType] || 0) + 1;
        return acc;
      },
      {},
    );
    const avgCredibility = total > 0
      ? citations.reduce((sum, c) => sum + c.credibility, 0) / total
      : 0;
    return { total, inReport, byType, avgCredibility };
  },
}));
