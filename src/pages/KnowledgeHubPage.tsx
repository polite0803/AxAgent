// SPDX-License-Identifier: AGPL-3.0-only

import { Tooltip } from "@/components/layout/Tooltip";
import { SourceManager } from "@/components/settings/SourceManager";
import { Button, theme } from "antd";
import { GitGraph } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

/**
 * LLM 知识库中心页（/knowledge 与 /llm-wiki 共用）。
 * 去掉冗余 header，让 SourceManager 自身承担标题栏，最大化利用垂直空间。
 *
 * 右上角浮动「学习图」入口：将原 /learning-graph 隐藏路由暴露给用户，
 * 作为知识源的衍生可视化能力（技能/记忆/洞察/实体关系图）。
 */
export function KnowledgeHubPage() {
  const { t } = useTranslation();
  const { token } = theme.useToken();
  const navigate = useNavigate();

  return (
    <div className="kb-layout" data-testid="knowledge-hub" style={{ position: "relative" }}>
      <SourceManager />
      <Tooltip title={t("nav.learningGraph")} placement="left">
        <Button
          type="default"
          shape="circle"
          icon={<GitGraph size={16} />}
          onClick={() => navigate("/learning-graph")}
          aria-label={t("nav.learningGraph")}
          style={{
            position: "absolute",
            top: 8,
            right: 12,
            zIndex: 10,
            backgroundColor: token.colorBgElevated,
            boxShadow: token.boxShadowSecondary,
            borderColor: token.colorBorderSecondary,
          }}
        />
      </Tooltip>
    </div>
  );
}
