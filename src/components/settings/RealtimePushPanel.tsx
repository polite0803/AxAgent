// SPDX-License-Identifier: AGPL-3.0-only

import { useDeviceSyncStore } from "@/stores/feature/deviceSyncStore";
import type { SyncSignal } from "@/types";
import {
  CheckCircleOutlined,
  CloudSyncOutlined,
  DisconnectOutlined,
  ReloadOutlined,
  SyncOutlined,
  ThunderboltOutlined,
  WarningOutlined,
} from "@ant-design/icons";
import { Button, Card, Empty, List, message, Space, Spin, Tag } from "antd";
import { useEffect } from "react";
import { useTranslation } from "react-i18next";

/**
 * 实时推送状态面板组件
 * 显示 WebSocket 连接状态和待处理的信令消息
 */
export function RealtimePushPanel() {
  const { t } = useTranslation();
  const {
    realtimePush,
    connectWebSocket,
    disconnectWebSocket,
    sendSignal,
    localDevice,
  } = useDeviceSyncStore();

  // 自动连接 WebSocket
  useEffect(() => {
    if (localDevice && realtimePush.wsStatus === "disconnected") {
      connectWebSocket().catch((e) => {
        console.error("Failed to connect WebSocket:", e);
      });
    }

    // 清理函数
    return () => {
      if (realtimePush.wsStatus === "connected") {
        disconnectWebSocket();
      }
    };
  }, [localDevice]);

  const handleReconnect = async () => {
    await connectWebSocket();
    message.success(t("deviceSync.realtime.reconnectSuccess"));
  };

  const handleDisconnect = () => {
    disconnectWebSocket();
    message.info(t("deviceSync.realtime.disconnected"));
  };

  const handleTestSignal = () => {
    if (!localDevice) {
      message.warning(t("deviceSync.realtime.noDevice"));
      return;
    }

    const testSignal: SyncSignal = {
      type: "ping",
      deviceId: localDevice.deviceId,
    };
    sendSignal(testSignal);
    message.success(t("deviceSync.realtime.signalSent"));
  };

  const getStatusIcon = () => {
    switch (realtimePush.wsStatus) {
      case "connected":
        return <CheckCircleOutlined style={{ color: "#52c41a" }} />;
      case "connecting":
        return <SyncOutlined spin style={{ color: "#1890ff" }} />;
      case "disconnected":
        return <DisconnectOutlined style={{ color: "#8c8c8c" }} />;
      case "error":
        return <WarningOutlined style={{ color: "#ff4d4f" }} />;
      default:
        return null;
    }
  };

  const getStatusTag = () => {
    switch (realtimePush.wsStatus) {
      case "connected":
        return <Tag color="success">{t("deviceSync.realtime.status.connected")}</Tag>;
      case "connecting":
        return <Tag color="processing">{t("deviceSync.realtime.status.connecting")}</Tag>;
      case "disconnected":
        return <Tag>{t("deviceSync.realtime.status.disconnected")}</Tag>;
      case "error":
        return <Tag color="error">{t("deviceSync.realtime.status.error")}</Tag>;
      default:
        return null;
    }
  };

  return (
    <Card
      title={
        <Space>
          <ThunderboltOutlined />
          <span>{t("deviceSync.realtime.title")}</span>
        </Space>
      }
      extra={
        <Space>
          <Button
            size="small"
            icon={<ReloadOutlined />}
            onClick={handleReconnect}
            disabled={realtimePush.wsStatus === "connecting"}
          >
            {t("deviceSync.realtime.reconnect")}
          </Button>
          <Button
            size="small"
            danger
            icon={<DisconnectOutlined />}
            onClick={handleDisconnect}
            disabled={realtimePush.wsStatus !== "connected"}
          >
            {t("deviceSync.realtime.disconnect")}
          </Button>
        </Space>
      }
      style={{ marginBottom: 16 }}
    >
      <Space direction="vertical" style={{ width: "100%" }} size="large">
        {/* 连接状态 */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "16px",
            background: "#f5f5f5",
            borderRadius: "8px",
          }}
        >
          <Space>
            <CloudSyncOutlined style={{ fontSize: 24 }} />
            <span>{t("deviceSync.realtime.connectionStatus")}</span>
          </Space>
          <Space>
            {getStatusIcon()}
            {getStatusTag()}
          </Space>
        </div>

        {/* 连接信息 */}
        {realtimePush.wsConnectionId && (
          <div
            style={{
              padding: "12px",
              background: "#e6f7ff",
              borderRadius: "4px",
            }}
          >
            <Space direction="vertical">
              <span>
                <strong>{t("deviceSync.realtime.connectionId")}:</strong> <code>{realtimePush.wsConnectionId}</code>
              </span>
              {realtimePush.lastSignalAt && (
                <span>
                  <strong>{t("deviceSync.realtime.lastSignal")}:</strong>{" "}
                  {new Date(realtimePush.lastSignalAt).toLocaleTimeString()}
                </span>
              )}
            </Space>
          </div>
        )}

        {/* 测试连接按钮 */}
        {realtimePush.wsStatus === "connected" && (
          <Button
            icon={<ThunderboltOutlined />}
            onClick={handleTestSignal}
            block
          >
            {t("deviceSync.realtime.testConnection")}
          </Button>
        )}

        {/* 待处理信令 */}
        <div>
          <h4>{t("deviceSync.realtime.pendingSignals")} ({realtimePush.pendingSignals.length})</h4>
          {realtimePush.pendingSignals.length === 0
            ? (
              <Empty
                description={t("deviceSync.realtime.noPendingSignals")}
                image={Empty.PRESENTED_IMAGE_SIMPLE}
              />
            )
            : (
              <List
                size="small"
                bordered
                dataSource={realtimePush.pendingSignals.slice(-5).reverse()}
                renderItem={(signal) => (
                  <List.Item>
                    <Space>
                      <Tag color="blue">{signal.type}</Tag>
                      {signal.deviceId && (
                        <span style={{ color: "#8c8c8c", fontSize: 12 }}>
                          {signal.deviceId}
                        </span>
                      )}
                    </Space>
                  </List.Item>
                )}
              />
            )}
        </div>

        {/* 无设备提示 */}
        {!localDevice && (
          <div
            style={{
              padding: "16px",
              background: "#fff2e8",
              borderRadius: "4px",
              textAlign: "center",
            }}
          >
            <Spin />
            <p style={{ marginTop: 8 }}>
              {t("deviceSync.realtime.registerDeviceFirst")}
            </p>
          </div>
        )}
      </Space>
    </Card>
  );
}
