import { memo } from 'react';
// Static barrel import — all icon components are already mostly bundled
// via providerConfig/modelConfig; marginal addition is small.
import * as LobeIcons from '@lobehub/icons/es/icons.js';

const iconsMap = LobeIcons as unknown as Record<string, any>;

interface DynamicLobeIconProps {
  iconId: string;
  size?: number;
  type?: 'color' | 'avatar' | 'mono';
}

/**
 * Renders a @lobehub/icons icon by its toc `id` (e.g., "Ai302", "OpenAI")
 * via direct component lookup, bypassing the incomplete keyword matching
 * in ProviderIcon/ModelIcon.
 */
export const DynamicLobeIcon = memo(function DynamicLobeIcon({
  iconId,
  size = 24,
  type = 'avatar',
}: DynamicLobeIconProps) {
  const IconModule = iconsMap[iconId];
  if (!IconModule) return <div style={{ width: size, height: size }} />;

  if (type === 'color' && IconModule.Color) {
    return <IconModule.Color size={size} />;
  }
  if (type === 'avatar' && IconModule.Avatar) {
    return <IconModule.Avatar size={size} />;
  }
  return <IconModule size={size} />;
});
