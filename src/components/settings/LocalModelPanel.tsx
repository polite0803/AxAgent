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
import type { EmbedTestResult, LocalFileModel, LocalModelStartConfig, LocalModelStatus } from "@/types";
import { PlayCircleOutlined, PoweroffOutlined, ReloadOutlined, RobotOutlined, StopOutlined } from "@ant-design/icons";
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
/** 默认启动配置 */
const DEFAULT_CONFIG: LocalModelStartConfig = {
  serverExe: "llama-server",
  modelPath: "E:\\llama-models\\bge-m3.Q5_K_M.gguf",
  host: "127.0.0.1",
  port: 8091,
  alias: "bge-m3",
  nCtx: null,
  nGpuLayers: null,
  embeddingMode: true,
  extraArgs: [],
};

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

  // 加载下载目录与本地模型文件（启动表单选路径用）
  useEffect(() => {
    void (async () => {
      try {
        const [dir, files] = await Promise.all([
          invoke<string>("local_model_get_download_dir"),
          invoke<LocalFileModel[]>("local_model_list_local_models"),
        ]);
        setDownloadDir(dir);
        setLocalModelFiles(files.filter((f) => !f.isDownloading));
      } catch (e) {
        logIpcError("local_model_list_local_models")(e);
      }
    })();
  }, [providerId]);

  const running = status?.running ?? false;
  const health = status?.health ?? "unknown";

  const handleStart = useCallback(async () => {
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
      showBackendError(message, e, { context: "local_model_start" });
    }
  }, [providerId, startConfig, message, t]);

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
            <Input
              value={startConfig.serverExe}
              onChange={(e) => setStartConfig({ ...startConfig, serverExe: e.target.value })}
              placeholder={t("settings.localModel.serverExePlaceholder")}
            />
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
              placeholder={"--threads 8\n--no-mmap"}
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
    </Card>
  );
}
