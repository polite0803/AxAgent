import { usePromptTemplateStore } from "@/stores/feature/promptTemplateStore";
import type { ImportPromptResult, ImportPromptTemplateInput } from "@/types";
import { DownloadOutlined, FolderOpenOutlined, GithubOutlined, InboxOutlined, LinkOutlined } from "@ant-design/icons";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Alert, Button, Form, Input, message, Modal, Progress, Space, Tabs, Tag, Typography, Upload } from "antd";
import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Paragraph } = Typography;

interface PromptImportModalProps {
  open: boolean;
  onClose: () => void;
}

export function PromptImportModal({ open, onClose }: PromptImportModalProps) {
  const { t } = useTranslation();
  const [messageApi, contextHolder] = message.useMessage();
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<ImportPromptResult | null>(null);
  const [urlForm] = Form.useForm();
  const [activeTab, setActiveTab] = useState<string>("url");

  const { importFromUrl, importTemplates, importFromFolder } = usePromptTemplateStore();

  const handleUrlImport = useCallback(async () => {
    try {
      const values = await urlForm.validateFields();
      setImporting(true);
      setResult(null);
      const res = await importFromUrl({
        url: values.url,
        categoryFilter: values.categoryFilter || undefined,
        overwriteExisting: values.overwriteExisting || false,
      });
      if (res) {
        setResult(res);
        if (res.imported.length > 0) {
          messageApi.success(t("promptTemplates.importSuccess", { count: res.imported.length }));
        } else {
          messageApi.warning(t("promptTemplates.importEmpty"));
        }
      }
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setImporting(false);
    }
  }, [urlForm, importFromUrl, messageApi, t]);

  const handleFileImport = useCallback(
    async (file: File) => {
      try {
        setImporting(true);
        setResult(null);
        const text = await file.text();

        let inputs: ImportPromptTemplateInput[] = [];

        if (file.name.endsWith(".json")) {
          const parsed = JSON.parse(text);
          inputs = Array.isArray(parsed)
            ? parsed.map((p: Record<string, unknown>) => ({
              name: p.name as string || "未命名",
              description: p.description as string | undefined,
              content: p.content as string || "",
              variablesSchema: p.variablesSchema as string | undefined,
              category: p.category as string | undefined,
              tags: p.tags as string[] | undefined,
              author: p.author as string | undefined,
              source: p.source as string | undefined,
              sourceType: "file_import",
              format: "json",
            }))
            : [];
        } else if (file.name.endsWith(".yaml") || file.name.endsWith(".yml")) {
          // 对于 YAML 文件，暂时提示使用 URL 导入方式
          messageApi.info(t("promptTemplates.useUrlImportForYaml"));
          setImporting(false);
          return false;
        } else if (file.name.endsWith(".md")) {
          // 单个 Markdown 文件
          inputs = [
            {
              name: file.name.replace(/\.md$/, ""),
              content: text,
              sourceType: "file_import",
              format: "markdown",
            },
          ];
        } else {
          messageApi.warning(t("promptTemplates.unsupportedFileFormat"));
          setImporting(false);
          return false;
        }

        const res = await importTemplates(inputs);
        if (res) {
          setResult(res);
          if (res.imported.length > 0) {
            messageApi.success(
              t("promptTemplates.importSuccess", { count: res.imported.length }),
            );
          }
        }
        setImporting(false);
        return false;
      } catch (e) {
        messageApi.error(String(e));
        setImporting(false);
        return false;
      }
    },
    [importTemplates, messageApi, t],
  );

  const handleFolderImport = useCallback(async () => {
    try {
      const selected = await openDialog({ directory: true, multiple: false, title: t("promptTemplates.selectFolder") });
      if (!selected) { return; // 用户取消
       }

      const folderPath = selected as string;
      setImporting(true);
      setResult(null);
      const res = await importFromFolder(folderPath);
      if (res) {
        setResult(res);
        if (res.imported.length > 0) {
          messageApi.success(t("promptTemplates.importSuccess", { count: res.imported.length }));
        } else {
          messageApi.warning(t("promptTemplates.importEmpty"));
        }
      }
    } catch (e) {
      messageApi.error(String(e));
    } finally {
      setImporting(false);
    }
  }, [importFromFolder, messageApi, t]);

  const totalImported = result?.imported.length ?? 0;
  const totalSkipped = result?.skipped.length ?? 0;
  const totalErrors = result?.errors.length ?? 0;

  return (
    <Modal
      title={t("promptTemplates.importTitle")}
      open={open}
      onCancel={onClose}
      footer={null}
      width={640}
      destroyOnClose
    >
      {contextHolder}
      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: "url",
            label: (
              <span>
                <LinkOutlined /> {t("promptTemplates.importFromUrl")}
              </span>
            ),
            children: (
              <div className="py-4">
                <Form form={urlForm} layout="vertical">
                  <Form.Item
                    name="url"
                    label={t("promptTemplates.repoUrl")}
                    rules={[{ required: true, message: t("promptTemplates.urlRequired") }]}
                    extra={
                      <Text type="secondary">
                        {t("promptTemplates.urlHint")}
                      </Text>
                    }
                  >
                    <Input
                      prefix={<GithubOutlined />}
                      placeholder="https://github.com/yaojingang/yao-open-prompts"
                    />
                  </Form.Item>
                  <Form.Item name="categoryFilter" label={t("promptTemplates.categoryFilter")}>
                    <Input
                      placeholder={t("promptTemplates.categoryFilterPlaceholder")}
                      allowClear
                    />
                  </Form.Item>
                  <Button
                    type="primary"
                    icon={<DownloadOutlined />}
                    loading={importing}
                    onClick={handleUrlImport}
                  >
                    {t("promptTemplates.startImport")}
                  </Button>
                </Form>
              </div>
            ),
          },
          {
            key: "file",
            label: (
              <span>
                <InboxOutlined /> {t("promptTemplates.importFromFile")}
              </span>
            ),
            children: (
              <div className="py-4">
                <Upload.Dragger
                  accept=".json,.md,.yaml,.yml"
                  maxCount={1}
                  beforeUpload={handleFileImport}
                  showUploadList={false}
                >
                  <p className="text-4xl text-gray-300 mb-2">
                    <InboxOutlined />
                  </p>
                  <p className="text-sm text-gray-500">
                    {t("promptTemplates.dropFileHint")}
                  </p>
                  <p className="text-xs text-gray-400">
                    {t("promptTemplates.supportedFormats")}: JSON, Markdown, YAML
                  </p>
                </Upload.Dragger>
              </div>
            ),
          },
          {
            key: "folder",
            label: (
              <span>
                <FolderOpenOutlined /> {t("promptTemplates.importFromFolder")}
              </span>
            ),
            children: (
              <div className="py-8 text-center">
                <p className="text-5xl text-gray-300 mb-4">
                  <FolderOpenOutlined />
                </p>
                <p className="text-sm text-gray-500 mb-4">
                  {t("promptTemplates.folderHint")}
                </p>
                <Button
                  type="primary"
                  size="large"
                  icon={<FolderOpenOutlined />}
                  loading={importing}
                  onClick={handleFolderImport}
                >
                  {t("promptTemplates.selectFolder")}
                </Button>
                <p className="text-xs text-gray-400 mt-3">
                  {t("promptTemplates.folderSupportedFormats")}: Markdown (.md)
                </p>
              </div>
            ),
          },
        ]}
      />

      {importing && (
        <div className="py-4">
          <Progress percent={99} status="active" />
        </div>
      )}

      {result && (
        <div className="py-2">
          <Space direction="vertical" className="w-full">
            <Alert
              type={totalErrors > 0 ? "warning" : "success"}
              message={
                <span>
                  {t("promptTemplates.imported")}: <Tag color="green">{totalImported}</Tag>{" "}
                  {t("promptTemplates.skipped")}: <Tag color="orange">{totalSkipped}</Tag>{" "}
                  {t("promptTemplates.errors")}: <Tag color="red">{totalErrors}</Tag>
                </span>
              }
            />
            {result.imported.length > 0 && (
              <div>
                <Text strong>{t("promptTemplates.importedList")}:</Text>
                <div className="flex flex-wrap gap-1 mt-1">
                  {result.imported.map((tpl) => (
                    <Tag key={tpl.id} color="green">
                      {tpl.name}
                    </Tag>
                  ))}
                </div>
              </div>
            )}
            {result.skipped.length > 0 && (
              <div>
                <Text type="secondary">{t("promptTemplates.skippedList")}:</Text>
                <Paragraph
                  type="secondary"
                  ellipsis={{ rows: 2, expandable: true }}
                  className="text-xs mt-1"
                >
                  {result.skipped.join(", ")}
                </Paragraph>
              </div>
            )}
            {result.errors.length > 0 && (
              <div>
                <Text type="danger">{t("promptTemplates.errorList")}:</Text>
                <Paragraph
                  type="danger"
                  ellipsis={{ rows: 2, expandable: true }}
                  className="text-xs mt-1"
                >
                  {result.errors.join("; ")}
                </Paragraph>
              </div>
            )}
          </Space>
        </div>
      )}
    </Modal>
  );
}
