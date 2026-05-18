import { memo, useEffect, useState } from "react";

const iconCache = new Map<string, any>();

interface DynamicLobeIconProps {
  iconId: string;
  size?: number;
  type?: "color" | "avatar" | "mono";
}

/**
 * Renders a @lobehub/icons icon by its toc `id` (e.g., "Ai302", "OpenAI")
 * via lazy dynamic import, bypassing the incomplete keyword matching
 * in ProviderIcon/ModelIcon.
 */
export const DynamicLobeIcon = memo(function DynamicLobeIcon({
  iconId,
  size = 24,
  type = "avatar",
}: DynamicLobeIconProps) {
  const [IconModule, setIconModule] = useState<any>(null);

  useEffect(() => {
    let cancelled = false;
    const cached = iconCache.get(iconId);
    if (cached) {
      setIconModule(cached);
      return;
    }

    import(`@lobehub/icons/es/icons/${iconId}.js`)
      .then((module) => {
        if (!cancelled) {
          iconCache.set(iconId, module);
          setIconModule(module);
        }
      })
      .catch(() => {});

    return () => {
      cancelled = true;
    };
  }, [iconId]);

  if (!IconModule) { return <div style={{ width: size, height: size }} />; }

  if (type === "color" && IconModule.Color) {
    return <IconModule.Color size={size} />;
  }
  if (type === "avatar" && IconModule.Avatar) {
    return <IconModule.Avatar size={size} />;
  }
  return <IconModule size={size} />;
});
