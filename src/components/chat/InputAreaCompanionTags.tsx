// SPDX-License-Identifier: AGPL-3.0-only

import { ModelIcon } from "@lobehub/icons";
import type { GlobalToken } from "antd";
import { Trash2, X } from "lucide-react";

interface CompanionDisplayInfo {
  providerId: string;
  model_id: string;
  modelName: string;
  providerName: string;
}

export function InputAreaCompanionTags(props: {
  currentMode: string;
  companionModels: Array<{ providerId: string; model_id: string }>;
  companionDisplayInfos: CompanionDisplayInfo[];
  removeCompanionModel: (index: number) => void;
  clearAllCompanionModels: () => void;
  token: GlobalToken;
  t: (key: string) => string;
}) {
  if (props.currentMode === "agent" || props.companionModels.length === 0) {
    return null;
  }

  const { companionDisplayInfos, removeCompanionModel, clearAllCompanionModels, token, t } = props;

  return (
    <div className="flex flex-wrap gap-1.5 px-3 pt-3 pb-1">
      <span
        className="inline-flex items-center px-2 py-0.5 text-xs"
        style={{ color: token.colorTextTertiary }}
      >
        {t("chat.multiModel.selectTitle")}:
      </span>
      {companionDisplayInfos.map((cm, idx) => (
        <span
          key={`${cm.providerId}-${cm.model_id}`}
          className="inline-flex items-center gap-1.5 pl-1.5 pr-1 py-0.5 text-xs"
          style={{
            backgroundColor: token.colorFillSecondary,
            borderRadius: token.borderRadiusSM,
            color: token.colorText,
          }}
        >
          <ModelIcon model={cm.model_id} size={14} type="avatar" />
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
            <span
              style={{ color: token.colorTextQuaternary, fontSize: 12 }}
            >
              {cm.providerName}
            </span>
          )}
          <X
            size={12}
            className="cursor-pointer shrink-0"
            style={{ color: token.colorTextTertiary }}
            onClick={() => removeCompanionModel(idx)}
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
            clearAllCompanionModels();
          }
        }}
        style={{
          borderRadius: token.borderRadiusSM,
          color: token.colorTextTertiary,
        }}
        onClick={clearAllCompanionModels}
      >
        <Trash2 size={11} />
        {t("chat.clearAll")}
      </span>
    </div>
  );
}
