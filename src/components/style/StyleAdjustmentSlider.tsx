import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import type { StyleDimensionKey } from "@/types/style";

interface StyleAdjustmentSliderProps {
  dimension: StyleDimensionKey;
  value: number;
  onChange: (dimension: StyleDimensionKey, value: number) => void;
  disabled?: boolean;
}

export default function StyleAdjustmentSlider({
  dimension,
  value,
  onChange,
  disabled = false,
}: StyleAdjustmentSliderProps) {
  const { t } = useTranslation();
  const [localValue, setLocalValue] = useState(value);

  const handleChange = useCallback(
    (newValue: number) => {
      setLocalValue(newValue);
      onChange(dimension, newValue);
    },
    [dimension, onChange]
  );

  const dimensionConfig: Record<
    StyleDimensionKey,
    { lowLabel: string; highLabel: string; description: string }
  > = {
    naming_score: {
      lowLabel: t("style.labels.snakeCase"),
      highLabel: t("style.labels.camelCase"),
      description: t("style.descriptions.naming"),
    },
    density_score: {
      lowLabel: t("style.labels.compact"),
      highLabel: t("style.labels.spacious"),
      description: t("style.descriptions.density"),
    },
    comment_ratio: {
      lowLabel: t("style.labels.minimal"),
      highLabel: t("style.labels.detailed"),
      description: t("style.descriptions.commentRatio"),
    },
    abstraction_level: {
      lowLabel: t("style.labels.concrete"),
      highLabel: t("style.labels.abstract"),
      description: t("style.descriptions.abstraction"),
    },
    formality_score: {
      lowLabel: t("style.labels.casual"),
      highLabel: t("style.labels.formal"),
      description: t("style.descriptions.formality"),
    },
    structure_score: {
      lowLabel: t("style.labels.simple"),
      highLabel: t("style.labels.structured"),
      description: t("style.descriptions.structure"),
    },
    technical_depth: {
      lowLabel: t("style.labels.basic"),
      highLabel: t("style.labels.advanced"),
      description: t("style.descriptions.technicalDepth"),
    },
    explanation_length: {
      lowLabel: t("style.labels.brief"),
      highLabel: t("style.labels.comprehensive"),
      description: t("style.descriptions.explanationLength"),
    },
  };

  const config = dimensionConfig[dimension];

  const getPresetButtons = () => {
    return [
      { label: t("style.presets.minimal"), value: 0.2 },
      { label: t("style.presets.neutral"), value: 0.5 },
      { label: t("style.presets.maximal"), value: 0.8 },
    ];
  };

  const handlePresetClick = (presetValue: number) => {
    handleChange(presetValue);
  };

  const handleReset = () => {
    handleChange(0.5);
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <label className="text-sm font-medium">{config.description}</label>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">
            {Math.round(localValue * 100)}%
          </span>
          <button
            onClick={handleReset}
            disabled={disabled}
            className="p-1 text-muted-foreground hover:text-foreground disabled:opacity-50 transition-colors"
            title={t("style.reset")}
          >
            <svg
              className="w-3.5 h-3.5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              strokeWidth={2}
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
              />
            </svg>
          </button>
        </div>
      </div>

      <div className="relative">
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={localValue}
          onChange={(e) => handleChange(parseFloat(e.target.value))}
          disabled={disabled}
          className="w-full h-2 bg-muted rounded-full appearance-none cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed
            [&::-webkit-slider-thumb]:appearance-none
            [&::-webkit-slider-thumb]:w-4
            [&::-webkit-slider-thumb]:h-4
            [&::-webkit-slider-thumb]:rounded-full
            [&::-webkit-slider-thumb]:bg-primary
            [&::-webkit-slider-thumb]:cursor-pointer
            [&::-webkit-slider-thumb]:transition-transform
            [&::-webkit-slider-thumb]:duration-150
            [&::-webkit-slider-thumb]:hover:scale-110
            [&::-moz-range-thumb]:w-4
            [&::-moz-range-thumb]:h-4
            [&::-moz-range-thumb]:rounded-full
            [&::-moz-range-thumb]:bg-primary
            [&::-moz-range-thumb]:border-0
            [&::-moz-range-thumb]:cursor-pointer"
        />

        <div className="flex justify-between mt-1">
          <span className="text-[10px] text-muted-foreground">{config.lowLabel}</span>
          <span className="text-[10px] text-muted-foreground">{config.highLabel}</span>
        </div>
      </div>

      <div className="flex items-center gap-1">
        {getPresetButtons().map((preset) => (
          <button
            key={preset.value}
            onClick={() => handlePresetClick(preset.value)}
            disabled={disabled}
            className={`flex-1 py-1 px-2 text-[10px] rounded transition-colors ${
              Math.abs(localValue - preset.value) < 0.05
                ? "bg-primary text-primary-foreground"
                : "bg-muted hover:bg-muted/80 text-muted-foreground disabled:opacity-50"
            }`}
          >
            {preset.label}
          </button>
        ))}
      </div>
    </div>
  );
}

interface StyleAdjustmentPanelProps {
  dimensions: Record<StyleDimensionKey, number>;
  onDimensionChange: (dimension: StyleDimensionKey, value: number) => void;
  disabled?: boolean;
}

export function StyleAdjustmentPanel({
  dimensions,
  onDimensionChange,
  disabled = false,
}: StyleAdjustmentPanelProps) {
  const { t } = useTranslation();

  const dimensionOrder: StyleDimensionKey[] = [
    "naming_score",
    "density_score",
    "comment_ratio",
    "abstraction_level",
    "formality_score",
    "structure_score",
    "technical_depth",
    "explanation_length",
  ];

  return (
    <div className="border border-border rounded-lg bg-background/50 overflow-hidden">
      <div className="px-3 py-2 border-b border-border/50 flex items-center justify-between">
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
              d="M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4"
            />
          </svg>
          <span className="text-sm font-medium">{t("style.adjustments")}</span>
        </div>
        <span className="text-xs text-muted-foreground">
          {t("style.dimensionsCount", { count: dimensionOrder.length })}
        </span>
      </div>

      <div className="p-3 space-y-4">
        {dimensionOrder.map((dim) => (
          <StyleAdjustmentSlider
            key={dim}
            dimension={dim}
            value={dimensions[dim]}
            onChange={onDimensionChange}
            disabled={disabled}
          />
        ))}
      </div>
    </div>
  );
}
