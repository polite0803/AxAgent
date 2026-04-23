// === Nudge System (P0: Self-Evolution) ===

export type NudgeUrgency = 'low' | 'medium' | 'high';

export type NudgeType = 'LowActivity' | 'BestPractice' | 'Improvement' | 'Reminder';

export type NudgeAction = 'AddedToMemory' | 'Dismissed' | 'Pending';

export interface Nudge {
  id: string;
  entityId: string;
  entityName: string;
  reason: string;
  urgency: NudgeUrgency;
  suggestedAction: string | null;
  presented: boolean;
  actionTaken: NudgeAction | null;
  createdAt: number;
  presentedAt: number | null;
  dismissedAt: number | null;
  snoozedUntil: number | null;
  recurrenceCount: number;
  lastRecurrenceAt: number | null;
}

export interface NudgeStats {
  totalNudges: number;
  presentedCount: number;
  addedToMemoryCount: number;
  dismissedCount: number;
  pendingCount: number;
  acceptanceRate: number;
}

// Closed-loop periodic nudge types
export type ClosedLoopNudgeType = 'memory_consolidation' | 'skill_creation' | 'pattern_learn' | 'review_reminder';

export interface PeriodicNudge {
  id: string;
  nudgeType: ClosedLoopNudgeType;
  title: string;
  description: string;
  suggestedAction: string;
  urgency: string;
  autoAction: { actionType: string; target: string } | null;
  createdAt: number;
  acknowledged: boolean;
}

// === Learning Insights (P3: Memory Flush) ===

export type InsightCategory = 'pattern' | 'preference' | 'improvement' | 'warning';

export interface LearningInsight {
  id: string;
  category: InsightCategory;
  title: string;
  description: string;
  confidence: number;
  evidence: string[];
  suggestedAction: string | null;
  createdAt: number;
}

export type FeedbackType = 'success' | 'failure' | 'partial' | 'correction';
export type FeedbackSource = 'user' | 'system' | 'self';
