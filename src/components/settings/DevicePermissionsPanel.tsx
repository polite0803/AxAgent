// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";
import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Modal,
  Row,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  Typography,
  message,
} from "antd";
import {
  EditOutlined,
  SafetyCertificateOutlined,
} from "@ant-design/icons";
import { useDeviceSyncStore } from "@/stores";
import type { DevicePermissions, PermissionUpdate } from "@/types";
import { useTranslation } from "react-i18next";

const { Title, Text } = Typography;

/** 信任级别选项 */
const TRUST_LEVEL_OPTIONS = [
  { value: "backup_only", label: "仅备份", color: "default" },
  { value: "standard", label: "标准", color: "blue" },
  { value: "full", label: "完全信任", color: "green" },
];

/** 权限编辑弹窗 */
function PermissionEditModal({
  open,
  permissions,
  onClose,
  onSave,
}: {
  open: boolean;
  permissions: DevicePermissions | null;
  onClose: () => void;
  onSave: (update: PermissionUpdate) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [saving, setSaving] = useState(false);
  const [trustLevel, setTrustLevel] = useState<string>("");
  const [allowPush, setAllowPush] = useState(true);
  const [allowPull, setAllowPull] = useState(true);
  const [allowFullSync, setAllowFullSync] = useState(false);
  const [allowResolveConflicts, setAllowResolveConflicts] = useState(true);
  const [allowManageDevices, setAllowManageDevices] = useState(false);
  const [allowModifyPolicy, setAllowModifyPolicy] = useState(false);

  useEffect(() => {
    if (permissions) {
      setTrustLevel(permissions.trust_level);
      setAllowPush(permissions.allow_push);
      setAllowPull(permissions.allow_pull);
      setAllowFullSync(permissions.allow_full_sync);
      setAllowResolveConflicts(permissions.allow_resolve_conflicts);
      setAllowManageDevices(permissions.allow_manage_devices);
      setAllowModifyPolicy(permissions.allow_modify_policy);
    }
  }, [permissions]);

  const handleSave = async () => {
    if (!permissions) return;
    setSaving(true);
    try {
      const update: PermissionUpdate = {
        trust_level: trustLevel as DevicePermissions["trust_level"],
        allow_push: allowPush,
        allow_pull: allowPull,
        allow_full_sync: allowFullSync,
        allow_resolve_conflicts: allowResolveConflicts,
        allow_manage_devices: allowManageDevices,
        allow_modify_policy: allowModifyPolicy,
      };
      await onSave(update);
      onClose();
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      title={
        <Space>
          <EditOutlined />
          <span>{t("deviceSync.editPermissions")}</span>
        </Space>
      }
      onCancel={onClose}
      onOk={handleSave}
      okText={t("common.save")}
      cancelText={t("common.cancel")}
      confirmLoading={saving}
      width={560}
    >
      {permissions && (
        <>
          <Descriptions column={1} size="small" bordered style={{ marginBottom: 16 }}>
            <Descriptions.Item label={t("deviceSync.deviceId")}>
              <Text code>{permissions.device_id}</Text>
            </Descriptions.Item>
          </Descriptions>

          <Title level={5}>{t("deviceSync.trustLevel")}</Title>
          <Select
            value={trustLevel}
            onChange={setTrustLevel}
            style={{ width: "100%", marginBottom: 16 }}
          >
            {TRUST_LEVEL_OPTIONS.map((opt) => (
              <Select.Option key={opt.value} value={opt.value}>
                <Tag color={opt.color}>{opt.label}</Tag>
              </Select.Option>
            ))}
          </Select>

          <Title level={5}>{t("deviceSync.operationPermissions")}</Title>
          <Row gutter={[16, 12]}>
            <Col span={12}>
              <Space>
                <Switch checked={allowPush} onChange={setAllowPush} />
                <Text>{t("deviceSync.allowPush")}</Text>
              </Space>
            </Col>
            <Col span={12}>
              <Space>
                <Switch checked={allowPull} onChange={setAllowPull} />
                <Text>{t("deviceSync.allowPull")}</Text>
              </Space>
            </Col>
            <Col span={12}>
              <Space>
                <Switch checked={allowFullSync} onChange={setAllowFullSync} />
                <Text>{t("deviceSync.allowFullSync")}</Text>
              </Space>
            </Col>
            <Col span={12}>
              <Space>
                <Switch
                  checked={allowResolveConflicts}
                  onChange={setAllowResolveConflicts}
                />
                <Text>{t("deviceSync.allowResolveConflicts")}</Text>
              </Space>
            </Col>
            <Col span={12}>
              <Space>
                <Switch
                  checked={allowManageDevices}
                  onChange={setAllowManageDevices}
                />
                <Text>{t("deviceSync.allowManageDevices")}</Text>
              </Space>
            </Col>
            <Col span={12}>
              <Space>
                <Switch checked={allowModifyPolicy} onChange={setAllowModifyPolicy} />
                <Text>{t("deviceSync.allowModifyPolicy")}</Text>
              </Space>
            </Col>
          </Row>
        </>
      )}
    </Modal>
  );
}

/** 设备权限管理面板 */
export function DevicePermissionsPanel() {
  const { t } = useTranslation();
  const deviceSyncStore = useDeviceSyncStore();
  const [editingPermissions, setEditingPermissions] =
    useState<DevicePermissions | null>(null);
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    deviceSyncStore.listAllPermissions();
  }, []);

  const handleEdit = (permissions: DevicePermissions) => {
    setEditingPermissions(permissions);
    setModalOpen(true);
  };

  const handleSave = async (update: PermissionUpdate) => {
    if (!editingPermissions) return;
    try {
      await deviceSyncStore.updateDevicePermissions(
        editingPermissions.device_id,
        update,
      );
      message.success(t("deviceSync.permissionSaved"));
    } catch {
      message.error(t("deviceSync.permissionSaveFailed"));
    }
  };

  const permissionsList = Array.from(
    deviceSyncStore.devicePermissions.values(),
  );

  const columns = [
    {
      title: t("deviceSync.deviceId"),
      dataIndex: "device_id",
      key: "device_id",
      render: (id: string) => <Text code>{id}</Text>,
    },
    {
      title: t("deviceSync.trustLevel"),
      dataIndex: "trust_level",
      key: "trust_level",
      width: 120,
      render: (level: string) => {
        const opt = TRUST_LEVEL_OPTIONS.find((o) => o.value === level);
        return <Tag color={opt?.color || "default"}>{opt?.label || level}</Tag>;
      },
    },
    {
      title: t("deviceSync.permissions"),
      key: "permissions",
      render: (_: unknown, record: DevicePermissions) => (
        <Space size={4}>
          {record.allow_push && <Tag color="blue">Push</Tag>}
          {record.allow_pull && <Tag color="green">Pull</Tag>}
          {record.allow_full_sync && <Tag color="red">Full Sync</Tag>}
          {record.allow_resolve_conflicts && <Tag color="purple">Resolve</Tag>}
          {record.allow_manage_devices && <Tag color="orange">Manage</Tag>}
          {record.allow_modify_policy && <Tag color="cyan">Policy</Tag>}
        </Space>
      ),
    },
    {
      title: t("deviceSync.updatedAt"),
      dataIndex: "updated_at",
      key: "updated_at",
      width: 160,
      render: (date: string) => (
        <Text type="secondary">{new Date(date).toLocaleString()}</Text>
      ),
    },
    {
      title: t("common.operation"),
      key: "action",
      width: 80,
      render: (_: unknown, record: DevicePermissions) => (
        <Button
          size="small"
          icon={<EditOutlined />}
          onClick={() => handleEdit(record)}
        >
          {t("common.edit")}
        </Button>
      ),
    },
  ];

  return (
    <Card
      title={
        <Space>
          <SafetyCertificateOutlined />
          <span>{t("deviceSync.devicePermissions")}</span>
        </Space>
      }
      style={{ marginBottom: 16 }}
      extra={
        <Space>
          <Button
            size="small"
            onClick={() => deviceSyncStore.listAllPermissions()}
          >
            {t("common.refresh")}
          </Button>
        </Space>
      }
    >
      {permissionsList.length === 0 ? (
        <Alert
          message={t("deviceSync.noPermissions")}
          description={t("deviceSync.noPermissionsDescription")}
          type="info"
          showIcon
        />
      ) : (
        <Table
          columns={columns}
          dataSource={permissionsList}
          rowKey="device_id"
          size="small"
          pagination={false}
        />
      )}

      <PermissionEditModal
        open={modalOpen}
        permissions={editingPermissions}
        onClose={() => setModalOpen(false)}
        onSave={handleSave}
      />
    </Card>
  );
}