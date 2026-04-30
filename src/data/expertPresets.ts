import type { ExpertRole } from "@/types/expert";

export const BUILTIN_EXPERT_PRESETS: ExpertRole[] = [
  {
    id: "general-assistant",
    displayName: "通用助手",
    description: "全能的 AI 助手，无特定领域偏向",
    category: "general",
    icon: "\uD83E\uDD16",
    systemPrompt: "",
    source: "builtin",
    tags: ["通用", "默认", "助手"],
  },
];
