import { IconEditor } from "@/components/shared/IconEditor";
import { StylePreviewPanel } from "@/components/style";
import { useStyleStore } from "@/stores/feature/styleStore";
import {
  type AvatarType,
  type CommentStyle,
  type DetailLevel,
  type IndentationStyle,
  type NamingConvention,
  type Tone,
  useUserProfileStore,
} from "@/stores/feature/userProfileStore";
import { Avatar, Divider, Input, Modal, Slider, Tabs, theme, Typography } from "antd";
import { User } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

interface UserProfileModalProps {
  open: boolean;
  onClose: () => void;
}

function SettingsSelect({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (val: string) => void;
  options: { label: string; value: string }[];
}) {
  const { token } = theme.useToken();
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      style={{
        padding: "4px 8px",
        borderRadius: token.borderRadius,
        border: `1px solid ${token.colorBorder}`,
        backgroundColor: token.colorBgContainer,
        color: token.colorText,
        fontSize: 13,
        cursor: "pointer",
      }}
    >
      {options.map((opt) => <option key={opt.value} value={opt.value}>{opt.label}</option>)}
    </select>
  );
}

export function UserProfileModal({ open, onClose }: UserProfileModalProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const profile = useUserProfileStore((s) => s.profile);
  const updateProfile = useUserProfileStore((s) => s.updateProfile);
  const {
    trajectoryProfile,
    loadTrajectoryProfile,
    updateCodingStyle,
    updateCommunicationPrefs,
  } = useUserProfileStore();
  const { currentProfile, loadStyleProfile, getStats } = useStyleStore();

  const [name, setName] = useState(profile.name);
  const [avatarType, setAvatarType] = useState<AvatarType>(profile.avatarType);
  const [avatarValue, setAvatarValue] = useState(profile.avatarValue);
  const [stats, setStats] = useState<{ total_profiles: number; total_samples: number } | null>(null);
  const [activeTab, setActiveTab] = useState("profile");

  useEffect(() => {
    if (open) {
      setName(profile.name);
      setAvatarType(profile.avatarType);
      setAvatarValue(profile.avatarValue);
      loadTrajectoryProfile();
      loadStyleProfile("default");
      getStats().then((s) => setStats(s));
    }
  }, [open, profile.name, profile.avatarType, profile.avatarValue, loadTrajectoryProfile, loadStyleProfile, getStats]);

  const handleSave = () => {
    updateProfile({ name: name.trim(), avatarType, avatarValue });
    onClose();
  };

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

  const codingStyleContent = trajectoryProfile && (
    <div style={{ padding: "8px 0" }}>
      <Typography.Text style={{ fontSize: 13, color: token.colorTextSecondary }}>
        {t("profile.codingStyleDesc")}
      </Typography.Text>
      <Divider style={{ margin: "8px 0" }} />
      <div style={rowStyle} className="flex items-center justify-between">
        <span>{t("profile.namingConvention")}</span>
        <SettingsSelect
          value={trajectoryProfile.codingStyle?.namingConvention || "snake_case"}
          onChange={(val) => updateCodingStyle({ namingConvention: val as NamingConvention })}
          options={namingOptions}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div style={rowStyle} className="flex items-center justify-between">
        <span>{t("profile.indentationStyle")}</span>
        <SettingsSelect
          value={trajectoryProfile.codingStyle?.indentationStyle || "spaces"}
          onChange={(val) => updateCodingStyle({ indentationStyle: val as IndentationStyle })}
          options={indentationOptions}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div style={rowStyle} className="flex items-center justify-between">
        <span>{t("profile.commentStyle")}</span>
        <SettingsSelect
          value={trajectoryProfile.codingStyle?.commentStyle || "documented"}
          onChange={(val) => updateCodingStyle({ commentStyle: val as CommentStyle })}
          options={commentOptions}
        />
      </div>
      <Divider style={{ margin: "12px 0" }} />
      <Typography.Text style={{ fontSize: 13, color: token.colorTextSecondary }}>
        {t("profile.communicationDesc")}
      </Typography.Text>
      <Divider style={{ margin: "8px 0" }} />
      <div style={rowStyle} className="flex items-center justify-between">
        <span>{t("profile.detailLevel")}</span>
        <SettingsSelect
          value={trajectoryProfile.communication?.detailLevel || "moderate"}
          onChange={(val) => updateCommunicationPrefs({ detailLevel: val as DetailLevel })}
          options={detailOptions}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div style={rowStyle} className="flex items-center justify-between">
        <span>{t("profile.tone")}</span>
        <SettingsSelect
          value={trajectoryProfile.communication?.tone || "neutral"}
          onChange={(val) => updateCommunicationPrefs({ tone: val as Tone })}
          options={toneOptions}
        />
      </div>
      <Divider style={{ margin: "4px 0" }} />
      <div style={rowStyle} className="flex items-center justify-between">
        <span>{t("profile.language")}</span>
        <Input
          value={trajectoryProfile.communication?.language || "en"}
          onChange={(e) => updateCommunicationPrefs({ language: e.target.value })}
          style={{ width: 150 }}
          size="small"
        />
      </div>
    </div>
  );

  const confidenceContent = currentProfile && (
    <div style={{ padding: "8px 0" }}>
      <Typography.Text style={{ fontSize: 13, color: token.colorTextSecondary }}>
        {t("profile.confidenceDesc")}
      </Typography.Text>
      <div style={{ padding: "8px 0" }}>
        <Slider
          min={0}
          max={100}
          value={Math.round(currentProfile.confidence * 100)}
          tooltip={{ formatter: (val) => `${val}%` }}
          disabled
        />
      </div>
    </div>
  );

  return (
    <Modal
      open={open}
      onCancel={onClose}
      mask={{ enabled: true, blur: true }}
      onOk={handleSave}
      okText={t("common.ok")}
      cancelText={t("common.cancel")}
      title={t("userProfile.title")}
      width={680}
      destroyOnHidden
    >
      <div style={{ minHeight: 400 }}>
        <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 16, padding: "16px 0" }}>
          <IconEditor
            iconType={avatarType}
            iconValue={avatarValue}
            onChange={(type, value) => {
              setAvatarType((type as AvatarType) ?? "icon");
              setAvatarValue(value ?? "");
            }}
            size={64}
            defaultIcon={
              <Avatar
                size={64}
                icon={<User size={16} />}
                style={{ cursor: "pointer", backgroundColor: token.colorPrimary }}
              />
            }
            showClear={false}
          />
          <Input
            placeholder={t("userProfile.namePlaceholder")}
            value={name}
            onChange={(e) => setName(e.target.value)}
            style={{ maxWidth: 280 }}
          />
        </div>

        <Tabs
          activeKey={activeTab}
          onChange={setActiveTab}
          size="small"
          items={[
            { key: "profile", label: t("userProfile.profileTab"), children: <div style={{ padding: "8px 0" }} /> },
            { key: "coding", label: t("profile.codingStyle"), children: codingStyleContent },
            { key: "confidence", label: t("profile.confidence"), children: confidenceContent },
            ...(stats
              ? [{
                key: "stats",
                label: t("style.stats"),
                children: (
                  <div style={{ padding: "8px 0" }}>
                    <div style={rowStyle} className="flex items-center justify-between">
                      <span>{t("style.totalProfiles")}</span>
                      <span style={{ fontSize: 18, fontWeight: 600 }}>{stats.total_profiles}</span>
                    </div>
                    <Divider style={{ margin: "4px 0" }} />
                    <div style={rowStyle} className="flex items-center justify-between">
                      <span>{t("style.totalSamples")}</span>
                      <span style={{ fontSize: 18, fontWeight: 600 }}>{stats.total_samples}</span>
                    </div>
                  </div>
                ),
              }]
              : []),
            {
              key: "preview",
              label: t("style.preview"),
              children: (
                <StylePreviewPanel code={`function example() {\n  return "Hello, World!";\n}`} language="typescript" />
              ),
            },
          ]}
          style={{ marginTop: 8 }}
        />
      </div>
    </Modal>
  );
}
