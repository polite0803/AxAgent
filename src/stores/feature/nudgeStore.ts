import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import type { Nudge, NudgeStats, PeriodicNudge, LearningInsight, FeedbackType, FeedbackSource } from '@/types/nudge';

interface NudgeStore {
  // Pending nudges for the current session
  pendingNudges: Nudge[];
  // Closed-loop periodic nudges
  closedLoopNudges: PeriodicNudge[];
  // Nudge statistics
  stats: NudgeStats | null;
  // Learning insights (P3)
  insights: LearningInsight[];
  // Loading state
  isLoading: boolean;

  // Actions
  fetchPendingNudges: (sessionId: string) => Promise<void>;
  fetchClosedLoopNudges: () => Promise<void>;
  fetchStats: () => Promise<void>;
  dismissNudge: (nudgeId: string) => Promise<void>;
  snoozeNudge: (nudgeId: string, untilMs: number) => Promise<void>;
  executeNudge: (nudgeId: string) => Promise<void>;
  acknowledgeClosedLoopNudge: (nudgeId: string) => Promise<void>;
  // P3: Insight & Memory Flush actions
  fetchInsights: () => Promise<void>;
  fetchInsightsByCategory: (category: string) => Promise<LearningInsight[]>;
  generateInsightReport: (sessionId: string, messageCount?: number) => Promise<void>;
  memoryFlush: (content: string, target?: string, category?: string) => Promise<void>;
  recordFeedback: (feedbackType: FeedbackType, source: FeedbackSource, content: string) => Promise<void>;
  clearSession: () => void;
}

export const useNudgeStore = create<NudgeStore>((set, get) => ({
  pendingNudges: [],
  closedLoopNudges: [],
  stats: null,
  insights: [],
  isLoading: false,

  fetchPendingNudges: async (sessionId: string) => {
    try {
      const nudges = await invoke<Nudge[]>('nudge_list', { sessionId });
      set({ pendingNudges: nudges });
    } catch (e) {
      console.warn('[nudgeStore] Failed to fetch pending nudges:', e);
    }
  },

  fetchClosedLoopNudges: async () => {
    try {
      const nudges = await invoke<PeriodicNudge[]>('nudge_closed_loop_list');
      set({ closedLoopNudges: nudges });
    } catch (e) {
      console.warn('[nudgeStore] Failed to fetch closed-loop nudges:', e);
    }
  },

  fetchStats: async () => {
    try {
      const stats = await invoke<NudgeStats>('nudge_stats');
      set({ stats });
    } catch (e) {
      console.warn('[nudgeStore] Failed to fetch nudge stats:', e);
    }
  },

  dismissNudge: async (nudgeId: string) => {
    try {
      await invoke<boolean>('nudge_dismiss', { nudgeId });
      set((state) => ({
        pendingNudges: state.pendingNudges.filter((n) => n.id !== nudgeId),
      }));
    } catch (e) {
      console.warn('[nudgeStore] Failed to dismiss nudge:', e);
    }
  },

  snoozeNudge: async (nudgeId: string, untilMs: number) => {
    try {
      await invoke<boolean>('nudge_snooze', { nudgeId, until: untilMs });
      // Remove from pending list (will reappear after snooze expires)
      set((state) => ({
        pendingNudges: state.pendingNudges.filter((n) => n.id !== nudgeId),
      }));
    } catch (e) {
      console.warn('[nudgeStore] Failed to snooze nudge:', e);
    }
  },

  executeNudge: async (nudgeId: string) => {
    try {
      await invoke<boolean>('nudge_execute', { nudgeId });
      set((state) => ({
        pendingNudges: state.pendingNudges.filter((n) => n.id !== nudgeId),
      }));
    } catch (e) {
      console.warn('[nudgeStore] Failed to execute nudge:', e);
    }
  },

  acknowledgeClosedLoopNudge: async (nudgeId: string) => {
    try {
      await invoke<void>('nudge_closed_loop_acknowledge', { nudgeId });
      set((state) => ({
        closedLoopNudges: state.closedLoopNudges.map((n) =>
          n.id === nudgeId ? { ...n, acknowledged: true } : n
        ),
      }));
    } catch (e) {
      console.warn('[nudgeStore] Failed to acknowledge closed-loop nudge:', e);
    }
  },

  clearSession: () => {
    set({ pendingNudges: [], closedLoopNudges: [], stats: null, insights: [] });
  },

  // P3: Insight & Memory Flush actions
  fetchInsights: async () => {
    try {
      const insights = await invoke<LearningInsight[]>('insight_list');
      set({ insights });
    } catch (e) {
      console.warn('[nudgeStore] Failed to fetch insights:', e);
    }
  },

  fetchInsightsByCategory: async (category: string) => {
    try {
      return await invoke<LearningInsight[]>('insight_get_by_category', { category });
    } catch (e) {
      console.warn('[nudgeStore] Failed to fetch insights by category:', e);
      return [];
    }
  },

  generateInsightReport: async (sessionId: string, messageCount?: number) => {
    try {
      await invoke('insight_report', { sessionId, messageCount });
      // Refresh insights after report generation
      await get().fetchInsights();
    } catch (e) {
      console.warn('[nudgeStore] Failed to generate insight report:', e);
    }
  },

  memoryFlush: async (content: string, target?: string, category?: string) => {
    try {
      await invoke('memory_flush', { content, target, category });
    } catch (e) {
      console.warn('[nudgeStore] Failed to flush memory:', e);
    }
  },

  recordFeedback: async (feedbackType: FeedbackType, source: FeedbackSource, content: string) => {
    try {
      await invoke('record_feedback', { feedbackType, source, content });
    } catch (e) {
      console.warn('[nudgeStore] Failed to record feedback:', e);
    }
  },
}));
