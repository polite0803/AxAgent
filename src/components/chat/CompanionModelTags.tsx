// SPDX-License-Identifier: AGPL-3.0-only

import { ModelIcon } from "@lobehub/icons";
import { theme } from "antd";
import { Trash2, X } from "lucide-react";
import { useTranslation } from "react-i18next";

interface CompanionModelInfo {
  providerId: string;
  modelId: string;
  modelName: string;
  providerName: string;
}

interface CompanionModelTagsProps {
  /** 伴生模型展示信息（含模型名与提供商名） */
  infos: CompanionModelInfo[];
  /** 移除单个伴生模型 */
  onRemove: (index: number) => void;
  /** 清空全部伴生模型 */
  onClearAll: () => void;
}

/**
 * 多模型伴生标签条：展示当前选中的伴生模型，支持单个移除与一键清空。
 */
export function CompanionModelTags({
  infos,
  onRemove,
  onClearAll,
}: CompanionModelTagsProps) {
  const { t } = useTranslation();
  const { token } = theme.useToken();

  return (
    <div className="flex flex-wrap gap-1.5 px-3 pt-3 pb-1">
      <span
        className="inline-flex items-center px-2 py-0.5 text-xs"
        style={{ color: token.colorTextTertiary }}
      >
        {t("chat.multiModel.selectTitle")}:
      </span>
      {infos.map((cm, idx) => (
        <span
          key={`${cm.providerId}-${cm.modelId}`}
          className="inline-flex items-center gap-1.5 pl-1.5 pr-1 py-0.5 text-xs"
          style={{
            backgroundColor: token.colorFillSecondary,
            borderRadius: token.borderRadiusSM,
            color: token.colorText,
          }}
        >
          <ModelIcon model={cm.modelId} size={14} type="avatar" />
          <span
            style={{
              maxWidth: 120,
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {cm.modelName}
          </span>
          {cm.providerName && (
            <span style={{ color: token.colorTextQuaternary, fontSize: 12 }}>
              {cm.providerName}
            </span>
          )}
          <X
            size={12}
            className="cursor-pointer shrink-0"
            style={{ color: token.colorTextTertiary }}
            onClick={() => onRemove(idx)}
          />
        </span>
      ))}
      {/* Clear all companion models */}
      <span
        className="inline-flex items-center gap-1 px-1.5 py-0.5 text-xs cursor-pointer"
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            onClearAll();
          }
        }}
        style={{
          borderRadius: token.borderRadiusSM,
          color: token.colorTextTertiary,
        }}
        onClick={onClearAll}
      >
        <Trash2 size={11} />
        {t("chat.clearAll")}
      </span>
    </div>
  );
}
