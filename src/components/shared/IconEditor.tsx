import { useResolvedAvatarSrc } from "@/hooks/useResolvedAvatarSrc";
import { invoke, isTauri } from "@/lib/invoke";
import type { AvatarType } from "@/stores";
import { Avatar, Button, Dropdown, Input, theme } from "antd";
import type { MenuProps } from "antd";
import { FileImage, Grid2x2, Link, Smile, Trash2 } from "lucide-react";
import { lazy, type ReactNode, Suspense, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { AvatarEditBadge } from "./AvatarEditBadge";
import { DynamicLobeIcon } from "./DynamicLobeIcon";
import { EmojiPicker } from "./EmojiPicker";

const IconPickerModal = lazy(() => import("@/components/settings/IconPickerModal"));

export interface IconEditorProps {
  /** Current icon type: 'emoji' | 'url' | 'file' | 'model_icon' | or custom */
  iconType?: string | null;
  /** Current icon value */
  iconValue?: string | null;
  /** Called when icon changes */
  onChange: (iconType: string | null, iconValue: string | null) => void;
  /** Display size in px */
  size?: number;
  /** Default icon when nothing is set */
  defaultIcon?: ReactNode;
  /** Extra menu items prepended before the standard ones */
  prependMenuItems?: MenuProps["items"];
  /** Extra menu items appended after the standard ones (e.g. "reset to default") */
  extraMenuItems?: MenuProps["items"];
  /** Whether to show the "clear" option when an icon is set */
  showClear?: boolean;
  /** Shape of the avatar: 'circle' | 'square' */
  shape?: "circle" | "square";
  /** Show "model/provider icons" option (opens IconPickerModal) */
  showModelIcons?: boolean;
  /** Default tab when opening IconPickerModal */
  modelIconsDefaultTab?: "model" | "provider";
}

/**
 * Shared icon editor with AvatarEditBadge + Dropdown.
 * Supports emoji, URL, file upload, and optionally model/provider icon library.
 */
export function IconEditor({
  iconType,
  iconValue,
  onChange,
  size = 32,
  defaultIcon,
  prependMenuItems,
  extraMenuItems,
  showClear = true,
  shape = "circle",
  showModelIcons = false,
  modelIconsDefaultTab = "model",
}: IconEditorProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [showEmojiPicker, setShowEmojiPicker] = useState(false);
  const [showUrlInput, setShowUrlInput] = useState(false);
  const [urlInput, setUrlInput] = useState("");
  const [showIconPicker, setShowIconPicker] = useState(false);
  const resolvedSrc = useResolvedAvatarSrc((iconType as AvatarType) ?? "icon", iconValue ?? "");

  const handleFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file || !file.type.startsWith("image/")) { return; }
    const reader = new FileReader();
    reader.onload = async () => {
      const dataUri = reader.result as string;
      const match = dataUri.match(/^data:([^;]+);base64,(.+)$/s);
      if (match && isTauri()) {
        try {
          const relativePath = await invoke<string>("save_avatar_file", { data: match[2], mimeType: match[1] });
          onChange("file", relativePath);
        } catch {
          onChange("file", dataUri);
        }
      } else {
        onChange("file", dataUri);
      }
    };
    reader.readAsDataURL(file);
    e.target.value = "";
  };

  // Build menu items
  const menuItems: MenuProps["items"] = [
    ...(prependMenuItems ?? []),
    ...(showModelIcons
      ? [{
        key: "model_icon",
        icon: <Grid2x2 size={14} />,
        label: t("settings.chooseIcon"),
        onClick: () => setShowIconPicker(true),
      }]
      : []),
    {
      key: "emoji",
      icon: <Smile size={14} />,
      label: t("userProfile.emoji"),
      onClick: () => {
        setShowEmojiPicker(true);
        setShowUrlInput(false);
      },
    },
    {
      key: "url",
      icon: <Link size={14} />,
      label: t("userProfile.imageUrl"),
      onClick: () => {
        setShowUrlInput(true);
        setShowEmojiPicker(false);
        setUrlInput(iconType === "url" ? (iconValue ?? "") : "");
      },
    },
    {
      key: "file",
      icon: <FileImage size={14} />,
      label: t("userProfile.selectImage"),
      onClick: () => fileInputRef.current?.click(),
    },
    ...(showClear && iconType
      ? [
        { type: "divider" as const },
        {
          key: "clear",
          icon: <Trash2 size={14} />,
          label: t("settings.memory.clearIcon"),
          danger: true as const,
          onClick: () => onChange(null, null),
        },
      ]
      : []),
    ...(extraMenuItems ?? []),
  ];

  // Parse model_icon value format: "group:iconId"
  const parseModelIcon = (value: string) => {
    const idx = value.indexOf(":");
    return idx > 0 ? value.substring(idx + 1) : value;
  };

  // Render current icon
  const renderIcon = () => {
    if (iconType === "emoji" && iconValue) {
      return (
        <div
          style={{
            width: size,
            height: size,
            borderRadius: shape === "circle" ? "50%" : token.borderRadius,
            backgroundColor: token.colorFillSecondary,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: size * 0.55,
            lineHeight: 1,
            cursor: "pointer",
          }}
        >
          {iconValue}
        </div>
      );
    }
    if ((iconType === "url" || iconType === "file") && iconValue) {
      const src = iconType === "file" ? (resolvedSrc ?? iconValue) : iconValue;
      return <Avatar size={size} src={src} shape={shape} style={{ cursor: "pointer" }} />;
    }
    if (iconType === "model_icon" && iconValue) {
      return (
        <div style={{ cursor: "pointer" }}>
          <DynamicLobeIcon iconId={parseModelIcon(iconValue)} size={size} type="avatar" />
        </div>
      );
    }
    if (defaultIcon) {
      return <div style={{ cursor: "pointer" }}>{defaultIcon}</div>;
    }
    return (
      <Avatar
        size={size}
        shape={shape}
        style={{ cursor: "pointer", backgroundColor: token.colorFillSecondary, color: token.colorTextSecondary }}
      />
    );
  };

  return (
    <>
      <Dropdown menu={{ items: menuItems }} trigger={["click"]} placement="bottomLeft">
        <div style={{ cursor: "pointer", display: "inline-flex" }}>
          <AvatarEditBadge size={size}>
            {renderIcon()}
          </AvatarEditBadge>
        </div>
      </Dropdown>
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        style={{ display: "none" }}
        onChange={handleFileSelect}
      />
      <EmojiPicker
        open={showEmojiPicker}
        onClose={() => setShowEmojiPicker(false)}
        onEmojiSelect={(emoji) => {
          onChange("emoji", emoji);
          setShowEmojiPicker(false);
        }}
      />
      {showModelIcons && (
        <Suspense>
          <IconPickerModal
            open={showIconPicker}
            onClose={() => setShowIconPicker(false)}
            defaultTab={modelIconsDefaultTab}
            onSelect={(iconId, group) => {
              onChange("model_icon", `${group}:${iconId}`);
            }}
          />
        </Suspense>
      )}
      {showUrlInput && (
        <div className="flex items-center gap-2" style={{ marginTop: 8 }}>
          <Input
            placeholder={t("settings.memory.iconUrlPlaceholder")}
            value={urlInput}
            onChange={(e) => setUrlInput(e.target.value)}
            autoFocus
            onPressEnter={() => {
              if (urlInput.trim()) {
                onChange("url", urlInput.trim());
              }
              setShowUrlInput(false);
            }}
            style={{ width: 260 }}
          />
          <Button
            size="small"
            onClick={() => {
              if (urlInput.trim()) {
                onChange("url", urlInput.trim());
              }
              setShowUrlInput(false);
            }}
          >
            {t("common.ok")}
          </Button>
          <Button size="small" onClick={() => setShowUrlInput(false)}>{t("common.cancel")}</Button>
        </div>
      )}
    </>
  );
}
