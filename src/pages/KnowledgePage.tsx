import { theme } from 'antd';
import KnowledgeSettings from '@/components/settings/KnowledgeSettings';

export function KnowledgePage() {
  const { token } = theme.useToken();

  return (
    <div className="h-full" style={{ overflow: 'hidden', backgroundColor: token.colorBgElevated }}>
      <KnowledgeSettings />
    </div>
  );
}
