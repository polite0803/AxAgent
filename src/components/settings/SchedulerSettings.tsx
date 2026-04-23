import { Switch, InputNumber, Select, Divider, Tag, App } from 'antd';
import { useTranslation } from 'react-i18next';
import { useSettingsStore, useBackupStore } from '@/stores';
import { isTauri, invoke } from '@/lib/invoke';
import { SettingsGroup } from './SettingsGroup';

const rowStyle: React.CSSProperties = { padding: '4px 0' };

export function SchedulerSettings() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const inTauri = isTauri();
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const backupSettings = useBackupStore((s) => s.backupSettings);
  const updateBackupSettings = useBackupStore((s) => s.updateBackupSettings);

  const handleAutoBackupChange = async (enabled: boolean) => {
    if (!backupSettings) return;
    const newSettings = { ...backupSettings, enabled };
    await updateBackupSettings(newSettings);
    message.success(t('settings.scheduler.saved'));
  };

  const handleBackupIntervalChange = async (intervalHours: number | null) => {
    if (!backupSettings || !intervalHours) return;
    const newSettings = { ...backupSettings, intervalHours };
    await updateBackupSettings(newSettings);
    message.success(t('settings.scheduler.saved'));
  };

  const handleWebdavSyncChange = async (syncEnabled: boolean) => {
    saveSettings({ webdav_sync_enabled: syncEnabled });
    if (inTauri) {
      try {
        await invoke('restart_webdav_sync');
      } catch (e) {
        console.warn('Failed to restart WebDAV sync:', e);
      }
    }
    message.success(t('settings.scheduler.saved'));
  };

  const handleWebdavIntervalChange = async (syncIntervalMinutes: number) => {
    saveSettings({ webdav_sync_interval_minutes: syncIntervalMinutes });
    if (inTauri) {
      try {
        await invoke('restart_webdav_sync');
      } catch (e) {
        console.warn('Failed to restart WebDAV sync:', e);
      }
    }
    message.success(t('settings.scheduler.saved'));
  };

  const handleClosedLoopChange = (closedLoopEnabled: boolean) => {
    saveSettings({ closed_loop_enabled: closedLoopEnabled });
    message.success(t('settings.scheduler.saved'));
  };

  const handleClosedLoopIntervalChange = (closedLoopIntervalMinutes: number) => {
    saveSettings({ closed_loop_interval_minutes: closedLoopIntervalMinutes });
    message.success(t('settings.scheduler.saved'));
  };

  const webdavIntervalOptions = [
    { value: 15, label: t('settings.scheduler.minutes', { count: 15 }) },
    { value: 30, label: t('settings.scheduler.minutes', { count: 30 }) },
    { value: 60, label: t('settings.scheduler.hour') },
    { value: 120, label: t('settings.scheduler.hours', { count: 2 }) },
    { value: 360, label: t('settings.scheduler.hours', { count: 6 }) },
    { value: 720, label: t('settings.scheduler.hours', { count: 12 }) },
    { value: 1440, label: t('settings.scheduler.hours', { count: 24 }) },
  ];

  const closedLoopIntervalOptions = [
    { value: 1, label: t('settings.scheduler.minutes', { count: 1 }) },
    { value: 5, label: t('settings.scheduler.minutes', { count: 5 }) },
    { value: 10, label: t('settings.scheduler.minutes', { count: 10 }) },
    { value: 15, label: t('settings.scheduler.minutes', { count: 15 }) },
    { value: 30, label: t('settings.scheduler.minutes', { count: 30 }) },
    { value: 60, label: t('settings.scheduler.hour') },
  ];

  return (
    <div>
      {/* Auto Backup Scheduler */}
      <SettingsGroup title={t('settings.scheduler.autoBackup')}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t('settings.scheduler.enabled')}</span>
          <Switch
            checked={backupSettings?.enabled ?? false}
            onChange={handleAutoBackupChange}
          />
        </div>
        <Divider style={{ margin: '4px 0' }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t('settings.scheduler.backupInterval')}</span>
          <div className="flex items-center gap-2">
            <InputNumber
              min={1}
              max={720}
              value={backupSettings?.intervalHours ?? 24}
              onChange={handleBackupIntervalChange}
              style={{ width: 80 }}
            />
            <span style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>
              {t('settings.scheduler.hoursUnit')}
            </span>
          </div>
        </div>
        <Divider style={{ margin: '4px 0' }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t('settings.scheduler.maxCount')}</span>
          <InputNumber
            min={1}
            max={100}
            value={backupSettings?.maxCount ?? 10}
            onChange={async (v) => {
              if (!backupSettings || !v) return;
              await updateBackupSettings({ ...backupSettings, maxCount: v });
            }}
            style={{ width: 80 }}
          />
        </div>
        {backupSettings?.enabled && (
          <>
            <Divider style={{ margin: '4px 0' }} />
            <div style={rowStyle} className="flex items-center justify-between">
              <span style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>
                {t('settings.scheduler.status')}
              </span>
              <Tag color="green">{t('settings.scheduler.running')}</Tag>
            </div>
          </>
        )}
      </SettingsGroup>

      {/* WebDAV Sync Scheduler */}
      <SettingsGroup title={t('settings.scheduler.webdavSync')}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t('settings.scheduler.enabled')}</span>
          <Switch
            checked={settings.webdav_sync_enabled ?? false}
            onChange={handleWebdavSyncChange}
          />
        </div>
        <Divider style={{ margin: '4px 0' }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t('settings.scheduler.syncInterval')}</span>
          <Select
            value={settings.webdav_sync_interval_minutes ?? 60}
            options={webdavIntervalOptions}
            onChange={handleWebdavIntervalChange}
            style={{ width: 120 }}
          />
        </div>
        <Divider style={{ margin: '4px 0' }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t('settings.scheduler.maxRemoteBackups')}</span>
          <InputNumber
            min={1}
            max={100}
            value={settings.webdav_max_remote_backups ?? 10}
            onChange={(v) => v && saveSettings({ webdav_max_remote_backups: v })}
            style={{ width: 80 }}
          />
        </div>
        {settings.webdav_sync_enabled && (
          <>
            <Divider style={{ margin: '4px 0' }} />
            <div style={rowStyle} className="flex items-center justify-between">
              <span style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>
                {t('settings.scheduler.status')}
              </span>
              <Tag color="green">{t('settings.scheduler.running')}</Tag>
            </div>
          </>
        )}
      </SettingsGroup>

      {/* Closed-Loop Nudge Scheduler */}
      <SettingsGroup title={t('settings.scheduler.closedLoop')}>
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t('settings.scheduler.enabled')}</span>
          <Switch
            checked={settings.closed_loop_enabled ?? true}
            onChange={handleClosedLoopChange}
          />
        </div>
        <Divider style={{ margin: '4px 0' }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <span>{t('settings.scheduler.nudgeInterval')}</span>
          <Select
            value={settings.closed_loop_interval_minutes ?? 5}
            options={closedLoopIntervalOptions}
            onChange={handleClosedLoopIntervalChange}
            style={{ width: 120 }}
          />
        </div>
        {(settings.closed_loop_enabled ?? true) && (
          <>
            <Divider style={{ margin: '4px 0' }} />
            <div style={rowStyle} className="flex items-center justify-between">
              <span style={{ fontSize: 12, color: 'var(--color-text-secondary)' }}>
                {t('settings.scheduler.status')}
              </span>
              <Tag color="blue">{t('settings.scheduler.running')}</Tag>
            </div>
          </>
        )}
      </SettingsGroup>
    </div>
  );
}
