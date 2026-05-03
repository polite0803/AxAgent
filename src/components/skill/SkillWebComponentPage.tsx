import { invoke } from "@/lib/invoke";
import { useCallback, useEffect, useState } from "react";
import { SkillErrorFallback, SkillLoadingSkeleton } from "./SkillErrorFallback";

interface SkillWebComponentPageProps {
  skillName: string;
  componentConfig: Record<string, unknown>;
}

export function SkillWebComponentPage({ skillName, componentConfig }: SkillWebComponentPageProps) {
  const tagName = (componentConfig.tagName as string) || "";
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadScript = useCallback(async () => {
    setError(null);
    if (!tagName) {
      setError("tagName 未在 componentConfig 中指定");
      return;
    }
    if (customElements.get(tagName)) {
      setReady(true);
      return;
    }
    try {
      const jsCode = await invoke<string>("skill_read_asset", {
        skillName,
        path: (componentConfig.entry as string) || "dist/component.js",
      });
      const blob = new Blob([jsCode], { type: "application/javascript" });
      const url = URL.createObjectURL(blob);
      await import(/* @vite-ignore */ url);
      URL.revokeObjectURL(url);
      setReady(true);
    } catch (e) {
      setError(`加载 WebComponent 失败: ${String(e)}`);
    }
  }, [skillName, tagName, componentConfig]);

  useEffect(() => {
    loadScript();
  }, [loadScript]);

  if (error) { return <SkillErrorFallback error={error} skillName={skillName} onRetry={loadScript} />; }
  if (!ready) { return <SkillLoadingSkeleton />; }

  // 动态渲染自定义元素
  return (
    <div
      ref={(el) => {
        if (el && !el.querySelector(tagName)) {
          const customEl = document.createElement(tagName);
          const props = (componentConfig.props as Record<string, unknown>) || {};
          for (const [k, v] of Object.entries(props)) {
            customEl.setAttribute(k, String(v));
          }
          el.appendChild(customEl);
        }
      }}
    />
  );
}
