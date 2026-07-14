// SPDX-License-Identifier: AGPL-3.0-only

import { Library } from "lucide-react";
import { type BaseRetrievalNodeData, createRetrievalNode } from "./BaseRetrievalNode";

export const WikiRetrievalNode = createRetrievalNode({
  i18nPrefix: "chat.wikiRetrieval",
  Icon: Library,
});

export type { BaseRetrievalNodeData as WikiRetrievalNodeData };
