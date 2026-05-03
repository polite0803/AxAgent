import { resolveIconComponent } from "@/lib/skillIcons";
import { useSkillExtensionStore } from "@/stores";
import type { MergedChatCommand } from "@/stores/feature/skillExtensionStore";

export interface ChatCommandItem {
  name: string;
  label: string;
  description: string;
  icon?: React.ReactNode;
  mode: "declarative" | "agentic";
  skillName: string;
  source: MergedChatCommand;
}

/** 获取所有 skill 注册的聊天命令（供 CommandSuggest 使用） */
export function useSkillChatCommands(): ChatCommandItem[] {
  const chatCommands = useSkillExtensionStore((s) => s.chatCommands);

  return chatCommands.map((cc) => {
    const IconComp = cc.icon ? resolveIconComponent(cc.icon) : undefined;
    return {
      name: cc.name,
      label: `/${cc.name}`,
      description: cc.description,
      icon: IconComp ? <IconComp size={14} /> : undefined,
      mode: cc.mode,
      skillName: cc.skillName,
      source: cc,
    };
  });
}
