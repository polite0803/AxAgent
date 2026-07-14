// SPDX-License-Identifier: AGPL-3.0-only

import { DynamicUIRenderer } from "@/components/dynamicUI/DynamicUIRenderer";
import { useDynamicUIStore } from "@/stores";
import type { DynamicAction, UISchema } from "@/types";
import { Alert, Spin } from "antd";
import { useEffect, useMemo, useRef, useState } from "react";

interface DynamicUIStandaloneProps {
  schemaId: string;
  instanceKey?: string;
  autosave?: boolean;
  autosaveDebounceMs?: number;
  onAction?: (action: DynamicAction) => void;
  onFormSubmit?: (values: Record<string, unknown>) => void;
}

export function DynamicUIStandalone({
  schemaId,
  instanceKey = "default",
  autosave = true,
  autosaveDebounceMs = 500,
  onAction,
  onFormSubmit,
}: DynamicUIStandaloneProps) {
  const { getSchema, loadFormData, saveFormData } = useDynamicUIStore();
  const [schema, setSchema] = useState<UISchema | null>(null);
  const [formData, setFormData] = useState<Record<string, unknown>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const savePendingRef = useRef(false);
  const formDataRef = useRef<Record<string, unknown>>({});
  const schemaIdRef = useRef(schemaId);
  const instanceKeyRef = useRef(instanceKey);

  useEffect(() => {
    formDataRef.current = formData;
  }, [formData]);
  useEffect(() => {
    schemaIdRef.current = schemaId;
  }, [schemaId]);
  useEffect(() => {
    instanceKeyRef.current = instanceKey;
  }, [instanceKey]);

  useEffect(() => {
    let mounted = true;
    void (async () => {
      setLoading(true);
      setError(null);
      try {
        const record = await getSchema(schemaId);
        if (!mounted) {
          return;
        }
        const parsed = JSON.parse(record.schema_json) as UISchema;
        setSchema(parsed);
        const savedData = await loadFormData(schemaId, instanceKey);
        if (!mounted) {
          return;
        }
        if (savedData) {
          setFormData(savedData);
          formDataRef.current = savedData;
        }
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err.message : String(err));
        }
      } finally {
        if (mounted) {
          setLoading(false);
        }
      }
    })();
    return () => {
      mounted = false;
    };
  }, [schemaId, instanceKey, getSchema, loadFormData]);

  const doSave = (data: Record<string, unknown>) => {
    if (debounceRef.current) {
      clearTimeout(debounceRef.current);
    }
    savePendingRef.current = true;
    debounceRef.current = setTimeout(async () => {
      try {
        await saveFormData({
          schema_id: schemaId,
          form_data_json: JSON.stringify(data),
          instance_key: instanceKey,
        });
      } finally {
        savePendingRef.current = false;
      }
    }, autosaveDebounceMs);
  };

  const handleAction = (action: DynamicAction) => {
    if (onAction) {
      onAction(action);
    }

    if (action.type === "store") {
      const config = action.config as Record<string, unknown> | undefined;
      const formValues = (config?.values as Record<string, unknown>)
        ?? (config?.formValues as Record<string, unknown>)
        ?? formDataRef.current;

      // 校验 formValues 是合法对象
      if (typeof formValues !== "object" || formValues === null || Array.isArray(formValues)) {
        console.warn("[DynamicUIStandalone] store action config 中的 values/formValues 必须为普通对象");
        return;
      }

      setFormData(formValues);
      formDataRef.current = formValues;
      if (autosave) {
        doSave(formValues);
      }
      if (onFormSubmit) {
        onFormSubmit(formValues);
      }
    }
  };

  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current);
      }
      if (savePendingRef.current) {
        const latest = formDataRef.current;
        const sid = schemaIdRef.current;
        const ikey = instanceKeyRef.current;
        void saveFormData({
          schema_id: sid,
          form_data_json: JSON.stringify(latest),
          instance_key: ikey,
        });
      }
    };
  }, [saveFormData]);

  const mergedContext = useMemo(() => ({
    ...formData,
    _schemaId: schemaId,
    _instanceKey: instanceKey,
  }), [formData, schemaId, instanceKey]);

  if (loading) {
    return <Spin />;
  }

  if (error || !schema) {
    return <Alert type="error" message={error || "Schema not found"} showIcon />;
  }

  return (
    <DynamicUIRenderer
      schema={schema}
      dataContext={mergedContext}
      onAction={handleAction}
    />
  );
}
