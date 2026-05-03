import type { SkillComponentType } from "@/types";
import { SkillHtmlPage } from "./SkillHtmlPage";
import { SkillIframePage } from "./SkillIframePage";
import { SkillMarkdownPage } from "./SkillMarkdownPage";
import { SkillReactPage } from "./SkillReactPage";
import { SkillWebComponentPage } from "./SkillWebComponentPage";

interface SkillPageRendererProps {
  componentType: SkillComponentType;
  componentConfig: Record<string, unknown>;
  skillName: string;
}

export function SkillPageRenderer({ componentType, componentConfig, skillName }: SkillPageRendererProps) {
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
