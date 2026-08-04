// SPDX-License-Identifier: AGPL-3.0-only

import { useDeviceSyncStore } from "@/stores/feature/deviceSyncStore";
import { KeyOutlined, LockOutlined, SafetyCertificateOutlined } from "@ant-design/icons";
import { Alert, Button, Card, Input, message, Select, Space, Switch, Typography } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/**
 * 加密设置面板组件
 * 用于配置端到端加密同步参数
 */
export function EncryptionSettingsPanel() {
  const { t } = useTranslation();
  const { encryption, updateEncryptionConfig } = useDeviceSyncStore();
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [testing, setTesting] = useState(false);

  const handleEncryptionToggle = (enabled: boolean) => {
    updateEncryptionConfig({ enabled });
    if (enabled) {
      message.success(t("deviceSync.encryption.enabled"));
    }
  };

  const handleAlgorithmChange = (algorithm: string) => {
    updateEncryptionConfig({
      algorithm: algorithm as "aes256_gcm",
    });
  };

  const handleKeyDerivationChange = (keyDerivation: string) => {
    updateEncryptionConfig({
      key_derivation: keyDerivation as "pre_shared_key" | "x25519",
    });
  };

  const handleTestEncryption = async () => {
    if (!password) {
      message.warning(t("deviceSync.encryption.passwordRequired"));
      return;
    }

    setTesting(true);
    try {
      const { encryptData, decryptData } = useDeviceSyncStore.getState();
      const testData = JSON.stringify({ test: "encryption-test", timestamp: Date.now() });

      const encrypted = await encryptData(testData);
      if (!encrypted) {
        message.error(t("deviceSync.encryption.testFailed"));
        return;
      }

      const decrypted = await decryptData(encrypted);
      if (decrypted === testData) {
        message.success(t("deviceSync.encryption.testSuccess"));
      } else {
        message.error(t("deviceSync.encryption.testFailed"));
      }
    } catch (e) {
      message.error(t("deviceSync.encryption.testFailed"));
    } finally {
      setTesting(false);
    }
  };

  return (
    <Card
      title={
        <Space>
          <LockOutlined />
          <span>{t("deviceSync.encryption.title")}</span>
        </Space>
      }
      style={{ marginBottom: 16 }}
    >
      <Space direction="vertical" style={{ width: "100%" }} size="large">
        {/* 加密开关 */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "12px 0",
          }}
        >
          <div>
            <Text strong>{t("deviceSync.encryption.enableLabel")}</Text>
            <div>
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("deviceSync.encryption.enableDescription")}
              </Text>
            </div>
          </div>
          <Switch
            checked={encryption.config.enabled}
            onChange={handleEncryptionToggle}
          />
        </div>

        {encryption.config.enabled && (
          <>
            <Alert
              message={t("deviceSync.encryption.enabledTitle")}
              description={t("deviceSync.encryption.enabledDescription")}
              type="info"
              showIcon
            />

            {/* 加密算法选择 */}
            <div>
              <Text strong>{t("deviceSync.encryption.algorithmLabel")}</Text>
              <Select
                value={encryption.config.algorithm}
                onChange={handleAlgorithmChange}
                style={{ width: "100%", marginTop: 8 }}
                options={[
                  {
                    value: "aes256_gcm",
                    label: (
                      <Space>
                        <SafetyCertificateOutlined />
                        {`AES-256-GCM ${t("deviceSync.encryption.recommended")}`}
                      </Space>
                    ),
                  },
                ]}
              />
            </div>

            {/* 密钥派生方式 */}
            <div>
              <Text strong>{t("deviceSync.encryption.keyDerivationLabel")}</Text>
              <Select
                value={encryption.config.key_derivation}
                onChange={handleKeyDerivationChange}
                style={{ width: "100%", marginTop: 8 }}
                options={[
                  {
                    value: "pre_shared_key",
                    label: t("deviceSync.encryption.preSharedKey"),
                  },
                  {
                    value: "x25519",
                    label: t("deviceSync.encryption.x25519"),
                  },
                ]}
              />
            </div>

            {/* 加密密码 */}
            <div>
              <Text strong>{t("deviceSync.encryption.passwordLabel")}</Text>
              <Input.Password
                prefix={<KeyOutlined />}
                placeholder={t("deviceSync.encryption.passwordPlaceholder")}
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                style={{ marginTop: 8 }}
                visibilityToggle={{
                  visible: showPassword,
                  onVisibleChange: (visible) => setShowPassword(visible),
                }}
              />
              <div style={{ marginTop: 4 }}>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  {t("deviceSync.encryption.passwordDescription")}
                </Text>
              </div>
            </div>

            {/* 密钥哈希显示 */}
            {encryption.config.key_hash && (
              <div>
                <Text strong>{t("deviceSync.encryption.keyHashLabel")}</Text>
                <Input
                  value={encryption.config.key_hash}
                  readOnly
                  style={{ marginTop: 8, fontFamily: "monospace" }}
                />
              </div>
            )}

            {/* 测试加密 */}
            <div>
              <Button
                type="primary"
                onClick={handleTestEncryption}
                loading={testing}
                disabled={!password}
              >
                {t("deviceSync.encryption.testButton")}
              </Button>
            </div>
          </>
        )}

        {/* 加密状态信息 */}
        {encryption.last_encrypted_at && (
          <Alert
            message={t("deviceSync.encryption.lastEncryptedTitle")}
            description={new Date(encryption.last_encrypted_at).toLocaleString()}
            type="success"
            showIcon
          />
        )}

        {encryption.encryption_error && (
          <Alert
            message={t("deviceSync.encryption.errorTitle")}
            description={encryption.encryption_error}
            type="error"
            showIcon
          />
        )}
      </Space>
    </Card>
  );
}
