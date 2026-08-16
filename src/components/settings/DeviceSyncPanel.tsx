// SPDX-License-Identifier: AGPL-3.0-only

import {
  CloudOutlined,
  DesktopOutlined,
  MobileOutlined,
  QrcodeOutlined,
  ReloadOutlined,
  SafetyOutlined,
  SyncOutlined,
  TableOutlined,
} from "@ant-design/icons";
import { Button, Card, Col, Input, List, message, Modal, Row, Space, Tag, Typography } from "antd";
import { useState } from "react";

import { useDeviceSyncStore } from "@/stores";
import type { ConflictInfo, DeviceInfo, TrustLevel } from "@/types";
import { useTranslation } from "react-i18next";
import { ConflictDetailModal } from "./ConflictDetailModal";
import { DevicePermissionsPanel } from "./DevicePermissionsPanel";
import { EncryptionSettingsPanel } from "./EncryptionSettingsPanel";
import { RealtimePushPanel } from "./RealtimePushPanel";
import { SyncHistoryPanel } from "./SyncHistoryPanel";
import { SyncPolicyPanel } from "./SyncPolicyPanel";

const { Title, Text, Paragraph } = Typography;

/** 信任级别配置 */
const getTrustLevelConfig = (t: (key: string) => string) => ({
  backup_only: {
    label: t("deviceSync.trust.backup_only"),
    color: "default",
    description: t("deviceSync.trust.backup_only_desc"),
  },
  standard: {
    label: t("deviceSync.trust.standard"),
    color: "blue",
    description: t("deviceSync.trust.standard_desc"),
  },
  full: {
    label: t("deviceSync.trust.full"),
    color: "green",
    description: t("deviceSync.trust.full_desc"),
  },
});

/** 设备类型图标 */
function DeviceIcon({ type }: { type: string }) {
  switch (type) {
    case "mobile":
      return <MobileOutlined style={{ fontSize: 24, color: "#1890ff" }} />;
    case "tablet":
      return <TableOutlined style={{ fontSize: 24, color: "#722ed1" }} />;
    case "server":
      return <CloudOutlined style={{ fontSize: 24, color: "#fa8c16" }} />;
    default:
      return <DesktopOutlined style={{ fontSize: 24, color: "#52c41a" }} />;
  }
}

/** 设备管理面板 */
export function DeviceSyncPanel() {
  const { t } = useTranslation();
  const deviceSyncStore = useDeviceSyncStore();
  const [pairingModalOpen, setPairingModalOpen] = useState(false);
  const [verificationModalOpen, setVerificationModalOpen] = useState(false);
  const [pairingCodeInput, setPairingCodeInput] = useState("");
  const [conflictDetailOpen, setConflictDetailOpen] = useState(false);
  const [selectedConflict, setSelectedConflict] = useState<ConflictInfo | null>(null);

  const {
    localDevice,
    devices,
    syncStatus,
    pendingConflicts,
    currentPairingCode,
    currentPairingRequest,
    loading,
    isSyncing,
  } = deviceSyncStore;

  /** 加载设备列表 */
  const handleLoadDevices = async () => {
    await deviceSyncStore.listDevices();
  };

  /** 生成配对码 */
  const handleGenerateCode = async () => {
    const code = await deviceSyncStore.generatePairingCode();
    if (code) {
      setPairingModalOpen(true);
    } else {
      message.error(t("deviceSync.generateCodeFailed"));
    }
  };

  /** 验证配对码 */
  const handleVerifyCode = async () => {
    if (!pairingCodeInput || pairingCodeInput.length !== 6) {
      message.warning(t("deviceSync.invalidCode"));
      return;
    }
    const request = await deviceSyncStore.verifyPairingCode(pairingCodeInput);
    if (request) {
      setVerificationModalOpen(true);
      setPairingModalOpen(false);
    } else {
      message.error(t("deviceSync.verifyCodeFailed"));
    }
  };

  /** 接受配对 */
  const handleAcceptPairing = async (trustLevel: TrustLevel) => {
    if (!currentPairingRequest) { return; }
    const response = await deviceSyncStore.acceptPairing(
      currentPairingRequest,
      trustLevel,
    );
    if (response?.success) {
      message.success(t("deviceSync.pairingSuccess"));
      setVerificationModalOpen(false);
      setPairingCodeInput("");
    } else {
      message.error(response?.message || t("deviceSync.pairingFailed"));
    }
  };

  /** 全量同步 */
  const handleFullSync = async (deviceId: string) => {
    const result = await deviceSyncStore.fullSync(deviceId);
    if (result?.success) {
      message.success(t("deviceSync.syncSuccess", { count: result.filesSynced }));
    } else {
      message.error(result?.errorMessage || t("deviceSync.syncFailed"));
    }
  };

  /** 增量同步 */
  const handleIncrementalSync = async (deviceId: string) => {
    const result = await deviceSyncStore.incrementalSync(deviceId);
    if (result?.success) {
      message.success(t("deviceSync.syncSuccess", { count: result.filesSynced }));
    } else {
      message.error(result?.errorMessage || t("deviceSync.syncFailed"));
    }
  };

  /** 取消配对 */
  const handleUnpair = async (deviceId: string) => {
    Modal.confirm({
      title: t("deviceSync.unpairTitle"),
      content: t("deviceSync.unpairContent"),
      okText: t("common.confirm"),
      cancelText: t("common.cancel"),
      onOk: async () => {
        await deviceSyncStore.unpairDevice(deviceId);
        message.success(t("deviceSync.unpairSuccess"));
      },
    });
  };

  return (
    <div style={{ padding: 24 }}>
      <Title level={3}>
        <SyncOutlined /> {t("deviceSync.title")}
      </Title>
      <Paragraph type="secondary">{t("deviceSync.description")}</Paragraph>

      {/* 当前设备信息 */}
      {localDevice && (
        <Card
          title={
            <Space>
              <DeviceIcon type={localDevice.deviceType} />
              <span>{t("deviceSync.currentDevice")}</span>
            </Space>
          }
          style={{ marginBottom: 16 }}
        >
          <Row gutter={[16, 16]}>
            <Col span={8}>
              <Text strong>{t("deviceSync.deviceName")}:</Text> {localDevice.name}
            </Col>
            <Col span={8}>
              <Text strong>{t("deviceSync.os")}:</Text> {localDevice.os}
            </Col>
            <Col span={8}>
              <Text strong>{t("deviceSync.version")}:</Text> {localDevice.appVersion}
            </Col>
          </Row>
        </Card>
      )}

      {/* 同步状态 */}
      {syncStatus && (
        <Card
          title={t("deviceSync.syncStatus")}
          style={{ marginBottom: 16 }}
          extra={
            <Button
              icon={<ReloadOutlined />}
              size="small"
              onClick={() => deviceSyncStore.getSyncStatus()}
            >
              {t("common.refresh")}
            </Button>
          }
        >
          <Row gutter={[16, 16]}>
            <Col span={6}>
              <Text strong>{t("deviceSync.connectedDevices")}:</Text> {syncStatus.connectedDevices}
            </Col>
            <Col span={6}>
              <Text strong>{t("deviceSync.pendingChanges")}:</Text> {syncStatus.pendingChanges}
            </Col>
            <Col span={6}>
              <Text strong>{t("deviceSync.lastSync")}:</Text> {syncStatus.lastSyncAt
                ? new Date(syncStatus.lastSyncAt).toLocaleString()
                : t("deviceSync.never")}
            </Col>
            <Col span={6}>
              <Tag color={isSyncing ? "blue" : "green"}>
                {isSyncing ? t("deviceSync.syncing") : t("deviceSync.idle")}
              </Tag>
            </Col>
          </Row>
        </Card>
      )}

      {/* 操作按钮 */}
      <Space style={{ marginBottom: 16 }}>
        <Button
          type="primary"
          icon={<QrcodeOutlined />}
          onClick={handleGenerateCode}
          loading={loading}
        >
          {t("deviceSync.generateCode")}
        </Button>
        <Button icon={<ReloadOutlined />} onClick={handleLoadDevices} loading={loading}>
          {t("deviceSync.refreshDevices")}
        </Button>
      </Space>

      {/* 已配对设备列表 */}
      <Card title={t("deviceSync.pairedDevices")} style={{ marginBottom: 16 }}>
        {devices.length === 0
          ? (
            <div style={{ textAlign: "center", padding: 24 }}>
              <Text type="secondary">{t("deviceSync.noDevices")}</Text>
            </div>
          )
          : (
            <List
              dataSource={devices}
              renderItem={(device: DeviceInfo) => (
                <List.Item
                  actions={[
                    <Button
                      size="small"
                      onClick={() => handleIncrementalSync(device.deviceId)}
                      loading={isSyncing}
                    >
                      {t("deviceSync.incrementalSync")}
                    </Button>,
                    <Button
                      size="small"
                      type="primary"
                      onClick={() => handleFullSync(device.deviceId)}
                      loading={isSyncing}
                    >
                      {t("deviceSync.fullSync")}
                    </Button>,
                    <Button
                      size="small"
                      danger
                      onClick={() => handleUnpair(device.deviceId)}
                    >
                      {t("deviceSync.unpair")}
                    </Button>,
                  ]}
                >
                  <List.Item.Meta
                    avatar={<DeviceIcon type={device.deviceType} />}
                    title={
                      <Space>
                        {device.name}
                        <Tag color={getTrustLevelConfig(t)[device.trustLevel].color}>
                          {getTrustLevelConfig(t)[device.trustLevel].label}
                        </Tag>
                      </Space>
                    }
                    description={
                      <Space direction="vertical" size={4}>
                        <Text type="secondary">{device.hostname}</Text>
                        <Text type="secondary" style={{ fontSize: 12 }}>
                          {t("deviceSync.lastActive")}: {new Date(device.lastActiveAt).toLocaleString()}
                        </Text>
                      </Space>
                    }
                  />
                </List.Item>
              )}
            />
          )}
      </Card>

      {/* 待解决冲突 */}
      {pendingConflicts.length > 0 && (
        <Card
          title={
            <Space>
              <SafetyOutlined style={{ color: "#faad14" }} />
              {t("deviceSync.pendingConflicts")} ({pendingConflicts.length})
            </Space>
          }
          style={{ marginBottom: 16 }}
        >
          <List
            dataSource={pendingConflicts}
            renderItem={(conflict) => (
              <List.Item
                actions={[
                  <Button
                    key="detail"
                    size="small"
                    type="primary"
                    onClick={() => {
                      setSelectedConflict(conflict);
                      setConflictDetailOpen(true);
                    }}
                  >
                    {t("deviceSync.viewDetails")}
                  </Button>,
                ]}
              >
                <List.Item.Meta
                  title={`${conflict.entityType}: ${conflict.entityId}`}
                  description={t("deviceSync.conflictDescription", {
                    localVer: conflict.localVector.map((v) => `${v.deviceId}:${v.counter}`).join(", "),
                    remoteVer: conflict.remoteVector.map((v) => `${v.deviceId}:${v.counter}`).join(", "),
                  })}
                />
              </List.Item>
            )}
          />
        </Card>
      )}

      {/* 实时推送状态 */}
      <RealtimePushPanel />

      {/* 加密设置 */}
      <EncryptionSettingsPanel />

      {/* P2: 同步策略配置 */}
      <SyncPolicyPanel />

      {/* P2: 同步历史与审计日志 */}
      <SyncHistoryPanel />

      {/* P2: 设备权限管理 */}
      <DevicePermissionsPanel />

      {/* P2: 冲突详情弹窗 */}
      <ConflictDetailModal
        open={conflictDetailOpen}
        conflict={selectedConflict}
        onClose={() => {
          setConflictDetailOpen(false);
          setSelectedConflict(null);
        }}
        onResolve={async (strategy) => {
          if (selectedConflict) {
            await deviceSyncStore.resolveConflict(selectedConflict.id, strategy);
            message.success(t("deviceSync.conflictResolved"));
          }
          setConflictDetailOpen(false);
          setSelectedConflict(null);
        }}
      />

      {/* 生成配对码弹窗 */}
      <Modal
        open={pairingModalOpen}
        title={t("deviceSync.pairingCode")}
        onCancel={() => setPairingModalOpen(false)}
        footer={[
          <Button key="close" onClick={() => setPairingModalOpen(false)}>
            {t("common.close")}
          </Button>,
        ]}
      >
        {currentPairingCode
          ? (
            <div style={{ textAlign: "center", padding: 16 }}>
              <QrcodeOutlined style={{ fontSize: 48, color: "#1890ff" }} />
              <Title level={2} style={{ margin: "16px 0" }}>
                {currentPairingCode.code}
              </Title>
              <Text type="secondary">
                {t("deviceSync.codeExpiresAt", {
                  time: new Date(currentPairingCode.expiresAt).toLocaleTimeString(),
                })}
              </Text>
              <Paragraph type="secondary" style={{ marginTop: 16 }}>
                {t("deviceSync.enterCodeOnOtherDevice")}
              </Paragraph>
            </div>
          )
          : (
            <div style={{ textAlign: "center", padding: 24 }}>
              <Text>{t("deviceSync.generatingCode")}</Text>
            </div>
          )}
      </Modal>

      {/* 验证配对码弹窗 */}
      <Modal
        open={verificationModalOpen}
        title={t("deviceSync.verifyPairingCode")}
        onCancel={() => {
          setVerificationModalOpen(false);
          setPairingCodeInput("");
        }}
        onOk={handleVerifyCode}
        okText={t("deviceSync.verify")}
      >
        <div style={{ padding: 16 }}>
          <Input.OTP
            value={pairingCodeInput}
            onChange={(e) => setPairingCodeInput(e)}
            length={6}
            style={{ justifyContent: "center" }}
          />
          <Paragraph type="secondary" style={{ marginTop: 16 }}>
            {t("deviceSync.enter6DigitCode")}
          </Paragraph>
        </div>
      </Modal>

      {/* 接受配对弹窗 */}
      {currentPairingRequest && (
        <Modal
          open={verificationModalOpen}
          title={t("deviceSync.acceptPairing")}
          onCancel={() => {
            setVerificationModalOpen(false);
            setPairingCodeInput("");
          }}
          footer={[
            <Button
              key="cancel"
              onClick={() => {
                setVerificationModalOpen(false);
                setPairingCodeInput("");
              }}
            >
              {t("common.cancel")}
            </Button>,
          ]}
        >
          <Card size="small" style={{ marginBottom: 16 }}>
            <Space>
              <DeviceIcon type={currentPairingRequest.device.deviceType} />
              <div>
                <Text strong>{currentPairingRequest.device.name}</Text>
                <br />
                <Text type="secondary">{currentPairingRequest.device.hostname}</Text>
              </div>
            </Space>
          </Card>
          <Paragraph>{t("deviceSync.selectTrustLevel")}:</Paragraph>
          <Row gutter={[8, 8]}>
            {Object.entries(getTrustLevelConfig(t)).map(([level, config]) => (
              <Col span={8} key={level}>
                <Button
                  block
                  onClick={() => handleAcceptPairing(level as TrustLevel)}
                >
                  <Tag color={config.color}>{config.label}</Tag>
                  <br />
                  <Text type="secondary" style={{ fontSize: 12 }}>
                    {config.description}
                  </Text>
                </Button>
              </Col>
            ))}
          </Row>
        </Modal>
      )}
    </div>
  );
}
