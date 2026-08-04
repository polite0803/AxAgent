// SPDX-License-Identifier: AGPL-3.0-only

import { useDeviceSyncStore } from "@/stores";
import type { AuditLogEntry, SyncHistoryEntry } from "@/types";
import { FileSearchOutlined, HistoryOutlined, ReloadOutlined } from "@ant-design/icons";
import { Button, Card, Select, Space, Table, Tabs, Tag, Typography } from "antd";
import type { TableProps } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

type HistoryColumns = TableProps<SyncHistoryEntry>["columns"];
type AuditColumns = TableProps<AuditLogEntry>["columns"];

/** 同步历史记录面板 */
export function SyncHistoryPanel() {
  const { t } = useTranslation();
  const deviceSyncStore = useDeviceSyncStore();
  const [historyLoading, setHistoryLoading] = useState(false);
  const [auditLoading, setAuditLoading] = useState(false);
  const [filterAction, setFilterAction] = useState<string | undefined>();

  useEffect(() => {
    loadHistory();
    loadAuditLogs();
  }, []);

  const loadHistory = async () => {
    setHistoryLoading(true);
    try {
      await deviceSyncStore.loadSyncHistory(50);
    } finally {
      setHistoryLoading(false);
    }
  };

  const loadAuditLogs = async () => {
    setAuditLoading(true);
    try {
      await deviceSyncStore.loadAuditLogs(100);
    } finally {
      setAuditLoading(false);
    }
  };

  const historyColumns: HistoryColumns = [
    {
      title: t("deviceSync.deviceId"),
      dataIndex: "device_id",
      key: "device_id",
      ellipsis: true,
    },
    {
      title: t("deviceSync.direction"),
      dataIndex: "direction",
      key: "direction",
      width: 100,
      render: (dir: string) => {
        const colorMap: Record<string, string> = {
          push: "blue",
          pull: "green",
          both: "purple",
        };
        return <Tag color={colorMap[dir] || "default"}>{dir}</Tag>;
      },
    },
    {
      title: t("deviceSync.syncType"),
      dataIndex: "sync_type",
      key: "sync_type",
      width: 100,
      render: (type: string) => {
        const colorMap: Record<string, string> = {
          full: "red",
          incremental: "blue",
          manual: "orange",
          scheduled: "cyan",
        };
        return <Tag color={colorMap[type] || "default"}>{type}</Tag>;
      },
    },
    {
      title: t("deviceSync.status"),
      dataIndex: ["result", "success"],
      key: "status",
      width: 80,
      render: (success: boolean) => (
        <Tag color={success ? "green" : "red"}>
          {success ? t("deviceSync.success") : t("deviceSync.failed")}
        </Tag>
      ),
    },
    {
      title: t("deviceSync.conflicts"),
      dataIndex: "conflicts",
      key: "conflicts",
      width: 80,
      render: (conflicts: SyncHistoryEntry["conflicts"]) =>
        conflicts.length > 0 ? <Tag color="orange">{conflicts.length}</Tag> : <Text type="secondary">0</Text>,
    },
    {
      title: t("deviceSync.duration"),
      dataIndex: ["result", "duration_ms"],
      key: "duration",
      width: 100,
      render: (ms: number) => <Text>{ms ? `${ms}ms` : "-"}</Text>,
    },
    {
      title: t("deviceSync.completedAt"),
      dataIndex: "completed_at",
      key: "completed_at",
      width: 180,
      render: (date: string) => <Text type="secondary">{new Date(date).toLocaleString()}</Text>,
    },
  ];

  const auditColumns: AuditColumns = [
    {
      title: t("deviceSync.timestamp"),
      dataIndex: "timestamp",
      key: "timestamp",
      width: 180,
      render: (date: string) => <Text type="secondary">{new Date(date).toLocaleString()}</Text>,
    },
    {
      title: t("deviceSync.action"),
      dataIndex: "action",
      key: "action",
      width: 140,
      render: (action: string) => {
        const colorMap: Record<string, string> = {
          device_registered: "blue",
          device_paired: "green",
          device_unpaired: "red",
          sync_started: "cyan",
          sync_completed: "green",
          sync_failed: "red",
          conflict_detected: "orange",
          conflict_resolved: "purple",
          policy_updated: "geekblue",
          permission_changed: "magenta",
          encryption_enabled: "gold",
          encryption_disabled: "default",
        };
        return <Tag color={colorMap[action] || "default"}>{action}</Tag>;
      },
    },
    {
      title: t("deviceSync.entityType"),
      dataIndex: "entity_type",
      key: "entity_type",
      width: 120,
    },
    {
      title: t("deviceSync.entityId"),
      dataIndex: "entity_id",
      key: "entity_id",
      ellipsis: true,
      render: (id: string) => <Text code>{id}</Text>,
    },
    {
      title: t("deviceSync.deviceId"),
      dataIndex: "device_id",
      key: "device_id",
      ellipsis: true,
    },
    {
      title: t("deviceSync.status"),
      dataIndex: "success",
      key: "success",
      width: 80,
      render: (success: boolean) => (
        <Tag color={success ? "green" : "red"}>
          {success ? t("deviceSync.success") : t("deviceSync.failed")}
        </Tag>
      ),
    },
    {
      title: t("deviceSync.details"),
      dataIndex: "details",
      key: "details",
      ellipsis: true,
      render: (details: string | null) => details ? <Text code>{details}</Text> : <Text type="secondary">-</Text>,
    },
  ];

  const filteredAuditLogs = filterAction
    ? deviceSyncStore.auditLogs.filter((log) => log.action === filterAction)
    : deviceSyncStore.auditLogs;

  return (
    <Card
      title={
        <Space>
          <HistoryOutlined />
          <span>{t("deviceSync.syncHistoryAndAudit")}</span>
        </Space>
      }
      style={{ marginBottom: 16 }}
      extra={
        <Space>
          <Button
            icon={<ReloadOutlined />}
            size="small"
            onClick={() => {
              loadHistory();
              loadAuditLogs();
            }}
          >
            {t("common.refresh")}
          </Button>
        </Space>
      }
    >
      <Tabs
        items={[
          {
            key: "history",
            label: (
              <Space>
                <FileSearchOutlined />
                {t("deviceSync.syncHistory")}
              </Space>
            ),
            children: (
              <Table
                columns={historyColumns}
                dataSource={deviceSyncStore.syncHistory}
                loading={historyLoading}
                rowKey="id"
                size="small"
                pagination={{ pageSize: 10, showSizeChanger: false }}
              />
            ),
          },
          {
            key: "audit",
            label: (
              <Space>
                <HistoryOutlined />
                {t("deviceSync.auditLogs")}
              </Space>
            ),
            children: (
              <>
                <Space style={{ marginBottom: 12 }}>
                  <Text>{t("deviceSync.filterByAction")}:</Text>
                  <Select
                    allowClear
                    placeholder={t("deviceSync.allActions")}
                    style={{ width: 200 }}
                    value={filterAction}
                    onChange={setFilterAction}
                  >
                    <Select.Option value="device_registered">
                      {t("deviceSync.action_device_registered")}
                    </Select.Option>
                    <Select.Option value="device_paired">
                      {t("deviceSync.action_device_paired")}
                    </Select.Option>
                    <Select.Option value="sync_completed">
                      {t("deviceSync.action_sync_completed")}
                    </Select.Option>
                    <Select.Option value="sync_failed">
                      {t("deviceSync.action_sync_failed")}
                    </Select.Option>
                    <Select.Option value="conflict_detected">
                      {t("deviceSync.action_conflict_detected")}
                    </Select.Option>
                    <Select.Option value="conflict_resolved">
                      {t("deviceSync.action_conflict_resolved")}
                    </Select.Option>
                    <Select.Option value="policy_updated">
                      {t("deviceSync.action_policy_updated")}
                    </Select.Option>
                    <Select.Option value="permission_changed">
                      {t("deviceSync.action_permission_changed")}
                    </Select.Option>
                  </Select>
                </Space>
                <Table
                  columns={auditColumns}
                  dataSource={filteredAuditLogs}
                  loading={auditLoading}
                  rowKey="id"
                  size="small"
                  pagination={{ pageSize: 10, showSizeChanger: false }}
                />
              </>
            ),
          },
        ]}
      />
    </Card>
  );
}
