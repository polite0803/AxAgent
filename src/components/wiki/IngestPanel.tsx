// SPDX-License-Identifier: AGPL-3.0-only

import { translateBackendError } from "@/lib/errorI18n";
import { invoke, listen, type UnlistenFn } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { useWikiStore } from "@/stores/feature/wikiStore";
import type { FolderImportPreviewItem, FolderImportResult, IngestResult } from "@/types";
import { DeleteOutlined, FileTextOutlined, FolderOutlined, LinkOutlined, UploadOutlined } from "@ant-design/icons";
import { Button, Card, Form, Input, Progress, Select, Space, Table, Tag, Typography, Upload } from "antd";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;
const { Dragger } = Upload;

interface IngestPanelProps {
  wikiId: string;
  onClose?: () => void;
}

export function IngestPanel({ wikiId, onClose }: IngestPanelProps) {
  const { t } = useTranslation();
  const { ingestSource, importFolderPreview, importFolder } = useLlmWikiStore();
  const [form] = Form.useForm();
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [results, setResults] = useState<IngestResult[]>([]);
  const [ingestType, setIngestType] = useState<"file" | "url" | "folder">("file");
  const [previewItems, setPreviewItems] = useState<FolderImportPreviewItem[]>([]);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importResult, setImportResult] = useState<FolderImportResult | null>(null);

  // F6: 摄取进度阶段 — "ingest" 等待后端生成笔记（无细粒度事件），
  // "indexing" 由 wiki-note-indexed 事件按 generatedNoteCount 逐条推进。
  const [stage, setStage] = useState<"idle" | "ingest" | "indexing">("idle");
  const [indexProgress, setIndexProgress] = useState({ indexed: 0, total: 0 });
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    return () => {
      unlistenRef.current?.();
      unlistenRef.current = null;
    };
  }, []);

  const handleIngest = async (values: {
    sourceType: string;
    url?: string;
    path?: string;
    title?: string;
  }) => {
    setUploading(true);
    setProgress(0);
    setStage("ingest");
    setIndexProgress({ indexed: 0, total: 0 });

    // 事件驱动真实进度：订阅须早于 invoke —— 后台批量索引在命令返回前就可能开始发事件
    const indexedIds = new Set<string>();
    let totalCount = 0;
    let finished = false;
    let fallbackTimer: ReturnType<typeof setTimeout> | null = null;

    const cleanup = () => {
      unlistenRef.current?.();
      unlistenRef.current = null;
      if (fallbackTimer) {
        clearTimeout(fallbackTimer);
        fallbackTimer = null;
      }
    };
    const finish = () => {
      if (finished) { return; }
      finished = true;
      cleanup();
      setProgress(100);
      setStage("idle");
      setUploading(false);
      onClose?.();
    };

    unlistenRef.current = await listen<{ noteId: string }>("wiki-note-indexed", (ev) => {
      if (finished || totalCount === 0) { return; }
      const noteId = ev.payload?.noteId;
      if (!noteId || indexedIds.has(noteId)) { return; }
      indexedIds.add(noteId);
      setIndexProgress({ indexed: indexedIds.size, total: totalCount });
      setProgress(Math.min(99, Math.round((indexedIds.size / totalCount) * 100)));
      // 每条事件重置兜底计时器：仅当事件流真正停摆 30s 才强制收尾
      if (fallbackTimer) {
        clearTimeout(fallbackTimer);
      }
      fallbackTimer = setTimeout(finish, 30_000);
      if (indexedIds.size >= totalCount) {
        finish();
      }
    });

    try {
      const result = await ingestSource(
        wikiId,
        values.sourceType,
        values.path || "",
        values.url,
        values.title,
      );

      if (!result) {
        // store 已记录错误详情
        finished = true;
        cleanup();
        setStage("idle");
        setUploading(false);
        return;
      }

      setResults((prev) => [...prev, result]);
      message.success(t("wiki.llm.ingestSuccess", { title: result.title }));
      form.resetFields();

      // F7：ingest 生成的笔记已入库，刷新 wikiStore 当前视图的笔记列表
      void useWikiStore.getState().refreshNotes(wikiId);

      totalCount = result.generatedNoteCount;
      setIndexProgress({ indexed: indexedIds.size, total: totalCount });

      if (totalCount === 0 || indexedIds.size >= totalCount) {
        // 无后台笔记或索引已随命令同步完成
        finish();
        return;
      }

      setStage("indexing");
      setProgress(Math.min(99, Math.round((indexedIds.size / totalCount) * 100)));
      // 兜底：事件丢失（如浏览器 mock）时 30s 后强制收尾，索引在后台继续
      fallbackTimer = setTimeout(finish, 30_000);
    } catch (e) {
      finished = true;
      cleanup();
      setStage("idle");
      setUploading(false);
      message.error(t("wiki.llm.ingestError", { error: translateBackendError(e) }));
    }
  };

  const handleFileUpload = async (file: File) => {
    try {
      const arrayBuffer = await file.arrayBuffer();
      const base64 = btoa(
        Array.from(new Uint8Array(arrayBuffer))
          .map((b) => String.fromCharCode(b))
          .join(""),
      );

      const ext = file.name.split(".").pop()?.toLowerCase() || "";
      const sourceType = ext === "pdf"
        ? "pdf"
        : ext === "docx"
        ? "docx"
        : ext === "xlsx"
        ? "xlsx"
        : ext === "pptx"
        ? "pptx"
        : ext === "html"
        ? "web"
        : "markdown";

      await invoke<string>("write_base64_to_file", {
        input: {
          wikiId,
          fileName: file.name,
          base64Content: base64,
          sourceType,
        },
      });

      form.setFieldsValue({ path: file.name });
      message.success(t("wiki.llm.fileUploaded", { name: file.name }));
    } catch (e) {
      message.error(t("wiki.llm.uploadError", { error: translateBackendError(e) }));
    }
    return false;
  };

  const handlePreviewFolder = async () => {
    const path = form.getFieldValue("path");
    if (!path) {
      message.warning(t("wiki.llm.folderPathRequired"));
      return;
    }

    setPreviewLoading(true);
    setPreviewItems([]);
    setImportResult(null);

    try {
      const items = await importFolderPreview(path);
      setPreviewItems(items);
      if (items.length === 0) {
        message.info(t("wiki.llm.folderEmpty"));
      } else {
        message.success(t("wiki.llm.previewFound", { count: items.length }));
      }
    } catch (e) {
      message.error(t("wiki.llm.previewError", { error: translateBackendError(e) }));
    } finally {
      setPreviewLoading(false);
    }
  };

  const handleImportFolder = async () => {
    if (previewItems.length === 0) { return; }

    setImporting(true);
    setImportResult(null);

    try {
      const path = form.getFieldValue("path");
      const result = await importFolder(wikiId, path);

      if (result) {
        setImportResult(result);
        if (result.importedCount > 0) {
          message.success(
            t("wiki.llm.folderImportSuccess", { count: result.importedCount }),
          );
          onClose?.();
        } else {
          message.warning(t("wiki.llm.folderImportFailed"));
        }
      }
    } catch (e) {
      message.error(t("wiki.llm.importError", { error: translateBackendError(e) }));
    } finally {
      setImporting(false);
    }
  };

  const removeResult = (index: number) => {
    setResults((prev) => prev.filter((_, i) => i !== index));
  };

  const previewColumns = [
    {
      title: t("wiki.llm.fileName"),
      dataIndex: "fileName",
      key: "fileName",
      ellipsis: true,
    },
    {
      title: t("wiki.llm.folderContext"),
      dataIndex: "folderContext",
      key: "folderContext",
      ellipsis: true,
      render: (v: string) => v || "-",
    },
    {
      title: t("wiki.llm.fileType"),
      dataIndex: "fileType",
      key: "fileType",
      width: 100,
      render: (v: string) => <Tag>{v}</Tag>,
    },
    {
      title: t("wiki.llm.fileSize"),
      dataIndex: "estimatedSize",
      key: "estimatedSize",
      width: 100,
      render: (v: number) => {
        if (v < 1024) { return `${v} B`; }
        if (v < 1024 * 1024) { return `${(v / 1024).toFixed(1)} KB`; }
        return `${(v / 1024 / 1024).toFixed(1)} MB`;
      },
    },
  ];

  return (
    <Space orientation="vertical" size="large" style={{ width: "100%" }}>
      <Form form={form} layout="vertical" onFinish={handleIngest}>
        <Form.Item label={t("wiki.ingest.type")} required>
          <Select
            value={ingestType}
            onChange={(v) => {
              setIngestType(v);
              setPreviewItems([]);
              setImportResult(null);
              form.resetFields(["path", "url", "title"]);
            }}
            options={[
              { label: t("wiki.ingest.file"), value: "file" },
              { label: t("wiki.ingest.url"), value: "url" },
              { label: t("wiki.ingest.folder"), value: "folder" },
            ]}
          />
        </Form.Item>

        {ingestType !== "folder" && (
          <Form.Item
            name="sourceType"
            label={t("wiki.ingest.sourceType")}
            rules={[
              { required: true, message: t("wiki.ingest.sourceTypeRequired") },
            ]}
          >
            <Select
              options={[
                { label: t("wiki.ingest.markdown"), value: "markdown" },
                { label: t("wiki.ingest.pdf"), value: "pdf" },
                { label: t("wiki.ingest.docx"), value: "docx" },
                { label: t("wiki.ingest.web"), value: "web" },
                { label: t("wiki.ingest.notion"), value: "notion" },
              ]}
            />
          </Form.Item>
        )}

        {ingestType === "file" && (
          <>
            <Form.Item label={t("wiki.ingest.uploadFile")}>
              <Dragger
                accept=".md,.pdf,.docx"
                beforeUpload={handleFileUpload}
                showUploadList={false}
                maxCount={1}
              >
                <p className="ant-upload-drag-icon">
                  <FileTextOutlined />
                </p>
                <p className="ant-upload-text">{t("wiki.ingest.uploadHint")}</p>
              </Dragger>
            </Form.Item>
            <Form.Item
              name="path"
              label={t("wiki.ingest.path")}
              rules={[{ required: true }]}
            >
              <Input
                name="path"
                prefix={<FolderOutlined />}
                placeholder={t("wiki.ingest.pathPlaceholder")}
              />
            </Form.Item>
          </>
        )}

        {ingestType === "url" && (
          <Form.Item
            name="url"
            label={t("wiki.ingest.url")}
            rules={[
              { required: true, message: t("wiki.ingest.urlRequired") },
              { type: "url", message: t("wiki.ingest.urlInvalid") },
            ]}
          >
            <Input
              name="url"
              prefix={<LinkOutlined />}
            />
          </Form.Item>
        )}

        {ingestType === "folder" && (
          <>
            <Form.Item
              name="path"
              label={t("wiki.ingest.folderPath")}
              rules={[
                { required: true, message: t("wiki.ingest.folderPathRequired") },
              ]}
            >
              <Input
                name="path"
                prefix={<FolderOutlined />}
                placeholder={t("wiki.ingest.folderPathPlaceholder")}
              />
            </Form.Item>

            <Space>
              <Button
                onClick={handlePreviewFolder}
                loading={previewLoading}
                icon={<FolderOutlined />}
              >
                {t("wiki.ingest.previewFolder")}
              </Button>
            </Space>

            {previewItems.length > 0 && (
              <Card
                size="small"
                title={t("wiki.ingest.previewResult", { count: previewItems.length })}
                extra={importResult
                  ? (
                    <Tag color={importResult.failedFiles.length > 0 ? "orange" : "green"}>
                      {t("wiki.llm.importedCount", {
                        count: importResult.importedCount,
                      })}
                      {importResult.failedFiles.length > 0
                        && ` (${
                          t("wiki.llm.failedCount", {
                            count: importResult.failedFiles.length,
                          })
                        })`}
                    </Tag>
                  )
                  : null}
                style={{ marginTop: 12 }}
              >
                <Table
                  dataSource={previewItems}
                  columns={previewColumns}
                  rowKey="filePath"
                  size="small"
                  pagination={{ pageSize: 10, size: "small" }}
                  scroll={{ y: 300 }}
                />

                {importResult?.failedFiles.length && (
                  <div style={{ marginTop: 12 }}>
                    <Text type="danger" style={{ fontSize: 12 }}>
                      {t("wiki.ingest.failedFiles")}:
                    </Text>
                    <ul style={{ paddingLeft: 16, margin: "4px 0 0" }}>
                      {importResult.failedFiles.map((f, i) => (
                        <li key={i}>
                          <Text type="danger" style={{ fontSize: 12 }}>{f}</Text>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {!importResult && (
                  <Button
                    type="primary"
                    block
                    loading={importing}
                    icon={<UploadOutlined />}
                    onClick={handleImportFolder}
                    style={{ marginTop: 12 }}
                  >
                    {t("wiki.ingest.confirmImport")}
                  </Button>
                )}
              </Card>
            )}
          </>
        )}

        {ingestType !== "folder" && (
          <Form.Item name="title" label={t("wiki.ingest.title")}>
            <Input name="title" placeholder={t("wiki.ingest.titlePlaceholder")} />
          </Form.Item>
        )}

        {uploading && (
          <Progress
            percent={progress}
            status="active"
            format={() =>
              stage === "indexing"
                ? t("wiki.llm.indexingProgress", {
                  indexed: indexProgress.indexed,
                  total: indexProgress.total,
                })
                : t("wiki.llm.stageIngest")}
          />
        )}

        {ingestType !== "folder" && (
          <Button
            type="primary"
            htmlType="submit"
            loading={uploading}
            block
            icon={<UploadOutlined />}
          >
            {t("wiki.ingest.start")}
          </Button>
        )}
      </Form>

      {results.length > 0 && (
        <Card title={t("wiki.ingest.results")}>
          <div className="divide-y divide-gray-100">
            {results.map((item, index) => (
              <div key={index} style={{ padding: "12px 0" }}>
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      alignItems: "flex-start",
                      gap: 12,
                    }}
                  >
                    <div>
                      <FileTextOutlined />
                    </div>
                    <div>
                      <div style={{ fontWeight: 500 }}>{item.title}</div>
                      <div
                        style={{
                          color: "var(--text-secondary, rgba(0,0,0,0.45))",
                          fontSize: 13,
                          marginTop: 2,
                        }}
                      >
                        <Text type="secondary" style={{ fontSize: 12 }}>
                          {item.rawPath}
                        </Text>
                      </div>
                    </div>
                  </div>
                  <Space>
                    <Button
                      key="remove"
                      type="text"
                      danger
                      size="small"
                      icon={<DeleteOutlined />}
                      onClick={() => removeResult(index)}
                    />
                  </Space>
                </div>
              </div>
            ))}
          </div>
        </Card>
      )}
    </Space>
  );
}
