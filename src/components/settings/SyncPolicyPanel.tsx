// SPDX-License-Identifier: AGPL-3.0-only

import { useDeviceSyncStore } from "@/stores";
import type { ConflictResolutionStrategy, EntityType, SyncPolicyUpdate } from "@/types";
import { SaveOutlined, SettingOutlined } from "@ant-design/icons";
import { Alert, Button, Card, Checkbox, Col, Form, InputNumber, message, Row, Select, Space, Switch, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** 实体类型选项 */
const ENTITY_TYPE_OPTIONS: { value: EntityType; label: string }[] = [
  { value: "conversation", label: "会话" },
  { value: "message", label: "消息" },
  { value: "setting", label: "设置" },
  { value: "file", label: "文件" },
  { value: "wiki", label: "Wiki" },
  { value: "knowledge", label: "知识库" },
  { value: "agent", label: "智能体" },
  { value: "workflow", label: "工作流" },
];

/** 冲突解决策略选项 */
const STRATEGY_OPTIONS: { value: ConflictResolutionStrategy; label: string }[] = [
  { value: "last_write_wins", label: "最后写入胜出" },
  { value: "keep_local", label: "保留本地" },
  { value: "keep_remote", label: "保留远程" },
  { value: "keep_both", label: "保留双方" },
];

/** 同步策略配置面板 */
export function SyncPolicyPanel() {
  const { t } = useTranslation();
  const deviceSyncStore = useDeviceSyncStore();
  const [form] = Form.useForm();
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    deviceSyncStore.loadSyncPolicy();
  }, []);

  useEffect(() => {
    if (deviceSyncStore.syncPolicy) {
      form.setFieldsValue({
        conflict_strategy: deviceSyncStore.syncPolicy.conflict_strategy,
        auto_sync_interval_secs: deviceSyncStore.syncPolicy.auto_sync_interval_secs,
        sync_scope: deviceSyncStore.syncPolicy.sync_scope,
        auto_resolve_conflicts: deviceSyncStore.syncPolicy.auto_resolve_conflicts,
        max_conflict_threshold: deviceSyncStore.syncPolicy.max_conflict_threshold,
        change_log_retention_enabled: deviceSyncStore.syncPolicy.change_log_retention_enabled,
        change_log_retention_days: deviceSyncStore.syncPolicy.change_log_retention_days,
        enabled: deviceSyncStore.syncPolicy.enabled,
      });
    }
  }, [deviceSyncStore.syncPolicy]);

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      setSaving(true);

      const update: SyncPolicyUpdate = {
        conflict_strategy: values.conflict_strategy,
        auto_sync_interval_secs: values.auto_sync_interval_secs,
        sync_scope: values.sync_scope,
        auto_resolve_conflicts: values.auto_resolve_conflicts,
        max_conflict_threshold: values.max_conflict_threshold,
        change_log_retention_enabled: values.change_log_retention_enabled,
        change_log_retention_days: values.change_log_retention_days,
        enabled: values.enabled,
      };

      await deviceSyncStore.updateSyncPolicy(update);
      message.success(t("deviceSync.policySaved"));
    } catch (e) {
      message.error(t("deviceSync.policySaveFailed"));
    } finally {
      setSaving(false);
    }
  };

  const currentPolicy = deviceSyncStore.syncPolicy;

  return (
    <Card
      title={
        <Space>
          <SettingOutlined />
          <span>{t("deviceSync.syncPolicy")}</span>
        </Space>
      }
      style={{ marginBottom: 16 }}
      extra={
        <Space>
          {currentPolicy && (
            <Tag color={currentPolicy.enabled ? "green" : "default"}>
              {currentPolicy.enabled
                ? t("deviceSync.enabled")
                : t("deviceSync.disabled")}
            </Tag>
          )}
          <Button
            type="primary"
            icon={<SaveOutlined />}
            loading={saving}
            onClick={handleSave}
          >
            {t("common.save")}
          </Button>
        </Space>
      }
    >
      {!currentPolicy
        ? (
          <Alert
            message={t("deviceSync.noPolicy")}
            description={t("deviceSync.noPolicyDescription")}
            type="info"
            showIcon
            style={{ marginBottom: 16 }}
          />
        )
        : (
          <Form form={form} layout="vertical" initialValues={{}}>
            <Row gutter={24}>
              <Col span={12}>
                <Form.Item
                  label={t("deviceSync.conflictStrategy")}
                  name="conflict_strategy"
                  rules={[{ required: true }]}
                >
                  <Select>
                    {STRATEGY_OPTIONS.map((opt) => (
                      <Select.Option key={opt.value} value={opt.value}>
                        {opt.label}
                      </Select.Option>
                    ))}
                  </Select>
                </Form.Item>

                <Form.Item
                  label={t("deviceSync.autoSyncInterval")}
                  name="auto_sync_interval_secs"
                  tooltip={t("deviceSync.autoSyncIntervalTooltip")}
                >
                  <InputNumber min={0} step={60} style={{ width: "100%" }} />
                </Form.Item>

                <Form.Item
                  label={t("deviceSync.maxConflictThreshold")}
                  name="max_conflict_threshold"
                  tooltip={t("deviceSync.maxConflictThresholdTooltip")}
                >
                  <InputNumber min={1} step={10} style={{ width: "100%" }} />
                </Form.Item>
              </Col>

              <Col span={12}>
                <Form.Item
                  label={t("deviceSync.autoResolveConflicts")}
                  name="auto_resolve_conflicts"
                  valuePropName="checked"
                >
                  <Switch />
                </Form.Item>

                <Form.Item
                  label={t("deviceSync.enabled")}
                  name="enabled"
                  valuePropName="checked"
                >
                  <Switch />
                </Form.Item>

                <Form.Item
                  label={t("deviceSync.changeLogRetention")}
                  name="change_log_retention_enabled"
                  valuePropName="checked"
                >
                  <Switch />
                </Form.Item>

                <Form.Item
                  label={t("deviceSync.retentionDays")}
                  name="change_log_retention_days"
                  dependencies={["change_log_retention_enabled"]}
                >
                  <InputNumber min={1} max={365} style={{ width: "100%" }} />
                </Form.Item>
              </Col>
            </Row>

            <Form.Item
              label={t("deviceSync.syncScope")}
              name="sync_scope"
              rules={[{ required: true }]}
            >
              <Checkbox.Group>
                <Row gutter={[16, 8]}>
                  {ENTITY_TYPE_OPTIONS.map((opt) => (
                    <Col span={8} key={opt.value}>
                      <Checkbox value={opt.value}>{opt.label}</Checkbox>
                    </Col>
                  ))}
                </Row>
              </Checkbox.Group>
            </Form.Item>
          </Form>
        )}
    </Card>
  );
}
