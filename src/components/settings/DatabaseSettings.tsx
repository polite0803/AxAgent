// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, logIpcError } from "@/lib/invoke";
import { Alert, Button, Card, Form, Input, InputNumber, message, Radio, Space, Switch, Typography } from "antd";
import { useEffect, useState } from "react";
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

export function DatabaseSettings() {
  const { t } = useTranslation();
  const [form] = Form.useForm<DbConfigForm>();
  const [loading, setLoading] = useState(false);
  const [initialLoading, setInitialLoading] = useState(true);
  const [testing, setTesting] = useState(false);
  const dbType = Form.useWatch("db_type", form);

  useEffect(() => {
    invoke<DbConfigForm>("get_db_config")
      .then((cfg) => {
        form.setFieldsValue(cfg);
      })
      .catch(logIpcError("get_db_config"))
      .finally(() => setInitialLoading(false));
  }, [form]);

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
        message={t("settings.database.restartHint")}
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
                message={t("settings.database.pgNote")}
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
    </Card>
  );
}
