import { Avatar, theme } from 'antd';
import { Brain } from 'lucide-react';
import { useResolvedAvatarSrc } from '@/hooks/useResolvedAvatarSrc';
import type { AvatarType } from '@/stores';
import type { MemoryNamespace } from '@/types';
import { CHAT_ICON_COLORS } from '@/lib/iconColors';

interface NamespaceIconProps {
  ns: MemoryNamespace;
  size?: number;
}

export function NamespaceIcon({ ns, size = 16 }: NamespaceIconProps) {
  const resolvedSrc = useResolvedAvatarSrc((ns.iconType as AvatarType) ?? 'icon', ns.iconValue ?? '');
  const { token } = theme.useToken();

  if (ns.iconType === 'emoji' && ns.iconValue) {
    return (
      <span style={{
        width: size, height: size, borderRadius: '50%',
        backgroundColor: token.colorFillSecondary,
        display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
        fontSize: size * 0.7, lineHeight: 1, flexShrink: 0,
      }}>
        {ns.iconValue}
      </span>
    );
  }
  if ((ns.iconType === 'url' || ns.iconType === 'file') && ns.iconValue) {
    const src = ns.iconType === 'file' ? resolvedSrc : ns.iconValue;
    return <Avatar size={size} src={src} style={{ flexShrink: 0 }} />;
  }
  return <Brain size={size} style={{ flexShrink: 0, color: CHAT_ICON_COLORS.Brain }} />;
}
