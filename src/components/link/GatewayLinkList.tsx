import { Button, Input, Switch, theme, App, Popconfirm } from 'antd';
import { Plus, Search, Trash2 } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useGatewayLinkStore } from '@/stores';
import type { GatewayLink, GatewayLinkStatus } from '@/types';

const STATUS_INDICATOR: Record<GatewayLinkStatus, { color: string; labelKey: string }> = {
  connected: { color: '#52c41a', labelKey: 'link.statusConnected' },
  disconnected: { color: '#d9d9d9', labelKey: 'link.statusDisconnected' },
  connecting: { color: '#faad14', labelKey: 'link.statusConnecting' },
  error: { color: '#ff4d4f', labelKey: 'link.statusError' },
};

function LinkTypeBadge({ type }: { type: string }) {
  const { t } = useTranslation();
  const labelMap: Record<string, string> = {
    openclaw: 'OpenClaw',
    hermes: 'Hermes',
    custom: t('link.typeCustom'),
  };
  return (
    <span style={{ fontSize: 11, opacity: 0.6 }}>
      {labelMap[type] ?? type}
    </span>
  );
}

export function GatewayLinkList({ onAdd }: { onAdd: () => void }) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { message } = App.useApp();
  const links = useGatewayLinkStore((s) => s.links);
  const selectedLinkId = useGatewayLinkStore((s) => s.selectedLinkId);
  const selectLink = useGatewayLinkStore((s) => s.selectLink);
  const toggleLink = useGatewayLinkStore((s) => s.toggleLink);
  const deleteLink = useGatewayLinkStore((s) => s.deleteLink);
  const fetchLinks = useGatewayLinkStore((s) => s.fetchLinks);

  const [search, setSearch] = useState('');

  const filteredLinks = useMemo(
    () => links.filter((l) => l.name.toLowerCase().includes(search.toLowerCase())),
    [links, search],
  );

  const handleToggle = async (link: GatewayLink, checked: boolean) => {
    try {
      await toggleLink(link.id, checked);
      void fetchLinks();
    } catch {
      message.error(t('link.addFailed'));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteLink(id);
      message.success(t('link.deleteSuccess'));
    } catch {
      message.error(t('link.deleteFailed'));
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="p-3 flex items-center gap-2">
        <Input
          prefix={<Search size={14} />}
          placeholder={t('link.searchGateways')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          allowClear
          style={{ flex: 1 }}
        />
        <Button
          type="default"
          icon={<Plus size={16} />}
          onClick={onAdd}
          style={{ flexShrink: 0 }}
        />
      </div>
      <div className="flex-1 overflow-y-auto p-2 flex flex-col gap-1">
        {filteredLinks.length === 0 && (
          <div
            className="flex items-center justify-center"
            style={{ color: token.colorTextSecondary, fontSize: 13, padding: 24 }}
          >
            {t('link.noGateways')}
          </div>
        )}
        {filteredLinks.map((link) => {
          const isSelected = selectedLinkId === link.id;
          const indicator = STATUS_INDICATOR[link.status];
          return (
            <div
              key={link.id}
              className="flex items-center cursor-pointer px-3 py-2.5 transition-colors"
              style={{
                borderRadius: token.borderRadius,
                backgroundColor: isSelected ? token.colorPrimaryBg : undefined,
                opacity: link.enabled ? 1 : 0.4,
              }}
              onClick={() => selectLink(link.id)}
              onMouseEnter={(e) => {
                if (!isSelected) {
                  e.currentTarget.style.backgroundColor = token.colorFillQuaternary;
                }
              }}
              onMouseLeave={(e) => {
                if (!isSelected) {
                  e.currentTarget.style.backgroundColor = '';
                }
              }}
            >
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span
                    style={{
                      width: 8,
                      height: 8,
                      borderRadius: '50%',
                      backgroundColor: indicator.color,
                      flexShrink: 0,
                    }}
                  />
                  <span
                    style={{
                      color: isSelected ? token.colorPrimary : undefined,
                      fontWeight: 500,
                      fontSize: 13,
                    }}
                  >
                    {link.name}
                  </span>
                  <LinkTypeBadge type={link.link_type} />
                </div>
                <div
                  style={{
                    fontSize: 11,
                    color: token.colorTextTertiary,
                    marginTop: 2,
                    paddingLeft: 16,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {link.endpoint}
                </div>
              </div>
              <Switch
                size="small"
                checked={link.enabled}
                onClick={(_, e) => e.stopPropagation()}
                onChange={(checked) => handleToggle(link, checked)}
              />
              <Popconfirm
                title={t('link.deleteConfirm')}
                onConfirm={() => handleDelete(link.id)}
                okText={t('common.confirm')}
                cancelText={t('common.cancel')}
              >
                <Button
                  type="text"
                  size="small"
                  icon={<Trash2 size={14} />}
                  onClick={(e) => e.stopPropagation()}
                  style={{ color: token.colorTextQuaternary, flexShrink: 0 }}
                />
              </Popconfirm>
            </div>
          );
        })}
      </div>
    </div>
  );
}
