import { useGatewayLinkStore } from "@/stores";
import type { GatewayLink } from "@/types";
import { App, Button, Card, Empty, Switch, Table, Tag } from "antd";
import type { Key } from "antd/es/table/interface";
import { RefreshCw, Upload } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface LinkModelsProps {
  link: GatewayLink;
}

export function LinkModels({ link }: LinkModelsProps) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const modelSyncs = useGatewayLinkStore((s) => s.modelSyncs);
  const pushModels = useGatewayLinkStore((s) => s.pushModels);
  const syncAllModels = useGatewayLinkStore((s) => s.syncAllModels);
  const fetchModelSyncs = useGatewayLinkStore((s) => s.fetchModelSyncs);
  const updateSyncSettings = useGatewayLinkStore((s) => s.updateSyncSettings);

  const handleAutoSyncChange = async (checked: boolean) => {
    try {
      await updateSyncSettings(link.id, checked, link.auto_sync_skills);
    } catch {
      message.error(t("link.updateSettingsFailed"));
    }
  };

  const handleSyncAll = async () => {
    try {
      await syncAllModels(link.id);
      message.success(t("link.syncAllSuccess"));
    } catch {
      message.error(t("link.syncAllFailed"));
    }
  };

  const handlePushSelected = async () => {
    if (selectedRowKeys.length === 0) {
      message.warning(t("link.noModelSelected"));
      return;
    }
    try {
      await pushModels(link.id, selectedRowKeys);
      message.success(t("link.pushModelsSuccess"));
      setSelectedRowKeys([]);
    } catch {
      message.error(t("link.pushModelsFailed"));
    }
  };

  const SYNC_STATUS_MAP: Record<string, { color: string; label: string }> = {
    synced: { color: "green", label: t("link.syncStatusSynced") },
    pending: { color: "orange", label: t("link.syncStatusPending") },
    failed: { color: "red", label: t("link.syncStatusFailed") },
    not_selected: { color: "default", label: t("link.syncStatusNotSelected") },
  };

  const columns = [
    {
      title: t("link.modelName"),
      dataIndex: "model_id",
      key: "model_id",
      ellipsis: true,
    },
    {
      title: t("link.providerName"),
      dataIndex: "provider_name",
      key: "provider_name",
      ellipsis: true,
    },
    {
      title: t("link.syncStatus"),
      dataIndex: "sync_status",
      key: "sync_status",
      width: 120,
      render: (status: string) => {
        const mapped = SYNC_STATUS_MAP[status] ?? { color: "default", label: status };
        return <Tag color={mapped.color}>{mapped.label}</Tag>;
      },
    },
    {
      title: t("link.lastSync"),
      dataIndex: "last_sync_at",
      key: "last_sync_at",
      width: 160,
      render: (v: number | null) => v ? new Date(v * 1000).toLocaleString() : "-",
    },
    {
      title: t("link.actions"),
      key: "actions",
      width: 100,
      render: (_: unknown) => (
        <Button
          size="small"
          icon={<Upload size={14} />}
          onClick={() => handlePushSelected()}
          disabled={link.status !== "connected"}
        >
          {t("link.push")}
        </Button>
      ),
    },
  ];

  const rowSelection = {
    selectedRowKeys,
    onChange: (keys: Key[]) => setSelectedRowKeys(keys as string[]),
    getCheckboxProps: () => ({
      disabled: link.status !== "connected",
    }),
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Switch
            size="small"
            checked={link.auto_sync_models}
            onChange={handleAutoSyncChange}
            disabled={link.status !== "connected"}
          />
          <span style={{ fontSize: 13 }}>{t("link.autoSyncModels")}</span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            icon={<RefreshCw size={14} />}
            onClick={() => fetchModelSyncs(link.id)}
          >
            {t("common.refresh")}
          </Button>
          <Button
            icon={<Upload size={14} />}
            onClick={handlePushSelected}
            disabled={link.status !== "connected" || selectedRowKeys.length === 0}
          >
            {t("link.pushSelected")} {selectedRowKeys.length > 0 && `(${selectedRowKeys.length})`}
          </Button>
          <Button
            type="primary"
            icon={<Upload size={14} />}
            onClick={handleSyncAll}
            disabled={link.status !== "connected"}
          >
            {t("link.syncAllModels")}
          </Button>
        </div>
      </div>

      <Card size="small">
        {modelSyncs.length === 0
          ? <Empty description={t("link.noModels")} image={Empty.PRESENTED_IMAGE_SIMPLE} />
          : (
            <Table
              dataSource={modelSyncs}
              columns={columns}
              rowKey="model_id"
              size="small"
              pagination={false}
              rowSelection={rowSelection}
            />
          )}
      </Card>
    </div>
  );
}
