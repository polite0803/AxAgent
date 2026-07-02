// SPDX-License-Identifier: AGPL-3.0-only

import { validateSchema } from "@/lib/dynamicUI/SchemaValidator";
import type { SchemaValidationResult, UISchema } from "@/types";
import { CheckCircleOutlined } from "@ant-design/icons";
import { Alert, Badge, Button, Card, Input, Space, Typography } from "antd";
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { DynamicUIRenderer } from "./DynamicUIRenderer";

const { Text } = Typography;
const { TextArea } = Input;

const DEFAULT_SCHEMA: UISchema = {
  version: "1.0",
  id: "root",
  type: "Column",
  props: {},
  children: [
    {
      version: "1.0",
      id: "card-1",
      type: "Card",
      props: { title: "示例卡片", bordered: true },
      children: [
        {
          version: "1.0",
          id: "text-1",
          type: "Text",
          props: { content: "Hello World! 这是一个动态 UI 示例。", type: "secondary" },
        },
      ],
    },
  ],
};

/**
 * Schema 预览/调试工具组件。
 * 左侧：文本编辑器（使用 Ant Design TextArea 作为简易替代，避免强依赖 Monaco）
 * 右侧：DynamicUIRenderer 实时渲染预览
 * 底部：SchemaValidator 校验结果
 */
export const DynamicUIPreview: React.FC = () => {
  const { t } = useTranslation();
  const [schemaText, setSchemaText] = useState(
    JSON.stringify(DEFAULT_SCHEMA, null, 2),
  );
  const [parseError, setParseError] = useState<string | null>(null);

  const { schema, validation } = useMemo((): {
    schema: UISchema | null;
    validation: SchemaValidationResult | null;
  } => {
    try {
      const parsed = JSON.parse(schemaText) as unknown;
      const result = validateSchema(parsed);
      return { schema: parsed as UISchema, validation: result };
    } catch {
      return { schema: null, validation: null };
    }
  }, [schemaText]);

  useEffect(() => {
    const timer = setTimeout(() => {
      if (schema) {
        setParseError(null);
      } else if (schemaText) {
        try {
          JSON.parse(schemaText);
        } catch (err) {
          setParseError(
            err instanceof Error ? err.message : t("dynamicUIPreview.jsonParseError"),
          );
        }
      }
    }, 0);
    return () => clearTimeout(timer);
  }, [schema, schemaText, t]);

  const handleReset = useCallback(() => {
    setSchemaText(JSON.stringify(DEFAULT_SCHEMA, null, 2));
  }, []);

  return (
    <div className="flex flex-col h-full gap-2 p-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <Text strong className="text-lg">
          Dynamic UI Preview
        </Text>
        <Button size="small" onClick={handleReset}>
          {t("dynamicUIPreview.reset")}
        </Button>
      </div>

      {/* Main Area */}
      <div className="flex-1 flex gap-4 min-h-0">
        {/* Left: Editor */}
        <Card
          title="UI Schema JSON"
          size="small"
          className="flex-1 min-w-0"
          styles={{ body: { flex: 1, padding: 0 } }}
        >
          <TextArea
            value={schemaText}
            onChange={(e) => setSchemaText(e.target.value)}
            className="font-mono text-xs w-full h-full resize-none"
            style={{
              minHeight: "400px",
              border: "none",
              borderRadius: 0,
            }}
            spellCheck={false}
          />
        </Card>

        {/* Right: Preview */}
        <Card
          title={t("dynamicUIPreview.livePreview")}
          size="small"
          className="flex-1 min-w-0 overflow-auto"
        >
          {parseError
            ? <Alert type="error" message={t("dynamicUIPreview.jsonParseError")} description={parseError} showIcon />
            : schema
            ? <DynamicUIRenderer schema={schema} />
            : <Alert type="info" message={t("dynamicUIPreview.waitingForJson")} showIcon />}
        </Card>
      </div>

      {/* Bottom: Validation */}
      <Card
        title={
          <Space>
            <span>{t("dynamicUIPreview.schemaValidation")}</span>
            {validation
              ? (
                validation.valid
                  ? (
                    <Badge
                      status="success"
                      text={<Text type="success">{t("dynamicUIPreview.pass")}</Text>}
                    />
                  )
                  : (
                    <Badge
                      status="error"
                      text={
                        <Text type="danger">
                          {t("dynamicUIPreview.errorsCount", { count: validation.errors.length })}
                        </Text>
                      }
                    />
                  )
              )
              : null}
          </Space>
        }
        size="small"
      >
        {parseError ? <Text type="secondary">{t("dynamicUIPreview.parseErrorCantValidate")}</Text> : validation
          ? (
            validation.valid
              ? (
                <Text type="success">
                  <CheckCircleOutlined className="mr-1" />
                  {t("dynamicUIPreview.schemaAllValid")}
                </Text>
              )
              : (
                <ul className="list-disc pl-4 m-0">
                  {validation.errors.map((err, i) => (
                    <li key={`${err.path}-${i}`} className="text-red-600 dark:text-red-400 text-sm">
                      <Text code>{err.path}</Text>: {err.message}
                    </li>
                  ))}
                </ul>
              )
          )
          : null}
      </Card>
    </div>
  );
};
