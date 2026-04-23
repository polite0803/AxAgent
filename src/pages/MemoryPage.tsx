import { theme } from 'antd';
import MemorySettings from '@/components/settings/MemorySettings';

export function MemoryPage() {
  const { token } = theme.useToken();

  return (
    <div className="h-full" style={{ overflow: 'hidden', backgroundColor: token.colorBgElevated }}>
      <MemorySettings />
    </div>
  );
}
