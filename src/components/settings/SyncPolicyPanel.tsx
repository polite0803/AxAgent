// SPDX-License-Identifier: AGPL-3.0-only

import { useDeviceSyncStore } from "@/stores";
import type { SyncPolicyUpdate } from "@/types";
import { SaveOutlined, SettingOutlined } from "@ant-design/icons";
import { Alert, Button, Card, Checkbox, Col, Form, InputNumber, message, Row, Select, Space, Switch, Tag } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

/** 实体类型选项 */
const getEntityTypeOptions = (t: (key: string) => string) => [
  { value: "conversation", label: t("deviceSync.entity.conversation") },
  { value: "message", label: t("deviceSync.entity.message") },
  { value: "setting", label: t("deviceSync.entity.setting") },
  { value: "file", label: t("deviceSync.entity.file") },
  { value: "wiki", label: t("deviceSync.entity.wiki") },
  { value: "knowledge", label: t("deviceSync.entity.knowledge") },
  { value: "agent", label: t("deviceSync.entity.agent") },
  { value: "workflow", label: t("deviceSync.entity.workflow") },
];

/** 冲突解决策略选项 */
const getStrategyOptions = (t: (key: string) => string) => [
  { value: "last_write_wins", label: t("deviceSync.strategy_last_write_wins") },
  { value: "keep_local", label: t("deviceSync.strategy_keep_local") },
  { value: "keep_remote", label: t("deviceSync.strategy_keep_remote") },
  { value: "keep_both", label: t("deviceSync.strategy_keep_both") },
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
        conflictStrategy: deviceSyncStore.syncPolicy.conflictStrategy,
        autoSyncIntervalSecs: deviceSyncStore.syncPolicy.autoSyncIntervalSecs,
        syncScope: deviceSyncStore.syncPolicy.syncScope,
        autoResolveConflicts: deviceSyncStore.syncPolicy.autoResolveConflicts,
        maxConflictThreshold: deviceSyncStore.syncPolicy.maxConflictThreshold,
        changeLogRetentionEnabled: deviceSyncStore.syncPolicy.changeLogRetentionEnabled,
        changeLogRetentionDays: deviceSyncStore.syncPolicy.changeLogRetentionDays,
        enabled: deviceSyncStore.syncPolicy.enabled,
      });
    }
  }, [deviceSyncStore.syncPolicy]);

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      setSaving(true);

      const update: SyncPolicyUpdate = {
        conflictStrategy: values.conflictStrategy,
        autoSyncIntervalSecs: values.autoSyncIntervalSecs,
        syncScope: values.syncScope,
        autoResolveConflicts: values.autoResolveConflicts,
        maxConflictThreshold: values.maxConflictThreshold,
        changeLogRetentionEnabled: values.changeLogRetentionEnabled,
        changeLogRetentionDays: values.changeLogRetentionDays,
        enabled: values.enabled,
      };

      await deviceSyncStore.updateSyncPolicy(update);
      message.success(t("deviceSync.policySaved"));
    } catch {
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
                  name="conflictStrategy"
                  rules={[{ required: true }]}
                >
                  <Select>
                    {getStrategyOptions(t).map((opt) => (
                      <Select.Option key={opt.value} value={opt.value}>
                        {opt.label}
                      </Select.Option>
                    ))}
                  </Select>
                </Form.Item>

                <Form.Item
                  label={t("deviceSync.autoSyncInterval")}
                  name="autoSyncIntervalSecs"
                  tooltip={t("deviceSync.autoSyncIntervalTooltip")}
                >
                  <InputNumber min={0} step={60} style={{ width: "100%" }} />
                </Form.Item>

                <Form.Item
                  label={t("deviceSync.maxConflictThreshold")}
                  name="maxConflictThreshold"
                  tooltip={t("deviceSync.maxConflictThresholdTooltip")}
                >
                  <InputNumber min={1} step={10} style={{ width: "100%" }} />
                </Form.Item>
              </Col>

              <Col span={12}>
                <Form.Item
                  label={t("deviceSync.autoResolveConflicts")}
                  name="autoResolveConflicts"
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
                  name="changeLogRetentionEnabled"
                  valuePropName="checked"
                >
                  <Switch />
                </Form.Item>

                <Form.Item
                  label={t("deviceSync.retentionDays")}
                  name="changeLogRetentionDays"
                  dependencies={["changeLogRetentionEnabled"]}
                >
                  <InputNumber min={1} max={365} style={{ width: "100%" }} />
                </Form.Item>
              </Col>
            </Row>

            <Form.Item
              label={t("deviceSync.syncScope")}
              name="syncScope"
              rules={[{ required: true }]}
            >
              <Checkbox.Group>
                <Row gutter={[16, 8]}>
                  {getEntityTypeOptions(t).map((opt) => (
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
