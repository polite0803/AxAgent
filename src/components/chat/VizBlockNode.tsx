// SPDX-License-Identifier: AGPL-3.0-only

import { Alert, Card } from "antd";
import type { NodeComponentProps } from "markstream-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { VizBlockRenderer } from "@/components/viz/VizBlockRenderer";
import type { VizBlock } from "@/lib/vizBlocks";

type VizBlockNodeData = {
  type: "viz-block";
  content?: string;
  attrs?: Record<string, string> | [string, string][];
};

/**
 * 渲染 `<viz-block>` 自定义标签为可视化图表（G15 集成点）。
 *
 * 数据格式：标签内部应为 JSON 字符串，符合 VizBlock schema。
 * 例如：
 * ```html
 * <viz-block data-axagent="1">
 * {"kind":"line","title":"股价走势","data":{"x":["2024-01","2024-02"],"series":[{"name":"收盘价","values":[10,11]}]}}
 * </viz-block>
 * ```
 *
 * 解析失败时显示错误提示，不会让整个消息渲染崩溃。
 */
export function VizBlockNode(props: NodeComponentProps<VizBlockNodeData>) {
  const { t } = useTranslation();
  const { node } = props;

  const parsed = useMemo<{ ok: true; block: VizBlock } | { ok: false; error: string }>(() => {
    const text = (node?.content ?? "").trim();
    if (!text) {
      return { ok: false, error: t("viz.noData") };
    }
    try {
      const json = JSON.parse(text);
      if (!json || typeof json !== "object" || !("kind" in json)) {
        return { ok: false, error: t("viz.invalidBlock") };
      }
      return { ok: true, block: json as VizBlock };
    } catch (e) {
      return {
        ok: false,
        error: `${t("viz.invalidBlock")}: ${e instanceof Error ? e.message : String(e)}`,
      };
    }
  }, [node?.content, t]);

  if (!node) {
    return null;
  }

  if (!parsed.ok) {
    return (
      <Card size="small" style={{ margin: "8px 0" }}>
        <Alert
          type="error"
          showIcon
          message={t("viz.invalidBlock")}
          description={parsed.error}
        />
      </Card>
    );
  }

  return (
    <Card size="small" style={{ margin: "8px 0" }} bodyStyle={{ padding: 12 }}>
      <VizBlockRenderer block={parsed.block} />
    </Card>
  );
}
