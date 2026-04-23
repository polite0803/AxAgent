import { useEffect, useState } from 'react';
import { theme } from 'antd';
import { useGatewayLinkStore } from '@/stores';
import { GatewayLinkList, GatewayLinkDetail, AddGatewayLinkModal } from '@/components/link';

export function LinkPage() {
  const { token } = theme.useToken();
  const fetchLinks = useGatewayLinkStore((s) => s.fetchLinks);
  const [addModalOpen, setAddModalOpen] = useState(false);

  useEffect(() => {
    void fetchLinks();
  }, [fetchLinks]);

  return (
    <div className="flex h-full">
      <div
        className="shrink-0"
        style={{ width: 280, borderRight: `1px solid ${token.colorBorderSecondary}` }}
      >
        <GatewayLinkList onAdd={() => setAddModalOpen(true)} />
      </div>
      <div className="min-w-0 flex-1 overflow-hidden">
        <GatewayLinkDetail />
      </div>
      <AddGatewayLinkModal
        open={addModalOpen}
        onClose={() => setAddModalOpen(false)}
      />
    </div>
  );
}
