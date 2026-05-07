import { SkillMarkdownPage } from "./SkillMarkdownPage";
import { SkillSandboxContainer } from "./SkillSandboxContainer";

interface SkillPageRendererProps {
  componentType: string;
  componentConfig: Record<string, unknown>;
  skillName: string;
}

/**
 * Skill 渲染分发器。
 * 统一使用 Sandbox 架构（安全隔离 + RPC 通信），Markdown 作为纯内容渲染路径。
 */
export function SkillPageRenderer({ componentType, componentConfig, skillName }: SkillPageRendererProps) {
  if (componentType === "Markdown") {
    return <SkillMarkdownPage skillName={skillName} />;
  }

  // 默认走 Sandbox（包括 "Sandbox" 及历史 "Html" 等类型均映射到此）
  const componentId = (componentConfig.id as string) || "default";
  const permissions = componentConfig.permissions as Record<string, string[]>;
  return (
    <SkillSandboxContainer
      skillName={skillName}
      componentId={componentId}
      componentConfig={componentConfig}
      permissions={permissions as any}
    />
  );
}
