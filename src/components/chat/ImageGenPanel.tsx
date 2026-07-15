// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import { message } from "@/lib/toast";
import { Button, Image, Input, Select, Slider, Space, Typography } from "antd";
import { Download, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface ImageGenConfig {
  default_provider: string;
  flux_api_token: string;
  openai_api_key: string;
  openai_base_url: string;
  default_width: number;
  default_height: number;
  default_steps: number;
  save_to_artifact: boolean;
}

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

interface CreateArtifactInput {
  conversationId: string;
  kind: string;
  title: string;
  content: string;
  format: string;
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

const QUALITY_OPTIONS = [
  { value: "standard", label: "Standard" },
  { value: "hd", label: "HD" },
];

interface ImageGenPanelProps {
  conversationId: string;
  apiKey?: string;
  defaultProvider?: string;
  onImageGenerated?: (images: GeneratedImage[]) => void;
}

/** 将 base64 或 URL 图片下载到本地 */
function downloadImage(img: GeneratedImage, index: number) {
  const link = document.createElement("a");
  if (img.base64) {
    link.href = `data:image/png;base64,${img.base64}`;
    link.download = `axagent-image-${index}.png`;
  } else if (img.url) {
    link.href = img.url;
    link.download = `axagent-image-${index}.png`;
  } else {
    return;
  }
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

/** 将生成结果保存为 Artifact */
async function saveToArtifact(
  conversationId: string,
  prompt: string,
  result: ImageGenResult,
  t: (key: string) => string,
) {
  const imageMd = result.images
    .map((img, i) => {
      const caption = `${prompt} (${i + 1}/${result.images.length})`;
      if (img.base64) {
        return `![${caption}](data:image/png;base64,${img.base64})`;
      }
      if (img.url) {
        return `![${caption}](${img.url})`;
      }
      return "";
    })
    .filter(Boolean)
    .join("\n\n");

  const content = [
    `${t("imageGen.generatedImageTitle")}: ${prompt}`,
    "",
    `${t("imageGen.model")}: ${result.model_used}`,
    `${t("imageGen.duration")}: ${(result.elapsed_ms / 1000).toFixed(1)}s`,
    "",
    imageMd,
  ].join("\n");

  const input: CreateArtifactInput = {
    conversationId,
    kind: "draft",
    title: `🎨 ${prompt.slice(0, 60)}${prompt.length > 60 ? "…" : ""}`,
    content,
    format: "markdown",
  };

  try {
    await invoke("create_artifact", input as unknown as Record<string, unknown>);
    message.success(t("imageGen.savedToArtifact"));
  } catch {
    // 静默失败，不阻断主流程
  }
}

export function ImageGenPanel({
  conversationId,
  apiKey: propApiKey,
  defaultProvider = "flux",
  onImageGenerated,
}: ImageGenPanelProps) {
  const { t } = useTranslation();
  const [prompt, setPrompt] = useState("");
  const [negativePrompt, setNegativePrompt] = useState("");
  const [provider, setProvider] = useState(defaultProvider);
  const [sizePreset, setSizePreset] = useState(0);
  const [steps, setSteps] = useState(4);
  const [quality, setQuality] = useState("standard");
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ImageGenResult | null>(null);
  const [storedApiKey, setStoredApiKey] = useState<string | null>(null);
  const [saveToArtifactSetting, setSaveToArtifactSetting] = useState(true);

  // 当未传入 apiKey prop 时，尝试从配置自动加载
  useEffect(() => {
    if (!propApiKey) {
      invoke<ImageGenConfig>("get_image_gen_config")
        .then((config) => {
          setSaveToArtifactSetting(config.save_to_artifact);
          const key = provider === "flux" || config.default_provider === "flux"
            ? config.flux_api_token
            : config.openai_api_key;
          if (key) { setStoredApiKey(key); }
        })
        .catch(() => {
          // 忽略，让 UI 显示 "请配置 API Key"
        });
    }
  }, [propApiKey, provider]);

  const effectiveApiKey = propApiKey || storedApiKey;

  const handleGenerate = async () => {
    if (!prompt.trim()) {
      message.warning(t("imageGen.enterPrompt"));
      return;
    }

    if (!effectiveApiKey) {
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
        quality: provider === "dall-e" ? quality : undefined,
        provider,
        apiKey: effectiveApiKey,
      });

      setResult(res);
      onImageGenerated?.(res.images);

      // save_to_artifact: 自动保存生成结果到 Artifact
      if (saveToArtifactSetting && conversationId) {
        await saveToArtifact(conversationId, prompt, res, t);
      }
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
      {provider === "dall-e" && (
        <Select
          value={quality}
          onChange={setQuality}
          options={QUALITY_OPTIONS}
          style={{ width: 180 }}
        />
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
              <div key={img.url || (img.base64 ? img.base64.slice(0, 20) : `img-${i}`)}>
                <Image
                  src={img.base64 ? `data:image/png;base64,${img.base64}` : img.url}
                  width={256}
                  style={{ borderRadius: 8 }}
                />
                <Button
                  type="text"
                  size="small"
                  icon={<Download size={12} />}
                  onClick={() => downloadImage(img, i)}
                  style={{ width: "100%", marginTop: 4 }}
                >
                  {t("common.download")}
                </Button>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
