export interface StyleDimensions {
  naming_score: number;
  density_score: number;
  comment_ratio: number;
  abstraction_level: number;
  formality_score: number;
  structure_score: number;
  technical_depth: number;
  explanation_length: number;
}

export interface StyleVector {
  dimensions: StyleDimensions;
  source_confidence: number;
  learned_at: string;
  sample_count: number;
}

export interface CodeTemplate {
  name: string;
  template: string;
  description: string;
}

export interface StylePattern {
  pattern_type: PatternType;
  original: string;
  transformed: string;
  context: string;
  usage_count: number;
}

export type PatternType = "Naming" | "Formatting" | "Structure" | "Comment";

export interface CodeStyleTemplate {
  name: string;
  patterns: StylePattern[];
  templates: CodeTemplate[];
}

export interface DocumentStyleProfile {
  formality_level: number;
  structure_level: number;
  technical_vocabulary_ratio: number;
  explanation_detail_level: number;
  preferred_format: DocumentFormat;
}

export type DocumentFormat = "PlainText" | "Markdown" | "Structured";

export interface UserStyleProfile {
  id: string;
  user_id: string;
  code_style_vector: StyleVector;
  document_style_profile: DocumentStyleProfile;
  code_templates: CodeStyleTemplate[];
  learned_patterns: LearnedPattern[];
  created_at: string;
  updated_at: string;
  total_samples: number;
  confidence: number;
}

export interface LearnedPattern {
  id: string;
  pattern_type: LearnedPatternType;
  original: string;
  transformed: string;
  context: string;
  usage_count: number;
  last_used: string;
}

export type LearnedPatternType =
  | "Naming"
  | "Formatting"
  | "Comment"
  | "Structure"
  | "Document";

export interface StyleMigratorStats {
  total_profiles: number;
  total_samples: number;
  average_confidence: number;
}

export interface CodeSample {
  code: string;
  language: string;
  timestamp: string;
}

export interface MessageSample {
  content: string;
  role: string;
  timestamp: string;
}

export type StyleDimensionKey = keyof StyleDimensions;

export interface StyleAdjustment {
  dimension: StyleDimensionKey;
  previousValue: number;
  newValue: number;
}

export interface StyleComparisonResult {
  dimension: StyleDimensionKey;
  sourceValue: number;
  targetValue: number;
  difference: number;
}

export interface StylePreview {
  original: string;
  styled: string;
  adjustments: StyleAdjustment[];
}
