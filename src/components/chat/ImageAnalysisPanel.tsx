import { Badge, Card, Typography } from "antd";
import { Camera, CheckCircle, FileImage, ImageIcon, Loader2, XCircle } from "lucide-react";

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

interface ImageAnalysisPanelProps {
  imageUrl?: string;
  result: VisionResult | null;
  loading?: boolean;
  error?: string | null;
}

const TASK_LABELS: Record<string, string> = {
  ImageDescription: "Image Description",
  Ocr: "OCR / Text Extraction",
  UiElementDetection: "UI Element Detection",
  ChartAnalysis: "Chart Analysis",
  CodeScreenshotReading: "Code Reading",
};

function ImageAnalysisPanel({ imageUrl, result, loading, error }: ImageAnalysisPanelProps) {
  const { t } = useTranslation();

  return (
    <Card size="small" className="image-analysis-panel">
      <div className="flex items-center gap-2 mb-3">
        <Camera size={16} className="text-purple-500" />
        <Title level={5} className="mb-0">{t("chat.vision.imageAnalysis")}</Title>
        {loading && <Loader2 size={14} className="animate-spin text-blue-500" />}
      </div>

      {imageUrl && (
        <div className="mb-3 rounded overflow-hidden border border-gray-200 dark:border-gray-700 max-h-48">
          <img
            src={imageUrl}
            alt={t("chat.vision.analyzedImage")}
            className="w-full h-full object-contain bg-gray-100 dark:bg-gray-800"
          />
        </div>
      )}

      {loading && (
        <div className="flex items-center gap-2 py-4 text-sm text-gray-500">
          <Loader2 size={14} className="animate-spin" />
          <span>{t("chat.vision.analyzing")}</span>
        </div>
      )}

      {error && (
        <div className="flex items-center gap-2 py-2 text-sm text-red-500">
          <XCircle size={14} />
          <span>{error}</span>
        </div>
      )}

      {result && !loading && (
        <div className="space-y-3">
          <Badge
            color="purple"
            text={<span className="text-xs">{TASK_LABELS[result.task] || result.task}</span>}
          />

          <div>
            <Text strong className="text-sm block mb-1">{t("chat.vision.description")}</Text>
            <Text className="text-sm">{result.description}</Text>
          </div>

          {result.text_content && (
            <div>
              <Text strong className="text-sm block mb-1">{t("chat.vision.extractedText")}</Text>
              <pre className="mt-1 p-2 bg-gray-100 dark:bg-gray-800 rounded text-xs max-h-48 overflow-auto whitespace-pre-wrap">
                {result.text_content}
              </pre>
            </div>
          )}

          {result.elements.length > 0 && (
            <div>
              <Text strong className="text-sm block mb-1">
                {t("chat.vision.elements")} ({result.elements.length})
              </Text>
              <div className="space-y-1">
                {result.elements.map((el, i) => (
                  <div
                    key={i}
                    className="flex items-center gap-2 px-2 py-1 rounded text-xs hover:bg-gray-50 dark:hover:bg-gray-800/50"
                  >
                    {el.actionable
                      ? <CheckCircle size={10} className="text-green-500 shrink-0" />
                      : <ImageIcon size={10} className="text-gray-400 shrink-0" />}
                    <span className="font-medium">{el.element_type}</span>
                    {el.label && <span className="text-gray-500">{el.label}</span>}
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="flex items-center gap-2 text-xs text-gray-400">
            <FileImage size={12} />
            <span>{t("chat.vision.model")}: {result.model}</span>
          </div>
        </div>
      )}
    </Card>
  );
}

export default ImageAnalysisPanel;
