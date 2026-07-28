// SPDX-License-Identifier: AGPL-3.0-only

import { SourceManager } from "@/components/settings/SourceManager";

/**
 * LLM 知识库中心页（/knowledge 与 /llm-wiki 共用）。
 * 去掉冗余 header，让 SourceManager 自身承担标题栏，最大化利用垂直空间。
 */
export function KnowledgeHubPage() {
  return (
    <div className="kb-layout" data-testid="knowledge-hub">
      <SourceManager />
    </div>
  );
}
