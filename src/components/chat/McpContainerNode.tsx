import { Collapse, Tag } from "antd";
import { CheckCircle, Loader, XCircle } from "lucide-react";
import type { NodeComponentProps, RenderContext, RenderNodeFn } from "markstream-react";
import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

function safeGetAttr(attrs: unknown, key: string): string | undefined {
  if (!attrs) {
    return undefined;
  }

  if (Array.isArray(attrs)) {
    for (const item of attrs) {
      if (Array.isArray(item)) {
        const [k, v] = item;
        if (k === key || k === `data-${key}`) {
          if (v == null) {
            return undefined;
          }
          return typeof v === "object" ? JSON.stringify(v) : String(v);
        }
      } else if (item && typeof item === "object" && "name" in item) {
        if (item.name === key || item.name === `data-${key}`) {
          const v = item.value;
          if (v == null) {
            return undefined;
          }
          return typeof v === "object" ? JSON.stringify(v) : String(v);
        }
      }
    }
    return undefined;
  }

  if (typeof attrs === "object" && attrs) {
    const obj = attrs as Record<string, unknown>;
    for (const k of [key, `data-${key}`]) {
      const v = obj[k];
      if (v == null) {
        continue;
      }
      return typeof v === "object" ? JSON.stringify(v) : String(v);
    }
  }

  return undefined;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function extractText(children: any[] | undefined): string {
  if (!children || children.length === 0) {
    return "";
  }
  const parts: string[] = [];
  for (const child of children) {
    if (typeof child === "string") {
      parts.push(child);
    } else if (child?.content != null) {
      parts.push(
        typeof child.content === "object"
          ? JSON.stringify(child.content)
          : String(child.content),
      );
    } else if (child?.children) {
      parts.push(extractText(child.children));
    }
  }
  return parts.join("");
}

/**
 * 单个子节点渲染组件 - 将 renderNode 函数调用包装为组件
 * 修复 react-doctor/no-render-in-render：直接返回 renderNode 结果，而非 JSX 内插值调用
 */
function NodeChild({
  child,
  indexKey,
  index,
  ctx,
  renderNode,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  child: any;
  indexKey: string | undefined;
  index: number;
  ctx: RenderContext;
  renderNode: RenderNodeFn;
}) {
  return renderNode(
    child,
    `${String(indexKey ?? "vmr-container")}-${index}`,
    ctx,
  );
}

/**
 * 渲染非 MCP 容器节点的提取组件
 * 修复 react-doctor/no-render-in-render：将 renderNode 调用移出 McpContainerNode
 */
function DefaultContainer({
  node,
  ctx,
  renderNode,
  indexKey,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  node: any;
  ctx: RenderContext;
  renderNode?: RenderNodeFn | undefined;
  indexKey: string | undefined;
}) {
  return (
    <div className={`vmr-container vmr-container-${node.name ?? "unknown"}`}>
      {Array.isArray(node.children) && ctx && renderNode
        ? node.children.map((child: any, i: number) => ( // eslint-disable-line @typescript-eslint/no-explicit-any
          <NodeChild
            key={child.id
              ?? child.name
              ?? `${String(indexKey ?? "vmr-container")}-${i}`}
            child={child}
            indexKey={indexKey}
            index={i}
            ctx={ctx}
            renderNode={renderNode}
          />
        ))
        : null}
    </div>
  );
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function McpContainerNode(props: NodeComponentProps<any>) {
  const { node } = props;

  if (node.name !== "mcp") {
    const { node: n, ctx, renderNode, indexKey } = props;
    return (
      <DefaultContainer
        node={n}
        ctx={ctx!}
        renderNode={renderNode}
        indexKey={indexKey as string | undefined}
      />
    );
  }

  return <McpToolCard node={node} />;
}

const monoStyle: React.CSSProperties = {
  display: "block",
  fontSize: 12,
  maxHeight: 300,
  overflow: "auto",
  whiteSpace: "pre-wrap",
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
  padding: "4px 0",
};

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function McpToolCard({ node }: { node: any }) {
  const { t } = useTranslation();

  const serverName = safeGetAttr(node.attrs, "name") ?? "MCP";
  const toolName = safeGetAttr(node.attrs, "tool") ?? "unknown";
  const rawArgs = safeGetAttr(node.attrs, "arguments");
  const isLoading = Boolean(node.loading);

  const [activeKey, setActiveKey] = useState<string[]>(isLoading ? ["1"] : []);

  useEffect(() => {
    if (isLoading) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setActiveKey(["1"]);
    } else {
      setActiveKey([]);
    }
  }, [isLoading]);

  const resultText = useMemo(() => {
    if (isLoading) {
      return "";
    }
    return extractText(node.children);
  }, [isLoading, node.children]);

  const isError = useMemo(() => {
    if (isLoading) {
      return false;
    }
    const trimmed = resultText.trim();
    return (
      trimmed.startsWith("Error:")
      || trimmed.startsWith("Error executing tool:")
      || trimmed.startsWith("错误")
    );
  }, [isLoading, resultText]);

  const status = isLoading ? "running" : isError ? "error" : "success";

  const statusIcon = useMemo(() => {
    if (status === "running") {
      return <Loader size={12} className="animate-spin" />;
    }
    if (status === "error") {
      return <XCircle size={12} />;
    }
    return <CheckCircle size={12} />;
  }, [status]);

  const statusColor = status === "running"
    ? "processing"
    : status === "error"
    ? "error"
    : "success";
  const statusLabel = status === "running"
    ? t("chat.tool.running")
    : status === "error"
    ? t("chat.tool.error")
    : t("chat.tool.success");

  const decodedArgs = useMemo(() => {
    if (!rawArgs) {
      return null;
    }
    try {
      const decoded = rawArgs.includes("%")
        ? decodeURIComponent(rawArgs)
        : rawArgs;
      return JSON.stringify(JSON.parse(decoded), null, 2);
    } catch {
      return rawArgs;
    }
  }, [rawArgs]);

  const headerLabel = (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
      <Tag icon={statusIcon} color={statusColor} style={{ margin: 0 }}>
        {statusLabel}
      </Tag>
      <Tag style={{ margin: 0 }}>{serverName}</Tag>
      <span style={{ fontSize: 13 }}>{toolName}</span>
    </span>
  );

  const hasContent = Boolean(decodedArgs) || Boolean(!isLoading && resultText);

  return (
    <span style={{ display: "block", margin: "8px 0" }}>
      <Collapse
        size="small"
        activeKey={activeKey}
        onChange={(keys) => setActiveKey(keys as string[])}
        items={hasContent
          ? [
            {
              key: "1",
              label: headerLabel,
              children: (
                <>
                  {decodedArgs && (
                    <div style={{ marginBottom: resultText ? 8 : 0 }}>
                      <div
                        style={{
                          fontSize: 12,
                          color: "var(--ant-color-text-secondary)",
                          marginBottom: 4,
                        }}
                      >
                        {t("chat.tool.input")}
                      </div>
                      <span style={{ ...monoStyle, maxHeight: 200 }}>
                        {decodedArgs}
                      </span>
                    </div>
                  )}
                  {!isLoading && resultText && (
                    <div>
                      <div
                        style={{
                          fontSize: 12,
                          color: "var(--ant-color-text-secondary)",
                          marginBottom: 4,
                        }}
                      >
                        {t("chat.tool.output")}
                      </div>
                      <span style={monoStyle}>{resultText}</span>
                    </div>
                  )}
                </>
              ),
            },
          ]
          : [
            {
              key: "1",
              label: headerLabel,
              children: null,
            },
          ]}
      />
    </span>
  );
}
