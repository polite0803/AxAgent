import { invoke } from "@/lib/invoke";
import { Button, Image, Input, message, Select, Slider, Space, Typography } from "antd";
import { Sparkles } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface GeneratedImage {
  url?: string;
  base64?: string;
  width: number;
  height: number;
  seed?: number;
}

interface ImageGenResult {
  images: GeneratedImage[];
  model_used: string;
  elapsed_ms: number;
}

const SIZE_PRESETS = [
  { label: "1:1 (1024×1024)", width: 1024, height: 1024 },
  { label: "16:9 (1344×768)", width: 1344, height: 768 },
  { label: "9:16 (768×1344)", width: 768, height: 1344 },
  { label: "4:3 (1152×896)", width: 1152, height: 896 },
];

const PROVIDERS = [
  { value: "flux", label: "Flux (Replicate)" },
  { value: "dall-e", label: "DALL-E 3 (OpenAI)" },
];

interface ImageGenPanelProps {
  apiKey?: string;
  defaultProvider?: string;
  onImageGenerated?: (images: GeneratedImage[]) => void;
}

export function ImageGenPanel({
  apiKey,
  defaultProvider = "flux",
  onImageGenerated,
}: ImageGenPanelProps) {
  const { t } = useTranslation();
  const [prompt, setPrompt] = useState("");
  const [negativePrompt, setNegativePrompt] = useState("");
  const [provider, setProvider] = useState(defaultProvider);
  const [sizePreset, setSizePreset] = useState(0);
  const [steps, setSteps] = useState(4);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ImageGenResult | null>(null);

  const handleGenerate = async () => {
    if (!prompt.trim()) {
      message.warning(t("imageGen.enterPrompt"));
      return;
    }

    if (!apiKey) {
      message.error(t("imageGen.configureApiKey"));
      return;
    }

    setLoading(true);
    setResult(null);

    try {
      const res = await invoke<ImageGenResult>("generate_image", {
        prompt,
        negativePrompt: negativePrompt || undefined,
        width: SIZE_PRESETS[sizePreset].width,
        height: SIZE_PRESETS[sizePreset].height,
        steps: provider === "flux" ? steps : undefined,
        provider,
        apiKey,
      });

      setResult(res);
      onImageGenerated?.(res.images);
    } catch (e) {
      message.error(String(e));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      style={{ padding: 16, display: "flex", flexDirection: "column", gap: 12 }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          marginBottom: 4,
        }}
      >
        <Sparkles size={18} style={{ color: "var(--purple, #722ed1)" }} />
        <Typography.Text strong>{t("imageGen.title")}</Typography.Text>
      </div>

      <Space>
        <Select
          value={provider}
          onChange={setProvider}
          options={PROVIDERS}
          style={{ width: 200 }}
        />
        <Select
          value={sizePreset}
          onChange={setSizePreset}
          options={SIZE_PRESETS.map((s, i) => ({ value: i, label: s.label }))}
          style={{ width: 180 }}
        />
      </Space>

      <Input.TextArea
        id="image-gen-panel-input-textarea-23"
        value={prompt}
        onChange={(e) => setPrompt(e.target.value)}
        placeholder={t("imageGen.promptPlaceholder")}
        rows={3}
      />

      <Input
        id="image-gen-panel-input-24"
        value={negativePrompt}
        onChange={(e) => setNegativePrompt(e.target.value)}
        placeholder={t("imageGen.negativePrompt")}
      />

      {provider === "flux" && (
        <div>
          <Typography.Text type="secondary">
            {t("imageGen.inferenceSteps")}: {steps}
          </Typography.Text>
          <Slider min={1} max={50} value={steps} onChange={setSteps} />
        </div>
      )}

      <Button
        type="primary"
        onClick={handleGenerate}
        loading={loading}
        block
        icon={<Sparkles size={14} />}
      >
        {t("imageGen.generateImage")}
      </Button>

      {result && (
        <div>
          <Typography.Text type="secondary">
            {t("imageGen.model")}: {result.model_used} | {t("imageGen.elapsed")}
            : {(result.elapsed_ms / 1000).toFixed(1)}s
          </Typography.Text>
          <div
            style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 8 }}
          >
            {result.images.map((img, i) => (
              <Image
                key={img.url || (img.base64 ? img.base64.slice(0, 20) : `img-${i}`)}
                src={img.base64 ? `data:image/png;base64,${img.base64}` : img.url}
                width={256}
                style={{ borderRadius: 8 }}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
