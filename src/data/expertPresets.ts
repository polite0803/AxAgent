// SPDX-License-Identifier: AGPL-3.0-only

export interface BuiltinExpertPreset {
  id: string;
  nameKey: string;
  name: string;
  descKey: string;
  description: string;
  systemPromptKey: string;
  systemPrompt: string;
  tagsKey: string;
  category: string;
  icon: string;
  source: string;
  agentRole: null;
  tags: string[];
  sortOrder: number;
  isEnabled: boolean;
  createdAt: number;
  updatedAt: number;
}

const NOW = Date.now();

// 内置专家预设：当前仅保留通用助手（general-assistant）。
// 其余内置专家（code-reviewer 等）已移除：前端运行时仅加载 general-assistant，
// 其余 id 仍由后端 n8n 导入映射（infer_agent_from_n8n）作为工作流 AgentNode 的
// profile/expert 标识符使用，不依赖本文件定义。
export const BUILTIN_EXPERT_PRESETS: BuiltinExpertPreset[] = [
  {
    id: "general-assistant",
    nameKey: "expertPreset.generalAssistant.name",
    name: "",
    descKey: "expertPreset.generalAssistant.desc",
    description: "",
    systemPromptKey: "expertPreset.generalAssistant.systemPrompt",
    systemPrompt: "",
    tagsKey: "expertPreset.generalAssistant.tags",
    category: "general",
    icon: "🤖",
    source: "builtin",
    agentRole: null,
    tags: [],
    sortOrder: 0,
    isEnabled: true,
    createdAt: NOW,
    updatedAt: NOW,
  },
];
