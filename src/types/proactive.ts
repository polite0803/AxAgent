export type CapabilityType = "ContextPrediction" | "ProactiveSuggestion" | "TaskPrefetch" | "RoutineReminder";

export type Priority = "low" | "medium" | "high" | "critical";

export type SuggestionType = "Completion" | "Refactor" | "Documentation" | "Test" | "Optimization" | "Learning";

export type PredictedIntent =
  | { type: "CodeCompletion"; language: string; context: string }
  | { type: "Documentation"; topic: string }
  | { type: "Search"; query_type: string }
  | { type: "Refactoring"; target: string }
  | { type: "Debug"; error: string }
  | { type: "TestGeneration"; target: string }
  | { type: "Unknown" };

export type RecurrenceFrequency = "daily" | "weekly" | "monthly";

export interface TriggerCondition {
  condition_type: TriggerConditionType;
  threshold?: number;
  context_key?: string;
}

export type TriggerConditionType = "FileOpened" | "ErrorDetected" | "TimeBased" | "PatternMatch" | "UserIdle" | "LowActivity";

export interface ProactiveAction {
  type: string;
  [key: string]: unknown;
}

export interface ProactiveCapability {
  capability_type: CapabilityType;
  confidence: number;
  trigger_conditions: TriggerCondition[];
  action: ProactiveAction;
}

export interface ContextWindow {
  files: string[];
  recent_actions: string[];
  current_language?: string;
  project_type?: string;
}

export interface ContextPrediction {
  predicted_intent: PredictedIntent;
  confidence: number;
  reasoning: string;
  suggested_actions: SuggestedAction[];
  context_window: ContextWindow;
  created_at: string;
}

export interface SuggestedAction {
  action_type: string;
  title: string;
  description: string;
  priority: Priority;
}

export interface ProactiveSuggestion {
  id: string;
  suggestion_type: SuggestionType;
  title: string;
  description: string;
  action: SuggestedAction;
  priority: Priority;
  created_at: string;
  expires_at: string;
  accepted?: boolean;
}

export interface Reminder {
  id: string;
  title: string;
  description: string;
  scheduled_at: string;
  recurrence?: ReminderRecurrence;
  completed: boolean;
  created_at: string;
}

export interface ReminderRecurrence {
  frequency: RecurrenceFrequency;
  interval: number;
}

export interface ProactiveConfig {
  enabled: boolean;
  max_suggestions: number;
  suggestion_ttl_minutes: number;
  prediction_confidence_threshold: number;
  prefetch_enabled: boolean;
  reminder_enabled: boolean;
}

export interface ContextFeatures {
  current_file?: string;
  current_language?: string;
  recent_actions: ActionType[];
  time_of_day: number;
  day_of_week: string;
  project_type?: string;
  user_activity_level: ActivityLevel;
  detected_errors: string[];
  detected_patterns: PatternMatch[];
}

export type ActionType =
  | "FileOpened"
  | "FileEdited"
  | "FileSaved"
  | "CommandExecuted"
  | "SearchPerformed"
  | "ToolUsed"
  | "ErrorEncountered"
  | "CodeGenerated"
  | "DocumentationViewed"
  | "TestRun";

export type ActivityLevel = "Low" | "Medium" | "High";

export interface PatternMatch {
  pattern_type: string;
  matched_text: string;
  confidence: number;
}

export interface PredictionResult {
  predictions: ContextPrediction[];
  top_prediction?: ContextPrediction;
}

export type PrefetchType = "codeCompletion" | "searchResults" | "documentation" | "contextAnalysis" | "toolCache";

export interface PrefetchResult {
  prefetch_type: PrefetchType;
  resource_id: string;
  data?: string;
  ready: boolean;
  estimated_prepare_time_ms: number;
  created_at: string;
}

export interface PrefetchResults {
  results: PrefetchResult[];
  total_estimated_time_ms: number;
  critical_path: string[];
}

export interface UserPreferenceProfile {
  user_id: string;
  coding_style: CodingStylePreference;
  communication_style: CommunicationStylePreference;
  work_habits: WorkHabitPreference;
  learning_enabled: boolean;
}

export interface CodingStylePreference {
  preferred_language?: string;
  documentation_level: DocumentationLevel;
  test_creation: boolean;
}

export type DocumentationLevel = "minimal" | "standard" | "comprehensive";

export interface CommunicationStylePreference {
  detail_level: DetailLevel;
  tone: CommunicationTone;
}

export type DetailLevel = "brief" | "moderate" | "detailed";
export type CommunicationTone = "formal" | "neutral" | "casual";

export interface WorkHabitPreference {
  peak_hours_start: number;
  peak_hours_end: number;
  multi_tasking_level: number;
}

export interface ReminderSchedule {
  reminder_id: string;
  next_trigger: string;
  recurrence?: ReminderRecurrence;
}

export interface ReminderNotification {
  notification_id: string;
  reminder: Reminder;
  triggered_at: string;
  acknowledged: boolean;
}

export interface SuggestionEngineConfig {
  max_suggestions: number;
  min_confidence_threshold: number;
  suggestion_ttl_minutes: number;
  personalization_enabled: boolean;
  habit_based_suggestions: boolean;
}

export interface PrefetcherConfig {
  enabled: boolean;
  max_cache_size: number;
  cache_ttl_seconds: number;
  parallel_prefetch: boolean;
  prioritize_critical_path: boolean;
}

export interface ReminderManagerConfig {
  enabled: boolean;
  max_active_reminders: number;
  snooze_duration_minutes: number;
  auto_cleanup_completed: boolean;
  cleanup_after_days: number;
}
