// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 本地模型下载面板（llama.cpp 供应商）。
 *
 * - 下载目录 / HF 镜像端点可配置（settings 持久化）
 * - 本地模型列表 = 下载目录中的 *.gguf 文件
 * - 支持从 HuggingFace 仓库或直接 URL 下载（后台任务 + 进度轮询）
 * - 下载完成/删除后自动刷新供应商模型列表（fetch_remote_models 扫描目录）
 */
import { showBackendError } from "@/lib/errorI18n";
import { invoke, logIpcError } from "@/lib/invoke";
import { useProviderStore } from "@/stores";
import type { DownloadRequest, DownloadTaskInfo, LocalFileModel, PresetModelDto } from "@/types";
import {
  CloudDownloadOutlined,
  DeleteOutlined,
  FolderOpenOutlined,
  ReloadOutlined,
  SaveOutlined,
} from "@ant-design/icons";
import { App, Button, Card, Form, Input, Popconfirm, Progress, Select, Space, Table, Tag, Typography } from "antd";
import type { ColumnsType } from "antd/es/table";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;

/** 下载进度轮询间隔（ms） */
const PROGRESS_POLL_MS = 1500;

function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || bytes <= 0) {
    return "-";
  }
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }
  return `${(bytes / 1024).toFixed(1)} KB`;
}

export function ModelDownloadPanel({ providerId }: { providerId: string }) {
  const { t } = useTranslation();
  const { message } = App.useApp();
  const fetchRemoteModels = useProviderStore((s) => s.fetchRemoteModels);

  const [downloadDir, setDownloadDir] = useState("");
  const [hfEndpoint, setHfEndpoint] = useState("https://huggingface.co");
  const [localModels, setLocalModels] = useState<LocalFileModel[]>([]);
  const [presets, setPresets] = useState<PresetModelDto[]>([]);
  const [tasks, setTasks] = useState<DownloadTaskInfo[]>([]);
  const [saving, setSaving] = useState(false);

  // 下载表单
  const [selectedPreset, setSelectedPreset] = useState<string | undefined>();
  const [filename, setFilename] = useState("");
  const [hfRepo, setHfRepo] = useState("");
  const [directUrl, setDirectUrl] = useState("");
  const [downloading, setDownloading] = useState(false);

  const loadAll = useCallback(async () => {
    try {
      const [dir, ep, models, ps, ts] = await Promise.all([
        invoke<string>("local_model_get_download_dir"),
        invoke<string>("local_model_get_hf_endpoint"),
        invoke<LocalFileModel[]>("local_model_list_local_models"),
        invoke<PresetModelDto[]>("local_model_get_presets"),
        invoke<DownloadTaskInfo[]>("local_model_download_progress"),
      ]);
      setDownloadDir(dir);
      setHfEndpoint(ep);
      setLocalModels(models);
      setPresets(ps);
      setTasks(ts);
    } catch (e) {
      logIpcError("local_model_list_local_models")(e);
    }
  }, []);

  // 初始加载
  useEffect(() => {
    void loadAll();
  }, [loadAll]);

  // 有下载中任务时轮询（任务完成/失败后自动停止）
  const hasActiveTask = tasks.some((x) => x.status === "downloading");
  useEffect(() => {
    if (!hasActiveTask) {
      return;
    }
    const timer = setInterval(() => {
      void loadAll();
    }, PROGRESS_POLL_MS);
    return () => clearInterval(timer);
  }, [hasActiveTask, loadAll]);

  const handleSaveDir = useCallback(async () => {
    setSaving(true);
    try {
      const dir = await invoke<string>("local_model_set_download_dir", {
        dir: downloadDir,
      });
      setDownloadDir(dir);
      message.success(t("settings.localModel.downloadDirSaved"));
      await loadAll();
    } catch (e) {
      showBackendError(message, e, { context: "local_model_set_download_dir" });
    } finally {
      setSaving(false);
    }
  }, [downloadDir, loadAll, message, t]);

  const handleSaveEndpoint = useCallback(async () => {
    setSaving(true);
    try {
      const ep = await invoke<string>("local_model_set_hf_endpoint", {
        endpoint: hfEndpoint,
      });
      setHfEndpoint(ep);
      message.success(t("settings.localModel.hfEndpointSaved"));
    } catch (e) {
      showBackendError(message, e, { context: "local_model_set_hf_endpoint" });
    } finally {
      setSaving(false);
    }
  }, [hfEndpoint, message, t]);

  const handleSelectPreset = useCallback(
    (value: string) => {
      setSelectedPreset(value);
      const preset = presets.find((p) => p.filename === value);
      if (preset) {
        setFilename(preset.filename);
        setHfRepo(preset.hfRepo ?? "");
        setDirectUrl(preset.directUrl ?? "");
      }
    },
    [presets],
  );

  const handleDownload = useCallback(async () => {
    if (!filename.trim()) {
      message.warning(t("settings.localModel.downloadNeedFilename"));
      return;
    }
    if (!hfRepo.trim() && !directUrl.trim()) {
      message.warning(t("settings.localModel.downloadNeedSource"));
      return;
    }
    setDownloading(true);
    try {
      const req: DownloadRequest = {
        filename: filename.trim(),
        hfRepo: hfRepo.trim() || null,
        directUrl: directUrl.trim() || null,
      };
      await invoke<DownloadTaskInfo>("local_model_download", { request: req });
      message.success(t("settings.localModel.downloadStarted"));
      setSelectedPreset(undefined);
      await loadAll();
    } catch (e) {
      showBackendError(message, e, { context: "local_model_download" });
    } finally {
      setDownloading(false);
    }
  }, [filename, hfRepo, directUrl, loadAll, message, t]);

  const handleDelete = useCallback(
    async (name: string) => {
      try {
        await invoke<void>("local_model_delete_local_model", { filename: name });
        message.success(t("settings.localModel.deleted"));
        await loadAll();
        // 刷新供应商模型列表（目录扫描）
        await fetchRemoteModels(providerId);
      } catch (e) {
        showBackendError(message, e, { context: "local_model_delete_local_model" });
      }
    },
    [loadAll, fetchRemoteModels, providerId, message, t],
  );

  const handleRefreshModels = useCallback(async () => {
    try {
      await fetchRemoteModels(providerId);
      message.success(t("settings.localModel.refreshModelsDone"));
    } catch (e) {
      showBackendError(message, e, { context: "fetch_remote_models" });
    }
  }, [fetchRemoteModels, providerId, message, t]);

  const taskOf = (name: string): DownloadTaskInfo | undefined => tasks.find((x) => x.filename === name);

  const columns: ColumnsType<LocalFileModel> = [
    {
      title: t("settings.localModel.filename"),
      dataIndex: "filename",
      key: "filename",
      ellipsis: true,
      render: (v: string) => {
        const task = taskOf(v);
        if (task && task.status === "failed") {
          return (
            <Space size={6}>
              <Text>{v}</Text>
              <Tag color="red">{t("settings.localModel.downloadFailed")}</Tag>
            </Space>
          );
        }
        return v;
      },
    },
    {
      title: t("settings.localModel.type"),
      dataIndex: "modelType",
      key: "modelType",
      width: 110,
      render: (v: string) => {
        const color = v === "embedding" ? "cyan" : v === "chat" ? "blue" : "purple";
        return <Tag color={color}>{v}</Tag>;
      },
    },
    {
      title: t("settings.localModel.size"),
      dataIndex: "sizeBytes",
      key: "size",
      width: 110,
      render: (v: number) => formatBytes(v),
    },
    {
      title: t("settings.localModel.status"),
      key: "status",
      width: 220,
      render: (_, r) => {
        const task = taskOf(r.filename);
        if (r.isDownloading || task?.status === "downloading") {
          const downloaded = task?.downloadedBytes ?? r.downloadBytes ?? 0;
          const total = task?.totalBytes ?? 0;
          const percent = total > 0
            ? Math.min(100, Math.round((downloaded / total) * 100))
            : 0;
          return (
            <Progress
              size="small"
              percent={percent}
              format={() => `${formatBytes(downloaded)}`}
            />
          );
        }
        if (task?.status === "done") {
          return <Tag color="green">{t("settings.localModel.downloadDone")}</Tag>;
        }
        if (task?.status === "failed") {
          return <Tag color="red">{t("settings.localModel.downloadFailed")}</Tag>;
        }
        return <Tag>{t("settings.localModel.ready")}</Tag>;
      },
    },
    {
      title: t("settings.localModel.action"),
      key: "action",
      width: 90,
      render: (_, r) => (
        <Popconfirm
          title={t("settings.localModel.deleteConfirm", { name: r.filename })}
          onConfirm={() => void handleDelete(r.filename)}
          okText={t("common.confirm")}
          cancelText={t("common.cancel")}
          okButtonProps={{ danger: true }}
        >
          <Button
            size="small"
            danger
            type="text"
            icon={<DeleteOutlined />}
            disabled={r.isDownloading}
          />
        </Popconfirm>
      ),
    },
  ];

  return (
    <Card
      title={
        <Space>
          <FolderOpenOutlined />
          <span>{t("settings.localModel.downloadTitle")}</span>
        </Space>
      }
      size="small"
      extra={
        <Button
          size="small"
          icon={<ReloadOutlined />}
          onClick={() => void loadAll()}
        >
          {t("settings.localModel.refresh")}
        </Button>
      }
    >
      {/* ── 目录与端点设置 ── */}
      <Space.Compact style={{ width: "100%", marginBottom: 8 }}>
        <Input
          prefix={<FolderOpenOutlined />}
          value={downloadDir}
          onChange={(e) => setDownloadDir(e.target.value)}
          placeholder={t("settings.localModel.downloadDirPlaceholder")}
          style={{ flex: 1 }}
        />
        <Button
          loading={saving}
          icon={<SaveOutlined />}
          onClick={() => void handleSaveDir()}
        >
          {t("settings.localModel.downloadDirSave")}
        </Button>
      </Space.Compact>
      <Space.Compact style={{ width: "100%", marginBottom: 12 }}>
        <Input
          prefix={<CloudDownloadOutlined />}
          value={hfEndpoint}
          onChange={(e) => setHfEndpoint(e.target.value)}
          placeholder={t("settings.localModel.hfEndpointPlaceholder")}
          style={{ flex: 1 }}
        />
        <Button
          loading={saving}
          icon={<SaveOutlined />}
          onClick={() => void handleSaveEndpoint()}
        >
          {t("settings.localModel.hfEndpointSave")}
        </Button>
      </Space.Compact>
      <Text type="secondary" style={{ fontSize: 12, display: "block", marginBottom: 12 }}>
        {t("settings.localModel.downloadHint")}
      </Text>

      {/* ── 下载表单 ── */}
      <Form layout="vertical" style={{ marginBottom: 16 }}>
        <Form.Item label={t("settings.localModel.selectPreset")} style={{ marginBottom: 8 }}>
          <Select
            value={selectedPreset}
            onChange={handleSelectPreset}
            allowClear
            placeholder={t("settings.localModel.selectPresetPlaceholder")}
            options={presets.map((p) => ({
              label: (
                <Space size={6}>
                  <span>{p.displayName}</span>
                  <Tag color={p.isDownloaded ? "success" : "default"}>
                    {p.isDownloaded
                      ? t("settings.localModel.downloaded")
                      : formatBytes(p.sizeBytes)}
                  </Tag>
                </Space>
              ),
              value: p.filename,
            }))}
          />
        </Form.Item>
        <Form.Item label={t("settings.localModel.filename")} required style={{ marginBottom: 8 }}>
          <Input
            value={filename}
            onChange={(e) => setFilename(e.target.value)}
            placeholder={t("settings.localModel.filenamePlaceholder")}
          />
        </Form.Item>
        <Form.Item label={t("settings.localModel.hfRepo")} style={{ marginBottom: 8 }}>
          <Input
            value={hfRepo}
            onChange={(e) => setHfRepo(e.target.value)}
            placeholder={t("settings.localModel.hfRepoPlaceholder")}
          />
        </Form.Item>
        <Form.Item label={t("settings.localModel.directUrl")} style={{ marginBottom: 8 }}>
          <Input
            value={directUrl}
            onChange={(e) => setDirectUrl(e.target.value)}
            placeholder={t("settings.localModel.directUrlPlaceholder")}
          />
        </Form.Item>
        <Space style={{ marginBottom: 4 }}>
          <Button
            type="primary"
            icon={<CloudDownloadOutlined />}
            loading={downloading}
            onClick={() => void handleDownload()}
          >
            {t("settings.localModel.download")}
          </Button>
          <Button icon={<ReloadOutlined />} onClick={() => void handleRefreshModels()}>
            {t("settings.localModel.refreshModels")}
          </Button>
        </Space>
      </Form>

      {/* ── 本地模型列表 ── */}
      <Table
        rowKey="filename"
        size="small"
        columns={columns}
        dataSource={localModels}
        pagination={false}
        locale={{ emptyText: t("settings.localModel.noModels") }}
      />
    </Card>
  );
}
