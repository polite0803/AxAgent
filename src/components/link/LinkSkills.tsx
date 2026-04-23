import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Card, Table, Button, Tag, Switch, Empty, App } from 'antd';
import type { Key } from 'antd/es/table/interface';
import { Upload, RefreshCw } from 'lucide-react';
import { useGatewayLinkStore } from '@/stores';
import type { GatewayLink } from '@/types';

interface LinkSkillsProps {
  link: GatewayLink;
}

export function LinkSkills({ link }: LinkSkillsProps) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const skillSyncs = useGatewayLinkStore((s) => s.skillSyncs);
  const pushSkills = useGatewayLinkStore((s) => s.pushSkills);
  const syncAllSkills = useGatewayLinkStore((s) => s.syncAllSkills);
  const fetchSkillSyncs = useGatewayLinkStore((s) => s.fetchSkillSyncs);
  const updateSyncSettings = useGatewayLinkStore((s) => s.updateSyncSettings);

  const handleAutoSyncChange = async (checked: boolean) => {
    try {
      await updateSyncSettings(link.id, link.auto_sync_models, checked);
    } catch {
      message.error(t('link.updateSettingsFailed'));
    }
  };

  const handleSyncAll = async () => {
    try {
      await syncAllSkills(link.id);
      message.success(t('link.syncAllSkillsSuccess'));
    } catch {
      message.error(t('link.syncAllSkillsFailed'));
    }
  };

  const handlePushSelected = async () => {
    if (selectedRowKeys.length === 0) {
      message.warning(t('link.noSkillSelected'));
      return;
    }
    try {
      await pushSkills(link.id, selectedRowKeys);
      message.success(t('link.pushSkillsSuccess'));
      setSelectedRowKeys([]);
    } catch {
      message.error(t('link.pushSkillsFailed'));
    }
  };

  const SYNC_STATUS_MAP: Record<string, { color: string; label: string }> = {
    synced: { color: 'green', label: t('link.syncStatusSynced') },
    pending: { color: 'orange', label: t('link.syncStatusPending') },
    failed: { color: 'red', label: t('link.syncStatusFailed') },
    not_selected: { color: 'default', label: t('link.syncStatusNotSelected') },
  };

  const columns = [
    {
      title: t('link.skillName'),
      dataIndex: 'skill_name',
      key: 'skill_name',
      ellipsis: true,
    },
    {
      title: t('link.skillVersion'),
      dataIndex: 'skill_version',
      key: 'skill_version',
      width: 100,
      render: (v: string | null) => v ?? '-',
    },
    {
      title: t('link.syncStatus'),
      dataIndex: 'sync_status',
      key: 'sync_status',
      width: 120,
      render: (status: string) => {
        const mapped = SYNC_STATUS_MAP[status] ?? { color: 'default', label: status };
        return <Tag color={mapped.color}>{mapped.label}</Tag>;
      },
    },
    {
      title: t('link.lastSync'),
      dataIndex: 'last_sync_at',
      key: 'last_sync_at',
      width: 160,
      render: (v: number | null) => v ? new Date(v * 1000).toLocaleString() : '-',
    },
    {
      title: t('link.actions'),
      key: 'actions',
      width: 100,
      render: (_: unknown) => (
        <Button
          size="small"
          icon={<Upload size={14} />}
          onClick={() => handlePushSelected()}
          disabled={link.status !== 'connected'}
        >
          {t('link.push')}
        </Button>
      ),
    },
  ];

  const rowSelection = {
    selectedRowKeys,
    onChange: (keys: Key[]) => setSelectedRowKeys(keys as string[]),
    getCheckboxProps: () => ({
      disabled: link.status !== 'connected',
    }),
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Switch
            size="small"
            checked={link.auto_sync_skills}
            onChange={handleAutoSyncChange}
            disabled={link.status !== 'connected'}
          />
          <span style={{ fontSize: 13 }}>{t('link.autoSyncSkills')}</span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            icon={<RefreshCw size={14} />}
            onClick={() => fetchSkillSyncs(link.id)}
          >
            {t('common.refresh')}
          </Button>
          <Button
            icon={<Upload size={14} />}
            onClick={handlePushSelected}
            disabled={link.status !== 'connected' || selectedRowKeys.length === 0}
          >
            {t('link.pushSelected')} {selectedRowKeys.length > 0 && `(${selectedRowKeys.length})`}
          </Button>
          <Button
            type="primary"
            icon={<Upload size={14} />}
            onClick={handleSyncAll}
            disabled={link.status !== 'connected'}
          >
            {t('link.syncAllSkills')}
          </Button>
        </div>
      </div>

      <Card size="small">
        {skillSyncs.length === 0 ? (
          <Empty description={t('link.noSkills')} image={Empty.PRESENTED_IMAGE_SIMPLE} />
        ) : (
          <Table
            dataSource={skillSyncs}
            columns={columns}
            rowKey="skill_name"
            size="small"
            pagination={false}
            rowSelection={rowSelection}
          />
        )}
      </Card>
    </div>
  );
}
