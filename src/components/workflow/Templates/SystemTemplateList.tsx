// SPDX-License-Identifier: AGPL-3.0-only

import { DropdownMenu } from "@/components/layout/DropdownMenu";
import { invoke } from "@/lib/invoke";
import { Button, Card, Empty, Spin, Tag, theme } from "antd";
import { BrainCircuit, Edit2, Eye, MoreVertical } from "lucide-react";
import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { WorkflowTemplateResponse } from "../types";

interface SystemTemplateListProps {
  /** 打开系统模板（认知编排器等）到工作流编辑器查看/编辑 */
  onOpenEditor: (templateId: string) => void;
}

/**
 * 系统模板列表（认知编排器等）。
 *
 * 与业务模板列表（TemplateList）物理分离：通过 `include_system=true` 从后端
 * 拉取系统模板，供工作流编辑器查看/编辑认知编排器 DAG。系统模板受后端保护，
 * 禁止删除/复制/导出，故此处不提供对应操作。
 */
export const SystemTemplateList: React.FC<SystemTemplateListProps> = ({
  onOpenEditor,
}) => {
  const { t } = useTranslation("translation");
  const { token } = theme.useToken();
  const [templates, setTemplates] = useState<WorkflowTemplateResponse[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    // 系统模板页专用命令：后端权威过滤（is_preset + cognitive_router），
    // 不依赖 include_system 参数传递。
    invoke<WorkflowTemplateResponse[]>("list_system_templates")
      .then((list) => {
        if (!cancelled) {
          setTemplates(Array.isArray(list) ? list : []);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setTemplates([]);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setIsLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleOpen = (templateId: string) => {
    // 打开系统模板（WorkflowEditor 全屏模式会以 include_system=true 自行加载）
    onOpenEditor(templateId);
  };

  if (isLoading) {
    return (
      <div
        style={{
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          height: 200,
        }}
      >
        <Spin size="large" />
      </div>
    );
  }

  return (
    <div style={{ padding: 16 }}>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))",
          gap: 12,
        }}
      >
        {templates.map((template) => (
          <Card
            key={template.id}
            size="small"
            hoverable
            onClick={() => handleOpen(template.id)}
            style={{
              background: token.colorBgContainer,
              border: `1px solid ${token.colorBorderSecondary}`,
              cursor: "pointer",
              transition: "box-shadow 0.2s, transform 0.2s",
            }}
            styles={{ body: { padding: 12 } }}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "flex-start",
              }}
            >
              <div style={{ flex: 1, minWidth: 0 }}>
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 8,
                    marginBottom: 6,
                  }}
                >
                  <span style={{ fontSize: 16 }} aria-hidden="true">
                    {template.icon || "🧠"}
                  </span>
                  <span
                    style={{
                      fontWeight: 500,
                      color: token.colorText,
                      fontSize: 14,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                      display: "inline-block",
                      maxWidth: "100%",
                    }}
                  >
                    {template.name}
                  </span>
                  <Tag color="purple" style={{ marginLeft: 4, fontSize: 12 }}>
                    {t("workflow.systemTemplateList.systemTag")}
                  </Tag>
                </div>
                <div
                  style={{
                    color: token.colorTextSecondary,
                    fontSize: 12,
                    marginBottom: 8,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {template.description
                    || t("workflow.systemTemplateList.noDescription")}
                </div>
                <div style={{ display: "flex", gap: 4, flexWrap: "wrap" }}>
                  {template.tags?.slice(0, 4).map((tag) => (
                    <Tag key={tag} style={{ fontSize: 12, margin: 0 }}>
                      {tag}
                    </Tag>
                  ))}
                </div>
              </div>
              <DropdownMenu
                items={[
                  {
                    key: "view",
                    icon: <Eye size={14} />,
                    label: t("workflow.systemTemplateList.view"),
                    onClick: () => handleOpen(template.id),
                  },
                  {
                    key: "edit",
                    icon: <Edit2 size={14} />,
                    label: t("workflow.systemTemplateList.edit"),
                    onClick: () => handleOpen(template.id),
                  },
                ]}
                trigger={["click"]}
              >
                <Button
                  type="text"
                  size="small"
                  icon={<MoreVertical size={14} />}
                  onClick={(e) => e.stopPropagation()}
                  aria-label={t("workflow.systemTemplateList.moreActions")}
                  aria-haspopup="menu"
                  style={{ color: token.colorTextTertiary }}
                />
              </DropdownMenu>
            </div>
          </Card>
        ))}
      </div>

      {templates.length === 0 && (
        <Empty
          description={t("workflow.systemTemplateList.noTemplates")}
          style={{ marginTop: 48 }}
        />
      )}

      <div
        style={{
          marginTop: 16,
          color: token.colorTextTertiary,
          fontSize: 12,
          display: "flex",
          alignItems: "center",
          gap: 6,
        }}
      >
        <BrainCircuit size={14} />
        {t("workflow.systemTemplateList.hint")}
      </div>
    </div>
  );
};
