import { invoke } from "@/lib/invoke";
import { useConversationStore } from "@/stores";
import { Button, Card, Image, Select, Spin, Typography, Upload } from "antd";
import { Camera, CheckCircle, FileImage, ImageIcon, UploadCloud, XCircle } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

const { Text, Title } = Typography;

interface UiElement {
  element_type: string;
  label: string | null;
  bounding_box: { x: number; y: number; width: number; height: number } | null;
  actionable: boolean;
}

interface VisionResult {
  task: string;
  description: string;
  elements: UiElement[];
  text_content: string | null;
  confidence: number;
  model: string;
}

const VISION_TASKS = [
  { value: "ImageDescription", labelKey: "chat.vision.taskLabels.ImageDescription" },
  { value: "Ocr", labelKey: "chat.vision.taskLabels.Ocr" },
  { value: "UiElementDetection", labelKey: "chat.vision.taskLabels.UiElementDetection" },
  { value: "ChartAnalysis", labelKey: "chat.vision.taskLabels.ChartAnalysis" },
  { value: "CodeScreenshotReading", labelKey: "chat.vision.taskLabels.CodeScreenshotReading" },
];

export function ImageAnalysisPanel() {
  const { t } = useTranslation();
  const [imageBase64, setImageBase64] = useState<string | null>(null);
  const [imagePreview, setImagePreview] = useState<string | null>(null);
  const [selectedTask, setSelectedTask] = useState("ImageDescription");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<VisionResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const activeConversationId = useConversationStore((s) => s.activeConversationId);
  const conversations = useConversationStore((s) => s.conversations);
  const activeConv = conversations.find((c) => c.id === activeConversationId);
  const providerId = activeConv?.provider_id ?? "";
  const modelId = activeConv?.model_id ?? "";

  const handleFileRead = (file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      const dataUrl = reader.result as string;
      setImagePreview(dataUrl);
      const base64 = dataUrl.split(",")[1] || "";
      setImageBase64(base64);
      setResult(null);
      setError(null);
    };
    reader.readAsDataURL(file);
    return false;
  };

  const handleAnalyze = async () => {
    if (!imageBase64) {
      return;
    }

    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await invoke<VisionResult>("analyze_image", {
        imageBase64,
        task: selectedTask,
        providerId,
        modelId,
      });
      setResult(res);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <Camera size={16} style={{ color: "#722ed1" }} />
        <Title level={5} style={{ margin: 0 }}>{t("chat.vision.imageAnalysis")}</Title>
      </div>

      <Upload.Dragger
        accept="image/*"
        showUploadList={false}
        beforeUpload={handleFileRead}
        style={{ padding: 12 }}
      >
        {imagePreview
          ? (
            <Image
              src={imagePreview}
              alt={t("chat.vision.analyzedImage")}
              style={{ maxHeight: 160, objectFit: "contain" }}
              preview={false}
            />
          )
          : (
            <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 4 }}>
              <UploadCloud size={24} style={{ color: "#999" }} />
              <Text type="secondary">{t("chat.vision.uploadImage")}</Text>
            </div>
          )}
      </Upload.Dragger>

      <Select
        value={selectedTask}
        onChange={setSelectedTask}
        options={VISION_TASKS.map((task) => ({
          value: task.value,
          label: t(task.labelKey),
        }))}
      />

      <Button
        type="primary"
        onClick={handleAnalyze}
        loading={loading}
        block
        disabled={!imageBase64 || !providerId || !modelId}
      >
        {t("chat.vision.analyze")}
      </Button>

      {loading && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            gap: 8,
            padding: 16,
          }}
        >
          <Spin size="small" />
          <Text type="secondary">{t("chat.vision.analyzing")}</Text>
        </div>
      )}

      {error && (
        <div style={{ display: "flex", alignItems: "flex-start", gap: 8, color: "#ff4d4f" }}>
          <XCircle size={14} style={{ marginTop: 2 }} />
          <Text type="danger" style={{ fontSize: 13 }}>{error}</Text>
        </div>
      )}

      {result && !loading && (
        <Card size="small">
          <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
            <Text strong style={{ fontSize: 13 }}>
              {t(`chat.vision.taskLabels.${result.task}`, result.task)}
            </Text>

            <div>
              <Text strong style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
                {t("chat.vision.description")}
              </Text>
              <Text style={{ fontSize: 13 }}>{result.description}</Text>
            </div>

            {result.text_content && (
              <div>
                <Text strong style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
                  {t("chat.vision.extractedText")}
                </Text>
                <pre
                  style={{
                    margin: 0,
                    padding: 8,
                    background: "var(--bg-secondary, #f5f5f5)",
                    borderRadius: 4,
                    fontSize: 12,
                    maxHeight: 160,
                    overflow: "auto",
                    whiteSpace: "pre-wrap",
                  }}
                >
                  {result.text_content}
                </pre>
              </div>
            )}

            {result.elements.length > 0 && (
              <div>
                <Text strong style={{ fontSize: 12, display: "block", marginBottom: 4 }}>
                  {t("chat.vision.elements")} ({result.elements.length})
                </Text>
                <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                  {result.elements.map((el, i) => (
                    <div
                      key={`${el.element_type}-${i}`}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 4,
                        padding: "2px 4px",
                        borderRadius: 4,
                        fontSize: 12,
                      }}
                    >
                      {el.actionable
                        ? <CheckCircle size={10} style={{ color: "#52c41a", flexShrink: 0 }} />
                        : <ImageIcon size={10} style={{ color: "#999", flexShrink: 0 }} />}
                      <Text strong style={{ fontSize: 12 }}>{el.element_type}</Text>
                      {el.label && <Text type="secondary" style={{ fontSize: 12 }}>{el.label}</Text>}
                    </div>
                  ))}
                </div>
              </div>
            )}

            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <FileImage size={12} style={{ color: "#999" }} />
              <Text type="secondary" style={{ fontSize: 12 }}>
                {t("chat.vision.model")}: {result.model}
                {" · "}
                {t("chat.vision.confidence")}: {(result.confidence * 100).toFixed(0)}%
              </Text>
            </div>
          </div>
        </Card>
      )}
    </div>
  );
}
