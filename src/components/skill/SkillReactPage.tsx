import { invoke } from "@/lib/invoke";
import { Suspense, useCallback, useEffect, useState } from "react";
import { SkillErrorBoundary, SkillErrorFallback, SkillLoadingSkeleton } from "./SkillErrorFallback";

interface SkillReactPageProps {
  skillName: string;
  componentConfig: Record<string, unknown>;
}

export function SkillReactPage({ skillName, componentConfig }: SkillReactPageProps) {
  const [Component, setComponent] = useState<React.ComponentType<Record<string, unknown>> | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadComponent = useCallback(async () => {
    setError(null);
    try {
      const jsCode = await invoke<string>("skill_read_asset", {
        skillName,
        path: (componentConfig.entry as string) || "dist/component.js",
      });
      const blob = new Blob([jsCode], { type: "application/javascript" });
      const url = URL.createObjectURL(blob);
      const mod = await import(/* @vite-ignore */ url);
      const exportName = (componentConfig.exportName as string) || "default";
      setComponent(() => mod[exportName] || mod.default);
      URL.revokeObjectURL(url);
    } catch (e) {
      setError(`加载 React 组件失败: ${String(e)}`);
    }
  }, [skillName, componentConfig]);

  useEffect(() => {
    loadComponent();
  }, [loadComponent]);

  if (error) { return <SkillErrorFallback error={error} skillName={skillName} onRetry={loadComponent} />; }
  if (!Component) { return <SkillLoadingSkeleton />; }

  const Comp = Component;
  return (
    <SkillErrorBoundary skillName={skillName}>
      <Suspense fallback={<SkillLoadingSkeleton />}>
        <Comp {...((componentConfig.props as Record<string, unknown>) || {})} />
      </Suspense>
    </SkillErrorBoundary>
  );
}
