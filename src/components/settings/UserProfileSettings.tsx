import { useUserProfileStore } from "@/stores/feature/userProfileStore";
import { useStyleStore } from "@/stores/feature/styleStore";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { StylePreviewPanel } from "@/components/style";

export function UserProfileSettings() {
  const { t } = useTranslation();
  const {
    trajectoryProfile,
    isLoading,
    loadTrajectoryProfile,
    updateCodingStyle,
    updateCommunicationPrefs,
  } = useUserProfileStore();

  const {
    currentProfile,
    loadStyleProfile,
    getStats,
  } = useStyleStore();

  const [stats, setStats] = useState<{ total_profiles: number; total_samples: number } | null>(null);

  useEffect(() => {
    loadTrajectoryProfile();
    loadStyleProfile("default");
    getStats().then((s) => setStats(s));
  }, [loadTrajectoryProfile, loadStyleProfile, getStats]);

  if (isLoading && !trajectoryProfile) {
    return (
      <div className="p-6 flex items-center justify-center min-h-50">
        <div className="text-muted-foreground">{t("profile.loading")}</div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6 max-w-4xl">
      {/* Profile Overview */}
      <section className="space-y-3">
        <h3 className="text-sm font-semibold">{t("profile.overview")}</h3>
        <div className="grid grid-cols-2 gap-4">
          <div className="border border-border rounded-lg p-4 bg-muted/20">
            <div className="text-xs text-muted-foreground mb-1">{t("profile.userId")}</div>
            <div className="text-sm font-mono">{trajectoryProfile?.userId || "default"}</div>
          </div>
          <div className="border border-border rounded-lg p-4 bg-muted/20">
            <div className="text-xs text-muted-foreground mb-1">{t("profile.lastUpdated")}</div>
            <div className="text-sm">
              {trajectoryProfile?.updatedAt
                ? new Date(trajectoryProfile.updatedAt).toLocaleDateString()
                : "—"}
            </div>
          </div>
        </div>
      </section>

      {/* Coding Style */}
      <section className="space-y-3">
        <h3 className="text-sm font-semibold">{t("profile.codingStyle")}</h3>
        <div className="grid grid-cols-3 gap-3">
          <div className="border border-border rounded-lg p-3">
            <div className="text-xs text-muted-foreground mb-1">{t("profile.namingConvention")}</div>
            <select
              value={trajectoryProfile?.codingStyle?.namingConvention || "snake_case"}
              onChange={(e) => updateCodingStyle({ namingConvention: e.target.value as any })}
              className="w-full text-sm bg-transparent border border-border rounded px-2 py-1"
            >
              <option value="snake_case">snake_case</option>
              <option value="camelCase">camelCase</option>
              <option value="PascalCase">PascalCase</option>
              <option value="kebab-case">kebab-case</option>
            </select>
          </div>
          <div className="border border-border rounded-lg p-3">
            <div className="text-xs text-muted-foreground mb-1">{t("profile.indentationStyle")}</div>
            <select
              value={trajectoryProfile?.codingStyle?.indentationStyle || "spaces"}
              onChange={(e) => updateCodingStyle({ indentationStyle: e.target.value as any })}
              className="w-full text-sm bg-transparent border border-border rounded px-2 py-1"
            >
              <option value="spaces">Spaces</option>
              <option value="tabs">Tabs</option>
            </select>
          </div>
          <div className="border border-border rounded-lg p-3">
            <div className="text-xs text-muted-foreground mb-1">{t("profile.commentStyle")}</div>
            <select
              value={trajectoryProfile?.codingStyle?.commentStyle || "documented"}
              onChange={(e) => updateCodingStyle({ commentStyle: e.target.value as any })}
              className="w-full text-sm bg-transparent border border-border rounded px-2 py-1"
            >
              <option value="minimal">Minimal</option>
              <option value="documented">Documented</option>
              <option value="verbose">Verbose</option>
            </select>
          </div>
        </div>
      </section>

      {/* Communication Preferences */}
      <section className="space-y-3">
        <h3 className="text-sm font-semibold">{t("profile.communication")}</h3>
        <div className="grid grid-cols-3 gap-3">
          <div className="border border-border rounded-lg p-3">
            <div className="text-xs text-muted-foreground mb-1">{t("profile.detailLevel")}</div>
            <select
              value={trajectoryProfile?.communication?.detailLevel || "moderate"}
              onChange={(e) => updateCommunicationPrefs({ detailLevel: e.target.value as any })}
              className="w-full text-sm bg-transparent border border-border rounded px-2 py-1"
            >
              <option value="concise">Concise</option>
              <option value="moderate">Moderate</option>
              <option value="detailed">Detailed</option>
            </select>
          </div>
          <div className="border border-border rounded-lg p-3">
            <div className="text-xs text-muted-foreground mb-1">{t("profile.tone")}</div>
            <select
              value={trajectoryProfile?.communication?.tone || "neutral"}
              onChange={(e) => updateCommunicationPrefs({ tone: e.target.value as any })}
              className="w-full text-sm bg-transparent border border-border rounded px-2 py-1"
            >
              <option value="formal">Formal</option>
              <option value="neutral">Neutral</option>
              <option value="casual">Casual</option>
            </select>
          </div>
          <div className="border border-border rounded-lg p-3">
            <div className="text-xs text-muted-foreground mb-1">{t("profile.language")}</div>
            <input
              type="text"
              value={trajectoryProfile?.communication?.language || "en"}
              onChange={(e) => updateCommunicationPrefs({ language: e.target.value })}
              className="w-full text-sm bg-transparent border border-border rounded px-2 py-1"
            />
          </div>
        </div>
      </section>

      {/* Style Stats */}
      {stats && (
        <section className="space-y-3">
          <h3 className="text-sm font-semibold">{t("style.stats")}</h3>
          <div className="grid grid-cols-2 gap-4">
            <div className="border border-border rounded-lg p-4 bg-muted/20">
              <div className="text-xs text-muted-foreground mb-1">{t("style.totalProfiles")}</div>
              <div className="text-2xl font-bold">{stats.total_profiles}</div>
            </div>
            <div className="border border-border rounded-lg p-4 bg-muted/20">
              <div className="text-xs text-muted-foreground mb-1">{t("style.totalSamples")}</div>
              <div className="text-2xl font-bold">{stats.total_samples}</div>
            </div>
          </div>
        </section>
      )}

      {/* Style Preview */}
      <section className="space-y-3">
        <h3 className="text-sm font-semibold">{t("style.preview")}</h3>
        <StylePreviewPanel
          code={`function example() {\n  return "Hello, World!";\n}`}
          language="typescript"
        />
      </section>

      {/* Profile Confidence */}
      {currentProfile && (
        <section className="space-y-3">
          <h3 className="text-sm font-semibold">{t("profile.confidence")}</h3>
          <div className="w-full bg-muted rounded-full h-2">
            <div
              className="bg-primary h-2 rounded-full transition-all"
              style={{ width: `${Math.round(currentProfile.confidence * 100)}%` }}
            />
          </div>
          <div className="text-xs text-muted-foreground text-right">
            {Math.round(currentProfile.confidence * 100)}%
          </div>
        </section>
      )}
    </div>
  );
}
