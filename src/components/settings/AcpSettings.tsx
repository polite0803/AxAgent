// SPDX-License-Identifier: AGPL-3.0-only

import { message } from "@/lib/toast";
import { type Session, useAxAgent } from "@/sdk";
import { useSettingsStore } from "@/stores";
import { Badge, Button, Descriptions, Divider, Empty, Input, Popconfirm, Space, Tag, theme, Typography } from "antd";
import { Link2, Plus, Power, RefreshCw, Server } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";

const { Text } = Typography;

const STORAGE_KEY = "acp_base_url";
const DEFAULT_BASE_URL = "http://localhost:9876";

export function AcpSettings() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const settingsAcqBaseUrl = useSettingsStore((s) => s.settings.acp_base_url);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const [baseUrl, setBaseUrl] = useState(
    () => settingsAcqBaseUrl ?? localStorage.getItem(STORAGE_KEY) ?? DEFAULT_BASE_URL,
  );
  const [connected, setConnected] = useState<boolean | null>(null);
  const [checking, setChecking] = useState(false);
  const [workDir, setWorkDir] = useState("");
  const [creating, setCreating] = useState(false);

  const {
    sessions,
    loading,
    error,
    createSession,
    closeSession,
    refreshSessions,
  } = useAxAgent(baseUrl);

  // P2-8: 当 settingsStore 中的 acp_base_url 变化（例如其他面板修改后），同步本地 state。
  // 用 ref 防止 saveSettings 触发的回流把用户正在输入的值覆盖。
  const lastSyncedRef = useRef<string | null>(null);
  useEffect(() => {
    const next = settingsAcqBaseUrl ?? null;
    if (next !== null && next !== lastSyncedRef.current && next !== baseUrl) {
      setBaseUrl(next);
      lastSyncedRef.current = next;
    }
  }, [settingsAcqBaseUrl, baseUrl]);

  // 地址变更时同时写入 localStorage（快速启动缓存）和 settingsStore（统一持久化）
  const handleBaseUrlChange = useCallback((val: string) => {
    setBaseUrl(val);
    localStorage.setItem(STORAGE_KEY, val);
    setConnected(null);
    // 异步保存到后端 settings，失败仅提示不影响 UI
    void saveSettings({ acp_base_url: val }).catch(() => {});
    lastSyncedRef.current = val;
  }, [saveSettings]);

  const handleTestConnection = useCallback(async () => {
    setChecking(true);
    try {
      const client = new (await import("@/sdk")).AxAgentClient(baseUrl);
      const ok = await client.healthCheck();
      setConnected(ok);
      if (ok) {
        message.success(t("acp.connected"));
        refreshSessions();
      } else {
        message.error(t("acp.connectFailed"));
      }
    } catch {
      setConnected(false);
      message.error(t("acp.connectFailedCheck"));
    } finally {
      setChecking(false);
    }
  }, [baseUrl, refreshSessions, t]);

  // 创建会话
  const handleCreateSession = async () => {
    if (!workDir.trim()) {
      message.warning(t("acp.workdirRequired"));
      return;
    }
    setCreating(true);
    try {
      await createSession({ workDir: workDir.trim() });
      message.success(t("acp.sessionCreated"));
      setWorkDir("");
    } catch {
      // error from hook already set
    } finally {
      setCreating(false);
    }
  };

  // 关闭会话
  const handleCloseSession = async (sessionId: string) => {
    try {
      await closeSession(sessionId);
      message.success(t("acp.sessionClosed"));
    } catch {
      message.error(t("acp.closeSessionFailed"));
    }
  };

  // 状态颜色和文本
  const statusConfig = () => {
    if (connected === null) {
      return {
        color: "default",
        text: t("acp.notDetected"),
        dot: <Badge status="default" />,
      };
    }
    if (connected) {
      return {
        color: "#22c55e",
        text: "Connected",
        dot: <Badge status="success" />,
      };
    }
    return {
      color: "#ef4444",
      text: "Disconnected",
      dot: <Badge status="error" />,
    };
  };

  const st = statusConfig();

  return (
    <div className="p-6 pb-12" style={{ overflowY: "auto" }} data-os-scrollbar>
      <SettingsGroup title={t("acp.serverTitle")}>
        <div
          style={{ padding: "6px 0" }}
          className="flex items-center justify-between"
          data-search-key="acp:serverAddress"
        >
          <span className="flex items-center gap-2">
            <Server size={14} /> {t("acp.serverAddress")}
          </span>
          <Input
            id="acp-settings-input-2"
            value={baseUrl}
            onChange={(e) => handleBaseUrlChange(e.target.value)}
            style={{ width: 320 }}
            size="small"
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div
          style={{ padding: "6px 0" }}
          className="flex items-center justify-between"
          data-search-key="acp:connectionStatus"
        >
          <span className="flex items-center gap-2">
            <Link2 size={14} /> {t("acp.connectionStatus")}
          </span>
          <Space size={8}>
            <Badge
              status={connected === null ? "default" : connected ? "success" : "error"}
              text={<Text style={{ fontSize: 13, color: st.color }}>{st.text}</Text>}
            />
            <Button
              size="small"
              icon={checking ? undefined : <RefreshCw size={14} />}
              loading={checking}
              onClick={handleTestConnection}
            >
              {t("acp.testConnection")}
            </Button>
          </Space>
        </div>
        {connected === false && (
          <div
            style={{
              marginTop: 8,
              padding: "6px 10px",
              borderRadius: 6,
              backgroundColor: token.colorErrorBg,
              border: `1px solid ${token.colorErrorBorder}`,
              fontSize: 12,
              color: token.colorError,
            }}
          >
            {t("acp.serverNotReachable")}
          </div>
        )}
      </SettingsGroup>

      {/* 创建会话 */}
      <SettingsGroup title={t("acp.createSessionTitle")}>
        <div
          style={{ padding: "6px 0" }}
          className="flex items-center justify-between"
          data-search-key="acp:workdir"
        >
          <span className="flex items-center gap-2">
            <Plus size={14} /> {t("acp.workdir")}
          </span>
          <Space size={8}>
            <Input
              id="acp-settings-input-3"
              value={workDir}
              onChange={(e) => setWorkDir(e.target.value)}
              placeholder={t("acp.workdirPlaceholder")}
              style={{ width: 280 }}
              size="small"
            />
            <Button
              size="small"
              type="primary"
              icon={<Plus size={14} />}
              loading={creating}
              onClick={handleCreateSession}
              disabled={connected !== true}
            >
              {t("acp.createSession")}
            </Button>
          </Space>
        </div>
      </SettingsGroup>

      {/* 活跃会话列表 */}
      <SettingsGroup
        title={`${t("acp.activeSessions")} (${sessions.length})`}
        extra={
          <Button
            size="small"
            icon={<RefreshCw size={13} />}
            onClick={refreshSessions}
            loading={loading}
            disabled={connected !== true}
          >
            {t("acp.refresh")}
          </Button>
        }
      >
        {connected !== true
          ? (
            <Empty
              description={t("acp.testFirst")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )
          : sessions.length === 0
          ? (
            <Empty
              description={t("acp.noSessions")}
              image={Empty.PRESENTED_IMAGE_SIMPLE}
            />
          )
          : (
            <div className="divide-y divide-gray-100">
              {sessions.map((s: Session) => (
                <div key={s.sessionId} className="py-3">
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex items-start gap-3 flex-1 min-w-0">
                      <div className="pt-1">
                        {s.status === "running"
                          ? <Badge status="processing" />
                          : s.status === "idle"
                          ? <Badge status="success" />
                          : <Badge status="default" />}
                      </div>
                      <div className="flex-1 min-w-0">
                        <Space size={8}>
                          <Text code style={{ fontSize: 12 }}>
                            {s.sessionId.slice(0, 12)}...
                          </Text>
                          <Tag
                            color={s.status === "running"
                              ? "processing"
                              : s.status === "idle"
                              ? "success"
                              : "default"}
                          >
                            {s.status}
                          </Tag>
                        </Space>
                        <Descriptions size="small" column={2} colon={false}>
                          <Descriptions.Item label={t("acp.directory")}>
                            <Text style={{ fontSize: 12 }}>{s.workDir}</Text>
                          </Descriptions.Item>
                          <Descriptions.Item label={t("acp.permission")}>
                            <Tag style={{ fontSize: 12 }}>{s.permissionMode}</Tag>
                          </Descriptions.Item>
                          <Descriptions.Item label={t("acp.activeTasks")}>
                            <Text style={{ fontSize: 12 }}>{s.activeTasks}</Text>
                          </Descriptions.Item>
                          <Descriptions.Item label={t("acp.lastActive")}>
                            <Text style={{ fontSize: 12 }}>
                              {new Date(s.lastActive).toLocaleString("zh-CN")}
                            </Text>
                          </Descriptions.Item>
                        </Descriptions>
                      </div>
                    </div>
                    <Popconfirm
                      key="close"
                      title={t("acp.confirmClose")}
                      onConfirm={() => handleCloseSession(s.sessionId)}
                      okText={t("common.confirm")}
                      cancelText={t("common.cancel")}
                    >
                      <Button
                        size="small"
                        type="text"
                        danger
                        icon={<Power size={13} />}
                      >
                        {t("acp.close")}
                      </Button>
                    </Popconfirm>
                  </div>
                </div>
              ))}
            </div>
          )}
      </SettingsGroup>

      {error && (
        <div
          style={{
            marginTop: 12,
            padding: "8px 12px",
            borderRadius: 6,
            backgroundColor: token.colorErrorBg,
            border: `1px solid ${token.colorErrorBorder}`,
            fontSize: 12,
            color: token.colorError,
          }}
        >
          {error}
        </div>
      )}
    </div>
  );
}
