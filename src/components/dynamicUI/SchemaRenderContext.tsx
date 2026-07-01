// SPDX-License-Identifier: AGPL-3.0-only

import type { UISchema } from "@/types";
import { createContext, useContext } from "react";

export interface SchemaRenderContextValue {
  renderSchema: (schema: UISchema, dataContext?: Record<string, unknown>) => React.ReactNode;
}

export const SchemaRenderContext = createContext<SchemaRenderContextValue | null>(null);

export function useSchemaRenderer(): SchemaRenderContextValue {
  const ctx = useContext(SchemaRenderContext);
  if (!ctx) {
    throw new Error("useSchemaRenderer must be used within DynamicUIRenderer tree");
  }
  return ctx;
}
