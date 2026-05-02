import { type StyleDimensions, useStyleStore } from "@/stores/feature/styleStore";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface StylePreviewPanelProps {
  code: string;
  language?: string;
  onStyleApplied?: (styledCode: string) => void;
}

export default function StylePreviewPanel({
  code,
  language: _language = "typescript",
  onStyleApplied,
}: StylePreviewPanelProps) {
  const { t } = useTranslation();
  const {
    currentProfile,
    appliedStyle,
    isApplying,
    loadStyleProfile,
    applyStyleToCode,
    adjustStyleDimension,
    resetToDefaults,
  } = useStyleStore();

  const [originalCode, setOriginalCode] = useState(code);
  const [styledCode, setStyledCode] = useState(code);
  const [activeTab, setActiveTab] = useState<"preview" | "dimensions">("preview");

  useEffect(() => {
    loadStyleProfile("default");
  }, [loadStyleProfile]);

  useEffect(() => {
    setOriginalCode(code);
  }, [code]);

  const handleApplyStyle = async () => {
    if (!code) { return; }
    const result = await applyStyleToCode(code);
    setStyledCode(result);
    onStyleApplied?.(result);
  };

  const handleDimensionChange = (dimension: keyof StyleDimensions, value: number) => {
    adjustStyleDimension(dimension, value);
  };

  const dimensionLabels: Record<keyof StyleDimensions, string> = {
    naming_score: "Naming Style",
    density_score: "Code Density",
    comment_ratio: "Comment Ratio",
    abstraction_level: "Abstraction Level",
    formality_score: "Formality",
    structure_score: "Structure",
    technical_depth: "Technical Depth",
    explanation_length: "Explanation Length",
  };

  const dimensionDescriptions: Record<keyof StyleDimensions, string> = {
    naming_score: "snake_case ↔ camelCase",
    density_score: "Compact ↔ Spacious",
    comment_ratio: "Minimal ↔ Detailed",
    abstraction_level: "Concrete ↔ Abstract",
    formality_score: "Casual ↔ Formal",
    structure_score: "Simple ↔ Structured",
    technical_depth: "Basic ↔ Advanced",
    explanation_length: "Brief ↔ Comprehensive",
  };

  const currentDimensions = appliedStyle?.dimensions || {
    naming_score: 0.5,
    density_score: 0.5,
    comment_ratio: 0.5,
    abstraction_level: 0.5,
    formality_score: 0.5,
    structure_score: 0.5,
    technical_depth: 0.5,
    explanation_length: 0.5,
  };

  return (
    <div className="border border-border rounded-lg bg-background/50 overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 border-b border-border/50">
        <div className="flex items-center gap-2">
          <svg
            className="w-4 h-4 text-muted-foreground"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            strokeWidth={2}
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9.53 16.122a3 3 0 00-5.78 1.128 2.25 2.25 0 01-2.4 2.245 4.5 4.5 0 008.4-2.245c0-.399-.078-.78-.22-1.128zm0 0a15.998 15.998 0 003.388-1.62m-5.043-.025a15.994 15.994 0 011.622-3.395m3.42 3.42a15.995 15.995 0 004.764-4.648l3.876-5.814a1.151 1.151 0 00-1.597-1.597L14.146 6.32a15.996 15.996 0 00-4.649 4.763m3.42 3.42a6.776 6.776 0 00-3.42-3.42"
            />
          </svg>
          <span className="text-sm font-medium">{t("style.preview")}</span>
          {currentProfile && (
            <span className="text-xs text-muted-foreground">
              ({Math.round(currentProfile.confidence * 100)}% confidence)
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setActiveTab("preview")}
            className={`px-2 py-1 text-xs rounded ${
              activeTab === "preview"
                ? "bg-primary/10 text-primary"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {t("style.preview")}
          </button>
          <button
            onClick={() => setActiveTab("dimensions")}
            className={`px-2 py-1 text-xs rounded ${
              activeTab === "dimensions"
                ? "bg-primary/10 text-primary"
                : "text-muted-foreground hover:text-foreground"
            }`}
          >
            {t("style.dimensions.label")}
          </button>
        </div>
      </div>

      {activeTab === "preview"
        ? (
          <div className="p-3 space-y-3">
            <div className="flex items-center gap-2">
              <button
                onClick={handleApplyStyle}
                disabled={isApplying || !code}
                className="px-3 py-1.5 text-xs bg-primary text-primary-foreground rounded hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isApplying ? t("style.applying") : t("style.applyStyle")}
              </button>
              <button
                onClick={resetToDefaults}
                className="px-3 py-1.5 text-xs text-muted-foreground hover:text-foreground border border-border rounded"
              >
                {t("style.reset")}
              </button>
            </div>

            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1">
                <span className="text-xs text-muted-foreground">{t("style.original")}</span>
                <pre className="text-xs bg-muted/30 rounded p-2 max-h-48 overflow-auto font-mono">
                <code>{originalCode.slice(0, 500)}{originalCode.length > 500 ? "..." : ""}</code>
                </pre>
              </div>
              <div className="space-y-1">
                <span className="text-xs text-muted-foreground">{t("style.styled")}</span>
                <pre className="text-xs bg-primary/5 rounded p-2 max-h-48 overflow-auto font-mono">
                <code>{styledCode.slice(0, 500)}{styledCode.length > 500 ? "..." : ""}</code>
                </pre>
              </div>
            </div>
          </div>
        )
        : (
          <div className="p-3 space-y-3 max-h-64 overflow-auto">
            {(Object.keys(dimensionLabels) as (keyof StyleDimensions)[]).map((dimension) => (
              <div key={dimension} className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-xs font-medium">{dimensionLabels[dimension]}</span>
                  <span className="text-xs text-muted-foreground">
                    {Math.round(currentDimensions[dimension] * 100)}%
                  </span>
                </div>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={currentDimensions[dimension] * 100}
                  onChange={(e) =>
                    handleDimensionChange(dimension, parseInt(e.target.value) / 100)}
                  className="w-full h-1.5 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
                />
                <span className="text-[10px] text-muted-foreground">
                  {dimensionDescriptions[dimension]}
                </span>
              </div>
            ))}
          </div>
        )}
    </div>
  );
}
