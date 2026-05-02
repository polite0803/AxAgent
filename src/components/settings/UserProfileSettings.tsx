import { StylePreviewPanel } from "@/components/style";
import { useStyleStore } from "@/stores/feature/styleStore";
import { useUserProfileStore } from "@/stores/feature/userProfileStore";
import { Divider, Input, Slider, theme, Typography } from "antd";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingsGroup } from "./SettingsGroup";
import { SettingsSelect } from "./SettingsSelect";

const { Text } = Typography;

export function UserProfileSettings() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
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
        <div style={{ color: token.colorTextSecondary }}>{t("profile.loading")}</div>
      </div>
    );
  }

  const rowStyle = { padding: "4px 0" };

  const namingOptions = [
    { label: t("profile.options.snake_case"), value: "snake_case" },
    { label: t("profile.options.camelCase"), value: "camelCase" },
    { label: t("profile.options.PascalCase"), value: "PascalCase" },
    { label: t("profile.options.kebab-case"), value: "kebab-case" },
  ];

  const indentationOptions = [
    { label: t("profile.options.spaces"), value: "spaces" },
    { label: t("profile.options.tabs"), value: "tabs" },
  ];

  const commentOptions = [
    { label: t("profile.options.minimal"), value: "minimal" },
    { label: t("profile.options.documented"), value: "documented" },
    { label: t("profile.options.verbose"), value: "verbose" },
  ];

  const detailOptions = [
    { label: t("profile.options.concise"), value: "concise" },
    { label: t("profile.options.moderate"), value: "moderate" },
    { label: t("profile.options.detailed"), value: "detailed" },
  ];

  const toneOptions = [
    { label: t("profile.options.formal"), value: "formal" },
    { label: t("profile.options.neutral"), value: "neutral" },
    { label: t("profile.options.casual"), value: "casual" },
  ];

  return (
    <div className="p-6 pb-12">
      <div style={{ marginBottom: 20 }}>
        <Text style={{ fontSize: 13, color: token.colorTextSecondary }}>
          {t("profile.description")}
        </Text>
      </div>

      <SettingsGroup title={t("profile.codingStyle")}>
        <div style={{ padding: "0 4px 8px" }}>
          <Text style={{ fontSize: 12, color: token.colorTextTertiary }}>
            {t("profile.codingStyleDesc")}
          </Text>
        </div>
        <div style={rowStyle} className="flex items-center justify-between">
          <div>
            <div>{t("profile.namingConvention")}</div>
            <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 1 }}>
              {t("profile.namingConventionDesc")}
            </div>
          </div>
          <SettingsSelect
            value={trajectoryProfile?.codingStyle?.namingConvention || "snake_case"}
            onChange={(val) => updateCodingStyle({ namingConvention: val as any })}
            options={namingOptions}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <div>
            <div>{t("profile.indentationStyle")}</div>
            <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 1 }}>
              {t("profile.indentationStyleDesc")}
            </div>
          </div>
          <SettingsSelect
            value={trajectoryProfile?.codingStyle?.indentationStyle || "spaces"}
            onChange={(val) => updateCodingStyle({ indentationStyle: val as any })}
            options={indentationOptions}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <div>
            <div>{t("profile.commentStyle")}</div>
            <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 1 }}>
              {t("profile.commentStyleDesc")}
            </div>
          </div>
          <SettingsSelect
            value={trajectoryProfile?.codingStyle?.commentStyle || "documented"}
            onChange={(val) => updateCodingStyle({ commentStyle: val as any })}
            options={commentOptions}
          />
        </div>
      </SettingsGroup>

      <SettingsGroup title={t("profile.communication")}>
        <div style={{ padding: "0 4px 8px" }}>
          <Text style={{ fontSize: 12, color: token.colorTextTertiary }}>
            {t("profile.communicationDesc")}
          </Text>
        </div>
        <div style={rowStyle} className="flex items-center justify-between">
          <div>
            <div>{t("profile.detailLevel")}</div>
            <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 1 }}>
              {t("profile.detailLevelDesc")}
            </div>
          </div>
          <SettingsSelect
            value={trajectoryProfile?.communication?.detailLevel || "moderate"}
            onChange={(val) => updateCommunicationPrefs({ detailLevel: val as any })}
            options={detailOptions}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <div>
            <div>{t("profile.tone")}</div>
            <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 1 }}>
              {t("profile.toneDesc")}
            </div>
          </div>
          <SettingsSelect
            value={trajectoryProfile?.communication?.tone || "neutral"}
            onChange={(val) => updateCommunicationPrefs({ tone: val as any })}
            options={toneOptions}
          />
        </div>
        <Divider style={{ margin: "4px 0" }} />
        <div style={rowStyle} className="flex items-center justify-between">
          <div>
            <div>{t("profile.language")}</div>
            <div style={{ fontSize: 11, color: token.colorTextTertiary, marginTop: 1 }}>
              {t("profile.languageDesc")}
            </div>
          </div>
          <Input
            value={trajectoryProfile?.communication?.language || "en"}
            onChange={(e) => updateCommunicationPrefs({ language: e.target.value })}
            style={{ width: 150 }}
            size="small"
          />
        </div>
      </SettingsGroup>

      {currentProfile && (
        <SettingsGroup title={t("profile.confidence")}>
          <div style={{ padding: "0 4px 8px" }}>
            <Text style={{ fontSize: 12, color: token.colorTextTertiary }}>
              {t("profile.confidenceDesc")}
            </Text>
          </div>
          <div style={{ padding: "8px 0" }}>
            <Slider
              min={0}
              max={100}
              value={Math.round(currentProfile.confidence * 100)}
              tooltip={{ formatter: (val) => `${val}%` }}
              disabled
            />
          </div>
        </SettingsGroup>
      )}

      {stats && (
        <SettingsGroup title={t("style.stats")}>
          <div style={rowStyle} className="flex items-center justify-between">
            <span>{t("style.totalProfiles")}</span>
            <span style={{ fontSize: 18, fontWeight: 600 }}>{stats.total_profiles}</span>
          </div>
          <Divider style={{ margin: "4px 0" }} />
          <div style={rowStyle} className="flex items-center justify-between">
            <span>{t("style.totalSamples")}</span>
            <span style={{ fontSize: 18, fontWeight: 600 }}>{stats.total_samples}</span>
          </div>
        </SettingsGroup>
      )}

      <SettingsGroup title={t("style.preview")}>
        <StylePreviewPanel
          code={`function example() {\n  return "Hello, World!";\n}`}
          language="typescript"
        />
      </SettingsGroup>
    </div>
  );
}
