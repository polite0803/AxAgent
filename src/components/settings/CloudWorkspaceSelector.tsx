import { invoke } from "@tauri-apps/api/core";
import { Button, Card, Form, Input, message, Modal, Select, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { AlertTriangle, CheckCircle, Cloud, FolderOpen, Globe, RefreshCw, Settings2, Upload } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettingsStore } from "../../stores";

const { Text } = Typography;

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

export default function CloudWorkspaceSelector() {
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

    setConfigModalOpen(true);
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
      const response = await invoke<CloudListResponse>(
        "list_cloud_directory",
        {
          request: {
            workspaceUri: uri,
            dirPath,
          },
        },
      );
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
      message.error(t("cloudWorkspace.loadConflictsFailed", { error: String(e) }));
    } finally {
      setConflictsLoading(false);
    }
  };

  const handleResolveConflict = async (key: string, resolution: string) => {
    const uri = settings.workspace_uri;
    if (!uri) { return; }

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
      const response = await invoke<CloudSyncResponse>(
        "sync_cloud_workspace",
        {
          request: { workspaceUri: uri },
        },
      );

      setSyncResult(response);

      const parts = [];
      if (response.downloaded > 0) {
        parts.push(t("cloudWorkspace.downloaded", { count: response.downloaded }));
      }
      if (response.uploaded > 0) {
        parts.push(t("cloudWorkspace.uploaded", { count: response.uploaded }));
      }
      if (response.conflicts_detected > 0) {
        parts.push(t("cloudWorkspace.conflicts", { count: response.conflicts_detected }));
      }

      const summary = parts.length > 0 ? parts.join("，") : t("cloudWorkspace.syncedUpToDate");

      message.success(summary);

      if (response.pending_conflicts > 0) {
        message.warning(
          t("cloudWorkspace.pendingConflicts", { count: response.pending_conflicts }),
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
            : <Globe size={14} className="text-gray-500" />}
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
        if (record.is_dir) { return "-"; }
        if (size < 1024) { return `${size} B`; }
        if (size < 1024 * 1024) { return `${(size / 1024).toFixed(1)} KB`; }
        return `${(size / (1024 * 1024)).toFixed(1)} MB`;
      },
    },
  ];

  return (
    <>
      <Card
        size="small"
        title={
          <Space>
            <Cloud size={16} />
            {t("cloudWorkspace.title")}
          </Space>
        }
        extra={
          <Space>
            <Button
              size="small"
              icon={<Settings2 size={14} />}
              onClick={openConfigModal}
            >
              {t("cloudWorkspace.configure")}
            </Button>
            <Button
              size="small"
              icon={<FolderOpen size={14} />}
              onClick={openBrowserModal}
            >
              {t("cloudWorkspace.browse")}
            </Button>
            {settings.workspace_uri && (
              <Button
                size="small"
                icon={<RefreshCw size={14} />}
                loading={syncing}
                onClick={handleSyncCloud}
              >
                {t("cloudWorkspace.sync")}
              </Button>
            )}
            {conflicts.length > 0 && (
              <Button
                size="small"
                icon={<AlertTriangle size={14} />}
                type="primary"
                danger
                onClick={loadConflicts}
              >
                {t("cloudWorkspace.viewConflicts", { count: conflicts.length })}
              </Button>
            )}
          </Space>
        }
      >
        {settings.workspace_uri
          ? (
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Text strong>{t("cloudWorkspace.currentWorkspace")}</Text>
                  <Tag icon={<Cloud size={12} />} color="blue">
                    {settings.workspace_uri}
                  </Tag>
                  {settings.cloud_backend === "webdav" && <Tag color="green">WebDAV</Tag>}
                  {settings.cloud_backend === "s3" && <Tag color="purple">S3</Tag>}
                </div>
                <Tag icon={<CheckCircle size={12} />} color="success">
                  {t("cloudWorkspace.active")}
                </Tag>
              </div>
              {syncResult && (
                <Text type="secondary" className="block text-xs">
                  {t("cloudWorkspace.syncResult", {
                    downloaded: syncResult.downloaded,
                    uploaded: syncResult.uploaded,
                    cacheDir: syncResult.local_cache_dir,
                  })}
                </Text>
              )}
            </div>
          )
          : (
            <div className="text-center py-4 text-gray-400">
              <Cloud size={32} className="mx-auto mb-2 opacity-50" />
              <p>{t("cloudWorkspace.notConfigured")}</p>
            </div>
          )}
      </Card>

      <Modal
        title={
          <Space>
            <Settings2 size={16} />
            {t("cloudWorkspace.configTitle")}
          </Space>
        }
        open={configModalOpen}
        onOk={handleSaveConfig}
        onCancel={() => setConfigModalOpen(false)}
        okText={t("common.save")}
        cancelText={t("common.cancel")}
        width={600}
      >
        <Form form={configForm} layout="vertical">
          <Form.Item label={t("cloudWorkspace.storageType")}>
            <Select
              value={storageType}
              onChange={setStorageType}
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
                />
              </Form.Item>
              <Form.Item
                name="s3Endpoint"
                label={t("cloudWorkspace.s3Endpoint")}
                rules={[
                  { required: true, message: t("cloudWorkspace.endpointRequired") },
                ]}
              >
                <Input placeholder="https://s3.amazonaws.com" />
              </Form.Item>
              <Form.Item
                name="s3AccessKey"
                label={t("cloudWorkspace.s3AccessKey")}
                rules={[
                  { required: true, message: t("cloudWorkspace.accessKeyRequired") },
                ]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                name="s3SecretKey"
                label={t("cloudWorkspace.s3SecretKey")}
                rules={[
                  { required: true, message: t("cloudWorkspace.secretKeyRequired") },
                ]}
              >
                <Input.Password />
              </Form.Item>
              <Form.Item name="s3Region" label={t("cloudWorkspace.s3Region")}>
                <Input placeholder="auto" />
              </Form.Item>
              <Form.Item
                name="s3Bucket"
                label={t("cloudWorkspace.s3Bucket")}
                rules={[
                  { required: true, message: t("cloudWorkspace.bucketRequired") },
                ]}
              >
                <Input placeholder={t("cloudWorkspace.bucketPlaceholder")} />
              </Form.Item>
              <Form.Item name="s3Root" label={t("cloudWorkspace.s3Root")}>
                <Input placeholder="/" />
              </Form.Item>
            </>
          )}

          {storageType === "webdav" && (
            <>
              <Form.Item
                name="webdavUrl"
                label={t("cloudWorkspace.webdavUrl")}
                rules={[
                  { required: true, message: t("cloudWorkspace.webdavUrlRequired") },
                ]}
              >
                <Input placeholder="https://dav.example.com/remote.php/webdav" />
              </Form.Item>
              <Form.Item
                name="webdavUsername"
                label={t("cloudWorkspace.webdavUsername")}
                rules={[
                  { required: true, message: t("cloudWorkspace.usernameRequired") },
                ]}
              >
                <Input />
              </Form.Item>
              <Form.Item
                name="webdavPassword"
                label={t("cloudWorkspace.webdavPassword")}
                rules={[
                  { required: true, message: t("cloudWorkspace.passwordRequired") },
                ]}
              >
                <Input.Password />
              </Form.Item>
              <Form.Item
                name="webdavPath"
                label={t("cloudWorkspace.webdavPath")}
              >
                <Input placeholder="/" />
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
            {t("cloudWorkspace.conflictStrategy", { strategy: conflictStrategy })}
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
    </>
  );
}
