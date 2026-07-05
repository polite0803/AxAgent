// SPDX-License-Identifier: AGPL-3.0-only

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import { useDynamicUIStore } from "@/stores";
import type { DynamicAction, UISchema } from "@/types";
import { Result, Spin } from "antd";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";

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
  const navigate = useNavigate();
  const getSchema = useDynamicUIStore((s) => s.getSchema);

  const [schema, setSchema] = useState<UISchema | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!schemaId) {
      setError(t("dynamicUI.schemaNotFound"));
      setLoading(false);
      return;
    }

    let cancelled = false;
    setLoading(true);
    setError(null);

    getSchema(schemaId)
      .then((record) => {
        if (cancelled) { return; }
        const parsed = parseSchema(record.schema_json);
        if (!parsed) {
          setError(t("dynamicUIManager.invalidSchema"));
        } else {
          setSchema(parsed);
        }
        setLoading(false);
      })
      .catch(() => {
        if (cancelled) { return; }
        setError(t("dynamicUI.schemaNotFound"));
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [schemaId, getSchema, t]);

  const handleAction = useCallback(
    (action: DynamicAction) => {
      switch (action.type) {
        case "navigate": {
          const path = action.config?.path as string | undefined;
          if (path) { navigate(path); }
          break;
        }
        // Other action types are handled internally by DynamicUIRenderer
        // via its EventHandlerEngine; we forward all actions here so the
        // renderer's internal schema-update CustomEvent mechanism still
        // receives them via onAction → executeActions pipeline.
        default:
          break;
      }
    },
    [navigate],
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full w-full" style={{ minHeight: 200 }}>
        <Spin size="large" />
      </div>
    );
  }

  if (error) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Result status="404" title="404" subTitle={error} />
      </div>
    );
  }

  if (!schema) {
    return (
      <div style={{ padding: 48, textAlign: "center" }}>
        <Result status="404" title="404" subTitle={t("dynamicUI.schemaNotFound")} />
      </div>
    );
  }

  return (
    <div className="p-6" style={{ flex: 1, overflow: "auto" }}>
      <DynamicUIRenderer schema={schema} onAction={handleAction} />
    </div>
  );
}
