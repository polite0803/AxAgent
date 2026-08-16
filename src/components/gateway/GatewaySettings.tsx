// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { showBackendError } from "@/lib/errorI18n";
import { invoke } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { useGatewayStore, useSettingsStore } from "@/stores";
import { open } from "@tauri-apps/plugin-dialog";
import { Alert, Button, Card, Divider, Input, InputNumber, Radio, Switch, theme } from "antd";
import { Info, ShieldAlert, Upload as UploadIcon } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

interface CertResult {
  cert_path: string;
  key_path: string;
}

export function GatewaySettings() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const { settings, fetchSettings, saveSettings } = useSettingsStore();
  const { status, fetchStatus, stopGateway } = useGatewayStore();

  useEffect(() => {
    fetchSettings();
    fetchStatus();
    const interval = window.setInterval(() => {
      fetchStatus();
    }, 5000);

    return () => window.clearInterval(interval);
  }, [fetchSettings, fetchStatus]);

  const settingsLocked = status.isRunning;

  const handleSave = useCallback(async (partial: Parameters<typeof saveSettings>[0]) => {
    try {
      await saveSettings(partial);
    } catch (e) {
      showBackendError(message, e);
    }
  }, [saveSettings]);

  const handleSelectFile = async (
    field: "gateway_ssl_cert_path" | "gateway_ssl_key_path",
  ) => {
    if (settingsLocked) {
      return;
    }
    try {
      const selected = await open({
        multiple: false,
        filters: field === "gateway_ssl_cert_path"
          ? [{ name: "Certificate", extensions: ["pem", "crt", "cer"] }]
          : [{ name: "Private Key", extensions: ["pem", "key"] }],
      });
      if (selected) {
        await handleSave({ [field]: selected });
      }
    } catch (e) {
      showBackendError(message, e);
    }
  };

  const handleGenerateSelfSigned = async () => {
    if (settingsLocked) {
      return;
    }
    try {
      const result = await invoke<CertResult>("generate_self_signed_cert");
      await handleSave({
        gatewaySslCertPath: result.cert_path,
        gatewaySslKeyPath: result.key_path,
      });
      message.success(t("gateway.sslGenerateSuccess"));
    } catch (e) {
      message.error(t("gateway.sslGenerateFailed") + ": " + String(e));
    }
  };

  const [sslPortValue, setSslPortValue] = useState<number>(
    settings.gatewaySslPort ?? 8443,
  );
  const [sslPortError, setSslPortError] = useState(false);
  const [portValue, setPortValue] = useState<number>(
    settings.gatewayPort ?? 8080,
  );
  const [portError, setPortError] = useState(false);
  const [listenAddressValue, setListenAddressValue] = useState<string>(
    settings.gatewayListenAddress ?? "127.1.0.0",
  );

  // Track port values that were blocked from saving due to a conflict so they
  // can be persisted as soon as the conflict is resolved.
  const portPendingSave = useRef(false);
  const sslPortPendingSave = useRef(false);

  // Each field syncs independently so an unsaved local edit in one field is never
  // clobbered when the other field's persisted value changes.
  useEffect(() => {
    setTimeout(() => setPortValue(settings.gatewayPort ?? 8080), 0);
  }, [settings.gatewayPort, setPortValue]);

  useEffect(() => {
    setTimeout(() => setSslPortValue(settings.gatewaySslPort ?? 8443), 0);
  }, [settings.gatewaySslPort, setSslPortValue]);

  useEffect(() => {
    setTimeout(() => setListenAddressValue(settings.gatewayListenAddress ?? "127.1.0.0"), 0);
  }, [settings.gatewayListenAddress, setListenAddressValue]);

  const handleSaveRef = useRef(handleSave);

  useEffect(() => {
    handleSaveRef.current = handleSave;
  }, [handleSave]);

  // Recompute conflict errors whenever SSL is toggled or either local port value changes.
  // If a conflict clears for a port that had a pending (blocked) save, flush it now.
  useEffect(() => {
    const sslEnabled = settings.gatewaySslEnabled ?? false;
    const newPortError = sslEnabled && portValue === sslPortValue;
    const newSslPortError = sslEnabled && sslPortValue === portValue;

    if (!newPortError && portPendingSave.current) {
      portPendingSave.current = false;
      handleSaveRef.current({ gatewayPort: portValue });
    }
    if (!newSslPortError && sslPortPendingSave.current) {
      sslPortPendingSave.current = false;
      handleSaveRef.current({ gatewaySslPort: sslPortValue });
    }

    setTimeout(() => {
      setPortError(newPortError);
      setSslPortError(newSslPortError);
    }, 0);
  }, [settings.gatewaySslEnabled, portValue, sslPortValue]);

  const handleSslPortChange = (val: number | null) => {
    if (val == null) {
      return;
    }
    setSslPortValue(val);
    if (val === portValue) {
      setSslPortError(true);
      sslPortPendingSave.current = true;
    } else {
      setSslPortError(false);
      sslPortPendingSave.current = false;
      handleSave({ gatewaySslPort: val });
    }
  };

  const handlePortChange = (val: number | null) => {
    if (val == null) {
      return;
    }
    setPortValue(val);
    if ((settings.gatewaySslEnabled ?? false) && val === sslPortValue) {
      setPortError(true);
      portPendingSave.current = true;
    } else {
      setPortError(false);
      portPendingSave.current = false;
      handleSave({ gatewayPort: val });
    }
  };

  const handleListenAddressCommit = () => {
    const trimmed = listenAddressValue.trim();
    if (trimmed) {
      handleSave({ gatewayListenAddress: trimmed });
    }
  };

  const handleStopNow = async () => {
    try {
      await stopGateway();
      message.success(t("gateway.stopped"));
    } catch (e) {
      showBackendError(message, e);
    }
  };

  return (
    <div className="p-6 pb-12">
      {settingsLocked && (
        <Alert
          type="warning"
          showIcon
          title={t("gateway.settingsLockedTitle")}
          description={t("gateway.settingsLockedDesc")}
          action={
            <Button danger size="small" onClick={handleStopNow}>
              {t("gateway.stopNow")}
            </Button>
          }
          style={{ marginBottom: 16 }}
        />
      )}

      <Card
        size="small"
        title={t("gateway.settingsService")}
        style={{ marginBottom: 16 }}
      >
        <div
          style={{ padding: "4px 0" }}
          className="flex items-center justify-between"
        >
          <span>{t("gateway.listenAddress")}</span>
          <Input
            id="gateway-settings-input-43"
            value={listenAddressValue}
            onChange={(e) => setListenAddressValue(e.target.value)}
            onBlur={handleListenAddressCommit}
            onPressEnter={handleListenAddressCommit}
            style={{ width: 200 }}
            disabled={settingsLocked}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div
          style={{ padding: "4px 0" }}
          className="flex items-center justify-between"
        >
          <span>{t("gateway.port")}</span>
          <div
            style={{
              display: "flex",
              flexDirection: "column",
              alignItems: "flex-end",
            }}
          >
            <InputNumber
              id="gateway-settings-inputnumber-44"
              value={portValue}
              onChange={handlePortChange}
              min={1}
              max={65535}
              style={{ width: 200 }}
              disabled={settingsLocked}
              status={portError ? "error" : undefined}
            />
            {portError && (
              <span
                style={{ color: token.colorError, fontSize: 12, marginTop: 4 }}
              >
                {t("gateway.sslPortConflict")}
              </span>
            )}
          </div>
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div
          style={{ padding: "4px 0" }}
          className="flex items-center justify-between"
        >
          <span>{t("gateway.autoStart")}</span>
          <Switch
            checked={settings.gatewayAutoStart ?? false}
            onChange={(checked) => handleSave({ gatewayAutoStart: checked })}
          />
        </div>
      </Card>

      <Card
        size="small"
        title={t("gateway.settingsSsl")}
        style={{ marginBottom: 16 }}
      >
        <div
          style={{ padding: "4px 0" }}
          className="flex items-center justify-between"
        >
          <div className="flex items-center gap-1">
            <span>{t("gateway.sslEnable")}</span>
            <Tooltip title={t("gateway.sslEnableTooltip")}>
              <Info
                size={12}
                style={{ color: token.colorTextSecondary, cursor: "help" }}
              />
            </Tooltip>
          </div>
          <Switch
            checked={settings.gatewaySslEnabled ?? false}
            onChange={(checked) => handleSave({ gatewaySslEnabled: checked })}
            disabled={settingsLocked}
          />
        </div>

        {settings.gatewaySslEnabled && (
          <>
            <Divider style={{ margin: "8px 0" }} />

            <div
              style={{ padding: "4px 0" }}
              className="flex items-center justify-between"
            >
              <span>{t("gateway.sslPort")}</span>
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  alignItems: "flex-end",
                }}
              >
                <InputNumber
                  id="gateway-settings-inputnumber-45"
                  value={sslPortValue}
                  onChange={handleSslPortChange}
                  min={1}
                  max={65535}
                  style={{ width: 200 }}
                  disabled={settingsLocked}
                  status={sslPortError ? "error" : undefined}
                />
                {sslPortError && (
                  <span
                    style={{
                      color: token.colorError,
                      fontSize: 12,
                      marginTop: 4,
                    }}
                  >
                    {t("gateway.sslPortConflict")}
                  </span>
                )}
              </div>
            </div>
            <Divider style={{ margin: "4px 0" }} />

            <div
              style={{ padding: "4px 0" }}
              className="flex items-center justify-between"
            >
              <div className="flex items-center gap-1">
                <span>{t("gateway.forceSsl")}</span>
                <Tooltip title={t("gateway.forceSslTooltip")}>
                  <Info
                    size={12}
                    style={{ color: token.colorTextSecondary, cursor: "help" }}
                  />
                </Tooltip>
              </div>
              <Switch
                checked={settings.gatewayForceSsl ?? false}
                onChange={(checked) => handleSave({ gatewayForceSsl: checked })}
                disabled={settingsLocked}
              />
            </div>
            <Divider style={{ margin: "8px 0" }} />

            <Alert
              type="warning"
              showIcon
              icon={<ShieldAlert size={16} />}
              title={t("gateway.sslWarning")}
              description={t("gateway.sslWarningDesc")}
              style={{ marginBottom: 12 }}
            />

            <Radio.Group
              value={settings.gatewaySslMode ?? "upload"}
              onChange={(e) => handleSave({ gatewaySslMode: e.target.value })}
              style={{ display: "flex", flexDirection: "column", gap: 12 }}
              disabled={settingsLocked}
            >
              <Radio value="upload">
                <span style={{ fontWeight: 500 }}>
                  {t("gateway.sslUpload")}
                </span>
                <div
                  style={{
                    color: token.colorTextSecondary,
                    fontSize: 12,
                    marginTop: 2,
                  }}
                >
                  {t("gateway.sslUploadDesc")}
                </div>
              </Radio>
              {(settings.gatewaySslMode ?? "upload") === "upload" && (
                <div
                  style={{
                    paddingLeft: 24,
                    display: "flex",
                    flexDirection: "column",
                    gap: 8,
                  }}
                >
                  <div className="flex items-center gap-2">
                    <span style={{ width: 80, flexShrink: 0 }}>
                      {t("gateway.sslCertFile")}
                    </span>
                    <Input
                      id="gateway-settings-input-46"
                      readOnly
                      value={settings.gatewaySslCertPath ?? ""}
                      placeholder={t("gateway.sslCertFilePlaceholder")}
                      style={{ flex: 1 }}
                      disabled={settingsLocked}
                    />
                    <Button
                      icon={<UploadIcon size={14} />}
                      onClick={() => handleSelectFile("gateway_ssl_cert_path")}
                      disabled={settingsLocked}
                    >
                      {t("gateway.sslSelectFile")}
                    </Button>
                  </div>
                  <div className="flex items-center gap-2">
                    <span style={{ width: 80, flexShrink: 0 }}>
                      {t("gateway.sslKeyFile")}
                    </span>
                    <Input
                      id="gateway-settings-input-47"
                      readOnly
                      value={settings.gatewaySslKeyPath ?? ""}
                      placeholder={t("gateway.sslKeyFilePlaceholder")}
                      style={{ flex: 1 }}
                      disabled={settingsLocked}
                    />
                    <Button
                      icon={<UploadIcon size={14} />}
                      onClick={() => handleSelectFile("gateway_ssl_key_path")}
                      disabled={settingsLocked}
                    >
                      {t("gateway.sslSelectFile")}
                    </Button>
                  </div>
                </div>
              )}

              <Radio value="selfsign">
                <span style={{ fontWeight: 500 }}>
                  {t("gateway.sslSelfSign")}
                </span>
                <div
                  style={{
                    color: token.colorTextSecondary,
                    fontSize: 12,
                    marginTop: 2,
                  }}
                >
                  {t("gateway.sslSelfSignDesc")}
                </div>
              </Radio>
              {(settings.gatewaySslMode ?? "upload") === "selfsign" && (
                <div style={{ paddingLeft: 24 }}>
                  <Alert
                    type="error"
                    showIcon
                    title={t("gateway.sslSelfSignWarning")}
                    style={{ marginBottom: 8 }}
                  />
                  <Button
                    type="primary"
                    onClick={handleGenerateSelfSigned}
                    disabled={settingsLocked}
                  >
                    {t("gateway.sslGenerateCert")}
                  </Button>
                  {settings.gatewaySslCertPath
                    && settings.gatewaySslMode === "selfsign" && (
                    <div
                      style={{
                        marginTop: 8,
                        fontSize: 12,
                        color: token.colorTextSecondary,
                      }}
                    >
                      {t("gateway.sslCertFile")}: {settings.gatewaySslCertPath}
                    </div>
                  )}
                </div>
              )}
            </Radio.Group>
          </>
        )}
      </Card>
    </div>
  );
}
