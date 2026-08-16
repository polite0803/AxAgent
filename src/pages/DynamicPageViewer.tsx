// SPDX-License-Identifier: AGPL-3.0-only

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import { SchemaIdContext } from "@/components/dynamicUI/SchemaIdContext";
import { RouteGuard } from "@/components/shared/RouteGuard";
import { useDynamicUIStore } from "@/stores";
import type { UISchema } from "@/types";
import { Result, Spin } from "antd";
import { useEffect, useReducer } from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router-dom";

function parseSchema(json: string): UISchema | null {
  try {
    return JSON.parse(json) as UISchema;
  } catch {
    return null;
  }
}

export function DynamicPageViewer() {
  const { schemaId } = useParams<{ schemaId: string }>();
  const { t } = useTranslation();
  const getSchema = useDynamicUIStore((s) => s.getSchema);

  type ViewState =
    | { kind: "loading" }
    | { kind: "error"; message: string }
    | { kind: "ready"; schema: UISchema };

  type ViewAction =
    | { type: "start_load" }
    | { type: "load_error"; message: string }
    | { type: "load_ok"; schema: UISchema }
    | { type: "reset" };

  function viewReducer(_state: ViewState, action: ViewAction): ViewState {
    switch (action.type) {
      case "start_load":
        return { kind: "loading" };
      case "load_error":
        return { kind: "error", message: action.message };
      case "load_ok":
        return { kind: "ready", schema: action.schema };
      case "reset":
        return { kind: "loading" };
    }
  }

  const [viewState, dispatch] = useReducer(viewReducer, { kind: "loading" });

  useEffect(() => {
    if (!schemaId) {
      return;
    }

    let cancelled = false;
    dispatch({ type: "start_load" });

    getSchema(schemaId)
      .then((record) => {
        if (cancelled) { return; }
        const parsed = parseSchema(record.schemaJson);
        if (!parsed) {
          dispatch({ type: "load_error", message: t("dynamicUIManager.invalidSchema") });
        } else {
          dispatch({ type: "load_ok", schema: parsed });
        }
      })
      .catch(() => {
        if (cancelled) { return; }
        dispatch({ type: "load_error", message: t("dynamicUI.schemaNotFound") });
      });

    return () => {
      cancelled = true;
    };
  }, [schemaId, getSchema, t]);

  if (!schemaId) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Result status="404" title="404" subTitle={t("dynamicUI.schemaNotFound")} />
      </div>
    );
  }

  if (viewState.kind === "loading") {
    return (
      <div className="flex items-center justify-center h-full w-full" style={{ minHeight: 200 }}>
        <Spin size="large" />
      </div>
    );
  }

  const isError = viewState.kind === "error";
  const errorSubTitle = isError ? viewState.message : undefined;
  const readySchema = viewState.kind === "ready" ? viewState.schema : null;

  return (
    <RouteGuard allowed={readySchema !== null} subTitle={errorSubTitle}>
      {readySchema && (
        <div className="p-6" style={{ flex: 1, overflow: "auto" }}>
          <SchemaIdContext.Provider value={{ schemaId }}>
            <DynamicUIRenderer schema={readySchema} />
          </SchemaIdContext.Provider>
        </div>
      )}
    </RouteGuard>
  );
}
