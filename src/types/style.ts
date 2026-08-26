// SPDX-License-Identifier: AGPL-3.0-only

export interface StyleDimensions {
  namingScore: number;
  densityScore: number;
  commentRatio: number;
  abstractionLevel: number;
  formalityScore: number;
  structureScore: number;
  technicalDepth: number;
  explanationLength: number;
}

export interface StyleVector {
  dimensions: StyleDimensions;
  sourceConfidence: number;
  learnedAt: string;
  sampleCount: number;
}

export interface CodeTemplate {
  name: string;
  template: string;
  description: string;
}

export interface StylePattern {
  patternType: PatternType;
  original: string;
  transformed: string;
  context: string;
  usageCount: number;
}

export type PatternType = "Naming" | "Formatting" | "Structure" | "Comment";

export interface CodeStyleTemplate {
  name: string;
  patterns: StylePattern[];
  templates: CodeTemplate[];
}

export interface DocumentStyleProfile {
  formalityLevel: number;
  structureLevel: number;
  technicalVocabularyRatio: number;
  explanationDetailLevel: number;
  preferredFormat: DocumentFormat;
}

export type DocumentFormat = "PlainText" | "Markdown" | "Structured";

export interface UserStyleProfile {
  id: string;
  userId: string;
  codeStyleVector: StyleVector;
  documentStyleProfile: DocumentStyleProfile;
  codeTemplates: CodeStyleTemplate[];
  learnedPatterns: LearnedPattern[];
  createdAt: string;
  updatedAt: string;
  totalSamples: number;
  confidence: number;
}

export interface LearnedPattern {
  id: string;
  patternType: LearnedPatternType;
  original: string;
  transformed: string;
  context: string;
  usageCount: number;
  lastUsed: string;
}

export type LearnedPatternType =
  | "Naming"
  | "Formatting"
  | "Comment"
  | "Structure"
  | "Document";

export interface StyleMigratorStats {
  totalProfiles: number;
  totalSamples: number;
  averageConfidence: number;
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
