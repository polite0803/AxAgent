// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { Alert, App, Button, Card, Form, Input, InputNumber, Radio, Space, Switch, Typography } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface DbConfigForm {
  db_type: "sqlite" | "postgres";
  sqlite_path?: string;
  pg_host?: string;
  pg_port?: number;
  pg_database?: string;
  pg_user?: string;
  pg_password?: string;
  pg_schema?: string;
  use_ssl?: boolean;
}

// P2-9: Schema 迁移状态（与后端 axagent_dao::migrations::SchemaMigrationStatus 对齐）
interface AppliedMigration {
  version: number;
  applied_at: number;
  description: string;
}

interface SchemaMigrationStatus {
  applied_version: number;
  latest_version: number;
  pending_count: number;
  applied: AppliedMigration[];
}

export function DatabaseSettings() {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const [form] = Form.useForm<DbConfigForm>();
  const [loading, setLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);
  const [testing, setTesting] = useState(false);
  const dbType = Form.useWatch("db_type", form);

  // P2-9: Schema 迁移状态
  const [schemaStatus, setSchemaStatus] = useState<SchemaMigrationStatus | null>(null);
  const [schemaLoading, setSchemaLoading] = useState(false);
  const [schemaError, setSchemaError] = useState<string | null>(null);
  const [repairing, setRepairing] = useState(false);

  const refreshSchemaStatus = useCallback(() => {
    setSchemaLoading(true);
    setSchemaError(null);
    invoke<SchemaMigrationStatus>("get_schema_status")
      .then((status) => {
        setSchemaStatus(status);
      })
      .catch((e) => {
        const msg = e instanceof Error ? e.message : String(e);
        setSchemaError(msg);
        logIpcError("get_schema_status")(e);
      })
      .finally(() => setSchemaLoading(false));
  }, []);

  const handleRepairSchema = useCallback(async () => {
    setRepairing(true);
    setSchemaError(null);
    try {
      const addedStr = await invoke<string>("repair_schema");
      const count = Number(addedStr);
      message.success(t("settings.database.schemaRepairSuccess", { count }));
      await refreshSchemaStatus();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setSchemaError(msg);
      logIpcError("repair_schema")(e);
    } finally {
      setRepairing(false);
    }
  }, [refreshSchemaStatus, t, message]);

  useEffect(() => {
    invoke<DbConfigForm>("get_db_config")
      .then((cfg) => {
        form.setFieldsValue(cfg);
      })
      .catch(logIpcError("get_db_config"))
      .finally(() => setInitialLoading(false));
    // P2-9: 同时加载 schema 状态
    refreshSchemaStatus();
  }, [form, refreshSchemaStatus]);

  const handleSave = async () => {
    setLoading(true);
    try {
      const values = await form.validateFields();
      await invoke("save_db_config", { config: values });
      message.success(t("settings.database.saved"));
    } catch (e) {
      logIpcError("save_db_config")(e);
    } finally {
      setLoading(false);
    }
  };

  const handleTest = async () => {
    setTesting(true);
    try {
      const values = await form.validateFields();
      const result = await invoke<string>("test_db_connection", { config: values });
      message.success(result || t("settings.database.testSuccess"));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      message.error(t("settings.database.testFailed", { error: msg }));
    } finally {
      setTesting(false);
    }
  };

  if (initialLoading) {
    return (
      <Card title={t("settings.database.title")} style={{ marginBottom: 16 }}>
        <Typography.Text>{t("settings.database.loading")}</Typography.Text>
      </Card>
    );
  }

  return (
    <Card
      title={t("settings.database.title")}
      style={{ marginBottom: 16 }}
      extra={
        <Space>
          <Button onClick={handleTest} loading={testing}>
            {t("settings.database.testButton")}
          </Button>
          <Button type="primary" onClick={handleSave} loading={loading}>
            {t("settings.database.saveButton")}
          </Button>
        </Space>
      }
    >
      <Alert
        type="info"
        showIcon
        style={{ marginBottom: 16 }}
        title={t("settings.database.restartHint")}
      />

      <Form
        form={form}
        layout="vertical"
        initialValues={{ db_type: "sqlite", pg_port: 5432, use_ssl: false }}
      >
        <Form.Item name="db_type" label={t("settings.database.typeLabel")}>
          <Radio.Group>
            <Radio value="sqlite">{t("settings.database.typeSqlite")}</Radio>
            <Radio value="postgres">{t("settings.database.typePostgres")}</Radio>
          </Radio.Group>
        </Form.Item>

        {dbType === "sqlite"
          ? (
            <Form.Item
              name="sqlite_path"
              label={t("settings.database.sqlitePathLabel")}
              tooltip={t("settings.database.sqliteNote")}
            >
              <Input placeholder={t("settings.database.sqlitePathPlaceholder")} />
            </Form.Item>
          )
          : (
            <>
              <Alert
                type="warning"
                showIcon
                style={{ marginBottom: 16 }}
                title={t("settings.database.pgNote")}
              />
              <Form.Item
                name="pg_host"
                label={t("settings.database.pgHostLabel")}
              >
                <Input placeholder={t("settings.database.pgHostPlaceholder")} />
              </Form.Item>
              <Form.Item
                name="pg_port"
                label={t("settings.database.pgPortLabel")}
              >
                <InputNumber min={1} max={65535} style={{ width: "100%" }} />
              </Form.Item>
              <Form.Item
                name="pg_database"
                label={t("settings.database.pgDatabaseLabel")}
              >
                <Input placeholder={t("settings.database.pgDatabasePlaceholder")} />
              </Form.Item>
              <Form.Item
                name="pg_user"
                label={t("settings.database.pgUserLabel")}
              >
                <Input placeholder={t("settings.database.pgUserPlaceholder")} />
              </Form.Item>
              <Form.Item
                name="pg_password"
                label={t("settings.database.pgPasswordLabel")}
              >
                <Input.Password
                  placeholder={t("settings.database.pgPasswordPlaceholder")}
                />
              </Form.Item>
              <Form.Item
                name="pg_schema"
                label={t("settings.database.pgSchemaLabel")}
              >
                <Input placeholder={t("settings.database.pgSchemaPlaceholder")} />
              </Form.Item>
              <Form.Item
                name="use_ssl"
                label={t("settings.database.useSslLabel")}
                valuePropName="checked"
              >
                <Switch />
              </Form.Item>
            </>
          )}
      </Form>

      {/* P2-9: Schema 迁移状态展示 */}
      <Card
        size="small"
        title={t("settings.database.schemaStatusTitle")}
        style={{ marginTop: 16 }}
        extra={
          <Space>
            <Button
              size="small"
              type="primary"
              danger
              loading={repairing}
              onClick={handleRepairSchema}
            >
              {repairing ? t("settings.database.schemaRepairRunning") : t("settings.database.schemaRepairButton")}
            </Button>
            <Button
              size="small"
              onClick={refreshSchemaStatus}
              loading={schemaLoading}
            >
              {t("settings.database.schemaStatusRefresh")}
            </Button>
          </Space>
        }
      >
        {schemaError
          ? (
            <Alert
              type="error"
              showIcon
              title={t("settings.database.schemaStatusError", { error: schemaError })}
            />
          )
          : schemaStatus
          ? (
            <Alert
              type={schemaStatus.pending_count > 0 ? "warning" : "success"}
              showIcon
              title={schemaStatus.pending_count > 0
                ? t("settings.database.schemaStatusPending", {
                  applied: schemaStatus.applied_version,
                  count: schemaStatus.pending_count,
                })
                : t("settings.database.schemaStatusUpToDate", {
                  version: schemaStatus.latest_version,
                })}
            />
          )
          : (
            <Typography.Text type="secondary">
              {t("settings.database.schemaStatusLoadFailed")}
            </Typography.Text>
          )}
      </Card>
    </Card>
  );
}
