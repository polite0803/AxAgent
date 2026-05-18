import { ModelIcon } from "@lobehub/icons";
import { theme, Tooltip } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { useConversationStore } from "@/stores";
import type { Message } from "@/types";

export function ModelTags({
  msg,
  conversationId,
  allVersions,
  getModelDisplayInfo,
}: {
  msg: Message;
  conversationId: string;
  allVersions: Message[];
  getModelDisplayInfo: (
    model_id?: string | null,
    providerId?: string | null,
  ) => { modelName: string; providerName: string };
}) {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const switchMessageVersion = useConversationStore(
    (s) => s.switchMessageVersion,
  );
  const pendingCompanionModels = useConversationStore(
    (s) => s.pendingCompanionModels,
  );
  const multiModelParentId = useConversationStore((s) => s.multiModelParentId);
  const multiModelDoneMessageIds = useConversationStore(
    (s) => s.multiModelDoneMessageIds,
  );

  const isMultiModelTarget = msg.parent_message_id === multiModelParentId;

  const modelGroups = useMemo(() => {
    const groups = new Map<string, Message[]>();
    for (const v of allVersions) {
      const key = v.model_id ?? "__unknown__";
      if (!groups.has(key)) {
        groups.set(key, []);
      }
      groups.get(key)!.push(v);
    }
    return groups;
  }, [allVersions]);

  const pendingModels = useMemo(() => {
    if (!isMultiModelTarget || !pendingCompanionModels.length) {
      return [];
    }
    return pendingCompanionModels.filter((cm) => !modelGroups.has(cm.model_id));
  }, [isMultiModelTarget, pendingCompanionModels, modelGroups]);

  const streamingModelIds = useMemo(() => {
    const ids = new Set<string>();
    if (!isMultiModelTarget) {
      return ids;
    }
    const doneIdSet = new Set(multiModelDoneMessageIds);
    for (const cm of pendingCompanionModels) {
      if (modelGroups.has(cm.model_id)) {
        const versions = modelGroups.get(cm.model_id)!;
        const isDone = versions.some((v) => doneIdSet.has(v.id));
        if (!isDone) {
          ids.add(cm.model_id);
        }
      }
    }
    return ids;
  }, [
    isMultiModelTarget,
    pendingCompanionModels,
    modelGroups,
    multiModelDoneMessageIds,
  ]);

  if (modelGroups.size <= 1 && pendingModels.length === 0) {
    return null;
  }

  const currentModelId = msg.model_id ?? "__unknown__";

  const handleTagClick = (model_id: string) => {
    if (model_id === currentModelId || !msg.parent_message_id) {
      return;
    }
    const versions = modelGroups.get(model_id);
    if (!versions || versions.length === 0) {
      return;
    }
    const sorted = versions.toSorted(
      (a, b) => b.version_index - a.version_index,
    );
    switchMessageVersion(conversationId, msg.parent_message_id, sorted[0].id);
  };

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 6,
        flexWrap: "wrap",
      }}
    >
      {Array.from(modelGroups.keys()).map((model_id) => {
        const isActive = model_id === currentModelId;
        const isStreaming = streamingModelIds.has(model_id);
        const { modelName } = getModelDisplayInfo(
          model_id,
          modelGroups.get(model_id)?.[0]?.provider_id,
        );
        return (
          <Tooltip key={model_id} title={modelName} mouseEnterDelay={0.3}>
            <div
              onClick={() => handleTagClick(model_id)}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleTagClick(model_id);
                }
              }}
              className={isStreaming ? "model-tag-streaming" : undefined}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                width: 26,
                height: 26,
                borderRadius: "50%",
                border: `1.5px solid ${isActive ? token.colorPrimary : "transparent"}`,
                cursor: isActive ? "default" : "pointer",
                transition: "border-color 0.2s",
                flexShrink: 0,
              }}
            >
              <ModelIcon model={model_id} size={20} type="avatar" />
            </div>
          </Tooltip>
        );
      })}
      {pendingModels.map((cm) => {
        const { modelName } = getModelDisplayInfo(cm.model_id, cm.providerId);
        return (
          <Tooltip
            key={`pending-${cm.model_id}`}
            title={`${modelName} (${t("chat.waiting")})`}
            mouseEnterDelay={0.3}
          >
            <div
              className="model-tag-pending"
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                width: 26,
                height: 26,
                borderRadius: "50%",
                border: `1.5px dashed ${token.colorTextQuaternary}`,
                opacity: 0.5,
                flexShrink: 0,
              }}
            >
              <ModelIcon model={cm.model_id} size={20} type="avatar" />
            </div>
          </Tooltip>
        );
      })}
    </div>
  );
}
