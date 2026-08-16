// SPDX-License-Identifier: AGPL-3.0-only

import { ModelIcon } from "@lobehub/icons";
import { theme } from "antd";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { Tooltip } from "@/components/layout/Tooltip";
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
    modelId?: string | null,
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

  const isMultiModelTarget = msg.parentMessageId === multiModelParentId;

  const modelGroups = useMemo(() => {
    const groups = new Map<string, Message[]>();
    for (const v of allVersions) {
      const key = v.modelId ?? "__unknown__";
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
    return pendingCompanionModels.filter((cm) => !modelGroups.has(cm.modelId));
  }, [isMultiModelTarget, pendingCompanionModels, modelGroups]);

  const streamingModelIds = useMemo(() => {
    const ids = new Set<string>();
    if (!isMultiModelTarget) {
      return ids;
    }
    const doneIdSet = new Set(multiModelDoneMessageIds);
    for (const cm of pendingCompanionModels) {
      if (modelGroups.has(cm.modelId)) {
        const versions = modelGroups.get(cm.modelId) ?? [];
        const isDone = versions.some((v) => doneIdSet.has(v.id));
        if (!isDone) {
          ids.add(cm.modelId);
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

  const currentModelId = msg.modelId ?? "__unknown__";

  const handleTagClick = (modelId: string) => {
    if (modelId === currentModelId || !msg.parentMessageId) {
      return;
    }
    const versions = modelGroups.get(modelId);
    if (!versions || versions.length === 0) {
      return;
    }
    const sorted = versions.toSorted(
      (a, b) => b.versionIndex - a.versionIndex,
    );
    switchMessageVersion(conversationId, msg.parentMessageId, sorted[0].id);
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
      {Array.from(modelGroups.keys()).map((modelId) => {
        const isActive = modelId === currentModelId;
        const isStreaming = streamingModelIds.has(modelId);
        const { modelName } = getModelDisplayInfo(
          modelId,
          modelGroups.get(modelId)?.[0]?.providerId,
        );
        return (
          <Tooltip key={modelId} title={modelName} mouseEnterDelay={0.3}>
            <div
              onClick={() => handleTagClick(modelId)}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  handleTagClick(modelId);
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
              <ModelIcon model={modelId} size={20} type="avatar" />
            </div>
          </Tooltip>
        );
      })}
      {pendingModels.map((cm) => {
        const { modelName } = getModelDisplayInfo(cm.modelId, cm.providerId);
        return (
          <Tooltip
            key={`pending-${cm.modelId}`}
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
              <ModelIcon model={cm.modelId} size={20} type="avatar" />
            </div>
          </Tooltip>
        );
      })}
    </div>
  );
}
