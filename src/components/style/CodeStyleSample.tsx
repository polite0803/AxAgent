import type { CodeStyleTemplate, LearnedPattern } from "@/types/style";
import { useState } from "react";
import { useTranslation } from "react-i18next";

interface CodeStyleSampleProps {
  templates: CodeStyleTemplate[];
  patterns: LearnedPattern[];
  language?: string;
  maxDisplayed?: number;
}

export default function CodeStyleSample({
  templates,
  patterns,
  language = "typescript",
  maxDisplayed = 3,
}: CodeStyleSampleProps) {
  const { t } = useTranslation();
  const [expandedTemplate, setExpandedTemplate] = useState<number | null>(null);
  const [activeTab, setActiveTab] = useState<"templates" | "patterns">("templates");

  const getLanguageLabel = (lang: string): string => {
    const labels: Record<string, string> = {
      typescript: "TypeScript",
      javascript: "JavaScript",
      python: "Python",
      rust: "Rust",
      go: "Go",
      java: "Java",
      cpp: "C++",
      csharp: "C#",
      unknown: "Unknown",
    };
    return labels[lang] || lang;
  };

  const getPatternTypeColor = (patternType: string): string => {
    const colors: Record<string, string> = {
      Naming: "bg-blue-500/20 text-blue-500",
      Formatting: "bg-green-500/20 text-green-500",
      Structure: "bg-purple-500/20 text-purple-500",
      Comment: "bg-orange-500/20 text-orange-500",
      Document: "bg-cyan-500/20 text-cyan-500",
    };
    return colors[patternType] || "bg-muted text-muted-foreground";
  };

  const displayedTemplates = templates.slice(0, maxDisplayed);
  const displayedPatterns = patterns.slice(0, maxDisplayed);

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
              d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
            />
          </svg>
          <span className="text-sm font-medium">{t("style.codeSamples")}</span>
          <span className="text-xs text-muted-foreground">
            ({getLanguageLabel(language)})
          </span>
        </div>

        <div className="flex items-center gap-1">
          <button
            onClick={() => setActiveTab("templates")}
            className={`px-2 py-1 text-xs rounded transition-colors ${
              activeTab === "templates"
                ? "bg-primary text-primary-foreground"
                : "hover:bg-muted"
            }`}
          >
            {t("style.templates")}
          </button>
          <button
            onClick={() => setActiveTab("patterns")}
            className={`px-2 py-1 text-xs rounded transition-colors ${
              activeTab === "patterns"
                ? "bg-primary text-primary-foreground"
                : "hover:bg-muted"
            }`}
          >
            {t("style.patterns")} ({patterns.length})
          </button>
        </div>
      </div>

      <div className="p-3">
        {activeTab === "templates" && (
          <div className="space-y-2">
            {displayedTemplates.length === 0
              ? (
                <p className="text-sm text-muted-foreground text-center py-4">
                  {t("style.noTemplates")}
                </p>
              )
              : (
                displayedTemplates.map((template, index) => (
                  <div
                    key={index}
                    className="border border-border rounded-md overflow-hidden"
                  >
                    <button
                      onClick={() =>
                        setExpandedTemplate(
                          expandedTemplate === index ? null : index,
                        )}
                      className="w-full px-3 py-2 flex items-center justify-between hover:bg-muted/50 transition-colors"
                    >
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium">{template.name}</span>
                        <span className="text-xs text-muted-foreground">
                          {template.templates.length} {t("style.variants")}
                        </span>
                      </div>
                      <svg
                        className={`w-4 h-4 text-muted-foreground transition-transform ${
                          expandedTemplate === index ? "rotate-180" : ""
                        }`}
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke="currentColor"
                        strokeWidth={2}
                      >
                        <path
                          strokeLinecap="round"
                          strokeLinejoin="round"
                          d="M19 9l-7 7-7-7"
                        />
                      </svg>
                    </button>

                    {expandedTemplate === index && (
                      <div className="border-t border-border p-3 bg-muted/30">
                        {template.templates.map((variant, vIndex) => (
                          <div key={vIndex} className="mb-3 last:mb-0">
                            <div className="text-xs font-medium text-muted-foreground mb-1">
                              {variant.name}
                            </div>
                            <pre className="text-xs bg-background p-2 rounded border border-border overflow-x-auto">
                            <code>{variant.template}</code>
                            </pre>
                            {variant.description && (
                              <div className="text-xs text-muted-foreground mt-1">
                                {variant.description}
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                ))
              )}

            {templates.length > maxDisplayed && (
              <button className="w-full py-2 text-xs text-muted-foreground hover:text-foreground transition-colors">
                {t("style.showMoreTemplates", { count: templates.length - maxDisplayed })}
              </button>
            )}
          </div>
        )}

        {activeTab === "patterns" && (
          <div className="space-y-2">
            {displayedPatterns.length === 0
              ? (
                <p className="text-sm text-muted-foreground text-center py-4">
                  {t("style.noPatterns")}
                </p>
              )
              : (
                displayedPatterns.map((pattern) => (
                  <div
                    key={pattern.id}
                    className="flex items-start gap-3 p-2 rounded-md hover:bg-muted/50 transition-colors"
                  >
                    <span
                      className={`px-1.5 py-0.5 text-[10px] font-medium rounded ${
                        getPatternTypeColor(
                          pattern.pattern_type,
                        )
                      }`}
                    >
                      {pattern.pattern_type}
                    </span>

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 text-xs">
                        <code className="text-foreground truncate">
                          {pattern.original}
                        </code>
                        <svg
                          className="w-3 h-3 text-muted-foreground shrink-0"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                          strokeWidth={2}
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            d="M17 8l4 4m0 0l-4 4m4-4H3"
                          />
                        </svg>
                        <code className="text-muted-foreground truncate">
                          {pattern.transformed}
                        </code>
                      </div>

                      <div className="flex items-center gap-2 mt-1">
                        <span className="text-[10px] text-muted-foreground">
                          {pattern.context}
                        </span>
                        <span className="text-[10px] text-muted-foreground">
                          •
                        </span>
                        <span className="text-[10px] text-muted-foreground">
                          {t("style.usedCount", { count: pattern.usage_count })}
                        </span>
                      </div>
                    </div>
                  </div>
                ))
              )}

            {patterns.length > maxDisplayed && (
              <button className="w-full py-2 text-xs text-muted-foreground hover:text-foreground transition-colors">
                {t("style.showMorePatterns", { count: patterns.length - maxDisplayed })}
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
