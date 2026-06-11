// SPDX-License-Identifier: AGPL-3.0-only

import { Descriptions, Tag, Typography } from "antd";
import React from "react";
import { useTranslation } from "react-i18next";
import type { ToolDependency } from "../../types";

const { Text, Paragraph } = Typography;

interface ToolGenerationPreviewProps {
  dependency: ToolDependency;
}

export const ToolGenerationPreview: React.FC<ToolGenerationPreviewProps> = ({
  dependency,
}) => {
  const { t } = useTranslation();
  return (
    <div style={{ padding: "12px 0" }}>
      <Descriptions
        size="small"
        column={1}
        bordered
        items={[
          {
            key: "name",
            label: t("decomposition.toolName"),
            children: (
              <Text code>
                generated_{dependency.name.replace(/[^a-zA-Z0-9]/g, "_")}
              </Text>
            ),
          },
          {
            key: "original",
            label: t("decomposition.originalName"),
            children: dependency.name,
          },
          {
            key: "type",
            label: t("decomposition.implementation"),
            children: <Tag color="blue">{t("decomposition.promptTemplate")}</Tag>,
          },
          {
            key: "description",
            label: t("decomposition.description"),
            children: (
              <Paragraph
                type="secondary"
                style={{ fontSize: 12, marginBottom: 0 }}
              >
                {t("decomposition.promptTemplateDesc")}
              </Paragraph>
            ),
          },
        ]}
      />
    </div>
  );
};
