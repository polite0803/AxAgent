import type { SkillComponentType } from "@/types";
import { SkillHtmlPage } from "./SkillHtmlPage";
import { SkillIframePage } from "./SkillIframePage";
import { SkillMarkdownPage } from "./SkillMarkdownPage";
import { SkillReactPage } from "./SkillReactPage";
import { SkillSandboxContainer } from "./SkillSandboxContainer";
import { SkillWebComponentPage } from "./SkillWebComponentPage";

interface SkillPageRendererProps {
  componentType: SkillComponentType;
  componentConfig: Record<string, unknown>;
  skillName: string;
}

/**
 * SkillPageRenderer — 多格式 Skill 渲染分发器
 *
 * 支持 V2 新架构（Sandbox）和 V1 旧架构（Html/Iframe/React/WebComponent/Markdown）。
 * 新 Skill 推荐使用 "Sandbox" 类型以获得安全隔离和 RPC 通信能力。
 * 旧 Skill 继续使用原有渲染方式。
 */
export function SkillPageRenderer({ componentType, componentConfig, skillName }: SkillPageRendererProps) {
  // V2: 统一 Sandbox 架构（推荐，具备安全隔离 + RPC 通信）
  if (componentType === "Sandbox") {
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

  // V1: 旧架构渲染路径（保持向后兼容）
  switch (componentType) {
    case "Html":
      return <SkillHtmlPage componentConfig={componentConfig} skillName={skillName} />;
    case "Iframe":
      return <SkillIframePage componentConfig={componentConfig} />;
    case "Markdown":
      return <SkillMarkdownPage skillName={skillName} />;
    case "React":
      return <SkillReactPage componentConfig={componentConfig} skillName={skillName} />;
    case "WebComponent":
      return <SkillWebComponentPage componentConfig={componentConfig} skillName={skillName} />;
    default:
      return (
        <div style={{ padding: 24, textAlign: "center", color: "var(--color-text-secondary)" }}>
          Unknown component type: "{componentType}"
        </div>
      );
  }
}
