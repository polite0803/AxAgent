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

const NOW = Date.now();

// 内置专家预设：当前仅保留通用助手（general-assistant）。
// 其余内置专家（code-reviewer 等）已移除：前端运行时仅加载 general-assistant，
// 其余 id 仍由后端 n8n 导入映射（infer_agent_from_n8n）作为工作流 AgentNode 的
// profile/expert 标识符使用，不依赖本文件定义。
export const BUILTIN_EXPERT_PRESETS: BuiltinExpertPreset[] = [
  {
    id: "general-assistant",
    nameKey: "",
    name: "通用助手",
    descKey: "",
    description: "通用的 AI 助手，适用于日常对话和通用问题解答",
    category: "general",
    icon: "🤖",
    systemPrompt: "你是一个智能 AI 助手，请根据用户的问题提供准确、有帮助的回答。",
    source: "builtin",
    agentRole: null,
    tags: ["通用", "助手", "对话"],
    sortOrder: 0,
    isEnabled: true,
    createdAt: NOW,
    updatedAt: NOW,
  },
];
