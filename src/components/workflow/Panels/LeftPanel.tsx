import { useWorkflowEditorStore } from "@/stores";
import { Input, Tabs, Tag, theme } from "antd";
import { FileText, Search } from "lucide-react";
import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { type DragPayload, setDragPayload } from "../dndState";
import { NODE_CATEGORIES, NODE_TYPE_MAP } from "../types";

export const LeftPanel: React.FC = () => {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const [search, setSearch] = useState("");
  const [templateSearch, setTemplateSearch] = useState("");
  const { templates, loadTemplate } = useWorkflowEditorStore();
  const dragRef = useRef<DragPayload | null>(null);
  const ghostRef = useRef<HTMLDivElement | null>(null);
  const isDraggingRef = useRef(false);

  useEffect(() => () => {
    if (ghostRef.current) {
      ghostRef.current.remove();
      ghostRef.current = null;
    }
    isDraggingRef.current = false;
    dragRef.current = null;
  }, []);

  const handleMouseDown = useCallback(
    (event: React.MouseEvent, nodeType: string, nodeLabel: string) => {
      if (event.button !== 0) {
        return;
      }

      event.preventDefault();

      const payload: DragPayload = { type: nodeType, label: nodeLabel };
      dragRef.current = payload;
      setDragPayload(payload);
      isDraggingRef.current = true;

      const ghost = document.createElement("div");
      ghost.textContent = nodeLabel;
      ghost.style.position = "fixed";
      ghost.style.pointerEvents = "none";
      ghost.style.zIndex = "99999";
      ghost.style.padding = "6px 12px";
      ghost.style.background = token.colorBorderSecondary;
      ghost.style.color = token.colorText;
      ghost.style.borderRadius = "4px";
      ghost.style.fontSize = "12px";
      ghost.style.whiteSpace = "nowrap";
      ghost.style.opacity = "0.85";
      ghost.style.left = `${event.clientX + 12}px`;
      ghost.style.top = `${event.clientY + 12}px`;
      document.body.appendChild(ghost);
      ghostRef.current = ghost;

      const handleMouseMove = (e: MouseEvent) => {
        if (ghostRef.current) {
          ghostRef.current.style.left = `${e.clientX + 12}px`;
          ghostRef.current.style.top = `${e.clientY + 12}px`;
        }
      };

      const handleMouseUp = () => {
        isDraggingRef.current = false;
        window.removeEventListener("mousemove", handleMouseMove);
        window.removeEventListener("mouseup", handleMouseUp);
        dragRef.current = null;
        if (ghostRef.current) {
          ghostRef.current.remove();
          ghostRef.current = null;
        }
      };

      window.addEventListener("mousemove", handleMouseMove);
      window.addEventListener("mouseup", handleMouseUp);
    },
    [token],
  );

  const filteredNodeTypes = Object.entries(NODE_TYPE_MAP).filter(
    ([type, info]) =>
      t(info.labelKey).toLowerCase().includes(search.toLowerCase())
      && !type.startsWith("_")
      && !t(info.labelKey).includes(t("workflow.leftPanel.legacySuffix")),
  );

  const groupedNodeTypes = NODE_CATEGORIES.flatMap((category) => {
    const items = filteredNodeTypes.filter(
      ([_, info]) => info.category === category.id,
    );
    return items.length > 0 ? [{ ...category, items }] : [];
  });

  const handleTemplateClick = (templateId: string) => {
    loadTemplate(templateId);
  };

  const filteredTemplates = templates.filter((template) => {
    if (!templateSearch.trim()) {
      return true;
    }
    const q = templateSearch.toLowerCase();
    return (
      template.name.toLowerCase().includes(q)
      || (template.description
        && template.description.toLowerCase().includes(q))
      || template.tags.some((tag) => tag.toLowerCase().includes(q))
    );
  });

  return (
    <div
      style={{
        width: 280,
        background: token.colorBgContainer,
        borderRight: `1px solid ${token.colorBorderSecondary}`,
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
      }}
    >
      <Tabs
        defaultActiveKey="nodes"
        size="small"
        style={{ height: "100%" }}
        items={[
          {
            key: "nodes",
            label: t("workflow.leftPanel.nodesTab"),
            children: (
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  height: "100%",
                }}
              >
                <Input
                  id="left-panel-input-74"
                  prefix={<Search size={14} style={{ color: token.colorTextTertiary }} />}
                  placeholder={t("workflow.leftPanel.searchNodes")}
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  style={{ margin: "8px", width: "auto" }}
                  size="small"
                />

                <div style={{ flex: 1, overflow: "auto", padding: "0 8px" }}>
                  {groupedNodeTypes.map((category) => (
                    <div key={category.id} style={{ marginBottom: 12 }}>
                      <div
                        style={{
                          fontSize: 12,
                          color: token.colorTextTertiary,
                          textTransform: "uppercase",
                          marginBottom: 6,
                          paddingLeft: 4,
                        }}
                      >
                        {t(category.labelKey)}
                      </div>
                      <div
                        style={{
                          display: "grid",
                          gridTemplateColumns: "1fr 1fr",
                          gap: 6,
                        }}
                      >
                        {category.items.map(([type, info]) => (
                          <div
                            key={type}
                            role="button"
                            tabIndex={0}
                            onMouseDown={(e) => handleMouseDown(e, type, t(info.labelKey))}
                            onKeyDown={(e) => {
                              if (e.key === "Enter" || e.key === " ") {
                                handleMouseDown(
                                  e as unknown as React.MouseEvent,
                                  type,
                                  t(info.labelKey),
                                );
                              }
                            }}
                            style={{
                              padding: "8px 6px",
                              background: token.colorBgElevated,
                              border: `1px solid ${info.color}40`,
                              borderRadius: 6,
                              cursor: "grab",
                              textAlign: "center",
                              fontSize: 12,
                              color: token.colorTextTertiary,
                              transition: "box-shadow 0.2s, transform 0.2s",
                              userSelect: "none",
                            }}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.borderColor = info.color;
                              e.currentTarget.style.background = `${info.color}10`;
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.borderColor = `${info.color}40`;
                              e.currentTarget.style.background = token.colorBgElevated;
                            }}
                          >
                            <div style={{ fontSize: 16, marginBottom: 4 }}>
                              {getNodeIcon(type)}
                            </div>
                            <div
                              style={{
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                              }}
                            >
                              {t(info.labelKey)}
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ),
          },
          {
            key: "templates",
            label: t("workflow.leftPanel.templatesTab"),
            children: (
              <div style={{ padding: "8px" }}>
                <Input
                  id="left-panel-input-75"
                  prefix={<Search size={14} style={{ color: token.colorTextTertiary }} />}
                  placeholder={t("workflow.leftPanel.searchTemplates")}
                  value={templateSearch}
                  onChange={(e) => setTemplateSearch(e.target.value)}
                  style={{ marginBottom: 8 }}
                  size="small"
                />
                <div
                  style={{ overflow: "auto", maxHeight: "calc(100vh - 200px)" }}
                >
                  {filteredTemplates.map((template) => (
                    <div
                      key={template.id}
                      role="button"
                      tabIndex={0}
                      onClick={() => handleTemplateClick(template.id)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          handleTemplateClick(template.id);
                        }
                      }}
                      style={{
                        padding: 10,
                        marginBottom: 6,
                        background: token.colorBgElevated,
                        borderRadius: 6,
                        cursor: "pointer",
                        border: "1px solid transparent",
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.borderColor = `${token.colorPrimary}40`;
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.borderColor = "transparent";
                      }}
                    >
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                        }}
                      >
                        <FileText size={14} style={{ color: token.colorPrimary }} />
                        <span style={{ color: token.colorTextTertiary, fontSize: 12 }}>
                          {template.name}
                        </span>
                        {template.is_preset && (
                          <Tag color="blue" style={{ fontSize: 12, margin: 0 }}>
                            {t("workflow.preset")}
                          </Tag>
                        )}
                      </div>
                      {template.description && (
                        <div
                          style={{
                            color: token.colorTextTertiary,
                            fontSize: 12,
                            marginTop: 4,
                            marginLeft: 22,
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                        >
                          {template.description}
                        </div>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            ),
          },
        ]}
      />
    </div>
  );
};

function getNodeIcon(type: string): string {
  const icons: Record<string, string> = {
    trigger: "⚡",
    agent: "🤖",
    llm: "🧠",
    condition: "❓",
    parallel: "⏩",
    loop: "🔄",
    merge: "🔗",
    delay: "⏱",
    atomicSkill: "⚛️",
    tool: "🔧",
    code: "💻",
    subWorkflow: "📦",
    httpRequest: "🌐",
    documentParser: "📄",
    vectorRetrieve: "🔍",
    end: "🏁",
    validation: "✓",
  };
  return icons[type] || "📦";
}
