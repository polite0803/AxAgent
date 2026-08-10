// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 本地 llama.cpp 模型管理面板。
 *
 * 挂在 llama_cpp 类型供应商的详情页中，提供：
 * - 运行状态查看：健康状态 / 进程信息 / 模型元数据（维度、量化、参数、上下文等）
 * - 启停操作：托管启动（配置表单）、停止、重启、状态刷新（自动轮询）
 * - 嵌入连通性测试
 * - 服务日志查看
 */
import { ModelDownloadPanel } from "@/components/settings/ModelDownloadPanel";
import { showBackendError } from "@/lib/errorI18n";
import { invoke, logIpcError } from "@/lib/invoke";
import type {
  EmbedTestResult,
  LlamaCppInstallStatus,
  LlamaCppVersionInfo,
  LocalFileModel,
  LocalModelStartConfig,
  LocalModelStatus,
} from "@/types";
import {
  CloudDownloadOutlined,
  PlayCircleOutlined,
  PoweroffOutlined,
  ReloadOutlined,
  RobotOutlined,
  StopOutlined,
} from "@ant-design/icons";
import {
  App,
  AutoComplete,
  Badge,
  Button,
  Card,
  Descriptions,
  Divider,
  Form,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Progress,
  Space,
  Switch,
  Tag,
  Typography,
} from "antd";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

/** 状态自动轮询间隔（ms） */
const POLL_INTERVAL_MS = 10_000;
/** 启动配置 localStorage 键 */
const CONFIG_STORAGE_KEY = "localModel.startConfig";
/** 默认启动配置（不含硬编码本地路径，首次使用需用户选择模型） */
const DEFAULT_CONFIG: LocalModelStartConfig = {
  serverExe: "llama-server",
  modelPath: "",
  host: "127.0.0.1",
  port: 8091,
  alias: null,
  nCtx: null,
  nGpuLayers: null,
  embeddingMode: true,
  extraArgs: [],
};

/** 主机校验正则：允许 localhost、IPv4、单主机名 */
const HOST_REGEX = /^(localhost|(\d{1,3}\.){3}\d{1,3}|[a-zA-Z0-9-]+)$/;

/** 简单的前端配置校验（启动前同步检查，后端会做最终校验） */
function validateStartConfig(
  cfg: LocalModelStartConfig,
): string | null {
  if (!cfg.serverExe.trim()) {
    return "serverExeRequired";
  }
  if (!cfg.modelPath.trim()) {
    return "modelPathRequired";
  }
  if (!HOST_REGEX.test(cfg.host.trim())) {
    return "hostInvalid";
  }
  if (cfg.port < 1 || cfg.port > 65535) {
    return "portInvalid";
  }
  return null;
}

function formatBytes(bytes: number | null): string {
  if (bytes == null || bytes <= 0) {
    return "-";
  }
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function healthColor(health: string): "success" | "warning" | "error" | "default" {
  if (health === "ok") {
    return "success";
  }
  if (health === "loading") {
    return "warning";
  }
  return health === "unreachable" ? "error" : "default";
}

export function LocalModelPanel({
  providerId,
  apiHost,
}: {
  providerId: string;
  apiHost: string;
}) {
  const { t } = useTranslation();
  const { message } = App.useApp();

  const [status, setStatus] = useState<LocalModelStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [startModalOpen, setStartModalOpen] = useState(false);
  const [logModalOpen, setLogModalOpen] = useState(false);
  const [logContent, setLogContent] = useState("");
  const [logLoading, setLogLoading] = useState(false);
  const [embedText, setEmbedText] = useState("");
  // 启动表单：下载目录中的模型文件（用于 model_path 快速选择）
  const [localModelFiles, setLocalModelFiles] = useState<LocalFileModel[]>([]);
  const [downloadDir, setDownloadDir] = useState("");
  const [embedResult, setEmbedResult] = useState<EmbedTestResult | null>(null);
  const [embedLoading, setEmbedLoading] = useState(false);
  const [serverExists, setServerExists] = useState<boolean | null>(null);
  const [installStatus, setInstallStatus] = useState<LlamaCppInstallStatus | null>(null);
  const [latestVersion, setLatestVersion] = useState<LlamaCppVersionInfo | null>(null);
  const [installModalOpen, setInstallModalOpen] = useState(false);
  const [installing, setInstalling] = useState(false);
  const [startConfig, setStartConfig] = useState<LocalModelStartConfig>(() => {
    try {
      const raw = localStorage.getItem(CONFIG_STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as Partial<LocalModelStartConfig>;
        return { ...DEFAULT_CONFIG, ...parsed };
      }
    } catch {
      // ignore corrupted storage
    }
    return { ...DEFAULT_CONFIG };
  });

  const refreshRef = useRef<() => void>(() => {});
  refreshRef.current = useCallback(async () => {
    try {
      const st = await invoke<LocalModelStatus>("local_model_status", {
        providerId,
      });
      setStatus(st);
      return st;
    } catch (e) {
      logIpcError("local_model_status")(e);
      return null;
    }
  }, [providerId]);

  const refresh = useCallback(async () => {
    setLoading(true);
    await refreshRef.current();
    setLoading(false);
  }, []);

  // 初次加载 + 自动轮询
  useEffect(() => {
    void refreshRef.current();
    const timer = setInterval(() => {
      void refreshRef.current();
    }, POLL_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [providerId]);

  // 加载下载目录与本地模型文件（启动表单选路径用）。
  // 若当前配置没有 modelPath，则自动填入下载目录中第一个就绪的模型文件，避免首次使用空配置。
  useEffect(() => {
    void (async () => {
      try {
        const [dir, files] = await Promise.all([
          invoke<string>("local_model_get_download_dir"),
          invoke<LocalFileModel[]>("local_model_list_local_models"),
        ]);
        const readyFiles = files.filter((f) => !f.isDownloading);
        setDownloadDir(dir);
        setLocalModelFiles(readyFiles);

        // 当用户未设置 modelPath 时自动填充下载目录中的第一个就绪模型
        setStartConfig((prev) => {
          if (prev.modelPath) {
            return prev;
          }
          const first = readyFiles[0];
          if (!first) {
            return prev;
          }
          const next = {
            ...prev,
            modelPath: `${dir}\\${first.filename}`,
            alias: first.filename.replace(/\.gguf$/i, ""),
          };
          try {
            localStorage.setItem(CONFIG_STORAGE_KEY, JSON.stringify(next));
          } catch {
            // ignore 持久化失败
          }
          return next;
        });
      } catch (e) {
        logIpcError("local_model_list_local_models")(e);
      }
    })();
  }, [providerId]);

  // 检测 llama-server 是否已安装
  useEffect(() => {
    void (async () => {
      try {
        // 1. 检查当前配置的 serverExe 是否存在
        const path = await invoke<string | null>("local_model_check_server", {
          serverExe: startConfig.serverExe,
        });
        setServerExists(path != null);

        // 2. 检查安装状态
        const status = await invoke<LlamaCppInstallStatus>("local_model_get_install_status");
        setInstallStatus(status);

        // 如果已安装但路径未在配置中，自动设置
        if (status.installed && status.executablePath) {
          const currentPath = startConfig.serverExe;
          // 如果当前配置的路径不存在，但已安装版本的路径存在，则自动更新
          const currentExists = path != null;
          if (!currentExists && status.executablePath !== currentPath) {
            const newConfig = { ...startConfig, serverExe: status.executablePath };
            setStartConfig(newConfig);
            localStorage.setItem(CONFIG_STORAGE_KEY, JSON.stringify(newConfig));
            setServerExists(true);
          }
        }
      } catch (e) {
        logIpcError("local_model_check_server")(e);
      }
    })();
  }, [providerId, startConfig.serverExe]);

  // 安装进度轮询
  useEffect(() => {
    if (!installing) { return; }
    const timer = setInterval(async () => {
      try {
        const status = await invoke<LlamaCppInstallStatus>("local_model_get_install_status");
        setInstallStatus(status);
        if (!status.isDownloading) {
          setInstalling(false);
          if (status.installed && status.executablePath) {
            // 安装成功，更新配置
            const newConfig = { ...startConfig, serverExe: status.executablePath };
            setStartConfig(newConfig);
            localStorage.setItem(CONFIG_STORAGE_KEY, JSON.stringify(newConfig));
            setServerExists(true);
            message.success(t("settings.localModel.installSuccess"));
          } else if (status.downloadError) {
            message.error(status.downloadError);
          }
        }
      } catch {
        // ignore polling errors
      }
    }, 1000);
    return () => clearInterval(timer);
  }, [installing, startConfig, message, t]);

  const running = status?.running ?? false;
  const health = status?.health ?? "unknown";

  const handleCheckUpdate = useCallback(async () => {
    try {
      const version = await invoke<LlamaCppVersionInfo>("local_model_get_latest_version");
      setLatestVersion(version);
      setInstallModalOpen(true);
    } catch (e) {
      showBackendError(message, e, { context: "local_model_get_latest_version" });
    }
  }, [message]);

  const handleInstall = useCallback(async () => {
    if (!latestVersion) {
      await handleCheckUpdate();
      return;
    }
    setInstalling(true);
    setInstallModalOpen(false);
    setInstallStatus({
      installed: false,
      version: latestVersion.tag,
      installPath: null,
      executablePath: null,
      isDownloading: true,
      downloadProgress: 0,
      downloadError: null,
    });
    try {
      // 立即提示已进入安装流程（后端实际下载通过轮询 install_status 跟踪进度）
      message.info(
        t("settings.localModel.installStarted", {
          version: latestVersion.tag,
        }),
      );
      await invoke<LlamaCppInstallStatus>("local_model_install_server", {
        tag: latestVersion.tag,
      });
    } catch (e) {
      setInstalling(false);
      // 同步写入错误态，确保 UI 即使在轮询前也能看到失败原因
      setInstallStatus((prev) =>
        prev
          ? { ...prev, isDownloading: false, downloadError: String(e) }
          : prev
      );
      showBackendError(message, e, { context: "local_model_install_server" });
    }
  }, [latestVersion, message, handleCheckUpdate, t]);

  const handleStart = useCallback(async () => {
    // 前端预校验（同步、快速失败）
    const invalidKey = validateStartConfig(startConfig);
    if (invalidKey) {
      message.error(t(`settings.localModel.${invalidKey}`));
      return;
    }
    if (serverExists === false) {
      message.warning(t("settings.localModel.serverNotFoundCannotStart"));
      setStartModalOpen(true);
      return;
    }
    try {
      await invoke<LocalModelStatus>("local_model_start", {
        providerId,
        config: startConfig,
      });
      localStorage.setItem(CONFIG_STORAGE_KEY, JSON.stringify(startConfig));
      message.success(t("settings.localModel.startSuccess"));
      setStartModalOpen(false);
      void refreshRef.current();
    } catch (e) {
      // 按后端错误码自动翻译：端口冲突、配置无效等都有独立 i18n 文案
      showBackendError(message, e, { context: "local_model_start" });
    }
  }, [providerId, startConfig, serverExists, message, t]);

  const handleStop = useCallback(async () => {
    try {
      await invoke<void>("local_model_stop", { providerId });
      message.success(t("settings.localModel.stopSuccess"));
      void refreshRef.current();
    } catch (e) {
      showBackendError(message, e, { context: "local_model_stop" });
    }
  }, [providerId, message, t]);

  const handleRestart = useCallback(async () => {
    await handleStop();
    // 等待端口释放（taskkill 后进程退出有延迟），避免新进程绑定端口失败
    await new Promise((resolve) => setTimeout(resolve, 1200));
    // 用已保存配置自动拉起；无有效配置时提示手动配置
    const cfg = startConfig;
    if (cfg.serverExe && cfg.modelPath) {
      try {
        await invoke<LocalModelStatus>("local_model_start", {
          providerId,
          config: cfg,
        });
        message.success(t("settings.localModel.restartSuccess"));
      } catch (e) {
        showBackendError(message, e, { context: "local_model_start(restart)" });
      }
    } else {
      message.info(t("settings.localModel.restartNeedConfig"));
      setStartModalOpen(true);
    }
    void refreshRef.current();
  }, [providerId, startConfig, handleStop, message, t]);

  const handleEmbedTest = useCallback(async () => {
    if (!embedText.trim()) {
      message.warning(t("settings.localModel.embedTestEmpty"));
      return;
    }
    setEmbedLoading(true);
    setEmbedResult(null);
    try {
      const res = await invoke<EmbedTestResult>("local_model_embed_test", {
        providerId,
        text: embedText,
      });
      setEmbedResult(res);
    } catch (e) {
      showBackendError(message, e, { context: "local_model_embed_test" });
    } finally {
      setEmbedLoading(false);
    }
  }, [providerId, embedText, message, t]);

  const handleOpenLogs = useCallback(async () => {
    setLogModalOpen(true);
    setLogLoading(true);
    try {
      const content = await invoke<string>("local_model_logs", {
        providerId,
        maxLines: 300,
      });
      setLogContent(content || t("settings.localModel.logEmpty"));
    } catch (e) {
      logIpcError("local_model_logs")(e);
      setLogContent("");
      showBackendError(message, e, { context: "local_model_logs" });
    } finally {
      setLogLoading(false);
    }
  }, [providerId, message, t]);

  const modelMeta = useMemo(() => status?.model ?? null, [status]);
  const propsMeta = useMemo(() => status?.props ?? null, [status]);

  return (
    <Card
      title={
        <Space>
          <RobotOutlined />
          <span>{t("settings.localModel.title")}</span>
          <Tag
            color={running ? "success" : "error"}
            style={{ marginInlineStart: 8 }}
          >
            {running
              ? t("settings.localModel.statusRunning")
              : t("settings.localModel.statusStopped")}
          </Tag>
        </Space>
      }
      size="small"
      extra={
        <Space>
          {serverExists === false && (
            <Button
              type="primary"
              size="small"
              icon={<CloudDownloadOutlined />}
              loading={installing}
              onClick={() => void handleCheckUpdate()}
            >
              {installing
                ? t("settings.localModel.installing")
                : t("settings.localModel.install")}
            </Button>
          )}
          <Button
            icon={<ReloadOutlined />}
            size="small"
            loading={loading}
            onClick={() => void refresh()}
          >
            {t("settings.localModel.refresh")}
          </Button>
          {running && (
            <Button
              size="small"
              icon={<StopOutlined />}
              danger
              onClick={() => void handleStop()}
            >
              {t("settings.localModel.stop")}
            </Button>
          )}
          {!running && (
            <Button
              type="primary"
              size="small"
              icon={<PlayCircleOutlined />}
              onClick={() => setStartModalOpen(true)}
            >
              {t("settings.localModel.start")}
            </Button>
          )}
        </Space>
      }
    >
      {/* ── 服务状态 ── */}
      <Descriptions
        size="small"
        column={{ xs: 1, sm: 2, md: 3 }}
        style={{ marginBottom: 8 }}
      >
        <Descriptions.Item label={t("settings.localModel.health")}>
          <Badge status={healthColor(health)} text={health} />
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.baseUrl")}>
          <Text code style={{ fontSize: 12 }}>
            {status?.baseUrl ?? apiHost}
          </Text>
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.managed")}>
          {status?.managed
            ? <Tag color="blue">{t("settings.localModel.managedYes")}</Tag>
            : <Tag>{t("settings.localModel.managedNo")}</Tag>}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.pid")}>
          {status?.pid ?? "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.processName")}>
          {status?.processName ?? "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.memory")}>
          {status?.memoryMb != null
            ? `${status.memoryMb} MB`
            : "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.serverStatus")}>
          {installing && installStatus?.isDownloading
            ? (
              <Space direction="vertical" size={4} style={{ width: 200 }}>
                <Text type="warning" style={{ fontSize: 12 }}>
                  {t("settings.localModel.installing")}
                </Text>
                <Progress
                  percent={Math.round(installStatus.downloadProgress ?? 0)}
                  size="small"
                  style={{ margin: 0 }}
                />
              </Space>
            )
            : serverExists == null
            ? <Text type="secondary">{t("common.loading")}</Text>
            : serverExists
            ? (
              <Tag color="success">
                {installStatus?.version
                  ? `v${installStatus.version}`
                  : t("settings.localModel.serverInstalled")}
              </Tag>
            )
            : (
              <Space>
                <Tag color="error">{t("settings.localModel.serverMissing")}</Tag>
                <Button
                  type="link"
                  size="small"
                  icon={<CloudDownloadOutlined />}
                  onClick={() => void handleCheckUpdate()}
                >
                  {t("settings.localModel.install")}
                </Button>
              </Space>
            )}
        </Descriptions.Item>
      </Descriptions>

      {/* ── 模型信息 ── */}
      <Divider style={{ margin: "8px 0" }} />
      <Descriptions
        size="small"
        column={{ xs: 1, sm: 2, md: 3 }}
        style={{ marginBottom: 8 }}
      >
        <Descriptions.Item label={t("settings.localModel.modelId")} span={2}>
          <Text code style={{ fontSize: 12 }}>
            {modelMeta?.id ?? propsMeta?.modelAlias ?? "-"}
          </Text>
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.modelPath")} span={3}>
          {propsMeta?.modelPath ?? "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.dimensions")}>
          {modelMeta?.nEmbd ?? "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.quantization")}>
          {modelMeta?.ftype ?? propsMeta?.modelFtype ?? "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.params")}>
          {modelMeta?.nParams != null
            ? `${(modelMeta.nParams / 1_000_000).toFixed(0)}M`
            : "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.size")}>
          {formatBytes(modelMeta?.sizeBytes ?? null)}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.ctxWindow")}>
          {modelMeta?.nCtx ?? propsMeta?.nCtx ?? "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.vocab")}>
          {modelMeta?.nVocab ?? "-"}
        </Descriptions.Item>
        <Descriptions.Item label={t("settings.localModel.slots")}>
          {propsMeta?.totalSlots ?? "-"}
        </Descriptions.Item>
      </Descriptions>

      {/* ── 操作区 ── */}
      <Divider style={{ margin: "8px 0" }} />
      <Space wrap style={{ marginBottom: 16 }}>
        {running && (
          <>
            <Popconfirm
              title={t("settings.localModel.stopConfirm")}
              onConfirm={() => void handleStop()}
              okText={t("common.confirm")}
              cancelText={t("common.cancel")}
              okButtonProps={{ danger: true }}
            >
              <Button icon={<PoweroffOutlined />} danger>
                {t("settings.localModel.stop")}
              </Button>
            </Popconfirm>
            <Popconfirm
              title={t("settings.localModel.restartConfirm")}
              onConfirm={() => void handleRestart()}
              okText={t("common.confirm")}
              cancelText={t("common.cancel")}
            >
              <Button icon={<ReloadOutlined />}>
                {t("settings.localModel.restart")}
              </Button>
            </Popconfirm>
          </>
        )}
        {!running && (
          <Button
            type="primary"
            icon={<PlayCircleOutlined />}
            onClick={() => setStartModalOpen(true)}
          >
            {t("settings.localModel.start")}
          </Button>
        )}
        <Button icon={<ReloadOutlined />} onClick={() => void refresh()}>
          {t("settings.localModel.refreshStatus")}
        </Button>
        <Button onClick={() => void handleOpenLogs()}>
          {t("settings.localModel.viewLogs")}
        </Button>
      </Space>

      {/* ── 嵌入测试 ── */}
      <Divider plain style={{ margin: "8px 0" }}>
        {t("settings.localModel.embedTest")}
      </Divider>
      <Space.Compact style={{ width: "100%", marginBottom: 8 }}>
        <Input.TextArea
          value={embedText}
          onChange={(e) => setEmbedText(e.target.value)}
          placeholder={t("settings.localModel.embedTestPlaceholder")}
          autoSize={{ minRows: 1, maxRows: 3 }}
          style={{ flex: 1 }}
        />
        <Button
          type="primary"
          loading={embedLoading}
          onClick={() => void handleEmbedTest()}
        >
          {t("settings.localModel.embedTestRun")}
        </Button>
      </Space.Compact>
      {embedResult && (
        <Paragraph
          style={{ marginBottom: 0, fontSize: 12 }}
        >
          <Text type="secondary">
            {t("settings.localModel.embedTestResult", {
              dims: String(embedResult.dimensions),
              ms: String(embedResult.elapsedMs),
            })}
          </Text>
          <br />
          <Text code style={{ fontSize: 11, wordBreak: "break-all" }}>
            [{embedResult.preview.map((v) => v.toFixed(4)).join(", ")}
            {embedResult.dimensions > embedResult.preview.length ? ", ..." : ""}]
          </Text>
        </Paragraph>
      )}

      {/* ── 模型下载与本地模型库 ── */}
      <ModelDownloadPanel providerId={providerId} />

      {/* ── 启动配置 Modal ── */}
      <Modal
        title={t("settings.localModel.startTitle")}
        open={startModalOpen}
        onOk={() => void handleStart()}
        onCancel={() => setStartModalOpen(false)}
        okText={t("settings.localModel.start")}
        cancelText={t("common.cancel")}
        width={560}
      >
        <Form layout="vertical" style={{ marginTop: 16 }}>
          <Form.Item label={t("settings.localModel.serverExe")} required>
            <Space.Compact style={{ width: "100%" }}>
              <Input
                value={startConfig.serverExe}
                onChange={(e) => setStartConfig({ ...startConfig, serverExe: e.target.value })}
                placeholder={t("settings.localModel.serverExePlaceholder")}
                style={{ flex: 1 }}
              />
              {serverExists === false && !installing && (
                <Button
                  icon={<CloudDownloadOutlined />}
                  onClick={() => void handleCheckUpdate()}
                >
                  {t("settings.localModel.install")}
                </Button>
              )}
            </Space.Compact>
            {serverExists === false && (
              <Text type="warning" style={{ fontSize: 12 }}>
                {t("settings.localModel.serverNotFoundHint")}
              </Text>
            )}
            {installing && installStatus?.isDownloading && (
              <Progress
                percent={Math.round(installStatus.downloadProgress ?? 0)}
                status="active"
                style={{ marginTop: 8 }}
              />
            )}
          </Form.Item>
          <Form.Item label={t("settings.localModel.modelPathLabel")} required>
            <AutoComplete
              value={startConfig.modelPath}
              onChange={(v) => setStartConfig({ ...startConfig, modelPath: v })}
              placeholder={t("settings.localModel.modelPathPlaceholder")}
              options={localModelFiles.map((f) => ({
                label: `${downloadDir}\\${f.filename}`,
                value: `${downloadDir}\\${f.filename}`,
              }))}
              filterOption={(inputValue, option) =>
                String(option?.value ?? "").toLowerCase().includes(inputValue.toLowerCase())}
            />
          </Form.Item>
          <Space.Compact style={{ width: "100%" }}>
            <Form.Item
              label={t("settings.localModel.host")}
              style={{ flex: 1, marginInlineEnd: 8 }}
            >
              <Input
                value={startConfig.host}
                onChange={(e) => setStartConfig({ ...startConfig, host: e.target.value })}
              />
            </Form.Item>
            <Form.Item label={t("settings.localModel.port")}>
              <InputNumber
                min={1}
                max={65535}
                value={startConfig.port}
                onChange={(v) => setStartConfig({ ...startConfig, port: v ?? 8091 })}
              />
            </Form.Item>
          </Space.Compact>
          <Form.Item label={t("settings.localModel.alias")}>
            <Input
              value={startConfig.alias ?? ""}
              onChange={(e) =>
                setStartConfig({
                  ...startConfig,
                  alias: e.target.value || null,
                })}
              placeholder={t("settings.localModel.aliasPlaceholder")}
            />
          </Form.Item>
          <Space.Compact style={{ width: "100%" }}>
            <Form.Item
              label={t("settings.localModel.nCtx")}
              style={{ flex: 1, marginInlineEnd: 8 }}
            >
              <InputNumber
                min={128}
                step={1024}
                style={{ width: "100%" }}
                value={startConfig.nCtx ?? undefined}
                onChange={(v) => setStartConfig({ ...startConfig, nCtx: v ?? null })}
              />
            </Form.Item>
            <Form.Item label={t("settings.localModel.nGpuLayers")}>
              <InputNumber
                min={-1}
                value={startConfig.nGpuLayers ?? undefined}
                onChange={(v) => setStartConfig({ ...startConfig, nGpuLayers: v ?? null })}
              />
            </Form.Item>
          </Space.Compact>
          <Form.Item
            label={t("settings.localModel.embeddingMode")}
            style={{ marginBottom: 8 }}
          >
            <Switch
              checked={startConfig.embeddingMode ?? true}
              onChange={(checked) => setStartConfig({ ...startConfig, embeddingMode: checked })}
            />
          </Form.Item>
          <Form.Item
            label={t("settings.localModel.extraArgs")}
            style={{ marginBottom: 0 }}
          >
            <Input.TextArea
              value={startConfig.extraArgs.join("\n")}
              onChange={(e) =>
                setStartConfig({
                  ...startConfig,
                  extraArgs: e.target.value
                    .split("\n")
                    .map((s) => s.trim())
                    .filter(Boolean),
                })}
              placeholder={"--threads 8\n--cont-batch-size 2048"}
              autoSize={{ minRows: 2, maxRows: 4 }}
            />
          </Form.Item>
        </Form>
      </Modal>

      {/* ── 日志 Modal ── */}
      <Modal
        title={t("settings.localModel.viewLogs")}
        open={logModalOpen}
        onCancel={() => setLogModalOpen(false)}
        footer={null}
        width={720}
      >
        <pre
          style={{
            maxHeight: 480,
            overflow: "auto",
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
            fontSize: 11,
            lineHeight: 1.5,
            margin: 0,
          }}
        >
          {logLoading ? t("common.loading") : logContent}
        </pre>
      </Modal>

      {/* ── 安装版本信息 Modal ── */}
      <Modal
        title={t("settings.localModel.installTitle")}
        open={installModalOpen}
        onCancel={() => setInstallModalOpen(false)}
        onOk={() => void handleInstall()}
        okText={t("settings.localModel.install")}
        cancelText={t("common.cancel")}
        confirmLoading={installing}
        width={480}
      >
        {latestVersion
          ? (
            <Descriptions column={1} size="small" style={{ marginTop: 8 }}>
              <Descriptions.Item label={t("settings.localModel.versionTag")}>
                <Tag color="blue">{latestVersion.tag}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label={t("settings.localModel.versionName")}>
                {latestVersion.name}
              </Descriptions.Item>
              <Descriptions.Item label={t("settings.localModel.publishedAt")}>
                {latestVersion.publishedAt}
              </Descriptions.Item>
              <Descriptions.Item label={t("settings.localModel.fileSize")}>
                {latestVersion.fileSize
                  ? formatBytes(latestVersion.fileSize)
                  : "-"}
              </Descriptions.Item>
              <Descriptions.Item label={t("settings.localModel.downloadUrl")}>
                <Text
                  copyable
                  code
                  style={{ fontSize: 11, wordBreak: "break-all" }}
                >
                  {latestVersion.downloadUrl}
                </Text>
              </Descriptions.Item>
            </Descriptions>
          )
          : (
            <div style={{ textAlign: "center", padding: "24px 0" }}>
              <Text type="secondary">{t("settings.localModel.fetchingVersion")}</Text>
            </div>
          )}
      </Modal>
    </Card>
  );
}
