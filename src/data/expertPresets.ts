// SPDX-License-Identifier: AGPL-3.0-only

export interface BuiltinExpertPreset {
  id: string;
  nameKey: string;
  name: string;
  descKey: string;
  description: string;
  category: string;
  icon: string;
  systemPrompt: string;
  source: string;
  agentRole: null;
  tags: string[];
  sortOrder: number;
  isEnabled: boolean;
  createdAt: number;
  updatedAt: number;
}

export const BUILTIN_EXPERT_PRESETS: BuiltinExpertPreset[] = [];
