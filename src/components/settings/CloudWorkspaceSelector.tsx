import { invoke } from "@/lib/invoke";
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Form,
  Input,
  message,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Typography,
} from "antd";
import type { ColumnsType } from "antd/es/table";
import {
  AlertTriangle,
  CheckCircle,
  Cloud,
  FolderOpen,
  Globe,
  Link,
  RefreshCw,
  Settings2,
  Upload,
  Wifi,
  WifiOff,
} from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettingsStore } from "../../stores";

const { Text, Title } = Typography;

type CloudStorageType = "s3" | "webdav";

interface CloudDirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  etag: string | null;
  conflict: boolean;
}

interface CloudListResponse {
  entries: CloudDirEntry[];
}

interface CloudSyncResponse {
  downloaded: number;
  uploaded: number;
  local_deletions_synced: number;
  remote_deletions_synced: number;
  conflicts_detected: number;
  conflicts_resolved: number;
  pending_conflicts: number;
  local_cache_dir: string;
}

interface CloudConflictDto {
  key: string;
  kind: string;
  resolution: string | null;
  local_size: number;
  remote_size: number;
  local_modified_at: number;
  remote_modified_at: number;
}

interface CloudConflictsResponse {
  pending_conflicts: CloudConflictDto[];
  strategy: string;
}

interface CloudProviderPresetDto {
  key: string;
  display_name: string;
  endpoint_template: string;
  default_region: string;
  use_path_style: boolean;
}

export function CloudWorkspaceSelector() {
  const { t } = useTranslation();
  const { settings, saveSettings, fetchSettings } = useSettingsStore();
  const [storageType, setStorageType] = useState<CloudStorageType>("s3");
  const [configModalOpen, setConfigModalOpen] = useState(false);
  const [browserModalOpen, setBrowserModalOpen] = useState(false);
  const [conflictsModalOpen, setConflictsModalOpen] = useState(false);
  const [configForm] = Form.useForm();
  const [presets, setPresets] = useState<CloudProviderPresetDto[]>([]);
  const [currentPath, setCurrentPath] = useState("");
  const [dirEntries, setDirEntries] = useState<CloudDirEntry[]>([]);
  const [browsing, setBrowsing] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [conflictsLoading, setConflictsLoading] = useState(false);
  const [conflicts, setConflicts] = useState<CloudConflictDto[]>([]);
  const [conflictStrategy, setConflictStrategy] = useState("latest_wins");
  const [syncResult, setSyncResult] = useState<CloudSyncResponse | null>(null);
  const [testingConnection, setTestingConnection] = useState(false);
  const [connectionStatus, setConnectionStatus] = useState<
    "unknown" | "success" | "failed"
  >("unknown");

  useEffect(() => {
    const loadPresets = async () => {
      try {
        const result = await invoke<CloudProviderPresetDto[]>(
          "list_cloud_provider_presets",
        );
        setPresets(result);
      } catch (e) {
        console.error("Failed to load presets:", e);
      }
    };
    loadPresets();
  }, []);

  const buildWorkspaceUri = (values: Record<string, unknown>) => {
    if (storageType === "webdav") {
      const host = values.webdavHost as string;
      const path = (values.webdavPath as string) || "/";
      return `webdav://${host}${path}`;
    }
    const bucket = values.s3Bucket as string;
    const root = (values.s3Root as string) || "/";
    return `s3://${bucket}${root}`;
  };

  const openConfigModal = () => {
    const {
      s3_endpoint = "",
      s3_access_key_id = "",
      s3_secret_access_key = "",
      s3_region = "",
      s3_provider_preset = "",
      s3_use_path_style = false,
      webdav_host = "",
      webdav_username = "",
      webdav_password = "",
      webdav_path = "",
      s3_bucket = "",
      s3_root = "",
    } = settings;

    configForm.setFieldsValue({
      s3Endpoint: s3_endpoint || "",
      s3AccessKey: s3_access_key_id || "",
      s3SecretKey: s3_secret_access_key || "",
      s3Region: s3_region || "",
      s3ProviderPreset: s3_provider_preset || "",
      s3UsePathStyle: s3_use_path_style || false,
      webdavUrl: webdav_host || "",
      webdavUsername: webdav_username || "",
      webdavPassword: webdav_password || "",
      webdavPath: webdav_path || "/",
      s3Bucket: s3_bucket || "",
      s3Root: s3_root || "",
    });

    if (webdav_host) {
      setStorageType("webdav");
    } else {
      setStorageType("s3");
    }

    setConnectionStatus("unknown");
    setConfigModalOpen(true);
  };

  const handleTestConnection = async () => {
    try {
      setTestingConnection(true);
      setConnectionStatus("unknown");

      const values = await configForm.validateFields();
      const config: Record<string, unknown> = {
        storageType,
      };

      if (storageType === "s3") {
        config.endpoint = values.s3Endpoint as string;
        config.region = (values.s3Region as string) || "auto";
        config.bucket = values.s3Bucket as string;
        config.accessKeyId = values.s3AccessKey as string;
        config.secretAccessKey = values.s3SecretKey as string;
        config.root = (values.s3Root as string) || "/";
        config.usePathStyle = values.s3UsePathStyle as boolean;
      } else {
        config.host = values.webdavUrl as string;
        config.username = values.webdavUsername as string;
        config.password = values.webdavPassword as string;
        config.path = (values.webdavPath as string) || "/";
      }

      const result = await invoke<boolean>("check_cloud_connection", {
        config,
      });
      setConnectionStatus(result ? "success" : "failed");

      if (result) {
        message.success(t("cloudWorkspace.connectionSuccess"));
      } else {
        message.error(t("cloudWorkspace.connectionFailed"));
      }
    } catch (e) {
      setConnectionStatus("failed");
      message.error(t("cloudWorkspace.connectionFailed") + ": " + String(e));
    } finally {
      setTestingConnection(false);
    }
  };

  const handleSaveConfig = async () => {
    try {
      const values = await configForm.validateFields();

      const uri = buildWorkspaceUri(values);
      const updates: Record<string, unknown> = {
        workspace_uri: uri,
        cloud_backend: storageType,
      };

      if (storageType === "s3") {
        updates.s3_endpoint = values.s3Endpoint as string;
        updates.s3_access_key_id = values.s3AccessKey as string;
        updates.s3_secret_access_key = values.s3SecretKey as string;
        updates.s3_region = values.s3Region as string;
        updates.s3_provider_preset = values.s3ProviderPreset as string;
        updates.s3_use_path_style = values.s3UsePathStyle as boolean;
        updates.s3_bucket = values.s3Bucket as string;
        updates.s3_root = values.s3Root as string;
      } else {
        updates.webdav_host = values.webdavUrl as string;
        updates.webdav_username = values.webdavUsername as string;
        updates.webdav_password = values.webdavPassword as string;
        updates.webdav_path = values.webdavPath as string;
      }

      await saveSettings(updates);
      await fetchSettings();
      message.success(t("cloudWorkspace.configSaved"));
      setConfigModalOpen(false);
    } catch (e) {
      message.error(t("common.operationFailed", { error: String(e) }));
    }
  };

  const loadCloudDirectory = async (dirPath: string) => {
    const values = configForm.getFieldsValue();
    const uri = buildWorkspaceUri(values);

    try {
      setBrowsing(true);
      const response = await invoke<CloudListResponse>("list_cloud_directory", {
        request: {
          workspaceUri: uri,
          dirPath,
        },
      });
      setDirEntries(response.entries);
    } catch (e) {
      message.error(t("cloudWorkspace.listFailed", { error: String(e) }));
    } finally {
      setBrowsing(false);
    }
  };

  const openBrowserModal = async () => {
    setBrowserModalOpen(true);
    setCurrentPath("");
    await loadCloudDirectory("");
  };

  const handleBrowseEntry = (entry: CloudDirEntry) => {
    if (entry.is_dir) {
      const dirPath = entry.path;
      setCurrentPath(dirPath);
      loadCloudDirectory(dirPath);
    }
  };

  const handleGoBack = () => {
    const parts = currentPath.split("/").filter(Boolean);
    if (parts.length > 1) {
      parts.pop();
      const parentPath = parts.join("/");
      setCurrentPath(parentPath);
      loadCloudDirectory(parentPath);
    } else {
      setCurrentPath("");
      loadCloudDirectory("");
    }
  };

  const loadConflicts = async () => {
    const uri = settings.workspace_uri;
    if (!uri) {
      message.warning(t("cloudWorkspace.noWorkspaceUri"));
      return;
    }

    try {
      setConflictsLoading(true);
      const response = await invoke<CloudConflictsResponse>(
        "get_cloud_conflicts",
        {
          request: { workspaceUri: uri },
        },
      );
      setConflicts(response.pending_conflicts);
      setConflictStrategy(response.strategy);
      setConflictsModalOpen(true);
    } catch (e) {
      message.error(
        t("cloudWorkspace.loadConflictsFailed", { error: String(e) }),
      );
    } finally {
      setConflictsLoading(false);
    }
  };

  const handleResolveConflict = async (key: string, resolution: string) => {
    const uri = settings.workspace_uri;
    if (!uri) {
      return;
    }

    try {
      await invoke("resolve_cloud_conflict", {
        request: { workspaceUri: uri, key, resolution },
      });
      message.success(t("cloudWorkspace.conflictResolved"));
      await loadConflicts();
    } catch (e) {
      message.error(t("cloudWorkspace.resolveFailed", { error: String(e) }));
    }
  };

  const handleSyncCloud = async () => {
    const uri = settings.workspace_uri;
    if (!uri) {
      message.warning(t("cloudWorkspace.noWorkspaceUri"));
      return;
    }

    try {
      setSyncing(true);
      const response = await invoke<CloudSyncResponse>("sync_cloud_workspace", {
        request: { workspaceUri: uri },
      });

      setSyncResult(response);

      const parts = [];
      if (response.downloaded > 0) {
        parts.push(
          t("cloudWorkspace.downloaded", { count: response.downloaded }),
        );
      }
      if (response.uploaded > 0) {
        parts.push(t("cloudWorkspace.uploaded", { count: response.uploaded }));
      }
      if (response.conflicts_detected > 0) {
        parts.push(
          t("cloudWorkspace.conflicts", { count: response.conflicts_detected }),
        );
      }

      const summary = parts.length > 0
        ? parts.join("，")
        : t("cloudWorkspace.syncedUpToDate");

      message.success(summary);

      if (response.pending_conflicts > 0) {
        message.warning(
          t("cloudWorkspace.pendingConflicts", {
            count: response.pending_conflicts,
          }),
        );
      }
    } catch (e) {
      message.error(t("cloudWorkspace.syncFailed", { error: String(e) }));
    } finally {
      setSyncing(false);
    }
  };

  const handleSetAsWorkspace = async () => {
    const values = configForm.getFieldsValue();
    const uri = buildWorkspaceUri(values);
    await saveSettings({ workspace_uri: uri, cloud_backend: storageType });
    message.success(t("cloudWorkspace.setAsWorkspaceSuccess", { uri }));
  };

  const conflictColumns: ColumnsType<CloudConflictDto> = [
    {
      title: t("cloudWorkspace.fileName"),
      dataIndex: "key",
      key: "key",
    },
    {
      title: t("cloudWorkspace.conflictType"),
      dataIndex: "kind",
      key: "kind",
      width: 150,
      render: (kind: string) => {
        const colorMap: Record<string, string> = {
          both_modified: "orange",
          modified_vs_deleted: "red",
          deleted_vs_modified: "purple",
          both_created: "blue",
        };
        return <Tag color={colorMap[kind] || "default"}>{kind}</Tag>;
      },
    },
    {
      title: t("cloudWorkspace.localSize"),
      dataIndex: "local_size",
      key: "local_size",
      width: 100,
      render: (size: number) => `${(size / 1024).toFixed(1)} KB`,
    },
    {
      title: t("cloudWorkspace.remoteSize"),
      dataIndex: "remote_size",
      key: "remote_size",
      width: 100,
      render: (size: number) => `${(size / 1024).toFixed(1)} KB`,
    },
    {
      title: t("cloudWorkspace.actions"),
      key: "actions",
      width: 300,
      render: (_, record) => (
        <Space size="small">
          <Button
            size="small"
            type="primary"
            onClick={() => handleResolveConflict(record.key, "keep_local")}
          >
            {t("cloudWorkspace.keepLocal")}
          </Button>
          <Button
            size="small"
            onClick={() => handleResolveConflict(record.key, "keep_remote")}
          >
            {t("cloudWorkspace.keepRemote")}
          </Button>
          <Button
            size="small"
            onClick={() => handleResolveConflict(record.key, "keep_both")}
          >
            {t("cloudWorkspace.keepBoth")}
          </Button>
        </Space>
      ),
    },
  ];

  const columns: ColumnsType<CloudDirEntry> = [
    {
      title: t("cloudWorkspace.fileName"),
      dataIndex: "name",
      key: "name",
      render: (name: string, record) => (
        <Space>
          {record.conflict && <AlertTriangle size={14} className="text-orange-500" />}
          {record.is_dir
            ? <FolderOpen size={14} className="text-blue-500" />
            : <Globe size={14} className="text-zinc-500" />}
          <span>{name}</span>
          {record.conflict && (
            <Tag color="orange" style={{ marginLeft: 8 }}>
              {t("cloudWorkspace.conflict")}
            </Tag>
          )}
        </Space>
      ),
    },
    {
      title: t("cloudWorkspace.size"),
      dataIndex: "size",
      key: "size",
      width: 100,
      render: (size: number, record) => {
        if (record.is_dir) {
          return "-";
        }
        if (size < 1024) {
          return `${size} B`;
        }
        if (size < 1024 * 1024) {
          return `${(size / 1024).toFixed(1)} KB`;
        }
        return `${(size / (1024 * 1024)).toFixed(1)} MB`;
      },
    },
  ];

  const isConfigured = !!settings.workspace_uri;
  const backendLabel = settings.cloud_backend === "webdav" ? "WebDAV" : "S3";

  return (
    <div className="p-6 pb-12 space-y-6">
      <div className="flex items-center justify-between">
        <Title level={4} style={{ margin: 0 }}>
          <Cloud size={20} className="inline mr-2" />
          {t("cloudWorkspace.title")}
        </Title>
        <Space>
          <Button icon={<Settings2 size={14} />} onClick={openConfigModal}>
            {isConfigured
              ? t("cloudWorkspace.configure")
              : t("cloudWorkspace.configureFirst")}
          </Button>
          {isConfigured && (
            <>
              <Button
                icon={<FolderOpen size={14} />}
                onClick={openBrowserModal}
              >
                {t("cloudWorkspace.browse")}
              </Button>
              <Button
                icon={<RefreshCw size={14} />}
                loading={syncing}
                onClick={handleSyncCloud}
                type="primary"
              >
                {t("cloudWorkspace.sync")}
              </Button>
              {conflicts.length > 0 && (
                <Button
                  icon={<AlertTriangle size={14} />}
                  danger
                  onClick={loadConflicts}
                >
                  {t("cloudWorkspace.viewConflicts", {
                    count: conflicts.length,
                  })}
                </Button>
              )}
            </>
          )}
        </Space>
      </div>

      {!isConfigured
        ? (
          <Card>
            <div className="text-center py-8">
              <Cloud size={48} className="mx-auto mb-4 opacity-40" />
              <Title level={5} type="secondary">
                {t("cloudWorkspace.notConfigured")}
              </Title>
              <Text type="secondary" className="block mb-4">
                {t("cloudWorkspace.notConfiguredDesc")}
              </Text>
              <Button
                type="primary"
                icon={<Settings2 size={14} />}
                onClick={openConfigModal}
              >
                {t("cloudWorkspace.configureFirst")}
              </Button>
            </div>
          </Card>
        )
        : (
          <>
            <Card size="small">
              <Descriptions column={2} size="small">
                <Descriptions.Item label={t("cloudWorkspace.currentWorkspace")}>
                  <Space>
                    <Tag icon={<Cloud size={12} />} color="blue">
                      {settings.workspace_uri}
                    </Tag>
                    <Tag
                      color={settings.cloud_backend === "webdav" ? "green" : "purple"}
                    >
                      {backendLabel}
                    </Tag>
                    <Tag icon={<CheckCircle size={12} />} color="success">
                      {t("cloudWorkspace.active")}
                    </Tag>
                  </Space>
                </Descriptions.Item>
                {settings.s3_endpoint && (
                  <Descriptions.Item label={t("cloudWorkspace.s3Endpoint")}>
                    <Text code>{settings.s3_endpoint}</Text>
                  </Descriptions.Item>
                )}
                {settings.s3_bucket && (
                  <Descriptions.Item label={t("cloudWorkspace.s3Bucket")}>
                    <Text code>{settings.s3_bucket}</Text>
                  </Descriptions.Item>
                )}
                {settings.s3_region && (
                  <Descriptions.Item label={t("cloudWorkspace.s3Region")}>
                    <Text code>{settings.s3_region}</Text>
                  </Descriptions.Item>
                )}
                {settings.webdav_host && (
                  <Descriptions.Item label={t("cloudWorkspace.webdavUrl")}>
                    <Text code>{settings.webdav_host}</Text>
                  </Descriptions.Item>
                )}
              </Descriptions>
            </Card>

            {syncResult && (
              <Card size="small" title={t("cloudWorkspace.lastSyncResult")}>
                <Space size="large">
                  <span>
                    <Link size={14} className="mr-1" />
                    {t("cloudWorkspace.downloaded", {
                      count: syncResult.downloaded,
                    })}
                  </span>
                  <span>
                    <Upload size={14} className="mr-1" />
                    {t("cloudWorkspace.uploaded", { count: syncResult.uploaded })}
                  </span>
                  {syncResult.conflicts_detected > 0 && (
                    <span>
                      <AlertTriangle size={14} className="mr-1 text-orange-500" />
                      {t("cloudWorkspace.conflicts", {
                        count: syncResult.conflicts_detected,
                      })}
                    </span>
                  )}
                </Space>
                <div className="mt-2">
                  <Text type="secondary" className="text-xs">
                    {t("cloudWorkspace.cacheDir")}: {syncResult.local_cache_dir}
                  </Text>
                </div>
              </Card>
            )}

            <Card size="small" title={t("cloudWorkspace.quickActions")}>
              <Space wrap>
                <Button
                  icon={<RefreshCw size={14} />}
                  loading={syncing}
                  onClick={handleSyncCloud}
                >
                  {t("cloudWorkspace.syncNow")}
                </Button>
                <Button
                  icon={<FolderOpen size={14} />}
                  onClick={openBrowserModal}
                >
                  {t("cloudWorkspace.browse")}
                </Button>
                <Button
                  icon={<AlertTriangle size={14} />}
                  onClick={loadConflicts}
                >
                  {t("cloudWorkspace.viewConflicts", { count: conflicts.length })}
                </Button>
                <Button icon={<Settings2 size={14} />} onClick={openConfigModal}>
                  {t("cloudWorkspace.configure")}
                </Button>
              </Space>
            </Card>
          </>
        )}

      <Modal
        title={
          <Space>
            <Settings2 size={16} />
            {t("cloudWorkspace.configTitle")}
          </Space>
        }
        open={configModalOpen}
        onCancel={() => setConfigModalOpen(false)}
        width={600}
        footer={
          <Space
            style={{
              display: "flex",
              justifyContent: "space-between",
              width: "100%",
            }}
          >
            <Button
              icon={connectionStatus === "success" ? <Wifi size={14} /> : <WifiOff size={14} />}
              loading={testingConnection}
              onClick={handleTestConnection}
            >
              {t("cloudWorkspace.testConnection")}
            </Button>
            <Space>
              <Button onClick={() => setConfigModalOpen(false)}>
                {t("common.cancel")}
              </Button>
              <Button type="primary" onClick={handleSaveConfig}>
                {t("common.save")}
              </Button>
            </Space>
          </Space>
        }
      >
        {connectionStatus === "success" && (
          <Alert
            type="success"
            message={t("cloudWorkspace.connectionSuccess")}
            showIcon
            style={{ marginBottom: 16 }}
          />
        )}
        {connectionStatus === "failed" && (
          <Alert
            type="error"
            message={t("cloudWorkspace.connectionFailed")}
            showIcon
            style={{ marginBottom: 16 }}
          />
        )}
        <Form form={configForm} layout="vertical">
          <Form.Item label={t("cloudWorkspace.storageType")}>
            <Select
              id="cloud-workspace-selector-select-42"
              value={storageType}
              onChange={(v) => {
                setStorageType(v);
                setConnectionStatus("unknown");
              }}
              options={[
                { label: "Amazon S3 / S3-Compatible", value: "s3" },
                { label: "WebDAV", value: "webdav" },
              ]}
            />
          </Form.Item>

          {storageType === "s3" && (
            <>
              <Form.Item
                name="s3ProviderPreset"
                label={t("cloudWorkspace.providerPreset")}
              >
                <Select
                  options={presets.map((p) => ({
                    label: p.display_name,
                    value: p.key,
                  }))}
                  placeholder={t("cloudWorkspace.providerPresetPlaceholder")}
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item
                name="s3Endpoint"
                label={t("cloudWorkspace.s3Endpoint")}
                rules={[
                  {
                    required: true,
                    message: t("cloudWorkspace.endpointRequired"),
                  },
                ]}
              >
                <Input
                  name="s3Endpoint"
                  placeholder="https://s3.amazonaws.com"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item
                name="s3AccessKey"
                label={t("cloudWorkspace.s3AccessKey")}
                rules={[
                  {
                    required: true,
                    message: t("cloudWorkspace.accessKeyRequired"),
                  },
                ]}
              >
                <Input
                  name="s3AccessKey"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item
                name="s3SecretKey"
                label={t("cloudWorkspace.s3SecretKey")}
                rules={[
                  {
                    required: true,
                    message: t("cloudWorkspace.secretKeyRequired"),
                  },
                ]}
              >
                <Input.Password
                  name="s3SecretKey"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item name="s3Region" label={t("cloudWorkspace.s3Region")}>
                <Input
                  name="s3Region"
                  placeholder="auto"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item
                name="s3Bucket"
                label={t("cloudWorkspace.s3Bucket")}
                rules={[
                  {
                    required: true,
                    message: t("cloudWorkspace.bucketRequired"),
                  },
                ]}
              >
                <Input
                  name="s3Bucket"
                  placeholder={t("cloudWorkspace.bucketPlaceholder")}
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item name="s3Root" label={t("cloudWorkspace.s3Root")}>
                <Input
                  name="s3Root"
                  placeholder="/"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
            </>
          )}

          {storageType === "webdav" && (
            <>
              <Form.Item
                name="webdavUrl"
                label={t("cloudWorkspace.webdavUrl")}
                rules={[
                  {
                    required: true,
                    message: t("cloudWorkspace.webdavUrlRequired"),
                  },
                ]}
              >
                <Input
                  name="webdavUrl"
                  placeholder="https://dav.example.com/remote.php/webdav"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item
                name="webdavUsername"
                label={t("cloudWorkspace.webdavUsername")}
                rules={[
                  {
                    required: true,
                    message: t("cloudWorkspace.usernameRequired"),
                  },
                ]}
              >
                <Input
                  name="webdavUsername"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item
                name="webdavPassword"
                label={t("cloudWorkspace.webdavPassword")}
                rules={[
                  {
                    required: true,
                    message: t("cloudWorkspace.passwordRequired"),
                  },
                ]}
              >
                <Input.Password
                  name="webdavPassword"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
              <Form.Item
                name="webdavPath"
                label={t("cloudWorkspace.webdavPath")}
              >
                <Input
                  name="webdavPath"
                  placeholder="/"
                  onChange={() => setConnectionStatus("unknown")}
                />
              </Form.Item>
            </>
          )}
        </Form>
      </Modal>

      <Modal
        title={
          <Space>
            <FolderOpen size={16} />
            {t("cloudWorkspace.browserTitle")}
          </Space>
        }
        open={browserModalOpen}
        onCancel={() => setBrowserModalOpen(false)}
        footer={
          <Space>
            <Button onClick={() => setBrowserModalOpen(false)}>
              {t("common.close")}
            </Button>
            <Button
              icon={<Upload size={14} />}
              onClick={handleSetAsWorkspace}
              type="primary"
            >
              {t("cloudWorkspace.setAsWorkspace")}
            </Button>
          </Space>
        }
        width={800}
      >
        {currentPath && (
          <div className="mb-3">
            <Text type="secondary" className="text-sm">
              {t("cloudWorkspace.currentPath")}:{" "}
            </Text>
            <Text code className="text-sm">
              {currentPath}
            </Text>
            {currentPath !== "" && (
              <Button size="small" onClick={handleGoBack} className="ml-2">
                {t("cloudWorkspace.goBack")}
              </Button>
            )}
          </div>
        )}
        <Table
          columns={columns}
          dataSource={dirEntries}
          rowKey={(record) => record.path}
          loading={browsing}
          pagination={false}
          size="small"
          onRow={(record) => ({
            onClick: () => handleBrowseEntry(record),
            style: { cursor: record.is_dir ? "pointer" : "default" },
          })}
          locale={{ emptyText: t("cloudWorkspace.emptyDir") }}
        />
      </Modal>

      <Modal
        title={
          <Space>
            <AlertTriangle size={16} className="text-orange-500" />
            {t("cloudWorkspace.conflictsTitle")}
          </Space>
        }
        open={conflictsModalOpen}
        onCancel={() => setConflictsModalOpen(false)}
        footer={
          <Button onClick={() => setConflictsModalOpen(false)}>
            {t("common.close")}
          </Button>
        }
        width={900}
      >
        <div className="mb-4">
          <Text type="secondary">
            {t("cloudWorkspace.conflictStrategy", {
              strategy: conflictStrategy,
            })}
          </Text>
        </div>
        <Table
          columns={conflictColumns}
          dataSource={conflicts}
          rowKey={(record) => record.key}
          loading={conflictsLoading}
          pagination={false}
          size="small"
          locale={{ emptyText: t("cloudWorkspace.noConflicts") }}
        />
      </Modal>
    </div>
  );
}
