// SPDX-License-Identifier: AGPL-3.0-only

import { BookOpen } from "lucide-react";
import { type BaseRetrievalNodeData, createRetrievalNode } from "./BaseRetrievalNode";

export const KnowledgeRetrievalNode = createRetrievalNode({
  i18nPrefix: "chat.knowledgeRetrieval",
  Icon: BookOpen,
});

export type { BaseRetrievalNodeData as KnowledgeRetrievalNodeData };
