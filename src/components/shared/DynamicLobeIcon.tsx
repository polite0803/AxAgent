import { memo, useEffect, useState } from "react";

type IconModuleType = React.ComponentType<{ size?: number }> & Record<string, React.ComponentType<{ size?: number }>>;

const iconCache = new Map<string, IconModuleType>();

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
  const [IconModule, setIconModule] = useState<IconModuleType | null>(null);
  const [loadError, setLoadError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const cached = iconCache.get(iconId);
    if (cached) {
      setIconModule(cached);
      return;
    }

    import(/* @vite-ignore */ `@lobehub/icons/es/icons/${iconId}.js`)
      .then((module) => {
        if (!cancelled) {
          iconCache.set(iconId, module);
          setIconModule(module);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setLoadError(true);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [iconId]);

  if (!IconModule) {
    return <div style={{ width: size, height: size }} />;
  }

  if (loadError) {
    return (
      <div
        style={{
          width: size,
          height: size,
          opacity: 0.3,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          fontSize: size * 0.6,
        }}
      >
        ?
      </div>
    );
  }

  if (type === "color" && IconModule.Color) {
    return <IconModule.Color size={size} />;
  }
  if (type === "avatar" && IconModule.Avatar) {
    return <IconModule.Avatar size={size} />;
  }
  return <IconModule size={size} />;
});
