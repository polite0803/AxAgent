// SPDX-License-Identifier: AGPL-3.0-only

// === 主动建议 / 上下文预测 / 提醒 / 预取 类型定义 ===
//
// 与后端 DTO 对齐：后端 struct 字段保持 snake_case，通过
// `#[serde(rename_all = "camelCase")]` 输出 camelCase，前端消费 camelCase。
// 枚举值字面量（如 "low"、"daily"、"FileOpened" 等）保持不变。

export type CapabilityType =
  | "ContextPrediction"
  | "ProactiveSuggestion"
  | "TaskPrefetch"
  | "RoutineReminder";

export type Priority = "low" | "medium" | "high" | "critical";

export type SuggestionType =
  | "Completion"
  | "Refactor"
  | "Documentation"
  | "Test"
  | "Optimization"
  | "Debug"
  | "Learning"
  | "CausalInsight";

export type PredictedIntent =
  | { type: "CodeCompletion"; language: string; context: string }
  | { type: "Documentation"; topic: string }
  | { type: "Search"; queryType: string }
  | { type: "Refactoring"; target: string }
  | { type: "Debug"; error: string }
  | { type: "TestGeneration"; target: string }
  | { type: "Unknown" };

export type RecurrenceFrequency = "daily" | "weekly" | "monthly";

export interface TriggerCondition {
  conditionType: TriggerConditionType;
  threshold?: number;
  contextKey?: string;
}

export type TriggerConditionType =
  | "FileOpened"
  | "ErrorDetected"
  | "TimeBased"
  | "PatternMatch"
  | "UserIdle"
  | "LowActivity";

export interface ProactiveAction {
  type: string;
  [key: string]: unknown;
}

export interface ProactiveCapability {
  capabilityType: CapabilityType;
  confidence: number;
  triggerConditions: TriggerCondition[];
  action: ProactiveAction;
}

export interface ContextWindow {
  files: string[];
  recentActions: string[];
  currentLanguage?: string;
  projectType?: string;
}

export interface ContextPrediction {
  predictedIntent: PredictedIntent;
  confidence: number;
  reasoning: string;
  suggestedActions: SuggestedAction[];
  contextWindow: ContextWindow;
  createdAt: string;
}

export interface SuggestedAction {
  actionType: string;
  title: string;
  description: string;
  priority: Priority;
}

export interface ProactiveSuggestion {
  id: string;
  suggestionType: SuggestionType;
  title: string;
  description: string;
  /** Backend serializes SuggestionAction as { type, language?, context?, target?, topic? } etc. */
  action: Record<string, unknown>;
  priority: Priority;
  createdAt: string;
  expiresAt: string;
  accepted?: boolean;
}

export interface Reminder {
  id: string;
  title: string;
  description: string;
  scheduledAt: string;
  recurrence?: ReminderRecurrence;
  completed: boolean;
  createdAt: string;
}

export interface ReminderRecurrence {
  frequency: RecurrenceFrequency;
  interval: number;
}

export interface ProactiveConfig {
  enabled: boolean;
  maxSuggestions: number;
  suggestionTtlMinutes: number;
  predictionConfidenceThreshold: number;
  prefetchEnabled: boolean;
  reminderEnabled: boolean;
}

export interface ContextFeatures {
  currentFile?: string;
  currentLanguage?: string;
  recentActions: ActionType[];
  timeOfDay: number;
  dayOfWeek: string;
  projectType?: string;
  userActivityLevel: ActivityLevel;
  detectedErrors: string[];
  detectedPatterns: PatternMatch[];
}

export type ActionType =
  | "fileopened"
  | "fileedited"
  | "filesaved"
  | "commandexecuted"
  | "searchperformed"
  | "toolused"
  | "errorencountered"
  | "codegenerated"
  | "documentationviewed"
  | "testrun";

/** Backend serializes with serde `rename_all = "lowercase"` */
export type ActivityLevel = "low" | "medium" | "high";

export interface PatternMatch {
  patternType: string;
  matchedText: string;
  confidence: number;
}

export interface PredictionResult {
  predictions: ContextPrediction[];
  topPrediction?: ContextPrediction;
}

export type PrefetchType =
  | "codeCompletion"
  | "searchResults"
  | "documentation"
  | "contextAnalysis"
  | "toolCache";

export interface PrefetchResult {
  prefetchType: PrefetchType;
  resourceId: string;
  data?: string;
  ready: boolean;
  estimatedPrepareTimeMs: number;
  createdAt: string;
}

export interface PrefetchResults {
  results: PrefetchResult[];
  totalEstimatedTimeMs: number;
  criticalPath: string[];
}

export interface UserPreferenceProfile {
  userId: string;
  codingStyle: CodingStylePreference;
  communicationStyle: CommunicationStylePreference;
  workHabits: WorkHabitPreference;
  learningEnabled: boolean;
}

export interface CodingStylePreference {
  preferredLanguage?: string;
  documentationLevel: DocumentationLevel;
  testCreation: boolean;
}

export type DocumentationLevel = "minimal" | "standard" | "comprehensive";

export interface CommunicationStylePreference {
  detailLevel: DetailLevel;
  tone: CommunicationTone;
}

export type DetailLevel = "brief" | "moderate" | "detailed";
export type CommunicationTone = "formal" | "neutral" | "casual";

export interface WorkHabitPreference {
  peakHoursStart: number;
  peakHoursEnd: number;
  multiTaskingLevel: number;
}

export interface ReminderSchedule {
  reminderId: string;
  nextTrigger: string;
  recurrence?: ReminderRecurrence;
}

export interface ReminderNotification {
  notificationId: string;
  reminder: Reminder;
  triggeredAt: string;
  acknowledged: boolean;
}

export interface SuggestionEngineConfig {
  maxSuggestions: number;
  minConfidenceThreshold: number;
  suggestionTtlMinutes: number;
  personalizationEnabled: boolean;
  habitBasedSuggestions: boolean;
}

export interface PrefetcherConfig {
  enabled: boolean;
  maxCacheSize: number;
  cacheTtlSeconds: number;
  parallelPrefetch: boolean;
  prioritizeCriticalPath: boolean;
}

export interface ReminderManagerConfig {
  enabled: boolean;
  maxActiveReminders: number;
  snoozeDurationMinutes: number;
  autoCleanupCompleted: boolean;
  cleanupAfterDays: number;
}

// ── Reminder backend DTO ──

export interface ReminderItem {
  id: string;
  title: string;
  description: string;
  scheduledAt: string;
  completed: boolean;
  recurrence?: ReminderRecurrence;
  createdAt: string;
}

export interface ReminderListResult {
  active: ReminderItem[];
  completed: ReminderItem[];
  pendingNotifications: ReminderNotificationItem[];
}

export interface ReminderNotificationItem {
  notificationId: string;
  reminderId: string;
  reminderTitle: string;
  triggeredAt: string;
  acknowledged: boolean;
}

// ── Awareness / Saliency backend DTO（`proactive_awareness_summary` 返回值）──

export type SignalSource =
  | "context_prediction"
  | "novelty"
  | "causal_insight"
  | "nudge"
  | "reminder"
  | "prefetch";

/** 单帧觉知快照（camelCase，对齐后端 `#[serde(rename_all = "camelCase")]`） */
export interface AwarenessFrame {
  arousal: number;
  cognitiveLoad: number;
  selfEfficacy: number;
  dominantSource: SignalSource | null;
  dominantOriginId: string | null;
  createdAt: string;
}

/** 置信度校准偏差摘要 */
export interface BiasSummary {
  avgBias: number;
  overconfidentRate: number;
  calibratedRate: number;
  underconfidentRate: number;
}

/** 仲裁器上一次广播包（camelCase，对齐后端 `#[serde(rename_all = "camelCase")]`） */
export interface BroadcastPacket {
  timestamp: string;
  winners: Array<{
    signal: {
      source: SignalSource;
      salience: number;
      originId: string;
      createdAt: string;
    };
    effective: number;
  }>;
}

export interface AwarenessSummary {
  frames: AwarenessFrame[];
  calibration: BiasSummary | null;
  lastBroadcast: BroadcastPacket | null;
}
