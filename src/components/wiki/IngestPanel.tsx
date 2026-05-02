import { invoke } from "@/lib/invoke";
import { IngestResult, useLlmWikiStore } from "@/stores/feature/llmWikiStore";
import { DeleteOutlined, FileTextOutlined, FolderOutlined, LinkOutlined, UploadOutlined } from "@ant-design/icons";
import { Button, Card, Form, Input, List, message, Progress, Select, Space, Typography, Upload } from "antd";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text } = Typography;
const { Dragger } = Upload;

interface IngestPanelProps {
  wikiId: string;
  onClose?: () => void;
}

export function IngestPanel({ wikiId, onClose }: IngestPanelProps) {
  const { t } = useTranslation();
  const { ingestSource } = useLlmWikiStore();
  const [form] = Form.useForm();
  const [uploading, setUploading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [results, setResults] = useState<IngestResult[]>([]);
  const [ingestType, setIngestType] = useState<"file" | "url" | "folder">("file");

  const handleIngest = async (values: { sourceType: string; url?: string; path?: string; title?: string }) => {
    setUploading(true);
    setProgress(0);

    try {
      const interval = setInterval(() => {
        setProgress((p) => Math.min(p + 10, 90));
      }, 200);

      const result = await ingestSource(
        wikiId,
        values.sourceType,
        values.path || "",
        values.url,
        values.title,
      );

      clearInterval(interval);
      setProgress(100);

      if (result) {
        setResults((prev) => [...prev, result]);
        message.success(t("wiki.llm.ingestSuccess", { title: result.title }));
        form.resetFields();
        onClose?.();
      }
    } catch (e) {
      message.error(t("wiki.llm.ingestError", { error: String(e) }));
    } finally {
      setUploading(false);
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
        wikiId,
        fileName: file.name,
        base64Content: base64,
        sourceType,
      });

      form.setFieldsValue({ path: file.name });
      message.success(t("wiki.llm.fileUploaded", { name: file.name }));
    } catch (e) {
      message.error(t("wiki.llm.uploadError", { error: String(e) }));
    }
    return false;
  };

  const removeResult = (index: number) => {
    setResults((prev) => prev.filter((_, i) => i !== index));
  };

  return (
    <Space direction="vertical" size="large" style={{ width: "100%" }}>
      <Form form={form} layout="vertical" onFinish={handleIngest}>
        <Form.Item label={t("wiki.ingest.type")} required>
          <Select
            value={ingestType}
            onChange={(v) => {
              setIngestType(v);
              form.resetFields(["path", "url", "title"]);
            }}
            options={[
              { label: t("wiki.ingest.file"), value: "file" },
              { label: t("wiki.ingest.url"), value: "url" },
              { label: t("wiki.ingest.folder"), value: "folder" },
            ]}
          />
        </Form.Item>

        <Form.Item
          name="sourceType"
          label={t("wiki.ingest.sourceType")}
          rules={[{ required: true, message: t("wiki.ingest.sourceTypeRequired") }]}
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
            <Form.Item name="path" label={t("wiki.ingest.path")} rules={[{ required: true }]}>
              <Input prefix={<FolderOutlined />} placeholder={t("wiki.ingest.pathPlaceholder")} />
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
            <Input prefix={<LinkOutlined />} placeholder="https://..." />
          </Form.Item>
        )}

        {ingestType === "folder" && (
          <Form.Item
            name="path"
            label={t("wiki.ingest.folderPath")}
            rules={[{ required: true, message: t("wiki.ingest.folderPathRequired") }]}
          >
            <Input prefix={<FolderOutlined />} placeholder={t("wiki.ingest.folderPathPlaceholder")} />
          </Form.Item>
        )}

        <Form.Item name="title" label={t("wiki.ingest.title")}>
          <Input placeholder={t("wiki.ingest.titlePlaceholder")} />
        </Form.Item>

        {uploading && <Progress percent={progress} status="active" />}

        <Button type="primary" htmlType="submit" loading={uploading} block icon={<UploadOutlined />}>
          {t("wiki.ingest.start")}
        </Button>
      </Form>

      {results.length > 0 && (
        <Card title={t("wiki.ingest.results")}>
          <List
            dataSource={results}
            renderItem={(item, index) => (
              <List.Item
                actions={[
                  <Button
                    key="remove"
                    type="text"
                    danger
                    size="small"
                    icon={<DeleteOutlined />}
                    onClick={() => removeResult(index)}
                  />,
                ]}
              >
                <List.Item.Meta
                  avatar={<FileTextOutlined />}
                  title={item.title}
                  description={
                    <Text type="secondary" style={{ fontSize: 12 }}>
                      {item.raw_path}
                    </Text>
                  }
                />
              </List.Item>
            )}
          />
        </Card>
      )}
    </Space>
  );
}
