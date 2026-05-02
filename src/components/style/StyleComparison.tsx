import type { StyleDimensionKey, StyleVector } from "@/types/style";
import { useTranslation } from "react-i18next";

interface StyleComparisonProps {
  sourceStyle: StyleVector;
  targetStyle: StyleVector;
  title?: string;
}

export default function StyleComparison({
  sourceStyle,
  targetStyle,
  title,
}: StyleComparisonProps) {
  const { t } = useTranslation();

  const dimensions: StyleDimensionKey[] = [
    "naming_score",
    "density_score",
    "comment_ratio",
    "abstraction_level",
    "formality_score",
    "structure_score",
    "technical_depth",
    "explanation_length",
  ];

  const dimensionLabels: Record<StyleDimensionKey, string> = {
    naming_score: t("style.dimensions.naming"),
    density_score: t("style.dimensions.density"),
    comment_ratio: t("style.dimensions.commentRatio"),
    abstraction_level: t("style.dimensions.abstraction"),
    formality_score: t("style.dimensions.formality"),
    structure_score: t("style.dimensions.structure"),
    technical_depth: t("style.dimensions.technicalDepth"),
    explanation_length: t("style.dimensions.explanationLength"),
  };

  const getDimensionDescription = (key: StyleDimensionKey, value: number): string => {
    const lowDescriptions: Record<StyleDimensionKey, string> = {
      naming_score: "snake_case",
      density_score: "Compact",
      comment_ratio: "Minimal",
      abstraction_level: "Concrete",
      formality_score: "Casual",
      structure_score: "Simple",
      technical_depth: "Basic",
      explanation_length: "Brief",
    };
    const highDescriptions: Record<StyleDimensionKey, string> = {
      naming_score: "camelCase",
      density_score: "Spacious",
      comment_ratio: "Detailed",
      abstraction_level: "Abstract",
      formality_score: "Formal",
      structure_score: "Structured",
      technical_depth: "Advanced",
      explanation_length: "Comprehensive",
    };

    if (value < 0.35) { return lowDescriptions[key]; }
    if (value > 0.65) { return highDescriptions[key]; }
    return "Neutral";
  };

  const calculateDifference = (source: number, target: number): number => {
    return Math.round((target - source) * 100);
  };

  const getDifferenceColor = (diff: number): string => {
    if (Math.abs(diff) < 5) { return "text-muted-foreground"; }
    return diff > 0 ? "text-green-500" : "text-red-500";
  };

  return (
    <div className="border border-border rounded-lg bg-background/50 overflow-hidden">
      {title && (
        <div className="px-3 py-2 border-b border-border/50 flex items-center gap-2">
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
              d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
            />
          </svg>
          <span className="text-sm font-medium">{title}</span>
        </div>
      )}

      <div className="p-3 space-y-3">
        {dimensions.map((dim) => {
          const sourceValue = sourceStyle.dimensions[dim];
          const targetValue = targetStyle.dimensions[dim];
          const diff = calculateDifference(sourceValue, targetValue);

          return (
            <div key={dim} className="space-y-1">
              <div className="flex items-center justify-between text-xs">
                <span className="font-medium">{dimensionLabels[dim]}</span>
                <div className="flex items-center gap-3">
                  <span className="text-muted-foreground">
                    {getDimensionDescription(dim, sourceValue)}
                  </span>
                  <span className={getDifferenceColor(diff)}>
                    {diff > 0 ? "+" : ""}
                    {diff}%
                  </span>
                  <span className="text-muted-foreground">
                    {getDimensionDescription(dim, targetValue)}
                  </span>
                </div>
              </div>

              <div className="relative h-2 bg-muted rounded-full overflow-hidden">
                <div
                  className="absolute h-full bg-primary/40 rounded-full"
                  style={{ width: `${sourceValue * 100}%` }}
                />
                <div
                  className="absolute h-full bg-primary rounded-full transition-all duration-300"
                  style={{ width: `${targetValue * 100}%`, left: 0 }}
                />
              </div>

              <div className="flex justify-between text-[10px] text-muted-foreground">
                <span>{dim === "naming_score" ? "snake" : "Low"}</span>
                <span>{dim === "naming_score" ? "camel" : "High"}</span>
              </div>
            </div>
          );
        })}
      </div>

      <div className="px-3 py-2 border-t border-border/50 flex items-center justify-between text-xs text-muted-foreground">
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded-full bg-primary/40" />
          <span>{t("style.sourceStyle")}</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-3 rounded-full bg-primary" />
          <span>{t("style.targetStyle")}</span>
        </div>
      </div>
    </div>
  );
}
